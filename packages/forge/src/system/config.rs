//! Configuration file parsing

#![allow(dead_code)]

use anyhow::Result;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

/// CPU metadata from host-info.json
#[derive(Debug, Clone, Deserialize)]
pub struct CpuMeta {
    pub vendor: String,
    pub model: String,
}

/// GPU metadata from host-info.json
#[derive(Debug, Clone, Deserialize)]
pub struct GpuMeta {
    pub vendor: String,
    pub model: Option<String>,
}

/// Host hardware metadata loaded from host-info.json
#[derive(Debug, Clone, Deserialize)]
pub struct HostMetadata {
    pub cpu: Option<CpuMeta>,
    pub gpu: Option<GpuMeta>,
    pub form_factor: Option<String>,
    pub ram: Option<String>,
}

/// Host configuration discovered from filesystem
#[derive(Debug, Clone)]
pub struct HostConfig {
    pub name: String,
    pub description: String,
    pub metadata: Option<HostMetadata>,
}

/// Load host metadata from host-info.json
fn load_host_metadata(host_path: &Path) -> Option<HostMetadata> {
    let metadata_path = host_path.join("host-info.json");
    if let Ok(content) = std::fs::read_to_string(&metadata_path) {
        serde_json::from_str(&content).ok()
    } else {
        None
    }
}

/// Discover available hosts from the hosts/ directory
/// Checks multiple locations in order: /tmp/nixos-config, ~/nixos-config, /etc/nixos
pub fn discover_hosts() -> Vec<HostConfig> {
    use crate::constants::{HOSTS_SUBDIR, NIXOS_CONFIG_HOME_DIR, NIXOS_CONFIG_SYSTEM, NIXOS_CONFIG_TEMP};

    let locations = [
        format!("{}/{}", NIXOS_CONFIG_TEMP, HOSTS_SUBDIR),
        dirs::home_dir()
            .map(|h| format!("{}/{}/{}", h.display(), NIXOS_CONFIG_HOME_DIR, HOSTS_SUBDIR))
            .unwrap_or_default(),
        format!("{}/{}", NIXOS_CONFIG_SYSTEM, HOSTS_SUBDIR),
    ];

    // Find first existing hosts directory
    let hosts_dir = locations
        .iter()
        .find(|p| std::path::Path::new(p).exists())
        .cloned();

    let mut hosts = Vec::new();

    if let Some(hosts_path) = hosts_dir {
        if let Ok(entries) = std::fs::read_dir(&hosts_path) {
            for entry in entries.flatten() {
                if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    let name = entry.file_name().to_string_lossy().to_string();
                    let host_path = entry.path();
                    let default_nix = host_path.join("default.nix");

                    // Extract description from first comment line
                    let description = if let Ok(content) = std::fs::read_to_string(&default_nix) {
                        parse_host_description(&content)
                    } else {
                        "Host configuration".to_string()
                    };

                    // Load hardware metadata if available
                    let metadata = load_host_metadata(&host_path);

                    hosts.push(HostConfig {
                        name,
                        description,
                        metadata,
                    });
                }
            }
        }
    }

    // Sort alphabetically by name
    hosts.sort_by(|a, b| a.name.cmp(&b.name));
    hosts
}

/// Parse description from first line comment: "# hostname - Description"
fn parse_host_description(content: &str) -> String {
    if let Some(first_line) = content.lines().next() {
        if first_line.starts_with('#') {
            // Format: "# hostname - Description"
            if let Some(pos) = first_line.find(" - ") {
                return first_line[pos + 3..].trim().to_string();
            }
        }
    }
    "Host configuration".to_string()
}

/// Browser backup configuration
#[derive(Debug, Clone, Default)]
pub struct BrowserBackupConfig {
    pub repo: String,
    pub age_recipient: String,
    pub age_key_1password: Option<String>,
    pub age_key_path: Option<String>,
    pub local_repo_path: String,
    pub backup_retention: u32,
}

/// Load browser backup configuration from file
pub fn load_browser_config(path: &Path) -> Result<BrowserBackupConfig> {
    let content = std::fs::read_to_string(path)?;
    let mut config = BrowserBackupConfig::default();
    let mut vars = HashMap::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            let value = value.trim().trim_matches('"');
            vars.insert(key.to_string(), value.to_string());
        }
    }

    config.repo = vars
        .get("BROWSER_BACKUP_REPO")
        .cloned()
        .unwrap_or_default();
    config.age_recipient = vars.get("AGE_RECIPIENT").cloned().unwrap_or_default();
    config.age_key_1password = vars.get("AGE_KEY_1PASSWORD").cloned();
    config.age_key_path = vars.get("AGE_KEY_PATH").cloned();
    config.local_repo_path = vars
        .get("LOCAL_REPO_PATH")
        .cloned()
        .unwrap_or_else(|| "~/.local/share/browser-backup".to_string());
    config.backup_retention = vars
        .get("BACKUP_RETENTION")
        .and_then(|v| v.parse().ok())
        .unwrap_or(3);

    Ok(config)
}

/// Expand ~ in paths to home directory
pub fn expand_tilde(path: &str) -> String {
    if path.starts_with("~/") {
        if let Some(home) = dirs::home_dir() {
            return format!("{}{}", home.display(), &path[1..]);
        }
    }
    path.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_tilde_with_home() {
        let path = "~/test/path";
        let expanded = expand_tilde(path);
        // Should expand to home directory
        assert!(!expanded.starts_with("~"), "Tilde should be expanded");
        assert!(expanded.contains("test/path"), "Path suffix should be preserved");
    }

    #[test]
    fn test_expand_tilde_no_tilde() {
        let path = "/absolute/path";
        let expanded = expand_tilde(path);
        assert_eq!(expanded, path, "Path without tilde should remain unchanged");
    }

    #[test]
    fn test_expand_tilde_only_tilde() {
        let path = "~";
        let expanded = expand_tilde(path);
        // ~ alone is not expanded (only ~/)
        assert_eq!(expanded, "~", "Single tilde should remain unchanged");
    }

    #[test]
    fn test_expand_tilde_in_middle() {
        let path = "/some/~/path";
        let expanded = expand_tilde(path);
        assert_eq!(expanded, path, "Tilde in middle of path should not expand");
    }

    #[test]
    fn test_parse_host_description_with_dash() {
        let content = "# hostname - This is a description\n{ config }:";
        let description = parse_host_description(content);
        assert_eq!(description, "This is a description");
    }

    #[test]
    fn test_parse_host_description_no_dash() {
        let content = "# Just a comment\n{ config }:";
        let description = parse_host_description(content);
        assert_eq!(description, "Host configuration");
    }

    #[test]
    fn test_parse_host_description_empty() {
        let content = "";
        let description = parse_host_description(content);
        assert_eq!(description, "Host configuration");
    }

    #[test]
    fn test_parse_host_description_no_comment() {
        let content = "{ config }:";
        let description = parse_host_description(content);
        assert_eq!(description, "Host configuration");
    }

    #[test]
    fn test_browser_backup_config_default() {
        let config = BrowserBackupConfig::default();
        assert!(config.repo.is_empty());
        assert!(config.age_recipient.is_empty());
        assert!(config.age_key_1password.is_none());
        assert!(config.age_key_path.is_none());
        assert!(config.local_repo_path.is_empty());
        assert_eq!(config.backup_retention, 0);
    }

    #[test]
    fn test_host_config_clone() {
        let config = HostConfig {
            name: "testhost".to_string(),
            description: "Test description".to_string(),
            metadata: None,
        };
        let cloned = config.clone();
        assert_eq!(cloned.name, "testhost");
        assert_eq!(cloned.description, "Test description");
        assert!(cloned.metadata.is_none());
    }

    #[test]
    fn test_host_metadata_with_values() {
        let metadata = HostMetadata {
            cpu: Some(CpuMeta {
                vendor: "AMD".to_string(),
                model: "Ryzen 9".to_string(),
            }),
            gpu: Some(GpuMeta {
                vendor: "NVIDIA".to_string(),
                model: Some("RTX 5090".to_string()),
            }),
            form_factor: Some("Desktop".to_string()),
            ram: Some("64 GB".to_string()),
        };
        assert_eq!(metadata.cpu.as_ref().unwrap().vendor, "AMD");
        assert_eq!(metadata.gpu.as_ref().unwrap().vendor, "NVIDIA");
        assert_eq!(metadata.form_factor, Some("Desktop".to_string()));
        assert_eq!(metadata.ram, Some("64 GB".to_string()));
    }
}
