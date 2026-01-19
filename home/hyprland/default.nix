# Hyprland window manager configuration
# Generates modular config for multi-shell environment
{ config, pkgs, lib, hostname, osConfig, ... }:

let
  # Get shell from NixOS config (set by specialisations)
  shell = osConfig.desktop.shell;
  # Import config generators (only used for noctalia)
  monitorsConfig = import ./monitors.nix { inherit hostname lib; };
  inputConfig = import ./input.nix {};
  looknfeelConfig = import ./looknfeel.nix { inherit hostname lib; };
  # Generate bindings for all shells (each specialisation sources its own)
  bindingsNoctalia = import ./bindings.nix { shell = "noctalia"; };
  bindingsIllogical = import ./bindings.nix { shell = "illogical"; };
  # Generate autostart for all shells (each specialisation sources its own)
  autostartNoctalia = import ./autostart.nix { shell = "noctalia"; };
  autostartIllogical = import ./autostart.nix { shell = "illogical"; };

  # Shell-specific Hyprland configuration
  # CLEAN APPROACH: Use our ENTIRE Hyprland config for both shells
  # Only difference is autostart (which shell to launch) and bindings (shell-specific launchers)
  # Illogical's Hyprland configs are incompatible with Hyprland 0.52+
  hyprlandExtraConfig = ''
    # Modular Hyprland configuration
    source = ~/.config/hypr/monitors.conf
    source = ~/.config/hypr/input.conf
    source = ~/.config/hypr/bindings-${shell}.conf
    source = ~/.config/hypr/looknfeel.conf
    source = ~/.config/hypr/autostart-${shell}.conf
  '' + lib.optionalString (shell == "noctalia") ''
    source = ~/.config/hypr/noctalia/noctalia-colors.conf
  '';

in {
  imports = [
    (import ./hypridle.nix { inherit shell; })
  ];

  wayland.windowManager.hyprland = {
    enable = true;
    settings = {};
    extraConfig = hyprlandExtraConfig;
  };

  # Modular config files in ~/.config/hypr/
  # Generate all shell-specific configs (each specialisation sources its own)
  xdg.configFile."hypr/monitors.conf".text = monitorsConfig;
  xdg.configFile."hypr/input.conf".text = inputConfig;
  xdg.configFile."hypr/bindings-noctalia.conf".text = bindingsNoctalia;
  xdg.configFile."hypr/bindings-illogical.conf".text = bindingsIllogical;
  xdg.configFile."hypr/looknfeel.conf".text = looknfeelConfig;
  xdg.configFile."hypr/autostart-noctalia.conf".text = autostartNoctalia;
  xdg.configFile."hypr/autostart-illogical.conf".text = autostartIllogical;
}
