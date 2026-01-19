# Key bindings configuration
# All keyboard shortcuts (shell-aware)
{ shell ? "noctalia" }:

let
  # Shell-specific launcher command
  launcherCmd = if shell == "illogical"
    then "fuzzel"
    else "noctalia-shell ipc call launcher toggle";

  # Shell-specific lock command
  lockCmd = if shell == "illogical"
    then "hyprlock"
    else "noctalia-shell ipc call lockScreen lock";

  # Standard launcher bind for shells
  standardLauncherBind = ''
    bind = $mainMod, SPACE, exec, ${launcherCmd}
  '';

  # Illogical Impulse wallpaper selector (Quickshell global shortcut)
  illogicalWallpaperBind = ''
    # Wallpaper selector - Super + Ctrl + T
    bindd = Ctrl+Super, T, Toggle wallpaper selector, global, quickshell:wallpaperSelectorToggle
  '';
in
''
  # Variables
  $mainMod = SUPER
  $terminal = ghostty
  $browser = google-chrome-stable

  # Application launchers
  bind = $mainMod, Return, exec, $terminal
  ${standardLauncherBind}
  ${if shell == "illogical" then illogicalWallpaperBind else ""}
  bind = $mainMod, E, exec, nautilus --new-window
  bind = $mainMod, B, exec, $browser
  bind = $mainMod SHIFT, B, exec, $browser --incognito

  # Universal copy/paste (sends Ctrl+Insert / Shift+Insert)
  bind = $mainMod, C, sendshortcut, CTRL, Insert,
  bind = $mainMod, V, sendshortcut, SHIFT, Insert,
  bind = $mainMod, X, sendshortcut, CTRL, X,
  bind = $mainMod SHIFT, V, exec, ~/.local/bin/clipboard-image-to-file

  # Window management
  bind = $mainMod, Q, killactive,
  bind = $mainMod, F, togglefloating,
  bind = $mainMod, P, pseudo,
  bind = $mainMod, J, togglesplit,
  bind = $mainMod, BACKSPACE, exec, hyprctl dispatch setprop address:$(hyprctl activewindow -j | jq -r '.address') alpha 0.85 toggle

  # Exit Hyprland
  bind = $mainMod SHIFT, M, exit,

  # Lock screen
  bind = $mainMod, L, exec, ${lockCmd}

  # Focus movement
  bind = $mainMod, left, movefocus, l
  bind = $mainMod, right, movefocus, r
  bind = $mainMod, up, movefocus, u
  bind = $mainMod, down, movefocus, d

  # Workspace switching
  bind = $mainMod, 1, workspace, 1
  bind = $mainMod, 2, workspace, 2
  bind = $mainMod, 3, workspace, 3
  bind = $mainMod, 4, workspace, 4
  bind = $mainMod, 5, workspace, 5
  bind = $mainMod, 6, workspace, 6
  bind = $mainMod, 7, workspace, 7
  bind = $mainMod, 8, workspace, 8
  bind = $mainMod, 9, workspace, 9
  bind = $mainMod, 0, workspace, 10

  # Move window to workspace
  bind = $mainMod SHIFT, 1, movetoworkspace, 1
  bind = $mainMod SHIFT, 2, movetoworkspace, 2
  bind = $mainMod SHIFT, 3, movetoworkspace, 3
  bind = $mainMod SHIFT, 4, movetoworkspace, 4
  bind = $mainMod SHIFT, 5, movetoworkspace, 5
  bind = $mainMod SHIFT, 6, movetoworkspace, 6
  bind = $mainMod SHIFT, 7, movetoworkspace, 7
  bind = $mainMod SHIFT, 8, movetoworkspace, 8
  bind = $mainMod SHIFT, 9, movetoworkspace, 9
  bind = $mainMod SHIFT, 0, movetoworkspace, 10

  # Scroll through workspaces
  bind = $mainMod, mouse_down, workspace, e+1
  bind = $mainMod, mouse_up, workspace, e-1

  # Screenshot bindings (wayfreeze + satty with auto-close)
  bind = $mainMod, grave, exec, ~/.local/bin/screenshot region
  bind = , Print, exec, ~/.local/bin/screenshot region
  bind = SHIFT, Print, exec, ~/.local/bin/screenshot fullscreen

  # App launchers
  bind = $mainMod, M, exec, spotify
  bind = $mainMod SHIFT, SLASH, exec, 1password
  bind = $mainMod, D, exec, $terminal -e lazydocker
  bind = $mainMod SHIFT, T, exec, $terminal -e btop

  # Web apps
  bind = $mainMod, W, exec, $browser --app=https://web.whatsapp.com/
  bind = $mainMod, Y, exec, $browser --app=https://youtube.com/
  bind = $mainMod, O, exec, $browser --app=https://outlook.office.com/
  bind = $mainMod, T, exec, $browser --app=https://teams.microsoft.com/
  bind = $mainMod SHIFT, A, exec, $browser --app=https://chatgpt.com/
  bind = $mainMod SHIFT, P, exec, $browser --app=https://photos.google.com/
  bind = $mainMod SHIFT, X, exec, $browser --app=https://x.com/

  # Media key bindings (repeat on hold)
  bindel = , XF86AudioRaiseVolume, exec, wpctl set-volume -l 1.0 @DEFAULT_AUDIO_SINK@ 5%+
  bindel = , XF86AudioLowerVolume, exec, wpctl set-volume @DEFAULT_AUDIO_SINK@ 5%-
  bindel = , XF86MonBrightnessUp, exec, brightnessctl set 5%+
  bindel = , XF86MonBrightnessDown, exec, brightnessctl set 5%-

  # Media key bindings (no repeat)
  bindl = , XF86AudioMute, exec, wpctl set-mute @DEFAULT_AUDIO_SINK@ toggle
  bindl = , XF86AudioMicMute, exec, wpctl set-mute @DEFAULT_AUDIO_SOURCE@ toggle
  bindl = , XF86AudioPlay, exec, playerctl play-pause
  bindl = , XF86AudioPause, exec, playerctl play-pause
  bindl = , XF86AudioNext, exec, playerctl next
  bindl = , XF86AudioPrev, exec, playerctl previous
  bindl = , XF86Calculator, exec, gnome-calculator

  # Mouse bindings
  bindm = $mainMod, mouse:272, movewindow
  bindm = $mainMod, mouse:273, resizewindow
''
