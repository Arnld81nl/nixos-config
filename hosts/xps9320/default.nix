# xps9320 - Dell XPS 9320 (12th Gen Intel Alder Lake + Iris Xe)
# Hardware reference: https://gist.github.com/p-alik/6ed132ffad59de8fcbc4fb10b54d745e
{ config, pkgs, lib, ... }:

{
  imports = [
    ./hardware-configuration.nix
    ../../modules/boot/limine-plymouth.nix
    ../../modules/hardware/intel.nix
  ];

  networking.hostName = "xps9320";

  # === Intel Alder Lake (12th Gen) GPU - Critical Fix ===
  # IMPORTANT: Disable PSR (Panel Self-Refresh) to prevent graphics hangs
  # The Iris Xe on XPS 9320 has PSR bugs causing "Selective fetch area calculation failed"
  # This OVERRIDES the intel.nix defaults
  boot.kernelParams = lib.mkForce [
    "i915.modeset=1"        # Enable kernel modesetting
    "i915.enable_psr=0"     # DISABLE PSR - causes hangs on XPS 9320!
    "i915.enable_fbc=1"     # Frame Buffer Compression (safe)
    "resume_offset=533760"  # Btrfs swapfile offset for hibernation
  ];

  # Early boot kernel modules (order matters for proper initialization)
  # - GPU modules first: enables early KMS for high-res Plymouth/console
  # - HID modules: ensures keyboard works for LUKS passphrase entry
  boot.initrd.kernelModules = lib.mkForce [
    "i915"         # GPU: early KMS for Plymouth and console
    "hid-generic"  # Input: generic HID driver for keyboards
    "usbhid"       # Input: USB HID for external keyboards
  ];

  # Kernel modules for Intel virtualization and thermal monitoring
  boot.kernelModules = [ "kvm-intel" "coretemp" ];

  # Dell XPS power management (TLP)
  services.tlp = {
    enable = true;
    settings = {
      # CPU scaling
      CPU_SCALING_GOVERNOR_ON_AC = "performance";
      CPU_SCALING_GOVERNOR_ON_BAT = "powersave";
      CPU_ENERGY_PERF_POLICY_ON_AC = "performance";
      CPU_ENERGY_PERF_POLICY_ON_BAT = "power";

      # CPU boost (disable on battery for thermals)
      CPU_BOOST_ON_AC = 1;
      CPU_BOOST_ON_BAT = 0;

      # Platform profile (Dell-specific)
      PLATFORM_PROFILE_ON_AC = "performance";
      PLATFORM_PROFILE_ON_BAT = "low-power";

      # Intel GPU power management
      INTEL_GPU_MIN_FREQ_ON_AC = 350;
      INTEL_GPU_MIN_FREQ_ON_BAT = 350;
      INTEL_GPU_MAX_FREQ_ON_AC = 1450;
      INTEL_GPU_MAX_FREQ_ON_BAT = 900;

      # Battery thresholds (extend battery lifespan)
      START_CHARGE_THRESH_BAT0 = 75;
      STOP_CHARGE_THRESH_BAT0 = 80;

      # WiFi power saving
      WIFI_PWR_ON_AC = "off";
      WIFI_PWR_ON_BAT = "on";

      # Runtime PM and USB
      RUNTIME_PM_ON_AC = "auto";
      RUNTIME_PM_ON_BAT = "auto";
      USB_AUTOSUSPEND = 1;
    };
  };

  # Disable power-profiles-daemon (conflicts with TLP)
  services.power-profiles-daemon.enable = false;

  # Firmware updates via fwupd (Dell laptops are well supported)
  services.fwupd.enable = true;

  # Touchpad support
  services.libinput.enable = true;

  # Fingerprint reader support
  services.fprintd.enable = true;

  # PAM fingerprint authentication
  security.pam.services = {
    login.fprintAuth = true;          # Console login
    greetd.fprintAuth = true;         # Display manager
    sudo.fprintAuth = true;           # Sudo commands
    polkit-1.fprintAuth = true;       # Polkit prompts (1Password, etc.)
    hyprlock.fprintAuth = true;       # Screen lock
  };

  # LUKS configuration is handled by disko (modules/disko/xps9320.nix)
  # Disko sets allowDiscards and bypassWorkqueues automatically

  # Hibernate support (swapfile on btrfs)
  boot.resumeDevice = "/dev/mapper/cryptroot";

  # Disable zram - using swapfile for hibernate support
  zramSwap.enable = lib.mkForce false;
}
