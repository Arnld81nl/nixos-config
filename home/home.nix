# Home Manager configuration
{ config, pkgs, inputs, lib, osConfig, username, ... }:

let
  # Get shell from NixOS config (set by specialisations)
  shell = osConfig.desktop.shell;

  # Load secrets from local file (gitignored) or use placeholders
  # Use absolute path because gitignored files aren't included in flake source
  secretsPath = /home/arnold/nixos-config/home/secrets.nix;
  hasSecrets = builtins.pathExists secretsPath;
  secrets = if hasSecrets then import secretsPath else {
    gitEmail = "your-email@example.com";
    onePassword = {
      account = "my";
      ageKey = "op://Private/age-key/private-key";
      sshKey = "op://Private/ssh-key/private key";
    };
    vpn = {
      vpn1 = { host = "0.0.0.0:10443"; opItem = "Vault/VPN-Item"; cert = ""; };
      vpn2 = { host = "0.0.0.0:443"; opItem = "Vault/VPN-Item"; cert = ""; };
      vpn3 = { host = "0.0.0.0:443"; opItem = "Vault/VPN-Item"; opAccount = "my"; cert = ""; ovpnConfig = ""; };
      vpn4 = { wgConfig = ""; };  
    };
  };

  # Dynamically load all wallpapers from ../wallpapers directory
  wallpapersDir = ../wallpapers;
  wallpaperFiles = builtins.readDir wallpapersDir;
  wallpaperEntries = lib.mapAttrs' (name: _: {
    name = "Pictures/Wallpapers/${name}";
    value = { source = wallpapersDir + "/${name}"; };
  }) (lib.filterAttrs (name: type: type == "regular") wallpaperFiles);
in
{
  imports = [
    ./hyprland        # Modular Hyprland config (includes hypridle)
    ./ghostty.nix
    ./neovim.nix      # Neovim with LazyVim dependencies
    ./1password-secrets.nix  # 1Password SSH agent integration
    ./app-backup  # App profile backup/restore (browsers)
    ./forge-notify.nix  # Background update checker
    # Always deploy Illogical Impulse dotfiles (Quickshell config)
    # Required because Home Manager evaluates with default shell at build time,
    # but specialisations need these files at boot time. See CLAUDE.md.
    ./shells/illogical/dotfiles-only.nix
  ] ++ (if shell == "illogical" then [
    ./shells/illogical
  ] else [
    inputs.noctalia.homeModules.default
    ./shells/noctalia
  ]);

  home.username = username;
  home.homeDirectory = "/home/${username}";

  # Let Home Manager manage itself
  programs.home-manager.enable = true;

  # Git configuration
  programs.git = {
    enable = true;
    settings.user = {
      name = "Arnld81nl";
      email = secrets.gitEmail;
    };
  };

  # XDG user directories
  xdg.userDirs = {
    enable = true;
    createDirectories = true;
    desktop = null;  # Don't create Desktop
    documents = "${config.home.homeDirectory}/Documents";
    download = "${config.home.homeDirectory}/Downloads";
    music = null;
    pictures = "${config.home.homeDirectory}/Pictures";
    publicShare = null;
    templates = null;
    videos = null;
    extraConfig = {
      XDG_CODE_DIR = "${config.home.homeDirectory}/Code";
    };
  };

  # D-Bus service for Nautilus quick preview (sushi)
  xdg.dataFile."dbus-1/services/org.gnome.NautilusPreviewer.service".source =
    "${pkgs.sushi}/share/dbus-1/services/org.gnome.NautilusPreviewer.service";

  # Home file entries (merged with wallpapers)
  home.file = wallpaperEntries // {
    # Ensure custom directories exist
    "Code/.keep".text = "";
    "Pictures/Screenshots/.keep".text = "";

    # Screenshot script
    ".local/bin/screenshot" = {
      source = ./scripts/screenshot;
      executable = true;
    };
    # Clipboard image -> file helper (for CLI tools expecting file URLs)
    ".local/bin/clipboard-image-to-file" = {
      source = ./scripts/clipboard-image-to-file;
      executable = true;
    };
    # Wrapper for Satty copy command (copies image + converts for CLI tools)
    ".local/bin/clipboard-copy-image" = {
      source = ./scripts/clipboard-copy-image;
      executable = true;
    };

    # User profile picture (used by GDM, SDDM, etc.)
    ".face".source = ../face;

    # npm config for global packages (avoids permission issues)
    ".npmrc".text = ''
      prefix=''${HOME}/.npm-global
    '';

    # VPN toggle scripts with 1Password integration
    # Reads VPN config from ~/.config/vpn/config (see ~/.config/vpn/config.example)
    ".local/bin/vpn-toggle" = {
      executable = true;
      text = ''
        #!/usr/bin/env bash
        # Generic VPN toggle script
        # Usage: vpn-toggle <name>
        # Config: ~/.config/vpn/config

        CONFIG_FILE="$HOME/.config/vpn/config"
        if [[ ! -f "$CONFIG_FILE" ]]; then
          echo "Error: VPN config not found at $CONFIG_FILE"
          echo "Copy ~/.config/vpn/config.example and fill in your values."
          exit 1
        fi
        source "$CONFIG_FILE"

        # Ensure we have the proper environment for 1Password CLI
        export XDG_RUNTIME_DIR="''${XDG_RUNTIME_DIR:-/run/user/$(id -u)}"

        # Import systemd user environment (for 1Password socket access)
        if command -v systemctl &>/dev/null; then
          eval "$(systemctl --user show-environment 2>/dev/null | sed 's/^/export /')"
        fi

        NAME="$1"
        NAME_UPPER=$(echo "$NAME" | tr '[:lower:]' '[:upper:]')

        # Get config for this VPN
        eval "HOST=\$VPN_''${NAME_UPPER}_HOST"
        eval "OP_ITEM=\$VPN_''${NAME_UPPER}_OP_ITEM"
        eval "TRUSTED_CERT=\$VPN_''${NAME_UPPER}_CERT"
        eval "VPN_TYPE=\$VPN_''${NAME_UPPER}_TYPE"

        if [[ -z "$HOST" ]]; then
          echo "Error: VPN '$NAME' not configured in $CONFIG_FILE"
          exit 1
        fi

        # Extract just IP for pgrep (HOST is ip:port, process shows "ip port")
        IP="''${HOST%%:*}"

        # Handle OpenVPN-based VPNs (like VPN3/SSL VPN vendor)
        if [[ "$VPN_TYPE" == "openvpn" ]]; then
          # Check if this specific OpenVPN is connected
          if pgrep -x openvpn > /dev/null 2>&1 && pgrep -fa openvpn | grep -q "$IP"; then
            echo "Disconnecting $NAME VPN..."
            sudo pkill -f "openvpn.*$IP"
            notify-send "VPN $NAME" "Disconnected" -i network-vpn-symbolic
            echo "Disconnected."
            exit 0
          fi

          echo "Connecting to $NAME VPN (OpenVPN)..."

          # Get username and password from 1Password
          USER=$(op read "op://$OP_ITEM/username" --account "$OP_ACCOUNT" 2>/dev/null)
          PASSWORD=$(op read "op://$OP_ITEM/password" --account "$OP_ACCOUNT" 2>/dev/null)
          if [ -z "$PASSWORD" ] || [ -z "$USER" ]; then
            notify-send "VPN $NAME" "Failed to get credentials from 1Password" -i dialog-error
            echo "Error: Could not retrieve credentials from 1Password."
            echo "Make sure 1Password is unlocked and item '$OP_ITEM' exists with username and password fields."
            exit 1
          fi

          notify-send "VPN $NAME" "Connecting..." -i network-vpn-acquiring-symbolic

          # Create temp file with credentials for auth-user-pass
          CREDS_FILE=$(mktemp)
          echo "$USER" > "$CREDS_FILE"
          echo "$PASSWORD" >> "$CREDS_FILE"
          chmod 600 "$CREDS_FILE"

          # Expand $HOME in the ovpn file paths
          OVPN_CONFIG="$HOME/.config/vpn/vpn3/vpn3.ovpn"

          # Connect using OpenVPN with credentials file
          sudo openvpn --config "$OVPN_CONFIG" --auth-user-pass "$CREDS_FILE" --daemon --log /tmp/vpn-$NAME.log

          # Clean up credentials file after a brief delay (give openvpn time to read it)
          (sleep 2 && rm -f "$CREDS_FILE") &

          # Wait a moment and check if connected
          sleep 5
          if pgrep -x openvpn > /dev/null 2>&1 && pgrep -fa openvpn | grep -q "$IP"; then
            notify-send "VPN $NAME" "Connected" -i network-vpn-symbolic
            echo "Connected to $NAME VPN."
          else
            notify-send "VPN $NAME" "Connection failed - check log" -i dialog-error
            echo "Connection failed. Check /tmp/vpn-$NAME.log"
            cat /tmp/vpn-$NAME.log
            exit 1
          fi
          exit 0
        fi

        # Handle Fortinet VPNs (default)
        # Check if this specific VPN is connected (by checking process)
        if pgrep -x openfortivpn > /dev/null 2>&1 && pgrep -fa openfortivpn | grep -q "$IP"; then
          echo "Disconnecting $NAME VPN..."
          sudo pkill -f "openfortivpn.*$IP"
          notify-send "VPN $NAME" "Disconnected" -i network-vpn-symbolic
          echo "Disconnected."
          exit 0
        fi

        echo "Connecting to $NAME VPN..."

        # Get username and password from 1Password
        USER=$(op read "op://$OP_ITEM/username" --account "$OP_ACCOUNT" 2>/dev/null)
        PASSWORD=$(op read "op://$OP_ITEM/password" --account "$OP_ACCOUNT" 2>/dev/null)
        if [ -z "$PASSWORD" ] || [ -z "$USER" ]; then
          notify-send "VPN $NAME" "Failed to get credentials from 1Password" -i dialog-error
          echo "Error: Could not retrieve credentials from 1Password."
          echo "Make sure 1Password is unlocked and item '$OP_ITEM' exists with username and password fields."
          exit 1
        fi

        notify-send "VPN $NAME" "Connecting..." -i network-vpn-acquiring-symbolic

        # Connect in background, redirect output to log
        sudo openfortivpn "$HOST" -u "$USER" -p "$PASSWORD" ''${TRUSTED_CERT:+--trusted-cert "$TRUSTED_CERT"} > /tmp/vpn-$NAME.log 2>&1 &

        # Wait a moment and check if connected
        sleep 3
        if pgrep -x openfortivpn > /dev/null 2>&1 && pgrep -fa openfortivpn | grep -q "$IP"; then
          notify-send "VPN $NAME" "Connected" -i network-vpn-symbolic
          echo "Connected to $NAME VPN."
        else
          notify-send "VPN $NAME" "Connection failed - check log" -i dialog-error
          echo "Connection failed. Check /tmp/vpn-$NAME.log"
          cat /tmp/vpn-$NAME.log
          exit 1
        fi
      '';
    };

    ".local/bin/vpn-status" = {
      executable = true;
      text = ''
        #!/usr/bin/env bash
        # Check status of all VPNs and output JSON for widgets

        CONFIG_FILE="$HOME/.config/vpn/config"
        if [[ ! -f "$CONFIG_FILE" ]]; then
          echo '{"vpn2": false, "vpn1": false, "vpn3": false}'
          exit 0
        fi
        source "$CONFIG_FILE"

        check_fortivpn() {
          local ip="$1"
          if pgrep -x openfortivpn > /dev/null 2>&1 && pgrep -fa openfortivpn 2>/dev/null | grep -q "$ip"; then
            echo "true"
          else
            echo "false"
          fi
        }

        check_openvpn_vpn3() {
          if pgrep -fa "openvpn.*vpn3" > /dev/null 2>&1; then
            echo "true"
          else
            echo "false"
          fi
        }

        check_wireguard_vpn4() {
          if ip link show wg-vpn4 > /dev/null 2>&1; then
            echo "true"
          else
            echo "false"
          fi
        }

        vpn2_connected=$(check_fortivpn "''${VPN_2_HOST%%:*}")
        vpn1_connected=$(check_fortivpn "''${VPN_1_HOST%%:*}")
        vpn3_connected=$(check_openvpn_vpn3)
        vpn4_connected=$(check_wireguard_vpn4)

        echo "{\"vpn2\": $vpn2_connected, \"vpn1\": $vpn1_connected, \"vpn3\": $vpn3_connected, \"vpn4\": $vpn4_connected}"
      '';
    };

    # Individual VPN status scripts for Noctalia CustomButton widgets
    # Output JSON: {"text": "NAME ●/○", "icon": ""}
    # Uses pgrep -x for exact process name match, then grep for IP
    ".local/bin/vpn-status-1" = {
      executable = true;
      text = ''
        #!/usr/bin/env bash
        CONFIG_FILE="$HOME/.config/vpn/config"
        if [[ -f "$CONFIG_FILE" ]]; then
          source "$CONFIG_FILE"
          IP="''${VPN_1_HOST%%:*}"
          if pgrep -x openfortivpn > /dev/null 2>&1 && pgrep -fa openfortivpn 2>/dev/null | grep -q "$IP"; then
            echo '{"text": "${secrets.vpn.vpn1.name or "VPN 1"} ●", "icon": ""}'
            exit 0
          fi
        fi
        echo '{"text": "${secrets.vpn.vpn1.name or "VPN 1"} ○", "icon": ""}'
      '';
    };

    ".local/bin/vpn-status-2" = {
      executable = true;
      text = ''
        #!/usr/bin/env bash
        CONFIG_FILE="$HOME/.config/vpn/config"
        if [[ -f "$CONFIG_FILE" ]]; then
          source "$CONFIG_FILE"
          IP="''${VPN_2_HOST%%:*}"
          if pgrep -x openfortivpn > /dev/null 2>&1 && pgrep -fa openfortivpn 2>/dev/null | grep -q "$IP"; then
            echo '{"text": "${secrets.vpn.vpn2.name or "VPN 2"} ●", "icon": ""}'
            exit 0
          fi
        fi
        echo '{"text": "${secrets.vpn.vpn2.name or "VPN 2"} ○", "icon": ""}'
      '';
    };

    ".local/bin/vpn-status-3" = {
      executable = true;
      text = ''
        #!/usr/bin/env bash
        # OpenVPN - check for tun0 with VPN subnet IP
        if ip addr show tun0 2>/dev/null | grep -q "0.0.0"; then
          echo '{"text": "${secrets.vpn.vpn3.name or "VPN 3"} ●", "icon": ""}'
        else
          echo '{"text": "${secrets.vpn.vpn3.name or "VPN 3"} ○", "icon": ""}'
        fi
      '';
    };

    ".local/bin/vpn-status-4" = {
      executable = true;
      text = ''
        #!/usr/bin/env bash
        # WireGuard VPN
        if ip link show wg-vpn4 > /dev/null 2>&1; then
          echo '{"text": "${secrets.vpn.vpn4.name or "VPN 4"} ●", "icon": ""}'
        else
          echo '{"text": "${secrets.vpn.vpn4.name or "VPN 4"} ○", "icon": ""}'
        fi
      '';
    };

    ".local/bin/vpn-2" = {
      executable = true;
      text = ''
        #!/usr/bin/env bash
        exec "$HOME/.local/bin/vpn-toggle" "2"
      '';
    };

    ".local/bin/vpn-1" = {
      executable = true;
      text = ''
        #!/usr/bin/env bash
        exec "$HOME/.local/bin/vpn-toggle" "1"
      '';
    };

    ".local/bin/vpn-3" = {
      executable = true;
      text = ''
        #!/usr/bin/env bash
        # OpenVPN-based VPN

        CONFIG="$HOME/.config/vpn/vpn3.ovpn"
        NAME="${secrets.vpn.vpn3.name or "VPN 3"}"
        OP_ITEM="${secrets.vpn.vpn3.opItem}"
        OP_ACCOUNT="${secrets.vpn.vpn3.opAccount}"

        # Check if already connected (look for openvpn with vpn3 config)
        if pgrep -fa "openvpn.*vpn3" > /dev/null 2>&1; then
          echo "Disconnecting $NAME VPN..."
          sudo pkill -f "openvpn.*vpn3"
          notify-send "VPN $NAME" "Disconnected" -i network-vpn-symbolic
          echo "Disconnected."
          exit 0
        fi

        if [[ ! -f "$CONFIG" ]]; then
          echo "Error: OpenVPN config not found at $CONFIG"
          exit 1
        fi

        echo "Connecting to $NAME VPN..."

        # Get username and password from 1Password
        USER=$(op read "op://$OP_ITEM/username" --account "$OP_ACCOUNT" 2>/dev/null)
        PASSWORD=$(op read "op://$OP_ITEM/password" --account "$OP_ACCOUNT" 2>/dev/null)
        if [ -z "$PASSWORD" ] || [ -z "$USER" ]; then
          notify-send "VPN $NAME" "Failed to get credentials from 1Password" -i dialog-error
          echo "Error: Could not retrieve credentials from 1Password."
          echo "Make sure 1Password is unlocked and item '$OP_ITEM' exists with username and password fields."
          exit 1
        fi

        notify-send "VPN $NAME" "Connecting..." -i network-vpn-acquiring-symbolic

        # Create temp credentials file (OpenVPN format: username on line 1, password on line 2)
        CREDS_FILE=$(mktemp)
        chmod 600 "$CREDS_FILE"
        echo "$USER" > "$CREDS_FILE"
        echo "$PASSWORD" >> "$CREDS_FILE"

        # Connect in background with credentials file
        sudo openvpn --config "$CONFIG" --auth-user-pass "$CREDS_FILE" --daemon --log /tmp/vpn-$NAME.log

        # Wait a moment then clean up credentials file
        sleep 2
        rm -f "$CREDS_FILE"

        # Check connection
        sleep 3
        if pgrep -fa "openvpn.*vpn3" > /dev/null 2>&1; then
          notify-send "VPN $NAME" "Connected" -i network-vpn-symbolic
          echo "Connected to $NAME VPN."
        else
          notify-send "VPN $NAME" "Connection failed - check log" -i dialog-error
          echo "Connection failed. Check /tmp/vpn-$NAME.log"
          cat /tmp/vpn-$NAME.log
          exit 1
        fi
      '';
    };

    # VPN 3 - OpenVPN config (OpenVPN)
    # Config content comes from secrets.nix
    ".config/vpn/vpn3.ovpn" = lib.mkIf (secrets.vpn.vpn3.ovpnConfig != "") {
      text = secrets.vpn.vpn3.ovpnConfig;
    };

    ".local/bin/vpn-4" = {
      executable = true;
      text = ''
        #!/usr/bin/env bash
        # VPN 4 - WireGuard

        CONFIG="$HOME/.config/vpn/wg-vpn4.conf"
        NAME="VPN 4"
        INTERFACE="wg-vpn4"

        # Check if already connected
        if ip link show "$INTERFACE" > /dev/null 2>&1; then
          echo "Disconnecting $NAME VPN..."
          sudo wg-quick down "$CONFIG"
          notify-send "VPN $NAME" "Disconnected" -i network-vpn-symbolic
          echo "Disconnected."
          exit 0
        fi

        if [[ ! -f "$CONFIG" ]]; then
          echo "Error: WireGuard config not found at $CONFIG"
          echo "Add your config to secrets.nix and rebuild."
          exit 1
        fi

        echo "Connecting to $NAME VPN..."
        notify-send "VPN $NAME" "Connecting..." -i network-vpn-acquiring-symbolic

        sudo wg-quick up "$CONFIG"

        # Check connection
        sleep 2
        if ip link show "$INTERFACE" > /dev/null 2>&1; then
          notify-send "VPN $NAME" "Connected" -i network-vpn-symbolic
          echo "Connected to $NAME VPN."
        else
          notify-send "VPN $NAME" "Connection failed" -i dialog-error
          echo "Connection failed."
          exit 1
        fi
      '';
    };

    # VPN 4 - WireGuard config
    # Config content comes from secrets.nix
    ".config/vpn/wg-vpn4.conf" = lib.mkIf (secrets.vpn.vpn4.wgConfig != "") {
      text = secrets.vpn.vpn4.wgConfig;
    };

    # VPN config example (user creates config from this)
    ".config/vpn/config.example" = {
      text = ''
        # VPN Configuration
        # Copy this to ~/.config/vpn/config and fill in your values

        # 1Password account for work items
        OP_ACCOUNT="my"

        # VPN 1 (username/password from 1Password)
        VPN_1_HOST="0.0.0.0:10443"
        VPN_1_OP_ITEM="Vault/VPN-Item"
        VPN_1_CERT=""  # Will be shown on first connect

        # VPN 2 (username/password from 1Password)
        VPN_2_HOST="0.0.0.0:443"
        VPN_2_OP_ITEM="Vault/VPN-Item"
        VPN_2_CERT=""  # Will be shown on first connect

        # VPN 3 (username/password from 1Password)
        VPN_VPN3_HOST="0.0.0.0:443"
        VPN_VPN3_OP_ITEM="Vault/VPN-Item"
        VPN_VPN3_CERT=""  # Needs separate certificates
      '';
    };

    # Actual VPN config (generated from secrets.nix)
    ".config/vpn/config" = {
      text = ''
        # VPN Configuration (auto-generated from secrets.nix)

        # 1Password account
        OP_ACCOUNT="${secrets.onePassword.account}"

        # VPN 1
        VPN_1_HOST="${secrets.vpn.vpn1.host}"
        VPN_1_OP_ITEM="${secrets.vpn.vpn1.opItem}"
        VPN_1_CERT="${secrets.vpn.vpn1.cert}"

        # VPN 2
        VPN_2_HOST="${secrets.vpn.vpn2.host}"
        VPN_2_OP_ITEM="${secrets.vpn.vpn2.opItem}"
        VPN_2_CERT="${secrets.vpn.vpn2.cert}"

        # VPN 3 (OpenVPN with client certificates)
        VPN_VPN3_HOST="${secrets.vpn.vpn3.host}"
        VPN_VPN3_OP_ITEM="${secrets.vpn.vpn3.opItem}"
        VPN_VPN3_CERT="${secrets.vpn.vpn3.cert}"
        VPN_VPN3_TYPE="openvpn"
      '';
    };

    # VPN3 VPN certificates (OpenVPN)
    ".config/vpn/vpn3/ca.crt" = {
      text = secrets.vpn.vpn3.caCert or "";
    };
    ".config/vpn/vpn3/client.crt" = {
      text = secrets.vpn.vpn3.clientCert or "";
    };
    ".config/vpn/vpn3/client.key" = {
      text = secrets.vpn.vpn3.clientKey or "";
    };
    ".config/vpn/vpn3/vpn3.ovpn" = {
      text = ''
        client
        dev tun
        proto tcp
        remote 0.0.0.0 443
        resolv-retry infinite
        nobind
        persist-key
        persist-tun
        ca /home/${username}/.config/vpn/vpn3/ca.crt
        cert /home/${username}/.config/vpn/vpn3/client.crt
        key /home/${username}/.config/vpn/vpn3/client.key
        cipher AES-256-GCM
        auth SHA256
        verb 3
        auth-user-pass
        auth-nocache
      '';
    };

    # PWA icons (Microsoft 365 apps) - multiple sizes for proper display
    # Note: No custom index.theme needed - system hicolor theme already declares these directories
    ".local/share/icons/hicolor/48x48/apps/outlook-pwa.png".source = ./icons/outlook-pwa-48.png;
    ".local/share/icons/hicolor/48x48/apps/teams-pwa.png".source = ./icons/teams-pwa-48.png;
    ".local/share/icons/hicolor/128x128/apps/outlook-pwa.png".source = ./icons/outlook-pwa.png;
    ".local/share/icons/hicolor/128x128/apps/teams-pwa.png".source = ./icons/teams-pwa.png;
    ".local/share/icons/hicolor/256x256/apps/outlook-pwa.png".source = ./icons/outlook-pwa-256.png;
    ".local/share/icons/hicolor/256x256/apps/teams-pwa.png".source = ./icons/teams-pwa-256.png;
    ".local/share/icons/hicolor/48x48/apps/onedrive.png".source = ./icons/onedrive-48.png;
    ".local/share/icons/hicolor/128x128/apps/onedrive.png".source = ./icons/onedrive.png;
    ".local/share/icons/hicolor/256x256/apps/onedrive.png".source = ./icons/onedrive-256.png;
  };

  # Desktop entry overrides for Wayland
  xdg.desktopEntries.termius-app = {
    name = "Termius";
    exec = "termius-app --enable-features=UseOzonePlatform,WaylandWindowDecorations --ozone-platform=wayland %U";
    icon = "termius-app";
    comment = "SSH platform for Mobile and Desktop";
    categories = [ "Network" "Security" ];
    mimeType = [ "x-scheme-handler/termius" "x-scheme-handler/ssh" ];
  };

  xdg.desktopEntries."1password" = {
    name = "1Password";
    exec = "1password --enable-features=UseOzonePlatform,WaylandWindowDecorations --ozone-platform=wayland %U";
    icon = "1password";
    comment = "Password Manager";
    categories = [ "Office" "Security" ];
  };

  xdg.desktopEntries.onlyoffice-desktopeditors = {
    name = "OnlyOffice Desktop Editors";
    exec = "onlyoffice-desktopeditors --enable-features=UseOzonePlatform,WaylandWindowDecorations --ozone-platform=wayland %U";
    icon = "onlyoffice-desktopeditors";
    comment = "Office productivity suite";
    categories = [ "Office" ];
    mimeType = [
      "application/vnd.oasis.opendocument.text"
      "application/vnd.oasis.opendocument.spreadsheet"
      "application/vnd.oasis.opendocument.presentation"
      "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
      "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
      "application/vnd.openxmlformats-officedocument.presentationml.presentation"
      "application/msword"
      "application/vnd.ms-excel"
      "application/vnd.ms-powerpoint"
    ];
  };

  # OneDriveGUI with proper icon
  xdg.desktopEntries.onedrivegui = {
    name = "OneDrive";
    exec = "onedrivegui";
    icon = "onedrive";
    comment = "OneDrive sync client";
    categories = [ "Utility" "Network" ];
    terminal = false;
  };

  # Microsoft 365 PWA apps (Chrome-based)
  xdg.desktopEntries.outlook-pwa = {
    name = "Outlook";
    exec = "google-chrome-stable --profile-directory=Default --app-id=eoficlgicibekocmfdomjbfnjmehnhcd %U";
    icon = "outlook-pwa";
    comment = "Microsoft Outlook web app";
    categories = [ "Network" "Email" "Office" ];
    terminal = false;
    mimeType = [ "x-scheme-handler/mailto" ];
    startupNotify = true;
    settings = {
      StartupWMClass = "crx_eoficlgicibekocmfdomjbfnjmehnhcd";
    };
  };

  xdg.desktopEntries.teams-pwa = {
    name = "Microsoft Teams";
    exec = "google-chrome-stable --profile-directory=Default --app-id=ompifgpmddkgmclendfeacglnodjjndh %U";
    icon = "teams-pwa";
    comment = "Microsoft Teams web app";
    categories = [ "Network" "InstantMessaging" "Office" ];
    terminal = false;
    mimeType = [ "x-scheme-handler/web+msteams" ];
    startupNotify = true;
    settings = {
      StartupWMClass = "crx_ompifgpmddkgmclendfeacglnodjjndh";
    };
  };

  # User packages
  home.packages = with pkgs; [
    # XDG portal for GTK apps (dark mode, file dialogs)
    xdg-desktop-portal-gtk

    # Screenshot tools
    grim
    slurp
    satty
    wayfreeze
    wl-clipboard
    hyprpicker

    # File management
    nautilus
    sushi # Quick preview for Nautilus (press SPACE)

    # Theming
    nwg-look

    # Media control
    brightnessctl
    playerctl

    # Applications
    remmina          # remote desktop client (RDP, VNC, SSH)
    openfortivpn             # Fortinet SSL VPN client
    openfortivpn-webview-qt  # SAML/SSO authentication helper
    openvpn                  # OpenVPN client (for OpenVPN)
    wireguard-tools          # WireGuard VPN client ()
    libnotify        # notify-send for VPN toggle notifications
    spotify
    lazydocker
    btop
    gnome-calculator
    gnome-text-editor
    fastfetch
    jq
    nodejs
    termius
    lazygit
    ripgrep
    fd

    # CLI enhancements
    bat              # cat with syntax highlighting

    # Media
    mpv              # video player
    imv              # image viewer
    pinta            # image editor

    # Productivity
    evince           # document/PDF viewer
    localsend        # local file sharing
    onlyoffice-desktopeditors  # office suite

    # Cloud storage
    onedrivegui      # OneDrive sync with GUI

    # Fonts
    font-awesome
    noto-fonts
    noto-fonts-color-emoji
    nerd-fonts.jetbrains-mono
    nerd-fonts.fira-code
  ];

  # Web browsers
  programs.google-chrome.enable = true;

  programs.firefox.enable = true;

  # Direnv - auto-activate nix develop shells when entering directories
  # Add `.envrc` with `use flake` to your Rust projects
  programs.direnv = {
    enable = true;
    nix-direnv.enable = true;  # Caches dev shell evaluation
  };

  # App profile backup/restore (browsers - encrypted, synced via GitHub)
  # IMPORTANT: After first rebuild, edit ~/.config/app-backup/config with your values
  # See ~/.config/app-backup/config.example for the template
  programs.app-backup = {
    enable = true;
    # These are placeholder values - override in ~/.config/app-backup/config after rebuild
    repoUrl = "git@github.com:YOUR_USER/private-settings.git";
    ageRecipient = "age1your-public-key-here";
    ageKey1Password = secrets.onePassword.ageKey;
    ageKeyPath = "~/.config/age/key.txt";
    sshKey1Password = secrets.onePassword.sshKey;
    sshKeyPath = "~/.ssh/id_ed25519";
  };

  # Default applications
  xdg.mimeApps = {
    enable = true;
    defaultApplications = {
      # Browser
      "text/html" = "google-chrome.desktop";
      "x-scheme-handler/http" = "google-chrome.desktop";
      "x-scheme-handler/https" = "google-chrome.desktop";
      "x-scheme-handler/about" = "google-chrome.desktop";
      "x-scheme-handler/unknown" = "google-chrome.desktop";

      # Images (imv)
      "image/png" = "imv.desktop";
      "image/jpeg" = "imv.desktop";
      "image/gif" = "imv.desktop";
      "image/webp" = "imv.desktop";
      "image/bmp" = "imv.desktop";
      "image/tiff" = "imv.desktop";

      # PDF (Evince)
      "application/pdf" = "org.gnome.Evince.desktop";

      # Videos (mpv)
      "video/mp4" = "mpv.desktop";
      "video/x-matroska" = "mpv.desktop";
      "video/webm" = "mpv.desktop";
      "video/x-msvideo" = "mpv.desktop";
      "video/quicktime" = "mpv.desktop";
    };
  };


  # Add npm global bin and Claude Code to PATH
  home.sessionPath = [
    "$HOME/.npm-global/bin"
    "$HOME/.local/bin"
  ];

  # Install Claude Code native binary if not present
  home.activation.installClaudeCode = lib.hm.dag.entryAfter [ "writeBoundary" ] ''
    if [ ! -x "$HOME/.local/bin/claude" ]; then
      # Use 3 second timeout for connectivity check
      if ${pkgs.curl}/bin/curl -m 3 -fsSL https://claude.ai/install.sh >/dev/null 2>&1; then
        PATH="${pkgs.curl}/bin:${pkgs.coreutils}/bin:${pkgs.gnutar}/bin:${pkgs.gzip}/bin:$PATH" \
          $DRY_RUN_CMD ${pkgs.bash}/bin/bash -c "curl -fsSL https://claude.ai/install.sh | bash" || \
          echo "Claude Code install failed (offline or installer issue)"
      else
        echo "Claude Code install skipped (offline)"
      fi
    fi
  '';

  # Install OpenAI Codex CLI via npm if not present
  home.activation.installCodexCLI = lib.hm.dag.entryAfter [ "writeBoundary" ] ''
    if [ ! -x "$HOME/.npm-global/bin/codex" ]; then
      # Use 3 second timeout for connectivity check
      if ${pkgs.curl}/bin/curl -m 3 -fsSL https://registry.npmjs.org/ >/dev/null 2>&1; then
        $DRY_RUN_CMD ${pkgs.nodejs}/bin/npm install -g @openai/codex || \
          echo "Codex CLI install failed (offline or npm issue)"
      else
        echo "Codex CLI install skipped (offline)"
      fi
    fi
  '';

  # GTK theme settings (affects Nautilus and other GTK apps)
  dconf.settings = {
    "org/gnome/desktop/interface" = {
      color-scheme = "prefer-dark";
    };
  };

  # Environment variables
  home.sessionVariables = {
    EDITOR = "nvim";
    BROWSER = "google-chrome-stable";
    TERMINAL = "ghostty";

    # 1Password SSH agent (ensures consistent use across all tools)
    SSH_AUTH_SOCK = "$HOME/.1password/agent.sock";

    # Wayland-specific (NIXOS_OZONE_WL is set in configuration.nix)
    MOZ_ENABLE_WAYLAND = "1";
    QT_QPA_PLATFORM = "wayland";
    SDL_VIDEODRIVER = "wayland";
    XDG_SESSION_TYPE = "wayland";
  };

  # === Battery notification service (laptops) ===
  # Sends desktop notifications at low battery levels and suspends at danger level
  services.batsignal = {
    enable = true;
    extraArgs = [
      "-w" "20"   # Warning at 20%
      "-c" "10"   # Critical at 10%
      "-d" "5"    # Danger at 5%
      "-p"        # Include battery percentage in notifications
      "-e"        # Notify on full battery too
      "-D" "systemctl suspend"  # Suspend at danger level
    ];
  };

  # === Mic mute LED sync service (G1a only) ===
  # The kernel's audio-micmute LED trigger doesn't sync with WirePlumber/PipeWire.
  # This service polls the mic mute state and updates the LED accordingly.
  systemd.user.services.mic-led-sync = lib.mkIf (osConfig.networking.hostName == "G1a") {
    Unit = {
      Description = "Sync mic mute LED with WirePlumber state";
      After = [ "pipewire.service" "wireplumber.service" ];
      PartOf = [ "graphical-session.target" ];
    };
    Service = {
      Type = "simple";
      ExecStart = pkgs.writeShellScript "mic-led-sync" ''
        LED_PATH="/sys/class/leds/hda::micmute/brightness"

        # Wait for LED interface to be available
        while [ ! -w "$LED_PATH" ]; do
          sleep 1
        done

        # Sync loop
        while true; do
          if ${pkgs.wireplumber}/bin/wpctl get-volume @DEFAULT_AUDIO_SOURCE@ 2>/dev/null | grep -q MUTED; then
            echo 1 > "$LED_PATH" 2>/dev/null || true
          else
            echo 0 > "$LED_PATH" 2>/dev/null || true
          fi
          sleep 0.3
        done
      '';
      Restart = "always";
      RestartSec = 5;
    };
    Install = {
      WantedBy = [ "graphical-session.target" ];
    };
  };

  # State version (should match NixOS)
  home.stateVersion = "24.11";
}
