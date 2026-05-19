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

  # Note: Do NOT use pcie_aspm=force here — it affects ALL PCIe devices system-wide
  # (NVMe, GPU, etc.) and can cause issues. The driver's own disable_aspm=1 option
  # is more targeted and sufficient.

  # Disable ASPM in driver for stable suspend/resume
  # Disable CLC to prevent random disconnects (known bug, fixed in kernel 6.19)
  boot.extraModprobeConfig = ''
    options mt7925e disable_aspm=1
    options mt7925-common disable_clc=1
  '';

  # WiFi backend: NetworkManager + wpa_supplicant (default).
  # iwd was tried first but its autoconnect state machine wedges after sleep /
  # AP-drop events, and the NM↔iwd D-Bus glue throws Station.GetOrderedNetworks
  # mismatches on this nixpkgs/iwd combo. wpa_supplicant handles WPA2-Enterprise
  # (eduroam) and roaming reliably enough; the marginal iwd power/roam advantages
  # weren't worth the daily breakage.
}
