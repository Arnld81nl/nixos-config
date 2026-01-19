//! Application state types and enums

use std::collections::VecDeque;

use crate::commands::update::flake::FlakeInputChange;
use crate::system::config::HostConfig;
use crate::system::disk::DiskInfo;
use crate::system::hardware::{CpuInfo, FormFactor, GpuInfo};

/// Main menu items
pub const MAIN_MENU_ITEMS: &[&str] = &[
    "Install NixOS (fresh installation)",
    "Update system",
    "App profiles",
    "Exit",
];

/// App profile menu items (browser sessions and logins)
pub const APP_MENU_ITEMS: &[&str] = &[
    "Backup & push to GitHub",
    "Pull & restore from GitHub",
    "Check for updates",
    "Back to main menu",
];

/// Application mode/screen
#[derive(Debug, Clone)]
pub enum AppMode {
    MainMenu { selected: usize },
    Install(InstallState),
    CreateHost(CreateHostState),
    Update(UpdateState),
    Apps(AppProfileState),
    Keys(KeysState),
    #[allow(dead_code)]
    Quit,
}

/// Which credential field is currently active
#[derive(Debug, Clone, PartialEq, Default)]
pub enum CredentialField {
    #[default]
    Username,
    Password,
    ConfirmPassword,
}

/// Swap mode selection for installation
#[derive(Debug, Clone, PartialEq, Default)]
pub enum SwapMode {
    /// Zram only - compressed RAM swap, no hibernate support
    #[default]
    ZramOnly,
    /// Hibernate support - disk swapfile sized at RAM + 2GB
    HibernateSupport,
}

/// User credentials and options collected during installation
#[derive(Debug, Clone, Default)]
pub struct InstallCredentials {
    pub username: String,
    pub password: String,
    pub confirm_password: String,
    pub swap_mode: SwapMode,
}

/// Installation state machine
#[derive(Debug, Clone)]
pub enum InstallState {
    /// Cloning repository from GitHub (live ISO only)
    CloneRepository {
        output: VecDeque<String>,
    },
    SelectHost {
        selected: usize,
    },
    SelectDisk {
        host: String,
        disks: Vec<DiskInfo>,
        selected: usize,
    },
    EnterCredentials {
        host: String,
        disk: DiskInfo,
        credentials: InstallCredentials,
        active_field: CredentialField,
        error: Option<String>,
    },
    SelectSwapMode {
        host: String,
        disk: DiskInfo,
        credentials: InstallCredentials,
        selected: usize,
        /// Total RAM in GB (for display)
        ram_gb: u64,
    },
    Overview {
        host: String,
        disk: DiskInfo,
        credentials: InstallCredentials,
        hardware_config: Option<NewHostConfig>,
        input: String,
    },
    Running {
        host: String,
        disk: DiskInfo,
        credentials: InstallCredentials,
        step: usize,
        steps: Vec<StepStatus>,
        output: VecDeque<String>,
    },
    Complete {
        success: bool,
        output: VecDeque<String>,
        /// None = auto-scroll, Some(n) = manual scroll at position n
        scroll_offset: Option<usize>,
    },
}

impl InstallState {
    pub fn new(hostname: Option<String>, disk: Option<String>) -> Self {
        match (hostname, disk) {
            (Some(host), Some(disk_path)) => {
                // Direct install with provided args - go to credentials
                let disk = DiskInfo {
                    path: disk_path,
                    size: "Unknown".to_string(),
                    size_bytes: 0,
                    model: None,
                    partitions: vec![],
                };
                InstallState::EnterCredentials {
                    host,
                    disk,
                    credentials: InstallCredentials::default(),
                    active_field: CredentialField::Username,
                    error: None,
                }
            }
            (Some(host), None) => {
                // Host provided, need disk selection
                InstallState::SelectDisk {
                    host,
                    disks: Vec::new(),
                    selected: 0,
                }
            }
            _ => InstallState::SelectHost { selected: 0 },
        }
    }
}

/// Validate a username for NixOS user creation
pub fn validate_username(username: &str) -> Option<String> {
    if username.is_empty() {
        return Some("Username cannot be empty".to_string());
    }
    if username.len() > 32 {
        return Some("Username too long (max 32 characters)".to_string());
    }
    // Safe: we already checked that username is not empty above
    if !username.chars().next().expect("username is not empty").is_ascii_lowercase() {
        return Some("Username must start with a lowercase letter".to_string());
    }
    if !username.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-') {
        return Some("Username can only contain lowercase letters, numbers, underscore, and hyphen".to_string());
    }
    // Reserved usernames
    let reserved = ["root", "nobody", "daemon", "bin", "sys", "sync", "games", "man", "lp", "mail", "news", "uucp", "proxy", "www-data", "backup", "list", "irc", "gnats", "systemd-network", "systemd-resolve"];
    if reserved.contains(&username) {
        return Some(format!("'{}' is a reserved username", username));
    }
    None
}

/// Validate password requirements
pub fn validate_password(password: &str, confirm: &str) -> Option<String> {
    if password.is_empty() {
        return Some("Password cannot be empty".to_string());
    }
    if password.len() < 8 {
        return Some("Password must be at least 8 characters".to_string());
    }
    if password != confirm {
        return Some("Passwords do not match".to_string());
    }
    None
}

/// Configuration being built during host creation wizard
#[derive(Debug, Clone)]
pub struct NewHostConfig {
    pub hostname: String,
    pub cpu: CpuInfo,
    pub gpu: GpuInfo,
    pub form_factor: FormFactor,
    pub disk: DiskInfo,
}

/// Create host wizard state machine
/// Flow: DetectingHardware → ConfirmCpu → ConfirmGpu → ConfirmFormFactor → SelectDisk → EnterHostname → Review → Generating → Complete
#[derive(Debug, Clone)]
pub enum CreateHostState {
    DetectingHardware,
    ConfirmCpu {
        cpu: CpuInfo,
        detected_gpu: GpuInfo,
        detected_form_factor: FormFactor,
        override_menu: bool,
        selected: usize,
    },
    ConfirmGpu {
        cpu: CpuInfo,
        gpu: GpuInfo,
        detected_form_factor: FormFactor,
        override_menu: bool,
        selected: usize,
    },
    ConfirmFormFactor {
        cpu: CpuInfo,
        gpu: GpuInfo,
        form_factor: FormFactor,
        override_menu: bool,
        selected: usize,
    },
    SelectDisk {
        cpu: CpuInfo,
        gpu: GpuInfo,
        form_factor: FormFactor,
        disks: Vec<DiskInfo>,
        selected: usize,
    },
    EnterHostname {
        cpu: CpuInfo,
        gpu: GpuInfo,
        form_factor: FormFactor,
        disk: DiskInfo,
        input: String,
        error: Option<String>,
    },
    Review {
        config: NewHostConfig,
    },
    Generating {
        config: NewHostConfig,
        step: usize,
        steps: Vec<StepStatus>,
        output: VecDeque<String>,
    },
    Complete {
        success: bool,
        config: NewHostConfig,
    },
}

impl CreateHostState {
    pub fn new() -> Self {
        CreateHostState::DetectingHardware
    }
}

/// Resolution options for local git changes
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LocalChangesResolution {
    /// Discard all changes and untracked files (git reset --hard + git clean -fd)
    Overwrite,
    /// Stash changes, update, then restore (git stash push + pop)
    Stash,
    /// Cancel the update
    Cancel,
}

/// Update state machine
#[derive(Debug, Clone)]
pub enum UpdateState {
    /// Prompt user about local changes before updating
    LocalChangesPrompt {
        changed_files: Vec<String>,
        selected: usize, // 0=Overwrite, 1=Stash, 2=Cancel
    },
    Running {
        step: usize,
        steps: Vec<StepStatus>,
        output: VecDeque<String>,
        /// Whether we stashed changes that need to be restored
        stashed: bool,
    },
    Complete {
        #[allow(dead_code)]
        success: bool,
        steps: Vec<StepStatus>,
        output: VecDeque<String>,
        /// None = auto-scroll, Some(n) = manual scroll at position n
        scroll_offset: Option<usize>,
        /// Whether we stashed changes that need to be restored
        stashed: bool,
    },
}

impl UpdateState {
    pub fn new() -> Self {
        Self::new_with_stash(false)
    }

    pub fn new_with_stash(stashed: bool) -> Self {
        UpdateState::Running {
            step: 0,
            steps: vec![
                StepStatus::new("Pulling configuration updates"),
                StepStatus::new("Updating flake inputs"),
                StepStatus::new("Rebuilding system"),
                StepStatus::new("Comparing packages"),
                StepStatus::new("Updating Claude Code"),
                StepStatus::new("Updating Codex CLI"),
                StepStatus::new("Checking browser profiles"),
            ],
            output: VecDeque::new(),
            stashed,
        }
    }
}

/// App profile management state (browser sessions and logins)
#[derive(Debug, Clone)]
pub enum AppProfileState {
    Menu { selected: usize },
    Running {
        operation: AppOp,
        output: VecDeque<String>,
        force: bool,
    },
    Status {
        output: VecDeque<String>,
    },
    Complete {
        success: bool,
        output: VecDeque<String>,
        /// None = auto-scroll, Some(n) = manual scroll at position n
        scroll_offset: Option<usize>,
    },
}

impl AppProfileState {
    pub fn new_menu() -> Self {
        AppProfileState::Menu { selected: 0 }
    }

    pub fn new_backup(force: bool) -> Self {
        AppProfileState::Running {
            operation: AppOp::Backup,
            output: VecDeque::new(),
            force,
        }
    }

    pub fn new_restore(force: bool) -> Self {
        AppProfileState::Running {
            operation: AppOp::Restore,
            output: VecDeque::new(),
            force,
        }
    }

    pub fn new_status() -> Self {
        AppProfileState::Status {
            output: VecDeque::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum AppOp {
    Backup,
    Restore,
}

/// Key management state
#[derive(Debug, Clone)]
pub enum KeysState {
    Running {
        operation: KeysOp,
        output: VecDeque<String>,
        force: bool,
    },
    Complete {
        success: bool,
        output: VecDeque<String>,
        /// None = auto-scroll, Some(n) = manual scroll at position n
        scroll_offset: Option<usize>,
    },
}

impl KeysState {
    pub fn new_setup() -> Self {
        KeysState::Running {
            operation: KeysOp::Setup,
            output: VecDeque::new(),
            force: false,
        }
    }

    pub fn new_backup() -> Self {
        KeysState::Running {
            operation: KeysOp::Backup,
            output: VecDeque::new(),
            force: false,
        }
    }

    pub fn new_restore(force: bool) -> Self {
        KeysState::Running {
            operation: KeysOp::Restore,
            output: VecDeque::new(),
            force,
        }
    }

    pub fn new_status() -> Self {
        KeysState::Running {
            operation: KeysOp::Status,
            output: VecDeque::new(),
            force: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum KeysOp {
    Setup,
    Backup,
    Restore,
    Status,
}

/// Step progress status
#[derive(Debug, Clone)]
pub struct StepStatus {
    pub name: String,
    pub status: StepState,
}

impl StepStatus {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            status: StepState::Pending,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum StepState {
    Pending,
    Running,
    Complete,
    Failed,
    Skipped,
}

/// Update summary data
#[derive(Debug, Clone, Default)]
pub struct UpdateSummary {
    pub flake_changes: Vec<FlakeInputChange>,         // Flake input changes with commits
    pub package_changes: Vec<(String, String, String)>, // (pkg, old_ver, new_ver)
    pub closure_summary: Option<String>,              // nvd closure size summary
    pub claude_old: Option<String>,
    pub claude_new: Option<String>,
    pub codex_old: Option<String>,
    pub codex_new: Option<String>,
    pub browser_status: String,
    pub rebuild_skipped: bool,
    pub rebuild_failed: bool,
    pub reboot_reasons: Vec<String>,
}

/// Information about a pending commit
#[derive(Debug, Clone)]
pub struct CommitInfo {
    pub hash: String,
    pub message: String,
}

/// Tracks which updates are available for the combined dialog
#[derive(Debug, Clone, Default)]
pub struct PendingUpdates {
    pub nixos_config: bool,
    pub app_profiles: bool,
    /// Pending commits for nixos-config
    pub commits: Vec<CommitInfo>,
    /// Selected option in the dialog (0 = first option)
    pub selected: usize,
    /// True when viewing the commit list
    pub viewing_commits: bool,
    /// Scroll position in commit list
    pub commit_scroll: usize,
    /// Selected commit index
    pub selected_commit: usize,
}

impl PendingUpdates {
    pub fn has_updates(&self) -> bool {
        self.nixos_config || self.app_profiles
    }

    pub fn clear(&mut self) {
        self.nixos_config = false;
        self.app_profiles = false;
        self.commits.clear();
        self.selected = 0;
        self.viewing_commits = false;
        self.commit_scroll = 0;
        self.selected_commit = 0;
    }
}

/// Check if a host directory already exists on the filesystem
pub fn host_dir_exists(hostname: &str) -> bool {
    crate::constants::host_dir_paths(hostname)
        .iter()
        .any(|p| p.exists())
}

/// Get the number of options in the update dialog based on available updates
pub fn get_update_dialog_option_count(pending: &PendingUpdates) -> usize {
    let mut count = 0;
    if pending.nixos_config {
        count += 1; // "View NixOS updates"
    }
    if pending.app_profiles {
        count += 1; // "Update app profiles"
    }
    if pending.nixos_config && pending.app_profiles {
        count += 1; // "Update all"
    }
    count += 1; // "Dismiss"
    count
}

/// Validate a hostname for NixOS configuration
pub fn validate_hostname(hostname: &str, hosts: &[HostConfig]) -> Option<String> {
    if hostname.is_empty() {
        return Some("Hostname cannot be empty".to_string());
    }
    if hostname.len() > 63 {
        return Some("Hostname too long (max 63 characters)".to_string());
    }
    // Safe: we already checked that hostname is not empty above
    if !hostname.chars().next().expect("hostname is not empty").is_alphanumeric() {
        return Some("Hostname must start with a letter or number".to_string());
    }
    if !hostname.chars().all(|c| c.is_alphanumeric() || c == '-') {
        return Some("Hostname can only contain letters, numbers, and hyphens".to_string());
    }
    // Check if host already exists
    let host_exists = hosts.iter().any(|h| h.name == hostname) || host_dir_exists(hostname);
    if host_exists {
        return Some(format!("Host '{}' already exists", hostname));
    }
    None
}
