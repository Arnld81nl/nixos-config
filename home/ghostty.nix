# Ghostty terminal configuration
{ config, pkgs, ... }:

let
  noctaliaTheme = ''
    palette = 0=#45475a
    palette = 1=#f38ba8
    palette = 2=#a6e3a1
    palette = 3=#f9e2af
    palette = 4=#89b4fa
    palette = 5=#f5c2e7
    palette = 6=#94e2d5
    palette = 7=#a6adc8
    palette = 8=#585b70
    palette = 9=#f37799
    palette = 10=#89d88b
    palette = 11=#ebd391
    palette = 12=#74a8fc
    palette = 13=#f2aede
    palette = 14=#6bd7ca
    palette = 15=#bac2de
    background = #1e1e2e
    foreground = #cdd6f4
    cursor-color = #f5e0dc
    cursor-text = #1e1e2e
    selection-background = #585b70
    selection-foreground = #cdd6f4
  '';
in
{
  programs.ghostty = {
    enable = true;

    settings = {
      # Font
      font-family = "JetBrainsMono Nerd Font";
      font-style = "Regular";
      font-size = 9;

      # Window
      window-padding-x = 14;
      window-padding-y = 14;
      confirm-close-surface = false;
      resize-overlay = "never";
      gtk-toolbar-style = "flat";

      # Cursor styling
      cursor-style = "block";
      cursor-style-blink = false;

      # Shell integration
      shell-integration-features = "no-cursor,ssh-env";

      # Mouse
      mouse-scroll-multiplier = 0.95;

      # Theme
      theme = "noctalia";

      # Keyboard bindings
      keybind = [
        "shift+insert=paste_from_clipboard"
        "control+insert=copy_to_clipboard"
        "super+control+shift+alt+arrow_down=resize_split:down,100"
        "super+control+shift+alt+arrow_up=resize_split:up,100"
        "super+control+shift+alt+arrow_left=resize_split:left,100"
        "super+control+shift+alt+arrow_right=resize_split:right,100"
        "shift+enter=text:\\x1b\\r"
      ];
    };

    # Don't use themes option - it creates files that conflict with backups
    # themes.noctalia = { ... };
  };

  # Manage theme file directly with force to prevent backup conflicts
  xdg.configFile."ghostty/themes/noctalia" = {
    text = noctaliaTheme;
    force = true;
  };
}
