//! Background update checker for Forge
//!
//! Checks for:
//! - NixOS config repo updates (changes from other machines)
//! - App profile updates (private-settings repo)
//! - Flake input updates (nixpkgs, home-manager, etc.)

pub mod checks;
pub mod constants;
pub mod flake;
pub mod paths;
pub mod state;

use anyhow::Result;

/// Status of all update checks
#[derive(Debug, Default)]
pub struct UpdateStatus {
    /// Commits available in nixos-config repo (hash, message)
    pub config_updates: Vec<(String, String)>,
    /// Whether app profiles have updates
    pub app_updates: bool,
    /// Flake inputs that have updates available
    pub flake_updates: Vec<String>,
}

impl UpdateStatus {
    /// Returns true if any updates are available
    pub fn has_updates(&self) -> bool {
        !self.config_updates.is_empty() || self.app_updates || !self.flake_updates.is_empty()
    }

    /// Build a notification summary message
    pub fn summary(&self) -> String {
        let mut lines = Vec::new();

        if !self.config_updates.is_empty() {
            let count = self.config_updates.len();
            lines.push(format!(
                "- {} config commit{} available",
                count,
                if count == 1 { "" } else { "s" }
            ));
        }

        if self.app_updates {
            lines.push("- App profiles updated".to_string());
        }

        if !self.flake_updates.is_empty() {
            let names = self.flake_updates.join(", ");
            lines.push(format!("- Flake inputs: {}", names));
        }

        lines.join("\n")
    }
}

/// Run all update checks concurrently
pub async fn check_all_updates() -> Result<UpdateStatus> {
    let (config_result, apps_result, flake_result) = tokio::join!(
        checks::check_nixos_config_updates(None),
        checks::check_app_updates(None),
        flake::check_flake_updates(),
    );

    Ok(UpdateStatus {
        config_updates: config_result.unwrap_or_default(),
        app_updates: apps_result.unwrap_or(false),
        flake_updates: flake_result.unwrap_or_default(),
    })
}
