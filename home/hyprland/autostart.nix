# Autostart configuration
# Programs to run at Hyprland startup (shell-aware)
{ shell ? "noctalia", hasFprintd ? false }:

let
  # Extra env vars for fingerprint-capable hosts
  pamServiceExport = if hasFprintd then " NOCTALIA_PAM_SERVICE" else "";

  # Shell-specific autostart commands
  illogicalAutostart = ''
    # Systemd integration - export environment for user services
    # Include HYPRLAND_INSTANCE_SIGNATURE so portal services can connect
    exec-once = systemctl --user import-environment WAYLAND_DISPLAY XDG_CURRENT_DESKTOP HYPRLAND_INSTANCE_SIGNATURE${pamServiceExport}
    exec-once = dbus-update-activation-environment --systemd WAYLAND_DISPLAY XDG_CURRENT_DESKTOP HYPRLAND_INSTANCE_SIGNATURE${pamServiceExport}
    exec-once = dbus-update-activation-environment --all

    # Restart portal services to pick up new environment (fixes restart via greetd)
    exec-once = sleep 1 && systemctl --user restart xdg-desktop-portal-hyprland xdg-desktop-portal

    # Core components
    exec-once = gnome-keyring-daemon --start --components=secrets
    exec-once = hypridle

    # Start Quickshell with illogical-impulse config
    exec-once = quickshell -c ~/.config/quickshell/ii

    # Clipboard history with quickshell integration
    exec-once = wl-paste --type text --watch cliphist store
    exec-once = wl-paste --type image --watch cliphist store

    # Set cursor theme
    exec-once = hyprctl setcursor Bibata-Modern-Classic 24
  '';

  noctaliaAutostart = ''
${if hasFprintd then ''
    # PAM service for Noctalia lock screen fingerprint auth
    env = NOCTALIA_PAM_SERVICE,noctalia
'' else ""}
    # Systemd integration - export environment for user services
    # Include HYPRLAND_INSTANCE_SIGNATURE so portal services can connect
    exec-once = systemctl --user import-environment WAYLAND_DISPLAY XDG_CURRENT_DESKTOP HYPRLAND_INSTANCE_SIGNATURE${pamServiceExport}
    exec-once = dbus-update-activation-environment --systemd WAYLAND_DISPLAY XDG_CURRENT_DESKTOP HYPRLAND_INSTANCE_SIGNATURE${pamServiceExport}

    # Portal setup: start GTK portal first, then restart main portal
    exec-once = sleep 1 && systemctl --user restart xdg-desktop-portal-hyprland xdg-desktop-portal

    # Polkit agent: badged supports fingerprint, hyprpolkitagent is password-only
    exec-once = ${if hasFprintd then "badged" else "systemctl --user start hyprpolkitagent"}

    # Ensure Noctalia Hyprland colors file is writable (not a Home Manager symlink)
    exec-once = bash -c 'f="$HOME/.config/hypr/noctalia/noctalia-colors.conf"; mkdir -p "$(dirname "$f")"; [ -L "$f" ] && rm -f "$f"; touch "$f"; chmod 0644 "$f"'

    # Start desktop shell
    exec-once = noctalia-shell
  '';

in
  if shell == "illogical" then illogicalAutostart
  else noctaliaAutostart
