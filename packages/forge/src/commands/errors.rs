//! Command error parsing and categorization
//!
//! Parses stderr output from failed commands and categorizes errors
//! to provide user-friendly messages with actionable suggestions.

use regex::Regex;
use std::sync::LazyLock;

/// Category of the error with associated details
#[derive(Debug, Clone)]
#[allow(dead_code)] // Reserved for future UI enhancements (icons, colors per category)
pub enum ErrorCategory {
    /// GitHub API issues (rate limit, timeout, 5xx errors)
    GitHubApi {
        code: Option<u16>,
        input: Option<String>,
    },
    /// Network issues (no connectivity, DNS failure)
    Network { detail: String },
    /// Nix evaluation errors
    NixEval {
        file: Option<String>,
        line: Option<u32>,
    },
    /// Nix build failures
    NixBuild { derivation: Option<String> },
    /// Git errors
    Git { operation: String },
    /// Permission denied
    Permission { path: Option<String> },
    /// Generic/unknown error
    Unknown,
}

/// Parsed error with user-friendly information
#[derive(Debug, Clone)]
pub struct ParsedError {
    /// Short summary (one line)
    pub summary: String,
    /// Longer description if available
    pub detail: Option<String>,
    /// User-friendly suggestion
    pub suggestion: String,
}

/// Context about what operation was running
pub struct ErrorContext {
    pub operation: String,
}

impl ParsedError {
    /// Parse stderr output into a categorized error
    pub fn from_stderr(stderr: &str, context: ErrorContext) -> Self {
        // Try each parser in order of specificity
        if let Some(err) = parse_github_api_error(stderr) {
            return err;
        }
        if let Some(err) = parse_network_error(stderr) {
            return err;
        }
        if let Some(err) = parse_nix_eval_error(stderr) {
            return err;
        }
        if let Some(err) = parse_nix_build_error(stderr) {
            return err;
        }
        if let Some(err) = parse_git_error(stderr) {
            return err;
        }
        if let Some(err) = parse_permission_error(stderr) {
            return err;
        }

        // Fallback to generic error with context
        Self::generic(stderr, context)
    }

    fn generic(stderr: &str, context: ErrorContext) -> Self {
        // Try to extract the first meaningful error line
        let first_error = stderr
            .lines()
            .find(|line| line.contains("error:"))
            .or_else(|| stderr.lines().find(|line| !line.trim().is_empty()))
            .unwrap_or("Unknown error");

        // Clean up the error line
        let detail = first_error
            .trim()
            .trim_start_matches("error:")
            .trim()
            .to_string();

        Self {
            summary: format!("{} failed", context.operation),
            detail: if detail.is_empty() {
                None
            } else {
                Some(detail)
            },
            suggestion: "Check the output above for details.".to_string(),
        }
    }
}

// GitHub API error patterns
static GITHUB_HTTP_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)HTTP\s+error\s+(\d{3})").unwrap());

static GITHUB_INPUT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"github(?:\.com)?[:/]([^/\s']+/[^/\s']+)").unwrap()
});

fn parse_github_api_error(stderr: &str) -> Option<ParsedError> {
    // Check for HTTP errors from GitHub
    if let Some(caps) = GITHUB_HTTP_RE.captures(stderr) {
        let code: u16 = caps.get(1)?.as_str().parse().ok()?;

        // Extract the input/repo name if present
        let input = GITHUB_INPUT_RE
            .captures(stderr)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().trim_end_matches(".git").to_string());

        let (summary, suggestion) = match code {
            504 | 503 | 502 => (
                format!("GitHub API timeout (HTTP {})", code),
                "GitHub's API is temporarily unavailable. Try again in a few minutes.".to_string(),
            ),
            403 => (
                "GitHub API rate limit exceeded".to_string(),
                "You've hit GitHub's rate limit. Wait an hour or set GITHUB_TOKEN.".to_string(),
            ),
            401 => (
                "GitHub authentication failed".to_string(),
                "Check your GITHUB_TOKEN or SSH key configuration.".to_string(),
            ),
            404 => (
                "GitHub repository not found".to_string(),
                "The flake input references a non-existent or private repository.".to_string(),
            ),
            _ => (
                format!("GitHub API error (HTTP {})", code),
                "GitHub returned an unexpected error. Try again later.".to_string(),
            ),
        };

        return Some(ParsedError {
            summary,
            detail: input.map(|i| format!("Repository: {}", i)),
            suggestion,
        });
    }
    None
}

fn parse_network_error(stderr: &str) -> Option<ParsedError> {
    let patterns = [
        (
            "Could not resolve host",
            "DNS resolution failed",
            "Check your internet connection. Try: ping github.com",
        ),
        (
            "Connection refused",
            "Connection refused",
            "The remote server refused the connection. Check if it's online.",
        ),
        (
            "Connection timed out",
            "Connection timed out",
            "Network request timed out. Check your connection and try again.",
        ),
        (
            "Network is unreachable",
            "Network unreachable",
            "No network connectivity. Check your internet connection.",
        ),
        (
            "No route to host",
            "Cannot reach host",
            "Network routing issue. Check your connection.",
        ),
    ];

    for (pattern, summary, suggestion) in patterns {
        if stderr.to_lowercase().contains(&pattern.to_lowercase()) {
            return Some(ParsedError {
                summary: summary.to_string(),
                detail: None,
                suggestion: suggestion.to_string(),
            });
        }
    }
    None
}

// Nix evaluation error pattern
static NIX_EVAL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"error:[^\n]*\n.*?at\s+(/[^:]+):(\d+):").unwrap());

fn parse_nix_eval_error(stderr: &str) -> Option<ParsedError> {
    if let Some(caps) = NIX_EVAL_RE.captures(stderr) {
        let file = caps.get(1).map(|m| m.as_str().to_string());
        let line: Option<u32> = caps.get(2).and_then(|m| m.as_str().parse().ok());

        // Extract the actual error message
        let error_msg = stderr
            .lines()
            .find(|l| l.trim().starts_with("error:"))
            .map(|l| l.trim().trim_start_matches("error:").trim())
            .unwrap_or("Evaluation error");

        let detail = match (&file, line) {
            (Some(f), Some(l)) => format!("{}\n  at {}:{}", error_msg, f, l),
            (Some(f), None) => format!("{}\n  at {}", error_msg, f),
            _ => error_msg.to_string(),
        };

        return Some(ParsedError {
            summary: "Nix evaluation error".to_string(),
            detail: Some(detail),
            suggestion: "Check the Nix expression at the indicated location.".to_string(),
        });
    }
    None
}

// Nix build failure pattern
static NIX_BUILD_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"builder for '([^']+)' failed").unwrap());

fn parse_nix_build_error(stderr: &str) -> Option<ParsedError> {
    if stderr.contains("builder for") && stderr.contains("failed") {
        let derivation = NIX_BUILD_RE
            .captures(stderr)
            .and_then(|c| c.get(1))
            .map(|m| {
                // Extract just the package name from the full derivation path
                let drv = m.as_str();
                drv.rsplit('/')
                    .next()
                    .unwrap_or(drv)
                    .trim_end_matches(".drv")
                    .to_string()
            });

        return Some(ParsedError {
            summary: "Nix build failed".to_string(),
            detail: derivation.map(|d| format!("Failed: {}", d)),
            suggestion: "Check the build output above for compiler or dependency errors.".to_string(),
        });
    }
    None
}

fn parse_git_error(stderr: &str) -> Option<ParsedError> {
    let stderr_lower = stderr.to_lowercase();

    let patterns = [
        (
            "permission denied (publickey)",
            "SSH authentication failed",
            "Ensure your SSH key is added to the agent. With 1Password: check agent settings.",
        ),
        (
            "fatal: not a git repository",
            "Not a git repository",
            "Initialize git or check you're in the correct directory.",
        ),
        (
            "could not read from remote",
            "Git remote access failed",
            "Check your SSH key or network connection.",
        ),
        (
            "merge conflict",
            "Git merge conflict",
            "Resolve conflicts manually: git status shows affected files.",
        ),
        (
            "your local changes would be overwritten",
            "Uncommitted local changes",
            "Commit or stash your changes first: git stash",
        ),
    ];

    for (pattern, summary, suggestion) in patterns {
        if stderr_lower.contains(pattern) {
            return Some(ParsedError {
                summary: summary.to_string(),
                detail: None,
                suggestion: suggestion.to_string(),
            });
        }
    }
    None
}

// Permission error pattern
static PERM_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)permission denied[:\s]*([^\n]*)").unwrap());

fn parse_permission_error(stderr: &str) -> Option<ParsedError> {
    if let Some(caps) = PERM_RE.captures(stderr) {
        let path = caps
            .get(1)
            .map(|m| m.as_str().trim().to_string())
            .filter(|s| !s.is_empty());

        return Some(ParsedError {
            summary: "Permission denied".to_string(),
            detail: path,
            suggestion: "Try running with sudo or check file ownership.".to_string(),
        });
    }
    None
}
