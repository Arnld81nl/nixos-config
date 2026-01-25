# Hardware configuration for xps9320 (Dell XPS 13 9320)
# Generated from actual hardware, then cleaned up for disko compatibility
# Filesystem declarations are handled by disko
{ config, lib, pkgs, modulesPath, ... }:

{
  imports = [
    (modulesPath + "/installer/scan/not-detected.nix")
  ];

  # Dell XPS 9320 hardware modules
  boot.initrd.availableKernelModules = [ "xhci_pci" "thunderbolt" "nvme" "btrfs" ];
  boot.initrd.kernelModules = [ ];
  boot.kernelModules = [ "kvm-intel" ];
  boot.extraModulePackages = [ ];

  # Filesystem declarations removed - handled by disko (modules/disko/xps9320.nix)

  nixpkgs.hostPlatform = lib.mkDefault "x86_64-linux";
  hardware.cpu.intel.updateMicrocode = lib.mkDefault config.hardware.enableRedistributableFirmware;
}
