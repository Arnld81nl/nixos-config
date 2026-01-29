# Common NixOS configuration shared across all machines
{ config, pkgs, lib, forge, username, ... }:

{
  imports = [
    ./gaming.nix      # Steam and gaming tools
    ./intune.nix      # Microsoft Intune device management
  ];

  # Enable flakes
  nix.settings.experimental-features = [ "nix-command" "flakes" ];

  # Binary caches (Cachix for Portal)
  nix.settings.substituters = [
    "https://cache.nixos.org"
    "https://digitalpals.cachix.org"
  ];
  nix.settings.trusted-public-keys = [
    "cache.nixos.org-1:6NCHdD59X431o0gWypbMrAURkbJ16ZPMQFGspcDShjY="
    "digitalpals.cachix.org-1:YWuWBw08EbEeTsIccpPfRTaqksfo4QtAVQaTRljYFm8="
  ];

  # Allow users to use extra substituters (for cachix in user nix.conf)
  nix.settings.trusted-users = [ "root" "@wheel" ];

  # Increase download buffer size for faster fetches
  nix.settings.download-buffer-size = 256 * 1024 * 1024; # 256 MiB

  # Automatic garbage collection to prevent store bloat
  nix.gc = {
    automatic = true;
    dates = "weekly";
    options = "--delete-older-than 14d";
  };

  # Optimize store automatically
  nix.settings.auto-optimise-store = true;

  # Parallel builds for faster compilation
  nix.settings.max-jobs = "auto";

  # Use bash as /bin/sh (instead of busybox ash)
  environment.binsh = "${pkgs.bash}/bin/bash";

  # Create /bin/bash symlink for script compatibility (#!/bin/bash shebangs)
  systemd.tmpfiles.rules = [
    "L+ /bin/bash - - - - ${pkgs.bash}/bin/bash"
  ];

  # Enable nix-ld for running dynamically linked executables
  programs.nix-ld = {
    enable = true;
    libraries = with pkgs; [
      stdenv.cc.cc.lib
      zlib
      openssl
      curl
      glib
      gtk3
      libGL
      # Additional libraries for CLI tools and games
      libxkbcommon
      wayland
      fontconfig
      libdrm
      xorg.libX11
      xorg.libXcursor
      xorg.libXrandr
      xorg.libXi
    ];
  };

  # Networking
  networking.networkmanager = {
    enable = true;
    wifi.powersave = false;  # Disable WiFi power save to prevent random disconnects
  };

  # Tailscale VPN
  services.tailscale.enable = true;

  # Firewall
  networking.firewall = {
    enable = true;
    allowedTCPPorts = [ 53317 ]; # LocalSend
    allowedUDPPorts = [ 53317 ]; # LocalSend
    # Allow Tailscale traffic
    trustedInterfaces = [ "tailscale0" ];
    checkReversePath = "loose";  # Required for Tailscale exit nodes
  };

  # Disable NetworkManager-wait-online to speed up boot
  systemd.services.NetworkManager-wait-online.enable = lib.mkForce false;

  # Timezone and locale
  time.timeZone = "Europe/Amsterdam";
  i18n.defaultLocale = "en_US.UTF-8";
  i18n.extraLocaleSettings = {
    LC_ADDRESS = "nl_NL.UTF-8";
    LC_IDENTIFICATION = "nl_NL.UTF-8";
    LC_MEASUREMENT = "nl_NL.UTF-8";
    LC_MONETARY = "nl_NL.UTF-8";
    LC_NAME = "nl_NL.UTF-8";
    LC_NUMERIC = "nl_NL.UTF-8";
    LC_PAPER = "nl_NL.UTF-8";
    LC_TELEPHONE = "nl_NL.UTF-8";
    LC_TIME = "nl_NL.UTF-8";
  };

  # Enable OpenGL (works for all GPUs)
  hardware.graphics = {
    enable = true;
    enable32Bit = true;
  };

  # Firmware for AMD GPUs and other hardware
  hardware.enableRedistributableFirmware = true;

  # Bluetooth
  hardware.bluetooth.enable = true;

  # Power management (use mkDefault so laptop can override with TLP)
  services.power-profiles-daemon.enable = lib.mkDefault true;
  services.upower.enable = true;

  # USB drive automounting (required for Nautilus to show removable drives)
  services.udisks2.enable = true;
  services.gvfs.enable = true;

  # Network discovery for Nautilus (SMB shares, printers, etc.)
  services.avahi = {
    enable = true;
    openFirewall = true;
    nssmdns4 = true;
  };

  # Swap (zram for memory pressure handling)
  zramSwap = {
    enable = true;
    memoryPercent = 25;
  };

  # SSD TRIM
  services.fstrim.enable = true;

  # Btrfs scrub (use mkDefault so laptops can disable to save battery)
  services.btrfs.autoScrub.enable = lib.mkDefault true;

  # Docker
  virtualisation.docker = {
    enable = true;
    enableOnBoot = true;
  };

  # Audio (PipeWire)
  services.pulseaudio.enable = false;
  security.rtkit.enable = true;
  services.pipewire = {
    enable = true;
    alsa.enable = true;
    alsa.support32Bit = true;
    pulse.enable = true;
  };

  # User configuration
  # mutableUsers allows setting password with passwd after installation
  users.mutableUsers = true;
  users.users.${username} = {
    isNormalUser = true;
    description = username;
    extraGroups = [ "networkmanager" "wheel" "video" "input" "docker" ];
    shell = pkgs.fish;
    # No initialPassword - password set via Forge installer
  };

  # Enable Fish system-wide (required for login shell)
  programs.fish.enable = true;

  # Programs and packages
  services.printing.enable = true;
  programs.firefox = {
    enable = true;
    policies = {
      ExtensionSettings = {
        # 1Password
        "{d634138d-c276-4fc8-924b-40a0ea21d284}" = {
          install_url = "https://addons.mozilla.org/firefox/downloads/latest/1password-x-password-manager/latest.xpi";
          installation_mode = "force_installed";
        };
      };
    };
  };

  nixpkgs.config.allowUnfree = true;

  programs._1password.enable = true;
  programs._1password-gui = {
    enable = true;
    polkitPolicyOwners = [ username ];
  };

  # Google Chrome extension policies (force-install 1Password)
  environment.etc."opt/chrome/policies/managed/extensions.json".text = builtins.toJSON {
    ExtensionInstallForcelist = [
      "aeblfdkhhhdcdjpifhhbdiojplfjncoa;https://clients2.google.com/service/update2/crx"
    ];
  };

  environment.systemPackages = with pkgs; [
    git
    gh
    gcc
    rustup
    bun
    wl-clipboard
    xdg-utils
    efibootmgr
    lm_sensors
    powertop
    nvd # Nix package version diff tool
    forge
  ];

  # Security - passwordless sudo (account has no password)
  security.sudo.wheelNeedsPassword = false;

  # GNOME Keyring - Auto-unlock on login
  services.gnome.gnome-keyring.enable = true;
  security.pam.services.greetd.enableGnomeKeyring = true;
  security.pam.services.login.enableGnomeKeyring = true;

  # Kernel hardening
  boot.kernel.sysctl = {
    # Restrict kernel pointer exposure
    "kernel.kptr_restrict" = 2;
    # Restrict dmesg access to root
    "kernel.dmesg_restrict" = 1;
    # Disable unprivileged BPF
    "kernel.unprivileged_bpf_disabled" = 1;
    # Restrict perf events
    "kernel.perf_event_paranoid" = 3;
    # Prevent null pointer dereference exploits
    "vm.mmap_min_addr" = 65536;
    # Restrict ptrace scope
    "kernel.yama.ptrace_scope" = 1;
    # Network hardening
    "net.ipv4.conf.all.rp_filter" = 1;
    "net.ipv4.conf.default.rp_filter" = 1;
    "net.ipv4.icmp_echo_ignore_broadcasts" = 1;
    "net.ipv4.conf.all.accept_redirects" = 0;
    "net.ipv4.conf.default.accept_redirects" = 0;
    "net.ipv6.conf.all.accept_redirects" = 0;
    "net.ipv6.conf.default.accept_redirects" = 0;
  };

  # I/O scheduler tuning for NVMe (use none/mq-deadline for best performance)
  services.udev.extraRules = ''
    # NVMe drives - use none scheduler (lowest latency)
    ACTION=="add|change", KERNEL=="nvme[0-9]*", ATTR{queue/scheduler}="none"
    # SATA SSDs - use mq-deadline
    ACTION=="add|change", KERNEL=="sd[a-z]", ATTR{queue/rotational}=="0", ATTR{queue/scheduler}="mq-deadline"
  '';

  # System state version
  system.stateVersion = "24.11";
}
