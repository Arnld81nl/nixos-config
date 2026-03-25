# AMD-specific hardware configuration
# Used in: G1a (Strix Halo), proart (Strix Point + NVIDIA)
{ lib, ... }:

{
  # Thermal monitoring
  boot.kernelModules = [ "k10temp" ];

  # CPU microcode updates (use mkDefault so host can override)
  hardware.cpu.amd.updateMicrocode = lib.mkDefault true;

  # AMD GPU kernel parameters (shared defaults only — host-specific params go in hosts/)
  # NOTE: Do NOT use mkDefault here — boot/limine-plymouth.nix also sets kernelParams,
  # and mkDefault would cause these params to be silently dropped (lower priority).
  boot.kernelParams = [
    # Enable GPU reset recovery (default is -1/auto which is disabled for GFX11).
    # Without this, a GPU hang freezes the system instead of recovering.
    "amdgpu.gpu_recovery=1"
  ];

  # Wayland environment variables for AMD systems
  environment.sessionVariables = {
    # Force Qt to use native Wayland (improves Quickshell/QML performance)
    QT_QPA_PLATFORM = "wayland";
    # Help Electron/Chromium apps use Wayland
    NIXOS_OZONE_WL = "1";
    # Disable Qt window decorations (Hyprland handles them)
    QT_WAYLAND_DISABLE_WINDOWDECORATION = "1";
  };
}
