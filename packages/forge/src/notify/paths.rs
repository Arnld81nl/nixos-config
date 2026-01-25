//! Path resolution for forge-notify
//!
//! Centralized directory path resolution with consistent fallback strategies.

use std::path::PathBuf;

// =============================================================================
// Path Constants (relative to home directory)
// =============================================================================

/// Forge data directory relative to home
const FORGE_DATA_SUBDIR: &str = ".local/share/forge";

/// Forge log filename
pub const FORGE_LOG_FILE: &str = "forge.log";

/// Notification state filename
const NOTIFY_STATE_FILE: &str = "notify-state.json";

/// App backup data directory relative to home
const APP_BACKUP_DATA_SUBDIR: &str = ".local/share/app-backup";

/// Legacy browser backup directory relative to home (for backwards compatibility)
const BROWSER_BACKUP_DATA_SUBDIR_LEGACY: &str = ".local/share/browser-backup";

/// NixOS config subdirectory name in home
const NIXOS_CONFIG_HOME_SUBDIR: &str = "nixos-config";

/// System NixOS configuration directory
const NIXOS_CONFIG_SYSTEM: &str = "/etc/nixos";

/// Flake.nix filename
const FLAKE_NIX: &str = "flake.nix";

// =============================================================================
// Fallback Paths
// =============================================================================

/// Fallback forge data directory when home is unavailable
const FALLBACK_FORGE_DATA_DIR: &str = "/tmp/forge";

/// Fallback notification state file when home is unavailable
const FALLBACK_NOTIFY_STATE_FILE: &str = "/tmp/forge-notify-state.json";

// =============================================================================
// Path Resolution Functions
// =============================================================================

/// Get the forge data directory path
/// Falls back to /tmp/forge if home directory is unavailable
pub fn forge_data_dir() -> PathBuf {
    dirs::home_dir()
        .map(|h| h.join(FORGE_DATA_SUBDIR))
        .unwrap_or_else(|| PathBuf::from(FALLBACK_FORGE_DATA_DIR))
}

/// Get the forge log file path
pub fn forge_log_path() -> PathBuf {
    forge_data_dir().join(FORGE_LOG_FILE)
}

/// Get the notification state file path
/// Falls back to /tmp/forge-notify-state.json if home directory is unavailable
pub fn notify_state_path() -> PathBuf {
    dirs::home_dir()
        .map(|h| h.join(FORGE_DATA_SUBDIR).join(NOTIFY_STATE_FILE))
        .unwrap_or_else(|| PathBuf::from(FALLBACK_NOTIFY_STATE_FILE))
}

/// Get the NixOS config directory
/// Checks multiple locations in order of preference:
/// 1. /etc/nixos (if it contains flake.nix)
/// 2. ~/nixos-config (if it contains flake.nix)
/// 3. Returns an empty path if neither exists
pub fn nixos_config_dir() -> PathBuf {
    // Check /etc/nixos first (system location, usually a symlink)
    let system_path = PathBuf::from(NIXOS_CONFIG_SYSTEM);
    if system_path.join(FLAKE_NIX).exists() {
        return system_path;
    }

    // Check ~/nixos-config
    if let Some(home) = dirs::home_dir() {
        let home_path = home.join(NIXOS_CONFIG_HOME_SUBDIR);
        if home_path.join(FLAKE_NIX).exists() {
            return home_path;
        }
    }

    PathBuf::new()
}

/// Get the app backup data directory
/// Checks for new app-backup location first, falls back to legacy browser-backup
pub fn app_backup_data_dir() -> PathBuf {
    dirs::home_dir()
        .map(|h| {
            let new_path = h.join(APP_BACKUP_DATA_SUBDIR);
            if new_path.join(".git").exists() {
                new_path
            } else {
                h.join(BROWSER_BACKUP_DATA_SUBDIR_LEGACY)
            }
        })
        .unwrap_or_default()
}

/// Get the flake.lock path for the nixos config directory
pub fn flake_lock_path() -> PathBuf {
    nixos_config_dir().join("flake.lock")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_forge_data_dir_contains_forge() {
        let path = forge_data_dir();
        let path_str = path.to_string_lossy();
        // Should either be a proper home path or the fallback
        assert!(
            path_str.contains("forge") || path_str == FALLBACK_FORGE_DATA_DIR,
            "Forge data dir should contain 'forge' or be fallback"
        );
    }

    #[test]
    fn test_notify_state_path_is_json() {
        let path = notify_state_path();
        let path_str = path.to_string_lossy();
        assert!(path_str.ends_with(".json"), "Notify state should be a JSON file");
    }

    #[test]
    fn test_forge_log_path_is_log() {
        let path = forge_log_path();
        let path_str = path.to_string_lossy();
        assert!(path_str.ends_with(".log"), "Forge log should be a .log file");
    }

    #[test]
    fn test_flake_lock_path_ends_correctly() {
        let path = flake_lock_path();
        let path_str = path.to_string_lossy();
        // If config dir exists, should end with flake.lock
        if !path_str.is_empty() {
            assert!(path_str.ends_with("flake.lock"), "Flake lock path should end with flake.lock");
        }
    }

    #[test]
    fn test_constants_not_empty() {
        assert!(!FORGE_DATA_SUBDIR.is_empty());
        assert!(!FORGE_LOG_FILE.is_empty());
        assert!(!NOTIFY_STATE_FILE.is_empty());
        assert!(!APP_BACKUP_DATA_SUBDIR.is_empty());
        assert!(!NIXOS_CONFIG_HOME_SUBDIR.is_empty());
        assert!(!NIXOS_CONFIG_SYSTEM.is_empty());
    }
}
