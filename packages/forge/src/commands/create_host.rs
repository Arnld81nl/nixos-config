//! Create new host configuration command

use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use tokio::sync::mpsc;

use super::errors::{ErrorContext, ParsedError};
use super::executor::run_command;
use super::CommandMessage;
use crate::app::{AppMode, CreateHostState, NewHostConfig};
use crate::system::hardware::{FormFactor, GpuVendor};
use crate::templates;

const REPO_URL: &str = "https://github.com/Arnld81nl/nixos-config.git";

/// Start the create host process
pub async fn start_create_host(tx: mpsc::Sender<CommandMessage>, mode: AppMode) -> Result<()> {
    // Extract config from mode
    let config = match mode {
        AppMode::CreateHost(CreateHostState::Generating { config, .. }) => config,
        _ => {
            let _ = tx
                .send(CommandMessage::StepFailed {
                    step: "host".to_string(),
                    error: ParsedError::from_stderr(
                        "Invalid state for create_host",
                        ErrorContext {
                            operation: "Create host".to_string(),
                        },
                    ),
                })
                .await;
            let _ = tx.send(CommandMessage::Done { success: false }).await;
            return Ok(());
        }
    };

    tokio::spawn(async move {
        if let Err(e) = run_create_host(&tx, &config).await {
            tracing::error!("Create host failed: {}", e);
            let _ = tx
                .send(CommandMessage::StepFailed {
                    step: "configuration".to_string(),
                    error: ParsedError::from_stderr(
                        &e.to_string(),
                        ErrorContext {
                            operation: "Create host".to_string(),
                        },
                    ),
                })
                .await;
            let _ = tx.send(CommandMessage::Done { success: false }).await;
        }
    });
    Ok(())
}

async fn run_create_host(
    tx: &mpsc::Sender<CommandMessage>,
    config: &NewHostConfig,
) -> Result<()> {
    // Clone repository if running from live ISO and no config exists
    if crate::system::is_live_iso_environment() {
        let temp_config = crate::constants::temp_config_dir();
        if !temp_config.join(crate::constants::FLAKE_NIX).exists() {
            tx.send(CommandMessage::Stdout(
                "Cloning configuration repository...".to_string(),
            ))
            .await?;

            let _ = fs::remove_dir_all(&temp_config);
            let temp_config_str = temp_config.to_string_lossy();
            std::env::set_var("NIX_CONFIG", "experimental-features = nix-command flakes");

            let success = run_command(
                tx,
                "nix-shell",
                &[
                    "-p",
                    "git",
                    "--run",
                    &format!("git clone --depth 1 {} {}", REPO_URL, temp_config_str),
                ],
            )
            .await?;

            if !success {
                anyhow::bail!("Failed to clone repository");
            }

            tx.send(CommandMessage::StepComplete {
                step: "repository".to_string(),
            })
            .await?;
        }
    }

    // Determine the config directory
    // When running from live ISO, we clone to /tmp first
    // When running from installed system, we use the actual path
    let config_dir = get_config_dir()?;

    tx.send(CommandMessage::Stdout(format!(
        "Creating host configuration for '{}'...",
        config.hostname
    )))
    .await?;

    // Step 1: Create host directory
    tx.send(CommandMessage::Stdout(format!(
        "Creating hosts/{}/...",
        config.hostname
    )))
    .await?;

    let host_dir = format!("{}/hosts/{}", config_dir, config.hostname);
    fs::create_dir_all(&host_dir)
        .with_context(|| format!("Failed to create host directory: {}", host_dir))?;

    tx.send(CommandMessage::StepComplete {
        step: "host".to_string(),
    })
    .await?;

    // Step 2: Generate hardware-configuration.nix
    tx.send(CommandMessage::Stdout(
        "Generating hardware configuration...".to_string(),
    ))
    .await?;

    let hw_config_path = format!("{}/hardware-configuration.nix", host_dir);

    // Try to use nixos-generate-config for accurate hardware detection
    // Falls back to template if not available (e.g., non-NixOS live environment)
    let hw_config_generated = match generate_hw_config_from_system(tx, &hw_config_path).await {
        Ok(()) => true,
        Err(e) => {
            tx.send(CommandMessage::Stdout(format!(
                "Note: Using template hardware config ({})",
                e
            )))
            .await?;
            false
        }
    };

    if !hw_config_generated {
        // Use template-based hardware config
        let hw_config = templates::generate_hardware_config(&config.cpu, &config.hostname);
        fs::write(&hw_config_path, hw_config)
            .with_context(|| format!("Failed to write hardware config: {}", hw_config_path))?;
    }

    tx.send(CommandMessage::StepComplete {
        step: "hardware".to_string(),
    })
    .await?;

    // Step 3: Generate host default.nix
    tx.send(CommandMessage::Stdout(
        "Creating host configuration...".to_string(),
    ))
    .await?;

    let default_nix_path = format!("{}/default.nix", host_dir);
    let default_nix = templates::generate_host_default_nix(config);
    fs::write(&default_nix_path, default_nix)
        .with_context(|| format!("Failed to write default.nix: {}", default_nix_path))?;

    tx.send(CommandMessage::StepComplete {
        step: "host config".to_string(),
    })
    .await?;

    // Step 4: Create disko configuration
    tx.send(CommandMessage::Stdout(
        "Creating disko configuration...".to_string(),
    ))
    .await?;

    let disko_path = format!("{}/modules/disko/{}.nix", config_dir, config.hostname);
    let disko_config = templates::generate_disko_config(&config.hostname, &config.disk.path);
    fs::write(&disko_path, disko_config)
        .with_context(|| format!("Failed to write disko config: {}", disko_path))?;

    tx.send(CommandMessage::StepComplete {
        step: "disko".to_string(),
    })
    .await?;

    // Step 5: Update flake.nix
    tx.send(CommandMessage::Stdout("Updating flake.nix...".to_string()))
        .await?;

    let flake_path = format!("{}/flake.nix", config_dir);
    let flake_content = fs::read_to_string(&flake_path)
        .with_context(|| format!("Failed to read flake.nix: {}", flake_path))?;

    let updated_flake = update_flake_nix(&flake_content, config)?;
    fs::write(&flake_path, updated_flake)
        .with_context(|| format!("Failed to write flake.nix: {}", flake_path))?;

    tx.send(CommandMessage::StepComplete {
        step: "flake".to_string(),
    })
    .await?;

    // Step 6: Generate host-info.json metadata
    tx.send(CommandMessage::Stdout(
        "Generating host metadata...".to_string(),
    ))
    .await?;

    write_host_metadata(&host_dir, config)?;

    tx.send(CommandMessage::StepComplete {
        step: "metadata".to_string(),
    })
    .await?;

    // Success message
    tx.send(CommandMessage::Stdout("\n".to_string())).await?;
    tx.send(CommandMessage::Stdout(format!(
        "Host '{}' created successfully!",
        config.hostname
    )))
    .await?;
    tx.send(CommandMessage::Stdout("".to_string())).await?;
    tx.send(CommandMessage::Stdout("Configuration summary:".to_string()))
        .await?;
    tx.send(CommandMessage::Stdout(format!(
        "  CPU: {} ({})",
        config.cpu.vendor, config.cpu.model_name
    )))
    .await?;
    tx.send(CommandMessage::Stdout(format!(
        "  GPU: {}{}",
        config.gpu.vendor,
        config
            .gpu
            .model
            .as_ref()
            .map(|m| format!(" ({})", m))
            .unwrap_or_default()
    )))
    .await?;
    tx.send(CommandMessage::Stdout(format!(
        "  Form factor: {}",
        config.form_factor
    )))
    .await?;
    tx.send(CommandMessage::Stdout(format!(
        "  Disk: {} ({})",
        config.disk.path, config.disk.size
    )))
    .await?;

    tx.send(CommandMessage::Done { success: true }).await?;
    Ok(())
}

/// Get the configuration directory path
fn get_config_dir() -> Result<String> {
    // First try PID-specific temp directory (for concurrent installs)
    let temp_dir = crate::constants::temp_config_dir();
    if temp_dir.join(crate::constants::FLAKE_NIX).exists() {
        return Ok(temp_dir.to_string_lossy().to_string());
    }

    // Try static temp location
    let temp_static = Path::new(crate::constants::NIXOS_CONFIG_TEMP);
    if temp_static.join(crate::constants::FLAKE_NIX).exists() {
        return Ok(temp_static.to_string_lossy().to_string());
    }

    // Use the standard nixos_config_dir which checks ~/nixos-config and /etc/nixos
    let config_dir = crate::constants::nixos_config_dir();
    if config_dir.join(crate::constants::FLAKE_NIX).exists() {
        return Ok(config_dir.to_string_lossy().to_string());
    }

    // If none found, try to get the current working directory
    let cwd = std::env::current_dir()?;
    if cwd.join(crate::constants::FLAKE_NIX).exists() {
        return Ok(cwd.to_string_lossy().to_string());
    }

    anyhow::bail!(
        "Could not find NixOS configuration directory. \
        Please run from within the nixos-config repository."
    )
}

/// Generate hardware configuration using nixos-generate-config
async fn generate_hw_config_from_system(
    tx: &mpsc::Sender<CommandMessage>,
    output_path: &str,
) -> Result<()> {
    // Create a temp directory for the generated config
    let temp_dir = format!("/tmp/hw-config-{}", std::process::id());
    fs::create_dir_all(&temp_dir)?;

    // Run nixos-generate-config
    let success = run_command(
        tx,
        "nixos-generate-config",
        &["--no-filesystems", "--dir", &temp_dir],
    )
    .await?;

    if !success {
        anyhow::bail!("nixos-generate-config failed");
    }

    // Copy the generated hardware-configuration.nix
    let generated_path = format!("{}/hardware-configuration.nix", temp_dir);
    fs::copy(&generated_path, output_path)?;

    // Clean up
    let _ = fs::remove_dir_all(&temp_dir);

    Ok(())
}

/// Update flake.nix to add the new host
fn update_flake_nix(content: &str, config: &NewHostConfig) -> Result<String> {
    // Generate the new host entry
    // Only include extraModules if we need hardware-specific modules
    let extra_modules_line = match config.gpu.vendor {
        GpuVendor::NVIDIA | GpuVendor::HybridNvidiaAmd => {
            "\n        extraModules = [ ./modules/hardware/nvidia.nix ];"
        }
        GpuVendor::Intel => "\n        extraModules = [ ./modules/hardware/intel.nix ];",
        _ => "", // AMD and None don't need extra modules
    };

    let description = match (&config.gpu.vendor, &config.form_factor) {
        (GpuVendor::NVIDIA, FormFactor::Desktop) => format!("{} - Desktop with NVIDIA GPU", config.hostname),
        (GpuVendor::NVIDIA, FormFactor::Laptop) => format!("{} - Laptop with NVIDIA GPU", config.hostname),
        (GpuVendor::HybridNvidiaAmd, FormFactor::Desktop) => {
            format!("{} - Desktop with Hybrid NVIDIA + AMD GPU", config.hostname)
        }
        (GpuVendor::HybridNvidiaAmd, FormFactor::Laptop) => {
            format!("{} - Laptop with Hybrid NVIDIA + AMD GPU", config.hostname)
        }
        (GpuVendor::AMD, FormFactor::Desktop) => format!("{} - Desktop with AMD GPU", config.hostname),
        (GpuVendor::AMD, FormFactor::Laptop) => format!("{} - Laptop with AMD GPU", config.hostname),
        (GpuVendor::Intel, FormFactor::Desktop) => format!("{} - Desktop with Intel GPU", config.hostname),
        (GpuVendor::Intel, FormFactor::Laptop) => format!("{} - Laptop with Intel GPU", config.hostname),
        (GpuVendor::None, FormFactor::Desktop) => format!("{} - Desktop", config.hostname),
        (GpuVendor::None, FormFactor::Laptop) => format!("{} - Laptop", config.hostname),
    };

    let host_entry = format!(
        r#"
      # {}
      {} = mkNixosSystem {{
        hostname = "{}";{}
      }};"#,
        description, config.hostname, config.hostname, extra_modules_line
    );

    // Find the nixosConfigurations block and insert before the closing brace
    // Look for the pattern: "nixosConfigurations = {" ... "};"
    // We want to insert just before the final "};" of nixosConfigurations

    // Find the end of nixosConfigurations block
    if let Some(configs_start) = content.find("nixosConfigurations = {") {
        // Find matching closing brace
        let after_start = &content[configs_start..];

        // Count braces to find the matching close
        let mut brace_count = 0;
        let mut insert_pos = None;

        for (i, c) in after_start.char_indices() {
            if c == '{' {
                brace_count += 1;
            } else if c == '}' {
                brace_count -= 1;
                if brace_count == 0 {
                    // Found the closing brace
                    insert_pos = Some(configs_start + i);
                    break;
                }
            }
        }

        if let Some(pos) = insert_pos {
            let mut result = String::new();
            result.push_str(&content[..pos]);
            result.push_str(&host_entry);
            result.push('\n');
            result.push_str("    ");
            result.push_str(&content[pos..]);
            return Ok(result);
        }
    }

    anyhow::bail!("Could not find nixosConfigurations block in flake.nix")
}

/// Write host-info.json metadata file
fn write_host_metadata(host_dir: &str, config: &NewHostConfig) -> Result<()> {
    // Detect RAM
    let ram = detect_ram();

    let metadata = serde_json::json!({
        "cpu": {
            "vendor": format!("{}", config.cpu.vendor),
            "model": config.cpu.model_name
        },
        "gpu": {
            "vendor": format!("{}", config.gpu.vendor),
            "model": config.gpu.model
        },
        "form_factor": format!("{}", config.form_factor),
        "ram": ram
    });

    let metadata_path = format!("{}/host-info.json", host_dir);
    let content = serde_json::to_string_pretty(&metadata)?;
    fs::write(&metadata_path, content)
        .with_context(|| format!("Failed to write host metadata: {}", metadata_path))?;

    Ok(())
}

/// Detect total RAM from /proc/meminfo
fn detect_ram() -> Option<String> {
    if let Ok(content) = fs::read_to_string("/proc/meminfo") {
        for line in content.lines() {
            if line.starts_with("MemTotal:") {
                // Parse "MemTotal:       32456789 kB"
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    if let Ok(kb) = parts[1].parse::<u64>() {
                        let gb = kb / 1024 / 1024;
                        return Some(format!("{} GB", gb));
                    }
                }
            }
        }
    }
    None
}
