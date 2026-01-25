# Theming configuration for Illogical Impulse
# GTK theme, cursor, and icons
{ config, pkgs, lib, ... }:

{
  # Cursor theme (Bibata-Modern-Classic)
  home.pointerCursor = {
    name = "Bibata-Modern-Classic";
    package = pkgs.bibata-cursors;
    size = 24;
    gtk.enable = true;
    x11.enable = true;
  };

  # GTK theming
  gtk = {
    enable = true;
    theme = {
      name = "adw-gtk3-dark";
      package = pkgs.adw-gtk3;
    };
    iconTheme = {
      name = "Papirus-Dark";
      package = pkgs.papirus-icon-theme;
    };
  };

  # Icon packages (fallbacks)
  home.packages = with pkgs; [
    papirus-icon-theme
    adwaita-icon-theme
    hicolor-icon-theme
  ];
}
