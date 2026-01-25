//! System utilities

pub mod config;
pub mod disk;
pub mod hardware;
pub mod network;

/// Check if we're running from a NixOS Live ISO environment
pub fn is_live_iso_environment() -> bool {
    use std::path::Path;

    // Check for read-only Nix store (typical for ISO)
    if Path::new("/nix/.ro-store").exists() {
        return true;
    }

    // Check if NIXOS_LUSTRATE exists (only on installed systems)
    if Path::new("/etc/NIXOS_LUSTRATE").exists() {
        return false;
    }

    // Check root filesystem type - squashfs/tmpfs/overlay indicates live system
    if let Ok(mounts) = std::fs::read_to_string("/proc/mounts") {
        for line in mounts.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 && parts[1] == "/" {
                let fstype = parts[2];
                if fstype == "squashfs" || fstype == "tmpfs" || fstype == "overlay" {
                    return true;
                }
            }
        }
    }

    false
}
