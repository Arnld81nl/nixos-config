# Forge Installer ISO
# Boots directly into Forge TUI with automatic network setup
{ config, lib, pkgs, modulesPath, plymouth-cybex, ... }:

let
  # Startup script that checks connectivity and launches Forge
  forgeStartup = pkgs.writeShellScriptBin "forge-startup" ''
    check_internet() {
      ${pkgs.iputils}/bin/ping -c 1 -W 2 github.com &>/dev/null
    }

    clear
    echo ""
    echo "  ╔═══════════════════════════════════════╗"
    echo "  ║     Forge NixOS Installer             ║"
    echo "  ╚═══════════════════════════════════════╝"
    echo ""

    while ! check_internet; do
      echo "  No internet connection detected."
      echo "  Opening network configuration..."
      echo ""
      sleep 2
      ${pkgs.networkmanager}/bin/nmtui
      clear
      echo ""
      echo "  Checking connection..."
    done

    echo "  Connected! Starting Forge..."
    echo ""
    sleep 1
    exec sudo nix run github:Arnld81nl/nixos-config
  '';
in
{
  imports = [
    "${modulesPath}/installer/cd-dvd/installation-cd-minimal.nix"
  ];

  # Use latest kernel for best hardware support
  boot.kernelPackages = pkgs.linuxPackages_latest;

  # Disable ZFS (broken with kernel 6.18.x, not needed for installer)
  boot.supportedFilesystems.zfs = lib.mkForce false;

  # ISO naming - includes kernel version
  isoImage.isoBaseName = lib.mkForce "NixOS-Cybex-${config.boot.kernelPackages.kernel.version}";
  isoImage.volumeID = "NIXOS_CYBEX";

  # Auto-login to nixos user
  services.getty.autologinUser = "nixos";

  # Enable flakes (required for nix run)
  nix.settings.experimental-features = [ "nix-command" "flakes" ];

  # NetworkManager for WiFi
  networking.networkmanager.enable = true;
  networking.wireless.enable = lib.mkForce false;

  # Plymouth boot splash with cybex theme
  boot.plymouth = {
    enable = true;
    themePackages = [ plymouth-cybex ];
    theme = "cybex";
  };
  boot.kernelParams = [ "quiet" "splash" ];
  boot.consoleLogLevel = 0;
  boot.initrd.verbose = false;

  # Debugging and utility packages
  environment.systemPackages = with pkgs; [
    # Startup script
    forgeStartup

    # Networking
    networkmanager
    iputils
    curl
    wget

    # Editors
    neovim
    nano

    # System monitoring
    btop
    htop

    # Disk utilities
    parted
    gptfdisk
    smartmontools
    nvme-cli

    # File utilities
    file
    less
    tree
    unzip

    # Git (for nix run)
    git
  ];

  # Run forge-startup on login to tty1
  programs.bash.loginShellInit = ''
    if [ "$(tty)" = "/dev/tty1" ] && [ -z "$FORGE_STARTED" ]; then
      export FORGE_STARTED=1
      exec forge-startup
    fi
  '';

  # Allow passwordless sudo for nixos user
  security.sudo.wheelNeedsPassword = false;

  # Disable unnecessary services for faster boot
  documentation.enable = false;
  documentation.nixos.enable = false;
}
