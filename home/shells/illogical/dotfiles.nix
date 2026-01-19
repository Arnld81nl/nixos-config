# Illogical Impulse shell-specific configuration
# Starship prompt config (from upstream dots-hyprland)
#
# Note: Quickshell dotfiles and activation script are in dotfiles-only.nix
# which is imported unconditionally to ensure files exist for all specialisations.
{ dots-hyprland, ... }:

let
  configSource = "${dots-hyprland}/dots/.config";
in
{
  # Starship prompt configuration from upstream dots-hyprland
  # Only applied when illogical shell is active (won't conflict with Noctalia's settings)
  xdg.configFile."starship.toml" = {
    source = "${configSource}/starship.toml";
  };
}
