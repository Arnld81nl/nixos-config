# MediaTek MT7925 WiFi 7 configuration
# Used in: G1a (HP ZBook Ultra G1a), proart (ASUS ProArt P16)
#
# Known issues with MT7925 in kernels 6.14-6.18:
# - Commit cb1353ef34735 causes speed drops on some routers
# - CLC (Country Location Code) feature causes instability
# - ASPM power management interferes with driver operation
# - WiFi power save causes disconnects
#
# These workarounds should be removable once kernel 6.19+ is available.
# See CLAUDE.md "MT7925 WiFi Stability" section for details.
{ config, pkgs, lib, ... }:

{
  # Load driver explicitly - udev auto-loading can be unreliable
  boot.kernelModules = [ "mt7925e" ];

  # Give Linux ASPM control so the mt7925e driver can disable it per-device
  boot.kernelParams = [ "pcie_aspm=force" ];

  # Disable ASPM in driver for stable suspend/resume
  # Disable CLC to prevent random disconnects (known bug, fixed in kernel 6.19)
  boot.extraModprobeConfig = ''
    options mt7925e disable_aspm=1
    options mt7925-common disable_clc=1
  '';

  # Use iwd instead of wpa_supplicant for faster WiFi reconnection after suspend
  # iwd handles suspend/resume much better than wpa_supplicant
  networking.wireless.iwd = {
    enable = true;
    settings = {
      General = {
        EnableNetworkConfiguration = false;  # Let NetworkManager handle IP config
      };
      Settings = {
        AutoConnect = true;
      };
    };
  };
  networking.networkmanager.wifi.backend = "iwd";
}
