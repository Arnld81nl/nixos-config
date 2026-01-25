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

      # System info
      fastfetch = "fastfetch -c archey";
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

  # Starship prompt - Noctalia custom theme
  programs.starship = {
    enable = true;
    enableFishIntegration = true;
    settings = {
      add_newline = true;
      command_timeout = 200;

      format = ''
        ╭─$sudo$os$directory$fill$git_branch$git_status$nodejs$package$python$java$php
        ╰─ ❯❯ '';

      fill.symbol = " ";

      username = {
        show_always = true;
        format = "[$user]($style)";
        style_user = "bold cyan";
        style_root = "bold red";
      };

      sudo = {
        disabled = false;
        format = "[󱐋]($style)";
        style = "bold red";
      };

      os = {
        disabled = false;
        format = "[$symbol ]($style)";
        style = "bold cyan";
      };

      os.symbols = {
        Arch = "󰣇";
        NixOS = "";
        Linux = "";
        Macos = "";
        Windows = "󰍲";
      };

      directory = {
        format = "[ 󰉖 $path]($style)";
        style = "bold cyan";
        truncation_length = 5;
        truncate_to_repo = false;
      };

      git_branch = {
        format = "[ ($symbol$branch)]($style)";
        symbol = "";
        style = "bold yellow";
      };

      git_status = {
        format = "([  $all_status$ahead_behind]($style))";
        style = "bold yellow";
        ahead = " +\${count}";
        behind = " -\${count}";
        diverged = " +\${ahead_count} -\${behind_count}";
        up_to_date = "";
        untracked = "?\${count}";
        stashed = "󰏖\${count}";
        modified = "!\${count}";
        staged = "+\${count}";
        renamed = "»\${count}";
        deleted = "✘\${count}";
        conflicted = "=\${count}";
      };

      nodejs = {
        format = "[ $version]($style)";
        style = "bold green";
        detect_files = ["package.json" ".node-version" ".nvmrc"];
      };

      package = {
        format = "[ $version]($style)";
        symbol = "";
        style = "bold red";
        display_private = false;
      };

      python = {
        format = "[ $version]($style)";
        style = "bold green";
        detect_files = [".python-version" "requirements.txt" "pyproject.toml" "Pipfile"];
      };

      java = {
        format = "[ $version]($style)";
        style = "bold magenta";
        detect_files = ["pom.xml" "build.gradle" "build.gradle.kts"];
      };

      php = {
        format = "[ $version]($style)";
        style = "bold blue";
        detect_files = ["composer.json"];
      };

      character.disabled = true;
    };
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

  # Required CLI tools
  home.packages = with pkgs; [
    eza
  ];
}
