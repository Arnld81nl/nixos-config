# Illogical Impulse Desktop Shell configuration
# Material Design 3 Quickshell-based shell for Hyprland
# Direct fetch from end-4/dots-hyprland (no external flake dependency)
{ ... }:

{
  imports = [
    ./dotfiles.nix            # Fetch and copy Quickshell configs
    ./packages.nix            # Qt, Quickshell, tools
    ./fish.nix                # Fish shell configuration
    ./theming.nix             # Cursor, GTK, icons
    ../restart-on-change.nix  # Auto-restart shell on store path change
  ];
}
