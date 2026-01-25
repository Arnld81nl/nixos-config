# Autostart configuration
# Programs to run at Hyprland startup (shell-aware)
{ shell ? "noctalia" }:

let
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

    # Restart portal services to pick up new environment (fixes restart via greetd)
    exec-once = sleep 1 && systemctl --user restart xdg-desktop-portal-hyprland xdg-desktop-portal

    # Start desktop shell
    exec-once = noctalia-shell
  '';

in
  if shell == "illogical" then illogicalAutostart
  else noctaliaAutostart
