# NVIDIA GPU configuration
{ config, pkgs, lib, ... }:

let
  nvidia-sleep = config.hardware.nvidia.package + "/bin/nvidia-sleep.sh";
in
{
  hardware.nvidia = {
    open = true;                    # Use open-source kernel modules
    modesetting.enable = true;      # Required for Wayland
    nvidiaSettings = true;          # Enable nvidia-settings GUI
    package = config.boot.kernelPackages.nvidiaPackages.stable;

    # Power management for better efficiency during idle
    powerManagement.enable = true;
  };

  # Load NVIDIA modules in initrd for high-resolution boot display
  boot.initrd.kernelModules = [
    "nvidia"
    "nvidia_modeset"
    "nvidia_uvm"
    "nvidia_drm"
  ];

  # Kernel parameters for NVIDIA DRM
  boot.kernelParams = [
    "nvidia-drm.modeset=1"
    "nvidia-drm.fbdev=1"
  ];

  # Environment variables for NVIDIA Wayland
  environment.sessionVariables = {
    GBM_BACKEND = "nvidia-drm";
    __GLX_VENDOR_LIBRARY_NAME = "nvidia";
    NIXOS_OZONE_WL = "1";
  };

  # Fix NVIDIA suspend/resume: Add system-sleep hook (like Arch has)
  # This ensures nvidia-sleep.sh resume is called after waking from sleep
  # Note: nvidia-sleep.sh requires chvt/fgconsole from kbd package in PATH
  powerManagement.powerDownCommands = "";
  powerManagement.resumeCommands = ''
    export PATH="${pkgs.kbd}/bin:$PATH"
    ${nvidia-sleep} resume
  '';

  # Ensure nvidia-resume also handles suspend-then-hibernate
  systemd.services.nvidia-resume = {
    after = [ "systemd-suspend-then-hibernate.service" ];
    requiredBy = [ "systemd-suspend-then-hibernate.service" ];
  };
}
