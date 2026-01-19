# AMD-specific hardware configuration
# Used in: G1a (Strix Halo), proart (Strix Point + NVIDIA)
{ lib, ... }:

{
  # Thermal monitoring
  boot.kernelModules = [ "k10temp" ];

  # CPU microcode updates (use mkDefault so host can override)
  hardware.cpu.amd.updateMicrocode = lib.mkDefault true;

  # AMD GPU kernel parameters (use mkDefault so hosts can override/extend)
  # - ppfeaturemask: Enable all power management features for better efficiency
  # - dcdebugmask: Helps with display initialization on newer AMD APUs (Strix Halo/Point)
  boot.kernelParams = lib.mkDefault [
    "amdgpu.ppfeaturemask=0xffffffff"
    "amdgpu.dcdebugmask=0x10"
  ];

  # Wayland environment variables for AMD systems
  environment.sessionVariables = lib.mkDefault {
    # Force Qt to use native Wayland (improves Quickshell/QML performance)
    QT_QPA_PLATFORM = "wayland";
    # Help Electron/Chromium apps use Wayland
    NIXOS_OZONE_WL = "1";
    # Disable Qt window decorations (Hyprland handles them)
    QT_WAYLAND_DISABLE_WINDOWDECORATION = "1";
  };
}
