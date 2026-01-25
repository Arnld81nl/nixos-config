//! Constants for forge-notify
//!
//! Centralized configuration values for timeouts, priority inputs, and other settings.

use std::time::Duration;

// =============================================================================
// Timeout Constants
// =============================================================================

/// Timeout for git fetch operations (seconds)
pub const GIT_FETCH_TIMEOUT_SECS: u64 = 10;

/// Timeout for flake update checks (seconds)
pub const FLAKE_CHECK_TIMEOUT_SECS: u64 = 15;

/// Timeout for HTTP client requests (seconds)
pub const HTTP_CLIENT_TIMEOUT_SECS: u64 = 10;

/// Duration for desktop notification display (milliseconds)
pub const NOTIFICATION_TIMEOUT_MS: i32 = 10000;

// =============================================================================
// Duration Helpers
// =============================================================================

/// Get git fetch timeout as Duration
pub fn git_fetch_timeout() -> Duration {
    Duration::from_secs(GIT_FETCH_TIMEOUT_SECS)
}

/// Get flake check timeout as Duration
pub fn flake_check_timeout() -> Duration {
    Duration::from_secs(FLAKE_CHECK_TIMEOUT_SECS)
}

/// Get HTTP client timeout as Duration
pub fn http_client_timeout() -> Duration {
    Duration::from_secs(HTTP_CLIENT_TIMEOUT_SECS)
}

// =============================================================================
// Priority Inputs
// =============================================================================

/// Priority inputs to check for updates (in order of importance)
/// We only check these to minimize API calls and avoid GitHub rate limiting.
/// The GitHub API has a limit of 60 requests/hour for unauthenticated requests.
pub const PRIORITY_INPUTS: &[&str] = &["nixpkgs"];

// =============================================================================
// Default Branch Names
// =============================================================================

/// Get the default branch for well-known repositories
pub fn default_branch_for_repo(owner: &str, repo: &str) -> &'static str {
    match (owner, repo) {
        ("NixOS", "nixpkgs") => "nixos-unstable",
        ("nix-community", "home-manager") => "master",
        _ => "main",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timeout_durations() {
        assert_eq!(git_fetch_timeout(), Duration::from_secs(10));
        assert_eq!(flake_check_timeout(), Duration::from_secs(15));
        assert_eq!(http_client_timeout(), Duration::from_secs(10));
    }

    #[test]
    fn test_priority_inputs_contains_nixpkgs() {
        assert!(PRIORITY_INPUTS.contains(&"nixpkgs"));
    }

    #[test]
    fn test_default_branch_nixpkgs() {
        assert_eq!(default_branch_for_repo("NixOS", "nixpkgs"), "nixos-unstable");
    }

    #[test]
    fn test_default_branch_home_manager() {
        assert_eq!(default_branch_for_repo("nix-community", "home-manager"), "master");
    }

    #[test]
    fn test_default_branch_other() {
        assert_eq!(default_branch_for_repo("someone", "something"), "main");
    }

    #[test]
    fn test_notification_timeout_is_positive() {
        assert!(NOTIFICATION_TIMEOUT_MS > 0);
    }
}
