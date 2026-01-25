# Neovim configuration with LazyVim
{ config, pkgs, lib, ... }:

{
  programs.neovim = {
    enable = true;
    defaultEditor = true;
    viAlias = true;
    vimAlias = true;

    # Tools LazyVim expects on PATH
    extraPackages = with pkgs; [
      # Core tools for LazyVim
      git
      ripgrep
      fd
      lazygit
      tree-sitter

      # LSP servers
      nodePackages.typescript-language-server  # TypeScript/JS
      vscode-langservers-extracted             # HTML/CSS/JSON
      pyright                                   # Python
      rust-analyzer                             # Rust
      lua-language-server                       # Lua (for Neovim config)

      # Formatters
      nodePackages.prettier                     # JS/TS/HTML/CSS/JSON
      black                                     # Python
      ruff                                      # Python linter/formatter
      rustfmt                                   # Rust
      stylua                                    # Lua

      # Additional utilities
      gcc                                       # For tree-sitter compilation
    ];
  };

  # Auto-install LazyVim starter config if not present
  home.activation.installLazyVim = lib.hm.dag.entryAfter [ "writeBoundary" ] ''
    if [ ! -d "$HOME/.config/nvim" ]; then
      ${pkgs.git}/bin/git clone https://github.com/LazyVim/starter "$HOME/.config/nvim"
      rm -rf "$HOME/.config/nvim/.git"
    fi
  '';
}
