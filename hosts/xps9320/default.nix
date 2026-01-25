# xps9320 - Laptop with Intel GPU
{ config, pkgs, lib, ... }:

{
boot.initrd.systemd.enable = lib.mkForce false;
boot.plymouth.enable = lib.mkForce false;
boot.initrd.verbose = lib.mkForce true;

boot.kernelParams = [
"loglevel=7"
"rd.systemd.show_status=1"
"systemd.show_status=1"
"systemd.log_level=debug"
"rd.udev.log_level=debug" 
];



  imports = [
    ./hardware-configuration.nix
    ../../modules/boot/limine-plymouth.nix
  ];

  networking.hostName = "xps9320";

  # Intel CPU configuration (override AMD default from common.nix)
  hardware.cpu.amd.updateMicrocode = lib.mkForce false;
  hardware.cpu.intel.updateMicrocode = true;
  boot.kernelModules = [ "kvm-intel" "coretemp" ];

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

  # Early KMS for Plymouth boot splash
  boot.initrd.kernelModules = lib.mkForce [
"btrfs"
    "i915"
    "hid-generic"
    "usbhid"
  ];
}
