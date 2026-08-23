# Autostart configuration
# Programs to run at Hyprland startup (shell-aware)
{ shell ? "noctalia", hasFprintd ? false }:

let
  # Noctalia v5 authenticates through the `login` PAM service directly, so the
  # NOCTALIA_PAM_SERVICE export that v4 needed is gone. Fingerprint unlock now
  # depends on /etc/pam.d/login carrying fprintd (NixOS does this when
  # services.fprintd.enable is set).

  # Shell-specific autostart commands
  illogicalAutostart = ''
    # Systemd integration - export environment for user services
    # Include HYPRLAND_INSTANCE_SIGNATURE so portal services can connect
    exec-once = systemctl --user import-environment WAYLAND_DISPLAY XDG_CURRENT_DESKTOP HYPRLAND_INSTANCE_SIGNATURE
    exec-once = dbus-update-activation-environment --systemd WAYLAND_DISPLAY XDG_CURRENT_DESKTOP HYPRLAND_INSTANCE_SIGNATURE
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
    # Systemd integration - export environment for user services
    # Include HYPRLAND_INSTANCE_SIGNATURE so portal services can connect
    exec-once = systemctl --user import-environment WAYLAND_DISPLAY XDG_CURRENT_DESKTOP HYPRLAND_INSTANCE_SIGNATURE
    exec-once = dbus-update-activation-environment --systemd WAYLAND_DISPLAY XDG_CURRENT_DESKTOP HYPRLAND_INSTANCE_SIGNATURE

    # Portal setup: start GTK portal first, then restart main portal
    exec-once = sleep 1 && systemctl --user restart xdg-desktop-portal-hyprland xdg-desktop-portal

    # Polkit agent: badged supports fingerprint, hyprpolkitagent is password-only
    exec-once = ${if hasFprintd then "badged" else "systemctl --user start hyprpolkitagent"}

    # Ensure the Noctalia palette file sourced by hyprland.conf exists and is a
    # real file (not a Home Manager symlink) before the theme template writes it
    exec-once = bash -c 'f="$HOME/.config/hypr/noctalia-colors.conf"; [ -L "$f" ] && rm -f "$f"; touch "$f"; chmod 0644 "$f"'

    # Noctalia v5 is started by its own systemd user unit
    # (programs.noctalia.systemd.enable in home/shells/noctalia/shell.nix),
    # which is PartOf hyprland-session.target — no exec-once needed.
  '';

in
  if shell == "illogical" then illogicalAutostart
  else noctaliaAutostart
