//! Network connectivity utilities

#![allow(dead_code)]

use anyhow::Result;
use std::process::Command;

/// Check if network is available by pinging github.com
pub fn check_connectivity() -> Result<bool> {
    let status = Command::new("ping")
        .args(["-c", "1", "-W", "5", "github.com"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()?;

    Ok(status.success())
}

/// Get current hostname
pub fn get_hostname() -> Result<String> {
    let output = Command::new("hostname").output()?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
