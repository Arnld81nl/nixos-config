# xps9320 - Dell XPS 9320 (12th Gen Intel Alder Lake + Iris Xe)
{ config, pkgs, lib, ... }:

{
  imports = [
    ./hardware-configuration.nix
    ../../modules/boot/limine-plymouth.nix
    ../../modules/hardware/intel.nix
  ];

  networking.hostName = "xps9320";

  # === Intel Alder Lake (12th Gen) Configuration ===
  # Intel Iris Xe Graphics - uses i915 kernel module
  # Early KMS is handled by intel.nix module

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

  # LUKS configuration is handled by disko (modules/disko/xps9320.nix)
  # Disko sets allowDiscards and bypassWorkqueues automatically

  # Hibernate support (swapfile on btrfs)
  boot.resumeDevice = "/dev/mapper/cryptroot";
  boot.kernelParams = [ "resume_offset=533760" ];

  # Disable zram - using swapfile for hibernate support
  zramSwap.enable = lib.mkForce false;
}
