//! Package comparison utilities using nvd

use anyhow::Result;
use tokio::sync::mpsc;

use super::out;
use crate::commands::executor::{get_output, run_capture};
use crate::commands::CommandMessage;

/// Result of package comparison containing version changes and closure summary
pub struct PackageCompareResult {
    pub changes: Vec<(String, String, String)>,
    pub closure_summary: Option<String>,
}

impl Default for PackageCompareResult {
    fn default() -> Self {
        Self {
            changes: Vec::new(),
            closure_summary: None,
        }
    }
}

/// Compare current system generation to previous generation using nvd
pub async fn parse_package_changes_from_history(
    tx: &mpsc::Sender<CommandMessage>,
) -> Result<PackageCompareResult> {
    // Get current generation number from /nix/var/nix/profiles/system
    let current_gen = match get_output("readlink", &["/nix/var/nix/profiles/system"]).await {
        Ok(s) => s.trim().to_string(),
        Err(_) => {
            out(tx, "    Could not read current generation").await;
            return Ok(PackageCompareResult::default());
        }
    };

    // Extract generation number (format: system-N-link -> we want N)
    let gen_num: u32 = current_gen
        .split('-')
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    if gen_num <= 1 {
        out(tx, "    No previous generation to compare").await;
        return Ok(PackageCompareResult::default());
    }

    let prev_gen = gen_num - 1;
    let current_path = format!("/nix/var/nix/profiles/system-{}-link", gen_num);
    let prev_path = format!("/nix/var/nix/profiles/system-{}-link", prev_gen);

    // Check if previous generation exists
    if !std::path::Path::new(&prev_path).exists() {
        out(tx, "    Previous generation not found").await;
        return Ok(PackageCompareResult::default());
    }

    out(
        tx,
        &format!("    Comparing generation {} → {}", prev_gen, gen_num),
    )
    .await;

    // Run nvd diff
    let (success, stdout, _stderr) =
        run_capture("nvd", &["diff", &prev_path, &current_path]).await?;

    if !success {
        out(tx, "    nvd diff failed").await;
        return Ok(PackageCompareResult::default());
    }

    parse_nvd_output(&stdout, tx).await
}

/// Compare two specific system paths using nvd
#[allow(dead_code)]
pub async fn parse_package_changes(
    old_system: Option<&str>,
    tx: &mpsc::Sender<CommandMessage>,
) -> Result<PackageCompareResult> {
    let old_path = match old_system {
        Some(p) if !p.is_empty() => p,
        _ => {
            tracing::debug!("parse_package_changes: no old system path provided");
            return Ok(PackageCompareResult::default());
        }
    };

    // Get new system path
    let new_system = match get_output("readlink", &["/run/current-system"]).await {
        Ok(s) => s.trim().to_string(),
        Err(_) => {
            tracing::debug!("parse_package_changes: could not read new system path");
            return Ok(PackageCompareResult::default());
        }
    };

    // Skip if paths are the same (no actual change)
    if old_path == new_system {
        tracing::debug!("parse_package_changes: system paths unchanged");
        return Ok(PackageCompareResult::default());
    }

    // Run nvd diff
    let (success, stdout, _stderr) = run_capture("nvd", &["diff", old_path, &new_system]).await?;

    if !success {
        out(tx, "    nvd diff failed").await;
        tracing::debug!("parse_package_changes: nvd diff failed");
        return Ok(PackageCompareResult::default());
    }

    parse_nvd_output(&stdout, tx).await
}

/// Parse nvd diff output into package changes and closure summary
async fn parse_nvd_output(
    stdout: &str,
    tx: &mpsc::Sender<CommandMessage>,
) -> Result<PackageCompareResult> {
    // Parse nvd output - extract version changes and closure summary
    // Update format: "[U.]  #015  firefox    146.0 -> 146.0.1"
    // Closure format: "Closure size: 2478 -> 2478 (8 paths added, 8 paths removed, delta +0, disk usage -2.8KiB)."
    let mut changes = Vec::new();
    let mut closure_summary = None;

    for line in stdout.lines() {
        let line = line.trim();

        // Capture closure size summary
        if line.starts_with("Closure size:") {
            // Extract the part after "Closure size: "
            let summary = line.strip_prefix("Closure size:").unwrap_or(line).trim();
            closure_summary = Some(summary.trim_end_matches('.').to_string());
            continue;
        }

        // Only process updates [U.] or [U*]
        if !line.starts_with("[U") {
            continue;
        }

        // Find the arrow to extract version info
        if let Some(arrow_pos) = line.find(" -> ") {
            // Skip the "[U.]  #NNN  " prefix to get package name
            if let Some(hash_pos) = line.find('#') {
                let after_hash = &line[hash_pos..];
                // Skip "#NNN " to get to package name and version
                if let Some(space_pos) = after_hash.find(char::is_whitespace) {
                    let rest = after_hash[space_pos..].trim();

                    // Split at arrow
                    let before_arrow = &rest[..rest.find(" -> ").unwrap_or(rest.len())];
                    let after_arrow = &line[arrow_pos + 4..];

                    // Package name is the first token
                    let parts: Vec<&str> = before_arrow.split_whitespace().collect();
                    if parts.is_empty() {
                        continue;
                    }
                    let pkg_name = parts[0];

                    // Old version is the first token after package name (may have comma)
                    // nvd lists multiple outputs comma-separated: "1.0, 1.0-fish-completions"
                    // We want just the first (main) version
                    let old_ver = if parts.len() > 1 {
                        parts[1].trim_end_matches(',')
                    } else {
                        continue; // No version info
                    };

                    // New version is the first token after arrow
                    let new_ver = after_arrow
                        .split(|c: char| c == ',' || c.is_whitespace())
                        .next()
                        .unwrap_or("")
                        .trim();

                    if !pkg_name.is_empty() && !old_ver.is_empty() && !new_ver.is_empty() {
                        out(tx, &format!("    {}: {} → {}", pkg_name, old_ver, new_ver)).await;
                        changes.push((
                            pkg_name.to_string(),
                            old_ver.to_string(),
                            new_ver.to_string(),
                        ));
                    }
                }
            }
        }
    }

    Ok(PackageCompareResult {
        changes,
        closure_summary,
    })
}
