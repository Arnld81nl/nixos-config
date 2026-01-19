# Forge background update checker
#
# Systemd user service and timer that checks for:
# - NixOS config repo updates (changes from other machines)
# - App profile updates (private-settings repo)
# - Flake input updates (nixpkgs, home-manager, etc.)
#
# Sends desktop notifications via libnotify when updates are available.
{ config, pkgs, lib, forge, ... }:

{
  # Systemd user service - runs the check
  systemd.user.services.forge-notify = {
    Unit = {
      Description = "Forge update checker";
      After = [ "network-online.target" ];
      Wants = [ "network-online.target" ];
    };
    Service = {
      Type = "oneshot";
      ExecStart = "${forge}/bin/forge-notify --once";
      # Don't fail the service if the check fails (network issues, etc.)
      # The binary already handles errors gracefully
    };
  };

  # Systemd user timer - triggers the service
  systemd.user.timers.forge-notify = {
    Unit = {
      Description = "Hourly Forge update check";
    };
    Timer = {
      # First check 2 minutes after login (give network time to connect)
      OnBootSec = "2min";
      # Then check every hour
      OnUnitActiveSec = "1h";
      # If system was off, run check on next boot
      Persistent = true;
    };
    Install = {
      WantedBy = [ "timers.target" ];
    };
  };
}
