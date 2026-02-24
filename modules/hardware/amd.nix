# AMD-specific hardware configuration
# Used in: G1a (Strix Halo), proart (Strix Point + NVIDIA)
{ lib, ... }:

{
  # Thermal monitoring
  boot.kernelModules = [ "k10temp" ];

  # CPU microcode updates (use mkDefault so host can override)
  hardware.cpu.amd.updateMicrocode = lib.mkDefault true;

  # AMD GPU and CPU kernel parameters (use mkDefault so hosts can override/extend)
  # - amd_pstate=active: Enable AMD P-State driver with autonomous mode for best efficiency
  # - ppfeaturemask: Enable all power management features for better efficiency
  # - dcdebugmask=0x200: Disable PSR2-SU to prevent stuttering/freezing on Strix Point/Halo
  #   See: https://wiki.archlinux.org/title/ASUS_Zenbook_UM5606
  boot.kernelParams = lib.mkDefault [
    "amd_pstate=active"
    "amdgpu.ppfeaturemask=0xffffffff"
    "amdgpu.dcdebugmask=0x200"
    # Mitigate intermittent GPU hangs/reset storms seen on some RDNA3/3.5 laptops under heavy
    # Wayland/Chromium GPU load (Hyprland + Chrome GPU process). Costs some idle power savings.
    "amdgpu.gfxoff=0"
    # Ensure GPU recovery is enabled (usually default, but make it explicit).
    "amdgpu.gpu_recovery=1"
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
