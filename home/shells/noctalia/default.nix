# Noctalia shell environment
# Imports all Noctalia-specific modules
{ ... }:

{
  imports = [
    ./shell.nix               # Noctalia desktop shell + TOML config
    ./fish.nix                # Fish + Zoxide + fzf
    ../oh-my-posh.nix         # oh-my-posh prompt (replaces Starship)
    ./theming.nix             # GTK, cursor, icons
    ../restart-on-change.nix  # Auto-restart shell on store path change
  ];
}
