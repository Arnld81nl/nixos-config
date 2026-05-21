# Common NixOS configuration shared across all machines
{ config, pkgs, lib, forge, username, ... }:

let
  # Load secrets (same pattern as home/home.nix)
  secretsPath = "/home/${username}/nixos-config/home/secrets.nix";
  hasSecrets = builtins.pathExists secretsPath;
  secrets = if hasSecrets then import secretsPath else {};
  eduroamIdentity = secrets.eduroam.identity or "user@example.edu";
  eduroamAnonymousIdentity = secrets.eduroam.anonymousIdentity or "anonymous@example.edu";
  eduroamPassword = secrets.eduroam.password or "placeholder";

  # Mic-mute LED: kernel default rule leaves brightness writable only by root,
  # which prevents WirePlumber's user service from syncing it. Hand ownership
  # to the active user with 0660 instead of the chmod 666 we used before.
  hostsWithMicMuteLed = [ "G1a" ];
  micMuteLedPermissions = pkgs.writeShellScript "mic-mute-led-permissions" ''
    set -eu
    brightness="$1"

    [ -e "$brightness" ] || exit 0
    ${pkgs.coreutils}/bin/chown ${username}:users "$brightness" || exit 0
    ${pkgs.coreutils}/bin/chmod 0660 "$brightness" || exit 0
  '';
in
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

  # Optimize store automatically (deduplicates via hard-linking)
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
      libx11
      libxcursor
      libxrandr
      libxi
    ];
  };

  # Make nix-ld libraries available to dlopen (for NixOS-compiled binaries
  # that load shared libs at runtime, e.g. ONNX runtime via fastembed)
  environment.sessionVariables.LD_LIBRARY_PATH = [ "/run/current-system/sw/share/nix-ld/lib" ];

  # Networking
  networking.networkmanager = {
    enable = true;
    wifi.powersave = false;  # Disable WiFi power save to prevent random disconnects
    # Eduroam WPA2-Enterprise profile with roaming support
    ensureProfiles.profiles.eduroam = {
      connection = {
        id = "eduroam";
        type = "wifi";
        autoconnect = "true";
        autoconnect-priority = "100";  # High priority - prefer eduroam when available
        autoconnect-retries = "0";     # Infinite retries (critical for roaming)
      };
      wifi = {
        mode = "infrastructure";
        ssid = "eduroam";
        # No bssid set - allows connecting to any eduroam access point
      };
      wifi-security = {
        key-mgmt = "wpa-eap";
      };
      "802-1x" = {
        eap = "peap;";
        identity = eduroamIdentity;
        anonymous-identity = eduroamAnonymousIdentity;
        password = eduroamPassword;
        phase2-auth = "mschapv2";
        # RADIUS cert chains to a public CA. Point at the system CA bundle
        # explicitly — system-ca-certs=true works for iwd but wpa_supplicant
        # needs a single bundle file, not the /etc/ssl/certs directory.
        ca-cert = "/etc/ssl/certs/ca-bundle.crt";
      };
      ipv4.method = "auto";
      ipv6.method = "auto";
    };
  };

  # Notify apps when network connectivity changes (helps Teams PWA reconnect)
  networking.networkmanager.dispatcherScripts = [
    {
      type = "basic";
      source = pkgs.writeShellScript "99-notify-network-change" ''
        case "$2" in
          up|connectivity-change)
            # Brief delay to let DNS/DHCP settle
            sleep 2
            # Send notification as logged-in user to wake Chrome's event loop
            DISPLAY_USER=$(logname 2>/dev/null || echo "")
            if [ -n "$DISPLAY_USER" ]; then
              UID=$(id -u "$DISPLAY_USER")
              su - "$DISPLAY_USER" -c \
                "DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/$UID/bus notify-send -t 3000 -u low 'Network' 'Connection restored on $1'" &
            fi
            ;;
        esac
      '';
    }
  ];

  # Tailscale VPN
  services.tailscale.enable = true;

  # strongSwan (IPsec) - used by IPsec VPN entries in ~/.config/vpn/config.
  # The daemon runs idle; connections are loaded on-demand by ~/.local/bin/vpn-toggle
  # via `swanctl --load-conns/--load-creds --file <tmpfile>`.
  services.strongswan-swanctl.enable = true;

  # OneDrive sync (abraunegg client with systemd monitor service)
  services.onedrive.enable = true;

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

  # After resume, iwd sometimes thinks wlan0 is still associated when the
  # radio actually dropped. Bounce the radio so autoconnect re-fires cleanly.
  powerManagement.resumeCommands = ''
    ${pkgs.networkmanager}/bin/nmcli radio wifi off || true
    sleep 2
    ${pkgs.networkmanager}/bin/nmcli radio wifi on || true
  '';

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

  # Thunderbolt device management daemon (boltd)
  # Manages device authorization, security, and hot-plug for TB3/TB4
  services.hardware.bolt.enable = true;

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

  # Firmware updates via LVFS (BIOS, EC, etc.)
  services.fwupd.enable = true;

  # SSD TRIM
  services.fstrim.enable = true;

  # Btrfs scrub (use mkDefault so laptops can disable to save battery)
  services.btrfs.autoScrub.enable = lib.mkDefault true;

  # Docker
  virtualisation.docker = {
    enable = true;
    enableOnBoot = true;
  };

  # GPU Screen Recorder needs cap_sys_admin for KMS access
  security.wrappers.gsr-kms-server = {
    owner = "root";
    group = "root";
    capabilities = "cap_sys_admin+ep";
    source = "${pkgs.gpu-screen-recorder}/bin/gsr-kms-server";
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

  # Enable dconf (required for GTK apps to read dark mode preference)
  programs.dconf.enable = true;

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

  # Google Chrome managed policies
  environment.etc."opt/chrome/policies/managed/extensions.json".text = builtins.toJSON {
    ExtensionInstallForcelist = [
      "aeblfdkhhhdcdjpifhhbdiojplfjncoa;https://clients2.google.com/service/update2/crx"
      # Keep Teams Awake - sends fake activity to keep presence "Available"
      # Workaround for Chromium bug: idle detection doesn't work on Wayland (non-GNOME)
      "acofimfooiojfhnokmddfgmlfnjnhobp;https://clients2.google.com/service/update2/crx"
    ];
  };
  environment.etc."opt/chrome/policies/managed/session.json".text = builtins.toJSON {
    # 1 = Restore the last session (suppresses the crash restore dialog)
    RestoreOnStartup = 1;
  };
  environment.etc."opt/chrome/policies/managed/idle-detection.json".text = builtins.toJSON {
    # Allow idle detection globally (fixes "Unknown" presence in Teams PWA)
    # 1 = Allow all sites to use IdleDetection API
    DefaultIdleDetectionSetting = 1;
  };
  environment.etc."opt/chrome/policies/managed/teams-background.json".text = builtins.toJSON {
    # Disable intensive wake-up throttling (background tabs limited to 1 timer/min)
    # Teams PWA needs frequent timers for presence heartbeats and reconnection
    IntensiveWakeUpThrottlingEnabled = false;
    # Prevent Chrome from freezing/discarding Teams tabs
    SleepingTabsBlockedForUrls = [
      "https://[*.]microsoft.com"
      "https://[*.]teams.microsoft.com"
      "https://[*.]cloud.microsoft"
      "https://teams.cloud.microsoft"
      "https://[*.]office.com"
      "https://[*.]live.com"
    ];
  };
  environment.etc."opt/chrome/policies/managed/microsoft-cookies.json".text = builtins.toJSON {
    # Allow third-party cookies for Microsoft auth domains.
    # Microsoft SSO relies on cross-domain cookies between login.microsoftonline.com,
    # login.live.com, and the app domains. Without these, auth tokens expire and
    # PWAs (Teams, Outlook) require daily QR-code 2FA re-authentication.
    ThirdPartyCookiesAllowedForUrls = [
      "https://[*.]microsoftonline.com"
      "https://[*.]microsoft.com"
      "https://[*.]cloud.microsoft"
      "https://teams.cloud.microsoft"
      "https://[*.]live.com"
      "https://[*.]office.com"
      "https://[*.]office365.com"
      "https://[*.]sharepoint.com"
      "https://[*.]teams.microsoft.com"
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
    bubblewrap
    efibootmgr
    lm_sensors
    powertop
    nvd # Nix package version diff tool
    forge
    strongswan # Provides swanctl CLI used by vpn-toggle for IPsec connections
  ];

  # Security - passwordless sudo (account has no password)
  security.sudo.wheelNeedsPassword = false;

  # GNOME Keyring - Auto-unlock on login
  services.gnome.gnome-keyring.enable = true;
  security.pam.services.greetd.enableGnomeKeyring = true;
  security.pam.services.login.enableGnomeKeyring = true;

  # Fingerprint auth support (only on hosts with fprintd enabled, e.g. G1a)
  # Clean PAM service for Noctalia lock screen (no GNOME Keyring interference)
  security.pam.services.noctalia = lib.mkIf config.services.fprintd.enable {};
  # Setuid wrapper for polkit-agent-helper-1 (needed by badged polkit agent)
  security.wrappers.polkit-agent-helper-1 = lib.mkIf config.services.fprintd.enable {
    setuid = true;
    owner = "root";
    group = "root";
    source = "${config.security.polkit.package.out}/lib/polkit-1/polkit-agent-helper-1";
  };

  # Kernel tuning and hardening
  boot.kernel.sysctl = {
    # Reduce swap eagerness (default 60 is too aggressive for workstations with plenty of RAM)
    "vm.swappiness" = 10;
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
    # TCP keepalive tuning - detect dead connections faster after network changes
    # Default keepalive_time is 7200s (2 hours!) which leaves stale WebSockets
    # undetected after WiFi roaming. Total detection time: 60 + (6 × 10) = 120s
    "net.ipv4.tcp_keepalive_time" = 60;    # Start probing after 60s idle
    "net.ipv4.tcp_keepalive_intvl" = 10;   # 10s between probes
    "net.ipv4.tcp_keepalive_probes" = 6;   # Give up after 6 failed probes
  };

  # I/O scheduler tuning for NVMe (use none/mq-deadline for best performance)
  services.udev.extraRules = lib.optionalString
    (builtins.elem config.networking.hostName hostsWithMicMuteLed) ''
    # Allow the active user service (WirePlumber) to sync the hardware mic mute LED.
    SUBSYSTEM=="leds", KERNEL=="hda::micmute", RUN+="${micMuteLedPermissions} %S%p/brightness"
  '' + ''
    # NVMe namespaces - use none scheduler (lowest latency)
    ACTION=="add|change", KERNEL=="nvme[0-9]n[0-9]", ATTR{queue/scheduler}="none"
    # SATA SSDs - use mq-deadline
    ACTION=="add|change", KERNEL=="sd[a-z]", ATTR{queue/rotational}=="0", ATTR{queue/scheduler}="mq-deadline"

    # Auto-authorize Thunderbolt devices when IOMMU DMA protection is active
    # This is safe because IOMMU provides hardware-level DMA attack protection
    ACTION=="add", SUBSYSTEM=="thunderbolt", ATTRS{iommu_dma_protection}=="1", ATTR{authorized}=="0", ATTR{authorized}="1"
  '';

  # CPU frequency scaling - schedutil adapts to scheduler load
  boot.kernelParams = [ "cpufreq.default_governor=schedutil" ];

  # System state version
  system.stateVersion = "24.11";
}
