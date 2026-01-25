# Gaming configuration - Steam with Proton support
{ config, pkgs, ... }:

{
  # Enable Steam
  programs.steam = {
    enable = true;
    remotePlay.openFirewall = true;
    dedicatedServer.openFirewall = true;
  };

  # GameMode for automatic performance optimization
  programs.gamemode.enable = true;

  # Additional gaming packages
  environment.systemPackages = with pkgs; [
    protonup-qt    # Manage Proton-GE versions
    mangohud       # FPS/performance overlay
  ];
}
