# Hypridle configuration
# Screen locking and power management (shell-aware)
{ shell ? "noctalia" }:

{ config, pkgs, lib, hostname, ... }:

let
  # Enable auto-suspend for known hosts
  shouldAutoSuspend = lib.hasPrefix "G1a" hostname || lib.hasPrefix "kraken" hostname;

  # Shell-specific lock command
  lockCmd = if shell == "illogical"
    then "hyprlock"
    else "pidof -q noctalia-shell && noctalia-shell ipc call lockScreen lock";

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

    listener {
      timeout = 600                    # 10 minutes
      on-timeout = hyprctl dispatch dpms off
      on-resume = hyprctl dispatch dpms on
    }
    ${suspendListener}
  '';
}
