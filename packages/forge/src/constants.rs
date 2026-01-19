//! Application-wide constants

use std::path::PathBuf;

// =============================================================================
// Buffer and Timeout Constants
// =============================================================================

/// Maximum lines to retain in output buffer
pub const OUTPUT_BUFFER_SIZE: usize = 1000;

/// Default command timeout in seconds (5 minutes)
pub const DEFAULT_COMMAND_TIMEOUT_SECS: u64 = 300;

/// Event poll timeout in milliseconds
pub const EVENT_POLL_TIMEOUT_MS: u64 = 100;

/// Spinner animation interval in milliseconds
pub const SPINNER_TICK_MS: u128 = 100;

/// Channel buffer size for command messages
pub const COMMAND_CHANNEL_SIZE: usize = 100;

/// Maximum length for user text input (prevents memory exhaustion)
pub const MAX_INPUT_LENGTH: usize = 100;

// =============================================================================
// User Constants
// =============================================================================

/// Primary user UID (first regular user on NixOS)
pub const PRIMARY_USER_UID: u32 = 1000;

/// Primary user GID (users group on NixOS)
pub const PRIMARY_USER_GID: u32 = 100;

// =============================================================================
// Path Constants
// =============================================================================

/// System NixOS configuration directory
pub const NIXOS_CONFIG_SYSTEM: &str = "/etc/nixos";

/// Temporary NixOS configuration directory (used during ISO install)
pub const NIXOS_CONFIG_TEMP: &str = "/tmp/nixos-config";

/// NixOS configuration subdirectory name in home
pub const NIXOS_CONFIG_HOME_DIR: &str = "nixos-config";

/// Hosts subdirectory within config
pub const HOSTS_SUBDIR: &str = "hosts";

/// Flake.nix filename
pub const FLAKE_NIX: &str = "flake.nix";

/// Mount point for NixOS installation
pub const INSTALL_MOUNT_POINT: &str = "/mnt";

/// Symlink path during installation
pub const INSTALL_SYMLINK_PATH: &str = "/mnt/etc/nixos";

// =============================================================================
// Forge Data Paths (relative to home directory)
// =============================================================================

/// Forge data directory (relative to home)
pub const FORGE_DATA_DIR: &str = ".local/share/forge";

/// Forge log filename
pub const FORGE_LOG_FILE: &str = "forge.log";

/// Screen log filename
pub const SCREEN_LOG_FILE: &str = "screen.log";

// =============================================================================
// App Backup Paths (relative to home directory)
// =============================================================================

/// App backup data directory (relative to home)
pub const APP_BACKUP_DATA_DIR: &str = ".local/share/app-backup";

/// Legacy browser backup directory (relative to home)
pub const BROWSER_BACKUP_DATA_DIR_LEGACY: &str = ".local/share/browser-backup";

/// App backup config directory (relative to home)
pub const APP_BACKUP_CONFIG_DIR: &str = ".config/app-backup";

/// App backup config filename
pub const APP_BACKUP_CONFIG_FILE: &str = "config";

// =============================================================================
// CLI Tool Paths (relative to home directory)
// =============================================================================

/// Claude CLI path (relative to home)
pub const CLAUDE_CLI_PATH: &str = ".local/bin/claude";

/// Codex CLI path (relative to home)
pub const CODEX_CLI_PATH: &str = ".npm-global/bin/codex";

// =============================================================================
// Helper Functions
// =============================================================================

/// Get the forge data directory path
pub fn forge_data_dir() -> PathBuf {
    dirs::home_dir()
        .map(|h| h.join(FORGE_DATA_DIR))
        .unwrap_or_else(|| PathBuf::from("/tmp/forge"))
}

/// Get the app backup data directory, checking both new and legacy paths
pub fn app_backup_data_dir() -> PathBuf {
    dirs::home_dir()
        .map(|h| {
            let new_path = h.join(APP_BACKUP_DATA_DIR);
            if new_path.join(".git").exists() {
                new_path
            } else {
                h.join(BROWSER_BACKUP_DATA_DIR_LEGACY)
            }
        })
        .unwrap_or_default()
}

/// Get the app backup config file path
pub fn app_backup_config_path() -> PathBuf {
    dirs::home_dir()
        .map(|h| h.join(APP_BACKUP_CONFIG_DIR).join(APP_BACKUP_CONFIG_FILE))
        .unwrap_or_default()
}

/// Get the NixOS config directory, checking multiple locations
pub fn nixos_config_dir() -> PathBuf {
    // Check /etc/nixos first (system location, usually a symlink)
    let system_path = PathBuf::from(NIXOS_CONFIG_SYSTEM);
    if system_path.join(FLAKE_NIX).exists() {
        return system_path;
    }

    // Check ~/nixos-config
    if let Some(home) = dirs::home_dir() {
        let home_path = home.join(NIXOS_CONFIG_HOME_DIR);
        if home_path.join(FLAKE_NIX).exists() {
            return home_path;
        }
    }

    // Fall back to current directory
    std::env::current_dir().unwrap_or_default()
}

/// Get temporary config directory for fresh install
pub fn temp_config_dir() -> PathBuf {
    PathBuf::from(NIXOS_CONFIG_TEMP)
}

/// Get hosts directory paths to check
pub fn host_dir_paths(hostname: &str) -> Vec<PathBuf> {
    let mut paths = vec![
        PathBuf::from(NIXOS_CONFIG_TEMP).join(HOSTS_SUBDIR).join(hostname),
    ];

    if let Some(home) = dirs::home_dir() {
        paths.push(home.join(NIXOS_CONFIG_HOME_DIR).join(HOSTS_SUBDIR).join(hostname));
    }

    paths.push(PathBuf::from(NIXOS_CONFIG_SYSTEM).join(HOSTS_SUBDIR).join(hostname));

    paths
}

/// Get the Claude CLI path
pub fn claude_cli_path() -> PathBuf {
    dirs::home_dir()
        .map(|h| h.join(CLAUDE_CLI_PATH))
        .unwrap_or_default()
}

/// Get the Codex CLI path
pub fn codex_cli_path() -> PathBuf {
    dirs::home_dir()
        .map(|h| h.join(CODEX_CLI_PATH))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_forge_data_dir_contains_forge() {
        let path = forge_data_dir();
        let path_str = path.to_string_lossy();
        assert!(path_str.contains("forge"), "Forge data dir should contain 'forge'");
    }

    #[test]
    fn test_temp_config_dir() {
        let path = temp_config_dir();
        let path_str = path.to_string_lossy();
        assert_eq!(path_str, NIXOS_CONFIG_TEMP, "Temp config dir should match NIXOS_CONFIG_TEMP");
        assert!(path_str.starts_with("/tmp"), "Temp config dir should be in /tmp");
    }

    #[test]
    fn test_host_dir_paths_returns_multiple() {
        let paths = host_dir_paths("testhost");
        assert!(!paths.is_empty(), "Should return at least one path");
        // All paths should contain the hostname
        for path in &paths {
            let path_str = path.to_string_lossy();
            assert!(path_str.contains("testhost"), "Path should contain hostname");
            assert!(path_str.contains("hosts"), "Path should contain 'hosts' directory");
        }
    }

    #[test]
    fn test_claude_cli_path_format() {
        let path = claude_cli_path();
        let path_str = path.to_string_lossy();
        // Should be empty or contain 'claude'
        if !path_str.is_empty() {
            assert!(path_str.contains("claude"), "Claude CLI path should contain 'claude'");
        }
    }

    #[test]
    fn test_codex_cli_path_format() {
        let path = codex_cli_path();
        let path_str = path.to_string_lossy();
        // Should be empty or contain 'codex'
        if !path_str.is_empty() {
            assert!(path_str.contains("codex"), "Codex CLI path should contain 'codex'");
        }
    }

    #[test]
    fn test_constants_not_empty() {
        assert!(!NIXOS_CONFIG_SYSTEM.is_empty());
        assert!(!NIXOS_CONFIG_TEMP.is_empty());
        assert!(!NIXOS_CONFIG_HOME_DIR.is_empty());
        assert!(!HOSTS_SUBDIR.is_empty());
        assert!(!FLAKE_NIX.is_empty());
        assert!(!FORGE_DATA_DIR.is_empty());
        assert!(!FORGE_LOG_FILE.is_empty());
        assert!(!SCREEN_LOG_FILE.is_empty());
    }

    #[test]
    fn test_paths_are_absolute_or_relative() {
        // System paths should be absolute
        assert!(NIXOS_CONFIG_SYSTEM.starts_with('/'));
        assert!(NIXOS_CONFIG_TEMP.starts_with('/'));
        assert!(INSTALL_MOUNT_POINT.starts_with('/'));
        assert!(INSTALL_SYMLINK_PATH.starts_with('/'));

        // Home-relative paths should not start with /
        assert!(!FORGE_DATA_DIR.starts_with('/'));
        assert!(!CLAUDE_CLI_PATH.starts_with('/'));
        assert!(!CODEX_CLI_PATH.starts_with('/'));
    }

    #[test]
    fn test_buffer_constants_reasonable() {
        assert!(OUTPUT_BUFFER_SIZE > 0);
        assert!(OUTPUT_BUFFER_SIZE <= 10000); // Reasonable upper bound
        assert!(DEFAULT_COMMAND_TIMEOUT_SECS > 0);
        assert!(EVENT_POLL_TIMEOUT_MS > 0);
        assert!(SPINNER_TICK_MS > 0);
        assert!(COMMAND_CHANNEL_SIZE > 0);
        assert!(MAX_INPUT_LENGTH > 0);
    }

    #[test]
    fn test_user_constants() {
        // Primary user should be first regular user
        assert_eq!(PRIMARY_USER_UID, 1000);
        assert_eq!(PRIMARY_USER_GID, 100);
    }
}
