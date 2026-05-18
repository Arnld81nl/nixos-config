# G1a configuration - HP ZBook Ultra G1a (Strix Halo)
{ config, pkgs, lib, ... }:

{
  imports = [
    ./hardware-configuration.nix
    ../../modules/boot/limine-plymouth.nix
    ../../modules/hardware/amd.nix
    ../../modules/hardware/mediatek-wifi.nix
  ];

  networking.hostName = "G1a";

  # Disable power-profiles-daemon — broken on Strix Halo with amd-pstate-epp
  # (fails writing policy*/boost, sticks to power-saver incorrectly).
  # Use direct sysfs writes instead.
  services.power-profiles-daemon.enable = lib.mkForce false;

  # Keep this laptop on "balanced" platform profile always.
  #
  # powerprofilesctl currently fails on this host when it tries to toggle AMD boost,
  # so we apply the durable bits directly through sysfs instead:
  # - always keep the platform profile on "balanced"
  # - on AC: prefer snappier boosting with EPP=balance_performance
  # - on battery: stay efficient with EPP=balance_power, but do not drop to power-saver
  #
  # This runs at boot and whenever AC online status changes.
  systemd.services.g1a-power-policy = {
    description = "G1a power policy (platform profile + AMD P-State EPP)";
    # Use graphical.target (not multi-user.target) to avoid a systemd ordering cycle:
    # multi-user.target → g1a-power-policy → PPD → After=multi-user.target → cycle!
    # graphical.target comes after multi-user.target, breaking the cycle.
    wantedBy = [ "graphical.target" ];

    serviceConfig = {
      Type = "oneshot";
    };

    script = ''
      set -euo pipefail

      ac_online=0
      if [ -r /sys/class/power_supply/AC/online ]; then
        ac_online="$(cat /sys/class/power_supply/AC/online || echo 0)"
      fi

      if [ "$ac_online" = "1" ]; then
        # AC: balanced platform profile, snappier EPP
        if [ -w /sys/firmware/acpi/platform_profile ]; then
          echo balanced > /sys/firmware/acpi/platform_profile || true
        fi
        for p in /sys/devices/system/cpu/cpufreq/policy*/energy_performance_preference; do
          [ -w "$p" ] || continue
          echo balance_performance > "$p" || true
        done
      else
        # Battery: balanced platform profile, efficient EPP
        if [ -w /sys/firmware/acpi/platform_profile ]; then
          echo balanced > /sys/firmware/acpi/platform_profile || true
        fi
        for p in /sys/devices/system/cpu/cpufreq/policy*/energy_performance_preference; do
          [ -w "$p" ] || continue
          echo balance_power > "$p" || true
        done
      fi
    '';
  };

  systemd.paths.g1a-power-policy = {
    description = "Trigger G1a power policy when AC online changes";
    wantedBy = [ "multi-user.target" ];
    pathConfig = {
      PathChanged = "/sys/class/power_supply/AC/online";
      Unit = "g1a-power-policy.service";
    };
  };

  # === AMD Strix Halo (RDNA 3.5) GPU Configuration ===
  # Enable official amdgpu initrd support for early KMS and Plymouth
  hardware.amdgpu.initrd.enable = true;

  # Fingerprint reader (Synaptics FS7606, power button)
  services.fprintd.enable = true;

  # PAM fingerprint authentication
  # NOTE: greetd and login are NOT enabled — auto-login makes them unnecessary
  # and they can cause PAM timeouts.
  security.pam.services = {
    sudo.fprintAuth = true;           # Sudo commands
    polkit-1.fprintAuth = true;       # Polkit prompts (1Password, etc.)
    hyprlock.fprintAuth = true;       # Screen lock (Illogical shell)
    noctalia.fprintAuth = true;       # Screen lock (Noctalia shell)
  };

  # Fingerprint reader suspend/resume: force USB reset on resume for this device
  # Without this, the device fails to resume from s2idle (kernel error -107,
  # endpoint stalled) and fprintd can't communicate with it.
  # The 'b' flag sets USB_QUIRK_RESET_RESUME for this specific device.
  # Display stability workarounds for this Strix Halo laptop:
  # - amd_pstate=active: AMD P-State driver with autonomous mode for best efficiency
  # - dcdebugmask=0x410: disable Panel Replay and PSR to prevent flip_done stalls
  # - sg_display=0: disable scatter/gather display to prevent external monitor corruption
  # - gfxoff=0: mitigate GPU hangs under heavy Wayland/Chromium GPU load
  boot.kernelParams = lib.mkAfter [
    "amd_pstate=active"
    "amdgpu.dcdebugmask=0x410"
    "amdgpu.sg_display=0"
    "amdgpu.gfxoff=0"
    "usbcore.quirks=06cb:0106:b"
  ];

  # Early boot kernel modules (order matters for proper initialization)
  # - GPU modules first: enables early KMS for high-res Plymouth/console
  # - HID modules: ensures keyboard works for LUKS passphrase entry
  # Using mkForce to override any defaults from imported modules
  boot.initrd.kernelModules = lib.mkForce [
    "amdgpu"       # GPU: early KMS for Plymouth and console
    "hid-generic"  # Input: generic HID driver for keyboards
    "usbhid"       # Input: USB HID for external keyboards
  ];

  # Mic-mute LED is handled in modules/common.nix (hostsWithMicMuteLed list).
  # Prevent USB autosuspend for the fingerprint reader so it stays responsive.
  services.udev.extraRules = ''
    # Prevent USB autosuspend for Synaptics fingerprint reader (06cb:0106)
    ACTION=="add", SUBSYSTEM=="usb", ATTR{idVendor}=="06cb", ATTR{idProduct}=="0106", ATTR{power/control}="on"
  '';
}
