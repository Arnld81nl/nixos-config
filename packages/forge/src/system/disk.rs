//! Disk enumeration utilities

use anyhow::Result;
use serde::Deserialize;
use std::process::Command;

/// Detected operating system type on a partition
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OsType {
    NixOS,
    Fedora,
    Ubuntu,
    Debian,
    Arch,
    Windows,
    Other(String),
    Unknown,
}

impl std::fmt::Display for OsType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OsType::NixOS => write!(f, "NixOS"),
            OsType::Fedora => write!(f, "Fedora"),
            OsType::Ubuntu => write!(f, "Ubuntu"),
            OsType::Debian => write!(f, "Debian"),
            OsType::Arch => write!(f, "Arch"),
            OsType::Windows => write!(f, "Windows"),
            OsType::Other(name) => write!(f, "{}", name),
            OsType::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Information about a partition on a disk
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartitionInfo {
    pub path: String,
    pub size: String,
    pub fstype: String,
    pub label: Option<String>,
    pub os_type: Option<OsType>,
}

/// Information about a disk device
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiskInfo {
    pub path: String,
    pub size: String,
    pub size_bytes: u64,
    pub model: Option<String>,
    pub partitions: Vec<PartitionInfo>,
}

/// JSON structure for lsblk output
#[derive(Debug, Deserialize)]
struct LsblkOutput {
    blockdevices: Vec<BlockDevice>,
}

#[derive(Debug, Deserialize)]
struct BlockDevice {
    name: String,
    size: Option<String>,
    model: Option<String>,
    #[serde(rename = "type")]
    device_type: Option<String>,
    fstype: Option<String>,
    label: Option<String>,
    #[serde(default)]
    children: Vec<BlockDevice>,
}

/// Get list of available disks (excluding loop, ram, rom, zram devices)
pub fn get_available_disks() -> Result<Vec<DiskInfo>> {
    // Use JSON output for reliable parsing (handles model names with spaces)
    // Include children to get partition info
    let output = Command::new("lsblk")
        .args(["-J", "-o", "NAME,SIZE,MODEL,TYPE,FSTYPE,LABEL"])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut disks = Vec::new();

    // Parse JSON output
    let lsblk: LsblkOutput = match serde_json::from_str(&stdout) {
        Ok(v) => v,
        Err(_) => {
            // Fallback to text parsing if JSON fails
            return get_available_disks_text_fallback();
        }
    };

    for device in lsblk.blockdevices {
        // Check if it's a disk
        if device.device_type.as_deref() != Some("disk") {
            continue;
        }

        let name = &device.name;

        // Skip non-physical devices
        if name.starts_with("loop")
            || name.starts_with("ram")
            || name.starts_with("zram")
            || name.starts_with("sr")
            || name.starts_with("fd")
        {
            continue;
        }

        let path = format!("/dev/{}", name);
        let size = device.size.clone().unwrap_or_default();
        let size_bytes = parse_size(&size);

        // Clean up model name (remove extra whitespace)
        let model = device
            .model
            .as_ref()
            .map(|m| m.trim().to_string())
            .filter(|m| !m.is_empty());

        // Process partitions (children)
        let partitions = process_partitions(&device.children);

        disks.push(DiskInfo {
            path,
            size,
            size_bytes,
            model,
            partitions,
        });
    }

    // Sort by size (largest first)
    disks.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));

    Ok(disks)
}

/// Process partition children from lsblk output
fn process_partitions(children: &[BlockDevice]) -> Vec<PartitionInfo> {
    children
        .iter()
        .filter(|child| child.device_type.as_deref() == Some("part"))
        .map(|child| {
            let path = format!("/dev/{}", child.name);
            let fstype = child.fstype.clone().unwrap_or_default();
            let label = child.label.clone();

            // Detect OS type based on filesystem and by probing
            let os_type = detect_os_type(&path, &fstype);

            PartitionInfo {
                path,
                size: child.size.clone().unwrap_or_default(),
                fstype,
                label,
                os_type,
            }
        })
        .collect()
}

/// Detect OS type on a partition
fn detect_os_type(partition_path: &str, fstype: &str) -> Option<OsType> {
    // NTFS is almost always Windows
    if fstype == "ntfs" {
        return Some(OsType::Windows);
    }

    // EFI partition - skip OS detection
    if fstype == "vfat" {
        return None;
    }

    // For Linux filesystems, try to detect the distro
    if matches!(fstype, "ext4" | "ext3" | "btrfs" | "xfs" | "f2fs") {
        return detect_linux_os(partition_path);
    }

    None
}

/// Try to detect Linux distro by mounting and checking /etc/os-release
fn detect_linux_os(partition_path: &str) -> Option<OsType> {
    use std::fs;
    use std::path::Path;

    // Create a unique temp mount point
    let mount_point = format!("/tmp/forge-detect-{}", std::process::id());

    // Create mount directory
    if fs::create_dir_all(&mount_point).is_err() {
        return None;
    }

    // Try to mount read-only
    let mount_result = Command::new("mount")
        .args(["-o", "ro,noexec,nosuid", partition_path, &mount_point])
        .output();

    let os_type = if mount_result.is_ok() && mount_result.unwrap().status.success() {
        // Check for NixOS first (has /etc/nixos directory)
        let nixos_path = Path::new(&mount_point).join("etc/nixos");
        if nixos_path.exists() {
            Some(OsType::NixOS)
        } else {
            // Check /etc/os-release
            let os_release_path = Path::new(&mount_point).join("etc/os-release");
            if let Ok(content) = fs::read_to_string(&os_release_path) {
                parse_os_release(&content)
            } else {
                None
            }
        }
    } else {
        None
    };

    // Always unmount and cleanup
    let _ = Command::new("umount").arg(&mount_point).output();
    let _ = fs::remove_dir(&mount_point);

    os_type
}

/// Parse /etc/os-release content to determine OS type
fn parse_os_release(content: &str) -> Option<OsType> {
    for line in content.lines() {
        if let Some(id) = line.strip_prefix("ID=") {
            let id = id.trim().trim_matches('"').to_lowercase();
            return Some(match id.as_str() {
                "nixos" => OsType::NixOS,
                "fedora" => OsType::Fedora,
                "ubuntu" => OsType::Ubuntu,
                "debian" => OsType::Debian,
                "arch" | "archlinux" => OsType::Arch,
                "opensuse" | "opensuse-leap" | "opensuse-tumbleweed" => {
                    OsType::Other("openSUSE".to_string())
                }
                "manjaro" => OsType::Other("Manjaro".to_string()),
                "pop" => OsType::Other("Pop!_OS".to_string()),
                "linuxmint" | "mint" => OsType::Other("Linux Mint".to_string()),
                "elementary" => OsType::Other("elementary OS".to_string()),
                "gentoo" => OsType::Other("Gentoo".to_string()),
                "void" => OsType::Other("Void Linux".to_string()),
                other if !other.is_empty() => OsType::Other(other.to_string()),
                _ => OsType::Unknown,
            });
        }
    }
    None
}

/// Fallback text-based parsing for older lsblk versions without JSON support
fn get_available_disks_text_fallback() -> Result<Vec<DiskInfo>> {
    let output = Command::new("lsblk")
        .args(["-dno", "NAME,SIZE,TYPE"])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut disks = Vec::new();

    for line in stdout.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 3 {
            continue;
        }

        let name = parts[0];
        let size = parts[1];
        let disk_type = parts[2];

        if disk_type != "disk" {
            continue;
        }

        // Skip non-physical devices
        if name.starts_with("loop")
            || name.starts_with("ram")
            || name.starts_with("zram")
            || name.starts_with("sr")
            || name.starts_with("fd")
        {
            continue;
        }

        let path = format!("/dev/{}", name);
        let size_bytes = parse_size(size);

        disks.push(DiskInfo {
            path,
            size: size.to_string(),
            size_bytes,
            model: None,       // Can't reliably parse model in text mode
            partitions: vec![], // No partition info in text fallback mode
        });
    }

    // Sort by size (largest first)
    disks.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));

    Ok(disks)
}

/// Parse size string like "1.8T" to bytes
fn parse_size(size: &str) -> u64 {
    let size = size.trim();
    if size.is_empty() {
        return 0;
    }

    let (num_str, unit) = if size.ends_with('T') {
        (&size[..size.len() - 1], 1024u64 * 1024 * 1024 * 1024)
    } else if size.ends_with('G') {
        (&size[..size.len() - 1], 1024u64 * 1024 * 1024)
    } else if size.ends_with('M') {
        (&size[..size.len() - 1], 1024u64 * 1024)
    } else if size.ends_with('K') {
        (&size[..size.len() - 1], 1024u64)
    } else {
        (size, 1u64)
    };

    match num_str.parse::<f64>() {
        Ok(num) => (num * unit as f64) as u64,
        Err(e) => {
            tracing::warn!("Failed to parse disk size '{}': {}", size, e);
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_size_terabytes() {
        assert_eq!(parse_size("1T"), 1024 * 1024 * 1024 * 1024);
        assert_eq!(parse_size("2T"), 2 * 1024 * 1024 * 1024 * 1024);
        // Fractional terabytes
        let size_1_8t = parse_size("1.8T");
        assert!(size_1_8t > 1024 * 1024 * 1024 * 1024, "1.8T should be > 1TB");
        assert!(size_1_8t < 2 * 1024 * 1024 * 1024 * 1024, "1.8T should be < 2TB");
    }

    #[test]
    fn test_parse_size_gigabytes() {
        assert_eq!(parse_size("500G"), 500 * 1024 * 1024 * 1024);
        assert_eq!(parse_size("1G"), 1024 * 1024 * 1024);
        // Fractional gigabytes
        let size_1_5g = parse_size("1.5G");
        assert!(size_1_5g > 1024 * 1024 * 1024, "1.5G should be > 1GB");
        assert!(size_1_5g < 2 * 1024 * 1024 * 1024, "1.5G should be < 2GB");
    }

    #[test]
    fn test_parse_size_megabytes() {
        assert_eq!(parse_size("256M"), 256 * 1024 * 1024);
        assert_eq!(parse_size("512M"), 512 * 1024 * 1024);
        assert_eq!(parse_size("1M"), 1024 * 1024);
    }

    #[test]
    fn test_parse_size_kilobytes() {
        assert_eq!(parse_size("1K"), 1024);
        assert_eq!(parse_size("512K"), 512 * 1024);
    }

    #[test]
    fn test_parse_size_bytes() {
        assert_eq!(parse_size("1024"), 1024);
        assert_eq!(parse_size("512"), 512);
    }

    #[test]
    fn test_parse_size_edge_cases() {
        assert_eq!(parse_size(""), 0);
        assert_eq!(parse_size("   "), 0);
        assert_eq!(parse_size("invalid"), 0);
        assert_eq!(parse_size("0G"), 0);
    }

    #[test]
    fn test_os_type_display() {
        assert_eq!(format!("{}", OsType::NixOS), "NixOS");
        assert_eq!(format!("{}", OsType::Fedora), "Fedora");
        assert_eq!(format!("{}", OsType::Ubuntu), "Ubuntu");
        assert_eq!(format!("{}", OsType::Debian), "Debian");
        assert_eq!(format!("{}", OsType::Arch), "Arch");
        assert_eq!(format!("{}", OsType::Windows), "Windows");
        assert_eq!(format!("{}", OsType::Other("Pop!_OS".to_string())), "Pop!_OS");
        assert_eq!(format!("{}", OsType::Unknown), "Unknown");
    }

    #[test]
    fn test_os_type_equality() {
        assert_eq!(OsType::NixOS, OsType::NixOS);
        assert_ne!(OsType::NixOS, OsType::Ubuntu);
        assert_eq!(OsType::Other("Foo".to_string()), OsType::Other("Foo".to_string()));
        assert_ne!(OsType::Other("Foo".to_string()), OsType::Other("Bar".to_string()));
    }

    #[test]
    fn test_parse_os_release_nixos() {
        let content = r#"ID=nixos
NAME="NixOS"
VERSION="23.11"
"#;
        let os_type = parse_os_release(content);
        assert_eq!(os_type, Some(OsType::NixOS));
    }

    #[test]
    fn test_parse_os_release_fedora() {
        let content = r#"ID=fedora
NAME="Fedora Linux"
VERSION="39"
"#;
        let os_type = parse_os_release(content);
        assert_eq!(os_type, Some(OsType::Fedora));
    }

    #[test]
    fn test_parse_os_release_ubuntu() {
        let content = r#"ID=ubuntu
NAME="Ubuntu"
VERSION="22.04"
"#;
        let os_type = parse_os_release(content);
        assert_eq!(os_type, Some(OsType::Ubuntu));
    }

    #[test]
    fn test_parse_os_release_arch() {
        let content = r#"ID=arch
NAME="Arch Linux"
"#;
        let os_type = parse_os_release(content);
        assert_eq!(os_type, Some(OsType::Arch));
    }

    #[test]
    fn test_parse_os_release_archlinux() {
        let content = r#"ID=archlinux
NAME="Arch Linux"
"#;
        let os_type = parse_os_release(content);
        assert_eq!(os_type, Some(OsType::Arch));
    }

    #[test]
    fn test_parse_os_release_other() {
        let content = r#"ID=manjaro
NAME="Manjaro Linux"
"#;
        let os_type = parse_os_release(content);
        assert_eq!(os_type, Some(OsType::Other("Manjaro".to_string())));
    }

    #[test]
    fn test_parse_os_release_quoted_id() {
        let content = r#"ID="fedora"
NAME="Fedora Linux"
"#;
        let os_type = parse_os_release(content);
        assert_eq!(os_type, Some(OsType::Fedora));
    }

    #[test]
    fn test_parse_os_release_empty() {
        let content = "";
        let os_type = parse_os_release(content);
        assert_eq!(os_type, None);
    }

    #[test]
    fn test_parse_os_release_no_id() {
        let content = r#"NAME="Some Linux"
VERSION="1.0"
"#;
        let os_type = parse_os_release(content);
        assert_eq!(os_type, None);
    }

    #[test]
    fn test_disk_info_clone() {
        let disk = DiskInfo {
            path: "/dev/nvme0n1".to_string(),
            size: "1T".to_string(),
            size_bytes: 1024 * 1024 * 1024 * 1024,
            model: Some("Samsung SSD".to_string()),
            partitions: vec![],
        };
        let cloned = disk.clone();
        assert_eq!(cloned.path, "/dev/nvme0n1");
        assert_eq!(cloned.size, "1T");
        assert_eq!(cloned.model, Some("Samsung SSD".to_string()));
    }

    #[test]
    fn test_partition_info_clone() {
        let partition = PartitionInfo {
            path: "/dev/nvme0n1p1".to_string(),
            size: "512M".to_string(),
            fstype: "vfat".to_string(),
            label: Some("EFI".to_string()),
            os_type: None,
        };
        let cloned = partition.clone();
        assert_eq!(cloned.path, "/dev/nvme0n1p1");
        assert_eq!(cloned.fstype, "vfat");
        assert_eq!(cloned.label, Some("EFI".to_string()));
    }

    #[test]
    fn test_disk_info_equality() {
        let disk1 = DiskInfo {
            path: "/dev/sda".to_string(),
            size: "500G".to_string(),
            size_bytes: 500 * 1024 * 1024 * 1024,
            model: None,
            partitions: vec![],
        };
        let disk2 = disk1.clone();
        assert_eq!(disk1, disk2);
    }
}
