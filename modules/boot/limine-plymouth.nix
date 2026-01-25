# Shared Limine bootloader and Plymouth boot splash configuration
{ config, pkgs, lib, plymouth-cybex, ... }:

{
  # Use latest kernel for best hardware support
  boot.kernelPackages = pkgs.linuxPackages_latest;

  # Limine bootloader configuration
  boot.loader.systemd-boot.enable = false;
  boot.loader.efi.canTouchEfiVariables = true;

  boot.loader.limine = {
    enable = true;
    maxGenerations = 10;

    # Catppuccin Mocha theme
    style = {
      interface.resolution = "1920x1080";
      interface.branding = "";
      wallpapers = [];  # Disable default NixOS wallpaper

      backdrop = "1e1e2e";
      graphicalTerminal = {
        palette = "1e1e2e;f38ba8;a6e3a1;f9e2af;89b4fa;f5c2e7;94e2d5;cdd6f4";
        brightPalette = "585b70;f38ba8;a6e3a1;f9e2af;89b4fa;f5c2e7;94e2d5;cdd6f4";
        foreground = "cdd6f4";
        background = "1e1e2e";
      };
    };
  };

  # Boot settings
  boot.loader.timeout = 5;
  boot.initrd.systemd.enable = true;

  # Base HID modules for keyboard input during boot
  # GPU modules should be set per-host (amdgpu for AMD, nvidia for NVIDIA)
  boot.initrd.kernelModules = lib.mkDefault [
    "hid-generic"
    "usbhid"
  ];

  # Plymouth boot splash
  boot.plymouth = {
    enable = true;
    themePackages = [ plymouth-cybex ];
    theme = "cybex";
  };

  # Make Plymouth wait for DRM device (fixes NVIDIA framebuffer takeover issue)
  boot.initrd.systemd.services.plymouth-start = {
    wants = [ "systemd-udev-settle.service" ];
    after = [ "systemd-udev-settle.service" ];
  };

  # Clean boot display
  boot.kernelParams = [
    "quiet"
    "splash"
    "rd.udev.log_level=3"
    "vt.global_cursor_default=0"
  ];
  boot.consoleLogLevel = 1;  # Allow critical errors through for debugging
  boot.initrd.verbose = false;
}
