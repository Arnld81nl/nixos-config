# Hardware configuration for x1yoga (Lenovo ThinkPad X1 Yoga Gen 6)
# Generated from actual hardware, then cleaned up for disko compatibility
# Filesystem declarations are handled by disko
{ config, lib, pkgs, modulesPath, ... }:

{
  imports = [
    (modulesPath + "/installer/scan/not-detected.nix")
  ];

  # Detected from actual ThinkPad X1 Yoga Gen 6 hardware
  boot.initrd.availableKernelModules = [ "xhci_pci" "thunderbolt" "nvme" "btrfs" ];
  boot.initrd.kernelModules = [ ];
  boot.kernelModules = [ "kvm-intel" ];
  boot.extraModulePackages = [ ];

  # Filesystem declarations removed - handled by disko (modules/disko/x1yoga.nix)

  nixpkgs.hostPlatform = lib.mkDefault "x86_64-linux";
  hardware.cpu.intel.updateMicrocode = lib.mkDefault config.hardware.enableRedistributableFirmware;
}
