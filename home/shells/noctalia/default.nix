# Noctalia shell environment
# Imports all Noctalia-specific modules
{ ... }:

{
  imports = [
    ./shell.nix               # Noctalia desktop shell + JSON configs
    ./fish.nix                # Fish + Starship + Zoxide + fzf
    ./theming.nix             # GTK, cursor, icons
    ../restart-on-change.nix  # Auto-restart shell on store path change
  ];
}
