# Fish shell configuration for Illogical Impulse
{ config, pkgs, lib, ... }:

{
  programs.fish = {
    enable = true;

    # Interactive shell initialization
    interactiveShellInit = ''
      # No greeting (illogical style)
      set -g fish_greeting

      # Add blank line after command output for visual separation
      function postexec_newline --on-event fish_postexec
        echo
      end

      # Add ~/.local/bin to PATH if not already present
      if not contains ~/.local/bin $PATH
        set -gx PATH ~/.local/bin $PATH
      end

      # VISUAL for programs that distinguish from EDITOR
      set -gx VISUAL nvim

      # Quickshell terminal integration (Material You colors)
      if test -f ~/.local/state/quickshell/user/generated/terminal/sequences.txt
        cat ~/.local/state/quickshell/user/generated/terminal/sequences.txt
      end
    '';

    # Shell aliases
    shellAliases = {
      # Illogical aliases
      ls = "eza --icons";
      clear = "printf '\\033[2J\\033[3J\\033[1;1H'";
      q = "quickshell -c ~/.config/quickshell/ii";

      # Nix shortcuts
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

  # Starship prompt (illogical uses default starship)
  programs.starship = {
    enable = true;
    enableFishIntegration = true;
  };

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

}
