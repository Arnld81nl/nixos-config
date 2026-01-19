//! NixOS configuration template generators

use crate::app::NewHostConfig;
use crate::system::hardware::{CpuInfo, CpuVendor, FormFactor, GpuInfo, GpuVendor};

/// Generate the host's default.nix configuration
pub fn generate_host_default_nix(config: &NewHostConfig) -> String {
    let gpu_config = generate_gpu_config(&config.gpu);
    let form_factor_config = generate_form_factor_config(&config.form_factor);
    let cpu_config = generate_cpu_config(&config.cpu.vendor);
    let initrd_modules = generate_initrd_modules(&config.gpu);

    format!(
        r#"# {hostname} - {description}
{{ config, pkgs, lib, ... }}:

{{
  imports = [
    ./hardware-configuration.nix
    ../../modules/boot/limine-plymouth.nix
  ];

  networking.hostName = "{hostname}";
{gpu_config}{cpu_config}{form_factor_config}
  # Early KMS for Plymouth boot splash
  boot.initrd.kernelModules = lib.mkForce [
{initrd_modules}  ];
}}
"#,
        hostname = config.hostname,
        description = generate_description(config),
        gpu_config = gpu_config,
        cpu_config = cpu_config,
        form_factor_config = form_factor_config,
        initrd_modules = initrd_modules,
    )
}

/// Generate a description for the host
fn generate_description(config: &NewHostConfig) -> String {
    let form = match config.form_factor {
        FormFactor::Laptop => "Laptop",
        FormFactor::Desktop => "Desktop",
    };

    let gpu = match config.gpu.vendor {
        GpuVendor::NVIDIA => "NVIDIA GPU",
        GpuVendor::AMD => "AMD GPU",
        GpuVendor::Intel => "Intel GPU",
        GpuVendor::HybridNvidiaAmd => "Hybrid NVIDIA + AMD GPU",
        GpuVendor::None => "integrated graphics",
    };

    format!("{} with {}", form, gpu)
}

/// Generate GPU-specific configuration
fn generate_gpu_config(gpu: &GpuInfo) -> String {
    match gpu.vendor {
        GpuVendor::NVIDIA => {
            // NVIDIA config is handled via nvidia.nix module imported in flake.nix
            String::new()
        }
        GpuVendor::AMD => {
            r#"
  # AMD GPU configuration
  hardware.amdgpu.initrd.enable = true;

  boot.kernelParams = [
    "amdgpu.ppfeaturemask=0xffffffff"
  ];
"#
            .to_string()
        }
        GpuVendor::HybridNvidiaAmd => {
            let (amd_bus_id, nvidia_bus_id) = if let Some(hybrid) = &gpu.hybrid {
                (
                    hybrid.amd_bus_id.as_deref().unwrap_or("PCI:0:0:0"),
                    hybrid.nvidia_bus_id.as_deref().unwrap_or("PCI:0:0:0"),
                )
            } else {
                ("PCI:0:0:0", "PCI:0:0:0")
            };
            format!(
                r#"
  # Hybrid NVIDIA + AMD GPU configuration (PRIME offload)
  # Verify bus IDs with: lspci -nn | grep -E "VGA|3D|Display"
  hardware.amdgpu.initrd.enable = true;
  hardware.nvidia.prime = {{
    offload.enable = true;
    offload.enableOffloadCmd = true;
    amdgpuBusId = "{amd_bus_id}";
    nvidiaBusId = "{nvidia_bus_id}";
  }};

  boot.kernelParams = [
    "amdgpu.ppfeaturemask=0xffffffff"
  ];
"#,
                amd_bus_id = amd_bus_id,
                nvidia_bus_id = nvidia_bus_id
            )
        }
        GpuVendor::Intel => {
            // Intel config is handled via intel.nix module imported in flake.nix
            String::new()
        }
        GpuVendor::None => String::new(),
    }
}

/// Generate CPU-specific configuration
fn generate_cpu_config(vendor: &CpuVendor) -> String {
    match vendor {
        CpuVendor::Intel => {
            r#"
  # Intel CPU configuration (override AMD default from common.nix)
  hardware.cpu.amd.updateMicrocode = lib.mkForce false;
  hardware.cpu.intel.updateMicrocode = true;
  boot.kernelModules = [ "kvm-intel" "coretemp" ];
"#
            .to_string()
        }
        CpuVendor::AMD => {
            // AMD is default in common.nix
            String::new()
        }
        CpuVendor::Unknown => String::new(),
    }
}

/// Generate form factor-specific configuration (power management)
fn generate_form_factor_config(form_factor: &FormFactor) -> String {
    match form_factor {
        FormFactor::Laptop => {
            r#"
  # Laptop power management (TLP)
  services.power-profiles-daemon.enable = false;
  services.tlp = {
    enable = true;
    settings = {
      CPU_SCALING_GOVERNOR_ON_AC = "performance";
      CPU_SCALING_GOVERNOR_ON_BAT = "powersave";
      CPU_ENERGY_PERF_POLICY_ON_AC = "performance";
      CPU_ENERGY_PERF_POLICY_ON_BAT = "power";
      CPU_BOOST_ON_AC = 1;
      CPU_BOOST_ON_BAT = 0;
      PLATFORM_PROFILE_ON_AC = "performance";
      PLATFORM_PROFILE_ON_BAT = "low-power";
      START_CHARGE_THRESH_BAT0 = 20;
      STOP_CHARGE_THRESH_BAT0 = 80;
      WIFI_PWR_ON_AC = "off";
      WIFI_PWR_ON_BAT = "on";
      RUNTIME_PM_ON_AC = "auto";
      RUNTIME_PM_ON_BAT = "auto";
      USB_AUTOSUSPEND = 1;
    };
  };
"#
            .to_string()
        }
        FormFactor::Desktop => {
            // Desktop uses power-profiles-daemon (default from common.nix)
            String::new()
        }
    }
}

/// Generate initrd kernel modules based on GPU type
fn generate_initrd_modules(gpu: &GpuInfo) -> String {
    match gpu.vendor {
        GpuVendor::NVIDIA => {
            r#"    "nvidia"
    "nvidia_modeset"
    "nvidia_uvm"
    "nvidia_drm"
    "hid-generic"
    "usbhid"
"#
            .to_string()
        }
        GpuVendor::AMD => {
            r#"    "amdgpu"
    "hid-generic"
    "usbhid"
"#
            .to_string()
        }
        GpuVendor::HybridNvidiaAmd => {
            r#"    "nvidia"
    "nvidia_modeset"
    "nvidia_uvm"
    "nvidia_drm"
    "amdgpu"
    "hid-generic"
    "usbhid"
"#
            .to_string()
        }
        GpuVendor::Intel => {
            r#"    "i915"
    "hid-generic"
    "usbhid"
"#
            .to_string()
        }
        GpuVendor::None => {
            r#"    "hid-generic"
    "usbhid"
"#
            .to_string()
        }
    }
}

/// Generate disko configuration for the host
pub fn generate_disko_config(hostname: &str, disk_path: &str) -> String {
    format!(
        r#"# Disko configuration for {hostname}
{{ ... }}:

{{
  imports = [ ./default.nix ];

  disko.devices.disk.main.device = "{disk_path}";
}}
"#,
        hostname = hostname,
        disk_path = disk_path,
    )
}

/// Generate hardware-configuration.nix template
pub fn generate_hardware_config(cpu: &CpuInfo, hostname: &str) -> String {
    let kvm_module = match cpu.vendor {
        CpuVendor::AMD => "kvm-amd",
        CpuVendor::Intel => "kvm-intel",
        CpuVendor::Unknown => "kvm-amd",
    };

    let cpu_vendor = match cpu.vendor {
        CpuVendor::AMD => "amd",
        CpuVendor::Intel => "intel",
        CpuVendor::Unknown => "amd",
    };

    format!(
        r#"# Hardware configuration for {hostname}
# Note: Run `nixos-generate-config --no-filesystems` on the target system
# to generate accurate hardware detection, then merge with this template.
{{ config, lib, pkgs, modulesPath, ... }}:

{{
  imports = [
    (modulesPath + "/installer/scan/not-detected.nix")
  ];

  boot.initrd.availableKernelModules = [ "nvme" "xhci_pci" "ahci" "thunderbolt" "usbhid" "uas" "sd_mod" "btrfs" ];
  boot.initrd.kernelModules = [ ];
  boot.kernelModules = [ "{kvm_module}" ];
  boot.extraModulePackages = [ ];

  nixpkgs.hostPlatform = lib.mkDefault "x86_64-linux";
  hardware.cpu.{cpu_vendor}.updateMicrocode = lib.mkDefault config.hardware.enableRedistributableFirmware;
}}
"#,
        hostname = hostname,
        kvm_module = kvm_module,
        cpu_vendor = cpu_vendor,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::system::disk::DiskInfo;
    use crate::system::hardware::GpuInfo;

    #[test]
    fn test_generate_disko_config() {
        let config = generate_disko_config("testhost", "/dev/nvme0n1");
        assert!(config.contains("testhost"));
        assert!(config.contains("/dev/nvme0n1"));
    }

    #[test]
    fn test_generate_host_default_nix_nvidia() {
        let config = NewHostConfig {
            hostname: "testhost".to_string(),
            cpu: CpuInfo {
                vendor: CpuVendor::AMD,
                model_name: "AMD Ryzen".to_string(),
            },
            gpu: GpuInfo {
                vendor: GpuVendor::NVIDIA,
                model: Some("RTX 5090".to_string()),
                hybrid: None,
            },
            form_factor: FormFactor::Desktop,
            disk: DiskInfo {
                path: "/dev/nvme0n1".to_string(),
                size: "1TB".to_string(),
                size_bytes: 0,
                model: None,
                partitions: vec![],
            },
        };

        let result = generate_host_default_nix(&config);
        assert!(result.contains("testhost"));
        assert!(result.contains("nvidia"));
        assert!(result.contains("nvidia_modeset"));
    }

    #[test]
    fn test_generate_host_default_nix_amd_laptop() {
        let config = NewHostConfig {
            hostname: "laptop".to_string(),
            cpu: CpuInfo {
                vendor: CpuVendor::AMD,
                model_name: "AMD Ryzen".to_string(),
            },
            gpu: GpuInfo {
                vendor: GpuVendor::AMD,
                model: Some("RX 7900".to_string()),
                hybrid: None,
            },
            form_factor: FormFactor::Laptop,
            disk: DiskInfo {
                path: "/dev/nvme0n1".to_string(),
                size: "512GB".to_string(),
                size_bytes: 0,
                model: None,
                partitions: vec![],
            },
        };

        let result = generate_host_default_nix(&config);
        assert!(result.contains("laptop"));
        assert!(result.contains("amdgpu"));
        assert!(result.contains("tlp"));
        assert!(result.contains("power-profiles-daemon.enable = false"));
    }
}
