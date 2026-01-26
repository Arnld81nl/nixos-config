# Hardware configuration for Dell XPS 9320
# Based on: https://gist.github.com/p-alik/6ed132ffad59de8fcbc4fb10b54d745e
# Filesystem declarations are handled by disko (modules/disko/xps9320.nix)
{ config, lib, pkgs, modulesPath, ... }:

{
  imports = [
    (modulesPath + "/installer/scan/not-detected.nix")
  ];

  # Kernel modules available in initrd for hardware detection
  # Order matters: storage/USB modules first, then graphics
  boot.initrd.availableKernelModules = [
    "xhci_pci"      # USB 3.x controller
    "thunderbolt"   # Thunderbolt 4 support
    "vmd"           # Intel Volume Management Device (NVMe)
    "nvme"          # NVMe storage
    "usb_storage"   # USB mass storage
    "usbhid"        # USB HID for keyboard in LUKS prompt
    "sd_mod"        # SCSI disk support
    "i915"          # Intel graphics (early KMS)
    "btrfs"         # Btrfs filesystem
  ];

  boot.initrd.kernelModules = [ ];
  boot.kernelModules = [ "kvm-intel" ];
  boot.extraModulePackages = [ ];

  # Thunderbolt 4 support (XPS 9320 has 2x TB4 ports)
  services.hardware.bolt.enable = true;

  nixpkgs.hostPlatform = lib.mkDefault "x86_64-linux";

  # Intel CPU microcode updates
  hardware.cpu.intel.updateMicrocode = lib.mkDefault config.hardware.enableRedistributableFirmware;

  # Power management - default to powersave on battery
  powerManagement.cpuFreqGovernor = lib.mkDefault "powersave";

  # === Intel IPU6 Camera (MIPI) ===
  # Dell XPS 9320 uses Intel IPU6EP for the webcam
  # See: https://github.com/NixOS/nixpkgs/issues/225743
  hardware.ipu6 = {
    enable = true;
    platform = "ipu6ep";
  };
}
