# Hypridle configuration
# Screen locking and power management (shell-aware)
{ shell ? "noctalia" }:

{ config, pkgs, lib, hostname, ... }:

let
  # Enable auto-suspend for known hosts
  shouldAutoSuspend = lib.hasPrefix "G1a" hostname || lib.hasPrefix "kraken" hostname;

  # DPMS is safe on AMD hosts; disabled on Intel (x1yoga) due to resume crashes
  isIntelHost = lib.hasPrefix "x1yoga" hostname;

  # Shell-specific lock command
  lockCmd = if shell == "illogical"
    then "hyprlock"
    else "pidof -q noctalia && noctalia msg session lock";

  # DPMS listener (disabled on Intel — causes crashes on resume)
  dpmsListener = if isIntelHost then ''

    # DPMS disabled on Intel — causes crashes on resume
  '' else ''

    listener {
      timeout = 600                    # 10 minutes
      on-timeout = hyprctl dispatch dpms off
      on-resume = hyprctl dispatch dpms on
    }
  '';

  # Auto-suspend listener
  suspendListener = if shouldAutoSuspend then ''

    listener {
      timeout = 1800                   # 30 minutes
      on-timeout = systemctl suspend   # suspend when timeout has passed
    }
  '' else "";
in
{
  # Enable hypridle service
  services.hypridle.enable = true;

  # Generate config file matching Arch style
  xdg.configFile."hypr/hypridle.conf".text = ''
    general {
      lock_cmd = ${lockCmd}
      before_sleep_cmd = ${lockCmd}
      after_sleep_cmd = hyprctl dispatch dpms on
    }

    listener {
      timeout = 300                    # 5 minutes
      on-timeout = ${lockCmd}
    }
    ${dpmsListener}
    ${suspendListener}
  '';
}
