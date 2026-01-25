# Look and feel configuration
# Animations, decorations, layout, and window rules
{ hostname, lib ? builtins }:

let
  # Check base hostname (handles -illogical suffix)
  isKraken = lib.hasPrefix "kraken" hostname;

  # Dwindle layout only for desktop (kraken) - wide monitors benefit from aspect ratio control
  layoutType = if isKraken then "dwindle" else "master";
  dwindleConfig = if isKraken then ''
    # Layout
    # https://wiki.hyprland.org/Configuring/Dwindle-Layout/
    dwindle {
      pseudotile = true
      preserve_split = true
      # Avoid overly wide single-window layouts on wide screens
      single_window_aspect_ratio = 1 1
    }
  '' else "";

  masterConfig = if !isKraken then ''
    # Master layout for laptop
    # https://wiki.hyprland.org/Configuring/Master-Layout/
    master {
      mfact = 0.5  # 50/50 split between master and stack
    }
  '' else "";
in
''
  # General appearance
  # https://wiki.hyprland.org/Configuring/Variables/#general
  general {
    gaps_in = 5
    gaps_out = 10
    border_size = 2
    col.active_border = rgba(33ccffee) rgba(00ff99ee) 45deg
    col.inactive_border = rgba(595959aa)
    layout = ${layoutType}
  }

  # Decorations
  # https://wiki.hyprland.org/Configuring/Variables/#decoration
  decoration {
    rounding = 10

    blur {
      enabled = true
      size = 3
      passes = 1
    }

    shadow {
      enabled = true
      range = 4
      render_power = 3
      color = rgba(1a1a1aee)
    }
  }

  # Animations
  # https://wiki.hyprland.org/Configuring/Variables/#animations
  animations {
    enabled = true

    bezier = myBezier, 0.05, 0.9, 0.1, 1.05

    animation = windows, 1, 7, myBezier
    animation = windowsOut, 1, 7, default, popin 80%
    animation = border, 1, 10, default
    animation = borderangle, 1, 8, default
    animation = fade, 1, 7, default
    animation = workspaces, 1, 6, default
  }

${dwindleConfig}
${masterConfig}
  # Misc settings
  misc {
    force_default_wallpaper = 0
    disable_hyprland_logo = true
  }

  # XWayland scaling - let Hyprland handle scaling for X11 apps
  xwayland {
    force_zero_scaling = true
  }

  # Window rules - Hyprland 0.53+ syntax
  # https://wiki.hyprland.org/Configuring/Window-Rules/

  # File dialogs
  windowrule = match:class xdg-desktop-portal-gtk, float on
  windowrule = match:class org\.gnome\.Nautilus, match:title Properties, float on
  windowrule = match:class org\.gnome\.Nautilus, match:title Open.*, float on
  windowrule = match:class org\.gnome\.Nautilus, match:title Save.*, float on

  # Suppress maximize for all windows
  windowrule = match:class .*, suppress_event maximize

  # Opaque by default (use SUPER+BACKSPACE to toggle transparency)
  windowrule = match:class .*, opacity 1.0 1.0

  # Floating windows - 1Password
  windowrule = match:class 1[pP]assword, float on, center on, size 875 600, no_screen_share on

  # Floating windows - Sushi (Nautilus quick preview)
  windowrule = match:class org\.gnome\.NautilusPreviewer, float on, center on, size 60% 70%

  # Floating windows - LocalSend
  windowrule = match:class localsend_app, float on, center on, size 875 600

  # Floating windows - Calculator
  windowrule = match:class org\.gnome\.Calculator, float on

  # Floating windows - Illogical Impulse settings (Quickshell)
  windowrule = match:class org\.quickshell, match:title illogical-impulse Settings, float on, center on, size 1100 750

  # Floating windows - Media viewers
  windowrule = match:class (imv|mpv), float on, center on

  # No transparency on media windows
  windowrule = match:class (vlc|mpv|imv|zoom), opacity 1 1
''
