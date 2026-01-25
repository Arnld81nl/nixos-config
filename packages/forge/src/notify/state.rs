//! State persistence for notification deduplication
//!
//! Tracks what updates we've already notified about to avoid spam.

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::paths::notify_state_path;

/// State file for tracking notified updates
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct NotifyState {
    /// Last time we performed a check
    pub last_check: Option<DateTime<Utc>>,

    /// Last notified state to avoid re-notifying
    pub last_notified: NotifiedState,
}

/// What we last notified the user about
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct NotifiedState {
    /// Latest config commit hash we notified about
    pub config_commit: Option<String>,

    /// Whether we notified about app updates (reset after user runs restore)
    pub app_updates: bool,

    /// Flake inputs we notified about (input_name@rev)
    pub flake_inputs: Vec<String>,
}

impl NotifyState {
    /// Load state from disk, or return default if not found
    pub fn load() -> Result<Self> {
        let path = notify_state_path();

        if !path.exists() {
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(&path)?;
        let state: NotifyState = serde_json::from_str(&content)?;
        Ok(state)
    }

    /// Save state to disk
    pub fn save(&self) -> Result<()> {
        let path = notify_state_path();

        // Ensure directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    /// Update the notified state based on current updates
    pub fn mark_notified(&mut self, status: &super::UpdateStatus) {
        self.last_check = Some(Utc::now());

        // Update config commit - store the first (newest) commit hash
        if let Some((hash, _)) = status.config_updates.first() {
            self.last_notified.config_commit = Some(hash.clone());
        }

        // Update app notification state - track current status
        // This allows re-notification when updates become available again after being applied
        self.last_notified.app_updates = status.app_updates;

        // Update flake inputs - track current set
        // If updates are empty, clear the list to allow re-notification for new updates
        self.last_notified.flake_inputs = status.flake_updates.clone();
    }

    /// Check if we should notify based on current status vs last notified
    pub fn should_notify(&self, status: &super::UpdateStatus) -> bool {
        // If no updates, don't notify
        if !status.has_updates() {
            return false;
        }

        // Check if config has new commits we haven't notified about
        if let Some((current_hash, _)) = status.config_updates.first() {
            if self.last_notified.config_commit.as_ref() != Some(current_hash) {
                return true;
            }
        }

        // Check if app updates are new
        if status.app_updates && !self.last_notified.app_updates {
            return true;
        }

        // Check if flake inputs have changed
        if !status.flake_updates.is_empty() {
            // Any input not in the last notified set triggers notification
            for input in &status.flake_updates {
                if !self.last_notified.flake_inputs.contains(input) {
                    return true;
                }
            }
        }

        false
    }

    /// Clear app update notification (called after user runs restore)
    /// Note: With the updated logic in mark_notified, this is automatically handled
    /// when the next check shows no app updates. This method is kept for manual clearing.
    pub fn clear_app_notification(&mut self) {
        self.last_notified.app_updates = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_notify_no_updates() {
        let state = NotifyState::default();
        let status = super::super::UpdateStatus::default();
        assert!(!state.should_notify(&status));
    }

    #[test]
    fn test_should_notify_new_config() {
        let state = NotifyState::default();
        let status = super::super::UpdateStatus {
            config_updates: vec![("abc1234".to_string(), "Test commit".to_string())],
            app_updates: false,
            flake_updates: vec![],
        };
        assert!(state.should_notify(&status));
    }

    #[test]
    fn test_should_not_notify_same_config() {
        let mut state = NotifyState::default();
        state.last_notified.config_commit = Some("abc1234".to_string());

        let status = super::super::UpdateStatus {
            config_updates: vec![("abc1234".to_string(), "Test commit".to_string())],
            app_updates: false,
            flake_updates: vec![],
        };
        assert!(!state.should_notify(&status));
    }
}
