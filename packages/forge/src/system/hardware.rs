//! Hardware detection utilities for CPU, GPU, and form factor

use anyhow::Result;
use std::fs;
use std::process::Command;

/// CPU vendor types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpuVendor {
    AMD,
    Intel,
    Unknown,
}

impl std::fmt::Display for CpuVendor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CpuVendor::AMD => write!(f, "AMD"),
            CpuVendor::Intel => write!(f, "Intel"),
            CpuVendor::Unknown => write!(f, "Unknown"),
        }
    }
}

/// GPU vendor types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuVendor {
    NVIDIA,
    AMD,
    Intel,
    HybridNvidiaAmd,
    None,
}

impl std::fmt::Display for GpuVendor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GpuVendor::NVIDIA => write!(f, "NVIDIA"),
            GpuVendor::AMD => write!(f, "AMD"),
            GpuVendor::Intel => write!(f, "Intel"),
            GpuVendor::HybridNvidiaAmd => write!(f, "Hybrid (NVIDIA + AMD)"),
            GpuVendor::None => write!(f, "None (integrated/software)"),
        }
    }
}

/// System form factor
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormFactor {
    Laptop,
    Desktop,
}

impl std::fmt::Display for FormFactor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FormFactor::Laptop => write!(f, "Laptop"),
            FormFactor::Desktop => write!(f, "Desktop"),
        }
    }
}

/// CPU information
#[derive(Debug, Clone)]
pub struct CpuInfo {
    pub vendor: CpuVendor,
    pub model_name: String,
}

/// GPU information
#[derive(Debug, Clone)]
pub struct GpuInfo {
    pub vendor: GpuVendor,
    pub model: Option<String>,
    pub hybrid: Option<HybridGpuInfo>,
}

/// Hybrid GPU details (NVIDIA dGPU + AMD iGPU)
#[derive(Debug, Clone)]
pub struct HybridGpuInfo {
    pub amd_bus_id: Option<String>,
    pub nvidia_bus_id: Option<String>,
    pub amd_model: Option<String>,
    pub nvidia_model: Option<String>,
}

#[derive(Debug, Clone)]
struct GpuDevice {
    vendor: GpuVendor,
    model: Option<String>,
    bus_id: Option<String>,
}

/// Complete hardware information
#[derive(Debug, Clone)]
pub struct HardwareInfo {
    pub cpu: CpuInfo,
    pub gpu: GpuInfo,
    pub form_factor: FormFactor,
}

/// Detect all hardware information
pub fn detect_all() -> Result<HardwareInfo> {
    let cpu = detect_cpu()?;
    let gpu = detect_gpu()?;
    let form_factor = detect_form_factor()?;

    Ok(HardwareInfo {
        cpu,
        gpu,
        form_factor,
    })
}

/// Detect CPU vendor and model from /proc/cpuinfo
pub fn detect_cpu() -> Result<CpuInfo> {
    let cpuinfo = fs::read_to_string("/proc/cpuinfo").unwrap_or_default();

    let mut vendor = CpuVendor::Unknown;
    let mut model_name = String::from("Unknown CPU");

    for line in cpuinfo.lines() {
        if line.starts_with("vendor_id") {
            if let Some(value) = line.split(':').nth(1) {
                let value = value.trim();
                vendor = match value {
                    "AuthenticAMD" => CpuVendor::AMD,
                    "GenuineIntel" => CpuVendor::Intel,
                    _ => CpuVendor::Unknown,
                };
            }
        } else if line.starts_with("model name") {
            if let Some(value) = line.split(':').nth(1) {
                model_name = value.trim().to_string();
            }
        }

        // Stop after finding both
        if vendor != CpuVendor::Unknown && model_name != "Unknown CPU" {
            break;
        }
    }

    Ok(CpuInfo { vendor, model_name })
}

/// Detect GPU vendor and model using lspci
pub fn detect_gpu() -> Result<GpuInfo> {
    // Run lspci to find VGA and 3D controllers
    let output = Command::new("lspci")
        .args(["-nn"])
        .output()
        .map_err(|e| anyhow::anyhow!("Failed to run lspci: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    let devices = detect_gpu_devices(&stdout);

    let mut best_gpu: Option<GpuDevice> = None;
    let mut amd_gpu: Option<GpuDevice> = None;
    let mut nvidia_gpu: Option<GpuDevice> = None;

    // Priority: NVIDIA > AMD > Intel
    // This handles cases where a system has both discrete and integrated GPUs
    for device in devices {
        match device.vendor {
            GpuVendor::NVIDIA => nvidia_gpu = Some(device.clone()),
            GpuVendor::AMD => amd_gpu = Some(device.clone()),
            _ => {}
        }

        let should_update = match (&best_gpu, device.vendor) {
            (None, _) => true,
            (Some(current), GpuVendor::NVIDIA) if current.vendor != GpuVendor::NVIDIA => true,
            (Some(current), GpuVendor::AMD)
                if current.vendor == GpuVendor::Intel || current.vendor == GpuVendor::None =>
            {
                true
            }
            _ => false,
        };

        if should_update {
            best_gpu = Some(device);
        }
    }

    let hybrid = match (amd_gpu, nvidia_gpu) {
        (Some(amd), Some(nvidia)) => Some(HybridGpuInfo {
            amd_bus_id: amd.bus_id,
            nvidia_bus_id: nvidia.bus_id,
            amd_model: amd.model,
            nvidia_model: nvidia.model,
        }),
        _ => None,
    };

    let (vendor, model) = if let Some(hybrid_ref) = hybrid.as_ref() {
        let amd_model = hybrid_ref.amd_model.as_deref().unwrap_or("AMD iGPU");
        let nvidia_model = hybrid_ref.nvidia_model.as_deref().unwrap_or("NVIDIA dGPU");
        (
            GpuVendor::HybridNvidiaAmd,
            Some(format!("{} + {}", nvidia_model, amd_model)),
        )
    } else if let Some(best) = best_gpu {
        (best.vendor, best.model)
    } else {
        (GpuVendor::None, None)
    };

    Ok(GpuInfo {
        vendor,
        model,
        hybrid,
    })
}

/// Extract GPU model name from lspci line
fn extract_gpu_model(line: &str, vendor_name: &str) -> Option<String> {
    // Line format: "XX:XX.X VGA compatible controller: Vendor Model [XXXX:YYYY]"
    // Extract everything between the controller type and the PCI ID brackets
    if let Some(start) = line.find(": ") {
        let after_colon = &line[start + 2..];
        // Remove the PCI ID brackets at the end
        if let Some(bracket_pos) = after_colon.rfind(" [") {
            let model = after_colon[..bracket_pos].trim();
            // Clean up common prefixes
            let model = model
                .trim_start_matches("NVIDIA Corporation ")
                .trim_start_matches("Advanced Micro Devices, Inc. ")
                .trim_start_matches("AMD/ATI ")
                .trim_start_matches("Intel Corporation ");
            return Some(format!("{} {}", vendor_name, model));
        }
    }
    None
}

fn detect_gpu_devices(stdout: &str) -> Vec<GpuDevice> {
    // PCI vendor IDs
    const NVIDIA_VENDOR: &str = "10de";
    const AMD_VENDOR: &str = "1002";
    const INTEL_VENDOR: &str = "8086";

    let mut devices = Vec::new();

    for line in stdout.lines() {
        // Look for VGA compatible controller, 3D controller, or Display controller
        // Display controller [0380] is used by some AMD GPUs (e.g., Strix Halo)
        if !line.contains("VGA compatible controller")
            && !line.contains("3D controller")
            && !line.contains("Display controller")
        {
            continue;
        }

        let line_lower = line.to_lowercase();
        let (vendor, model) = if line_lower.contains(&format!("[{}:", NVIDIA_VENDOR)) {
            (GpuVendor::NVIDIA, extract_gpu_model(line, "NVIDIA"))
        } else if line_lower.contains(&format!("[{}:", AMD_VENDOR)) {
            (GpuVendor::AMD, extract_gpu_model(line, "AMD"))
        } else if line_lower.contains(&format!("[{}:", INTEL_VENDOR)) {
            (GpuVendor::Intel, extract_gpu_model(line, "Intel"))
        } else {
            continue;
        };

        devices.push(GpuDevice {
            vendor,
            model,
            bus_id: parse_pci_bus_id(line),
        });
    }

    devices
}

fn parse_pci_bus_id(line: &str) -> Option<String> {
    let slot = line.split_whitespace().next()?;
    let mut parts = slot.split([':', '.']);
    let bus_hex = parts.next()?;
    let device_hex = parts.next()?;
    let function_hex = parts.next()?;

    let bus = u32::from_str_radix(bus_hex, 16).ok()?;
    let device = u32::from_str_radix(device_hex, 16).ok()?;
    let function = u32::from_str_radix(function_hex, 16).ok()?;

    Some(format!("PCI:{}:{}:{}", bus, device, function))
}

pub fn gpu_vendor_label(vendor: GpuVendor) -> &'static str {
    match vendor {
        GpuVendor::NVIDIA => "NVIDIA",
        GpuVendor::AMD => "AMD",
        GpuVendor::Intel => "Intel",
        GpuVendor::HybridNvidiaAmd => "Hybrid (NVIDIA + AMD)",
        GpuVendor::None => "None (integrated/software)",
    }
}

pub fn gpu_vendor_options(has_hybrid: bool) -> Vec<GpuVendor> {
    let mut options = Vec::new();
    if has_hybrid {
        options.push(GpuVendor::HybridNvidiaAmd);
    }
    options.extend([
        GpuVendor::NVIDIA,
        GpuVendor::AMD,
        GpuVendor::Intel,
        GpuVendor::None,
    ]);
    options
}

/// Detect form factor by checking for battery presence
pub fn detect_form_factor() -> Result<FormFactor> {
    // Check /sys/class/power_supply for battery
    let power_supply_path = "/sys/class/power_supply";

    if let Ok(entries) = fs::read_dir(power_supply_path) {
        for entry in entries.flatten() {
            let type_path = entry.path().join("type");
            if let Ok(psu_type) = fs::read_to_string(&type_path) {
                if psu_type.trim().eq_ignore_ascii_case("battery") {
                    return Ok(FormFactor::Laptop);
                }
            }
        }
    }

    // Fallback: check DMI chassis type
    if let Ok(chassis_type) = fs::read_to_string("/sys/class/dmi/id/chassis_type") {
        let chassis_type = chassis_type.trim();
        // Laptop chassis types: 8, 9, 10, 11, 14, 31, 32
        // See: https://www.dmtf.org/standards/smbios
        match chassis_type {
            "8" | "9" | "10" | "11" | "14" | "31" | "32" => {
                return Ok(FormFactor::Laptop);
            }
            _ => {}
        }
    }

    // Default to desktop
    Ok(FormFactor::Desktop)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_vendor_display() {
        assert_eq!(format!("{}", CpuVendor::AMD), "AMD");
        assert_eq!(format!("{}", CpuVendor::Intel), "Intel");
        assert_eq!(format!("{}", CpuVendor::Unknown), "Unknown");
    }

    #[test]
    fn test_gpu_vendor_display() {
        assert_eq!(format!("{}", GpuVendor::NVIDIA), "NVIDIA");
        assert_eq!(format!("{}", GpuVendor::AMD), "AMD");
        assert_eq!(format!("{}", GpuVendor::Intel), "Intel");
        assert_eq!(
            format!("{}", GpuVendor::HybridNvidiaAmd),
            "Hybrid (NVIDIA + AMD)"
        );
        assert_eq!(format!("{}", GpuVendor::None), "None (integrated/software)");
    }

    #[test]
    fn test_form_factor_display() {
        assert_eq!(format!("{}", FormFactor::Laptop), "Laptop");
        assert_eq!(format!("{}", FormFactor::Desktop), "Desktop");
    }

    #[test]
    fn test_extract_gpu_model_nvidia() {
        let nvidia_line = "01:00.0 VGA compatible controller: NVIDIA Corporation GA102 [GeForce RTX 3090] [10de:2204]";
        let model = extract_gpu_model(nvidia_line, "NVIDIA");
        assert!(model.is_some());
        let model_str = model.unwrap();
        assert!(model_str.contains("NVIDIA"), "Model should contain vendor prefix");
        assert!(model_str.contains("RTX 3090"), "Model should contain GPU name");
    }

    #[test]
    fn test_extract_gpu_model_amd() {
        let amd_line = "06:00.0 VGA compatible controller: Advanced Micro Devices, Inc. [AMD/ATI] Navi 21 [Radeon RX 6800/6800 XT] [1002:73bf]";
        let model = extract_gpu_model(amd_line, "AMD");
        assert!(model.is_some());
        let model_str = model.unwrap();
        assert!(model_str.contains("AMD"), "Model should contain vendor prefix");
        assert!(model_str.contains("Navi") || model_str.contains("Radeon"), "Model should contain GPU identifier");
    }

    #[test]
    fn test_extract_gpu_model_intel() {
        let intel_line = "00:02.0 VGA compatible controller: Intel Corporation UHD Graphics 630 [8086:3e92]";
        let model = extract_gpu_model(intel_line, "Intel");
        assert!(model.is_some());
        let model_str = model.unwrap();
        assert!(model_str.contains("Intel"), "Model should contain vendor prefix");
        assert!(model_str.contains("UHD") || model_str.contains("Graphics"), "Model should contain GPU identifier");
    }

    #[test]
    fn test_extract_gpu_model_display_controller() {
        // Some AMD GPUs show as Display controller instead of VGA compatible controller
        let amd_display = "c1:00.0 Display controller: Advanced Micro Devices, Inc. [AMD/ATI] Device 1900 [1002:1900]";
        let model = extract_gpu_model(amd_display, "AMD");
        assert!(model.is_some());
    }

    #[test]
    fn test_extract_gpu_model_no_brackets() {
        // Line without PCI ID brackets should return None
        let no_brackets = "01:00.0 VGA compatible controller: Unknown GPU";
        let model = extract_gpu_model(no_brackets, "Unknown");
        assert!(model.is_none());
    }

    #[test]
    fn test_cpu_vendor_equality() {
        assert_eq!(CpuVendor::AMD, CpuVendor::AMD);
        assert_ne!(CpuVendor::AMD, CpuVendor::Intel);
        assert_ne!(CpuVendor::Intel, CpuVendor::Unknown);
    }

    #[test]
    fn test_gpu_vendor_equality() {
        assert_eq!(GpuVendor::NVIDIA, GpuVendor::NVIDIA);
        assert_ne!(GpuVendor::NVIDIA, GpuVendor::AMD);
        assert_ne!(GpuVendor::AMD, GpuVendor::Intel);
        assert_ne!(GpuVendor::Intel, GpuVendor::None);
    }

    #[test]
    fn test_form_factor_equality() {
        assert_eq!(FormFactor::Laptop, FormFactor::Laptop);
        assert_eq!(FormFactor::Desktop, FormFactor::Desktop);
        assert_ne!(FormFactor::Laptop, FormFactor::Desktop);
    }

    #[test]
    fn test_cpu_info_clone() {
        let cpu = CpuInfo {
            vendor: CpuVendor::AMD,
            model_name: "AMD Ryzen 9 7950X".to_string(),
        };
        let cloned = cpu.clone();
        assert_eq!(cloned.vendor, CpuVendor::AMD);
        assert_eq!(cloned.model_name, "AMD Ryzen 9 7950X");
    }

    #[test]
    fn test_gpu_info_clone() {
        let gpu = GpuInfo {
            vendor: GpuVendor::NVIDIA,
            model: Some("GeForce RTX 5090".to_string()),
            hybrid: None,
        };
        let cloned = gpu.clone();
        assert_eq!(cloned.vendor, GpuVendor::NVIDIA);
        assert_eq!(cloned.model, Some("GeForce RTX 5090".to_string()));
    }

    #[test]
    fn test_gpu_info_none_model() {
        let gpu = GpuInfo {
            vendor: GpuVendor::None,
            model: None,
            hybrid: None,
        };
        assert_eq!(gpu.vendor, GpuVendor::None);
        assert!(gpu.model.is_none());
    }

    #[test]
    fn test_hardware_info_clone() {
        let hw = HardwareInfo {
            cpu: CpuInfo {
                vendor: CpuVendor::Intel,
                model_name: "Intel Core i9-14900K".to_string(),
            },
            gpu: GpuInfo {
                vendor: GpuVendor::NVIDIA,
                model: Some("RTX 4090".to_string()),
                hybrid: None,
            },
            form_factor: FormFactor::Desktop,
        };
        let cloned = hw.clone();
        assert_eq!(cloned.cpu.vendor, CpuVendor::Intel);
        assert_eq!(cloned.gpu.vendor, GpuVendor::NVIDIA);
        assert_eq!(cloned.form_factor, FormFactor::Desktop);
    }
}
