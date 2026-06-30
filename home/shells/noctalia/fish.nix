# Fish shell configuration for Noctalia
# Custom prompt theme and shell tools
{ config, pkgs, lib, ... }:

{
  programs.fish = {
    enable = true;

    # Interactive shell initialization
    interactiveShellInit = ''
      # Disable greeting
      set -g fish_greeting

      # Add ~/.local/bin to PATH if not already present
      if not contains ~/.local/bin $PATH
        set -gx PATH ~/.local/bin $PATH
      end

      # VISUAL for programs that distinguish from EDITOR
      set -gx VISUAL nvim
    '';

    # Shell aliases
    shellAliases = {
      # eza for better ls
      ls = "eza --icons";

      # Nix shortcuts (auto-detects hostname from flake)
      rebuild = "sudo nixos-rebuild switch --flake /etc/nixos";
      rebuild-test = "sudo nixos-rebuild test --flake /etc/nixos";
      rebuild-boot = "sudo nixos-rebuild boot --flake /etc/nixos";
      update = "nix flake update /etc/nixos";

      # Common shortcuts
      ll = "ls -la";
      la = "ls -A";
      l = "ls -CF";

      # Navigation
      ".." = "cd ..";
      "..." = "cd ../..";

      # Git shortcuts
      gs = "git status";
      ga = "git add";
      gc = "git commit";
      gp = "git push";
      gl = "git log --oneline";
      lg = "lazygit";

      # Hyprland shortcuts
      hypr-reload = "hyprctl reload";
      hypr-monitors = "hyprctl monitors";
      hypr-workspaces = "hyprctl workspaces";

      # System info (uses ~/.config/fastfetch/config.jsonc)
      fastfetch = "command fastfetch";
    };

    # Fish functions
    functions = {
      nixedit = {
        body = ''
          cd /etc/nixos
          $EDITOR .
        '';
        description = "Open NixOS configuration in editor";
      };

      nixgc = {
        body = ''
          echo "Removing old generations..."
          sudo nix-collect-garbage -d
          echo "Optimizing store..."
          nix store optimise
        '';
        description = "Clean up Nix store";
      };

      nixgen = {
        body = ''
          sudo nix-env --list-generations --profile /nix/var/nix/profiles/system
        '';
        description = "List NixOS generations";
      };
    };

    # Fish plugins
    plugins = [
      {
        name = "colored-man-pages";
        src = pkgs.fishPlugins.colored-man-pages.src;
      }
    ];
  };

  # Prompt is provided by the shared oh-my-posh module (../oh-my-posh.nix),
  # imported from default.nix. Starship is disabled there.

  # Zoxide (smart cd)
  programs.zoxide = {
    enable = true;
    enableFishIntegration = true;
  };

  # fzf for fuzzy finding
  programs.fzf = {
    enable = true;
    enableFishIntegration = true;
  };

  # Required CLI tools
  home.packages = with pkgs; [
    eza
  ];
}
