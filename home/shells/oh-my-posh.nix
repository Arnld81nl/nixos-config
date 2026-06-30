# Shared oh-my-posh prompt (replaces Starship)
# Imported by both the Noctalia and Illogical shell modules so the active shell
# gets the same prompt. The theme is EDM115-newline2 (home/shells/).
{ config, pkgs, lib, ... }:

let
  ohMyPoshConfigHash = builtins.hashFile "sha256" ./EDM115-newline2.omp.json;
  ohMyPoshCacheHash = builtins.hashString "sha256" "${toString pkgs.oh-my-posh}:${ohMyPoshConfigHash}";
in
{
  # Initialize oh-my-posh in fish. Home Manager merges this interactiveShellInit
  # with the per-shell fish.nix block.
  programs.fish.interactiveShellInit = ''
    # NixOS prompt OS icon for oh-my-posh
    set -gx POSH_OS_ICON ""

    # Initialize oh-my-posh prompt
    ${pkgs.oh-my-posh}/bin/oh-my-posh init fish --config ~/.config/oh-my-posh/EDM115-newline2.omp.json | source
  '';

  # Disable Starship (using oh-my-posh instead)
  programs.starship.enable = lib.mkForce false;

  # oh-my-posh theme file
  xdg.configFile."oh-my-posh/EDM115-newline2.omp.json".source = ./EDM115-newline2.omp.json;

  home.packages = [ pkgs.oh-my-posh ];

  # oh-my-posh caches its generated Fish init script and embeds the binary's
  # absolute Nix store path in that cache. When the package path changes after
  # a rebuild, clear the cache so new shells regenerate it with the new path.
  home.activation.clearOhMyPoshCacheOnStorePathChange = lib.hm.dag.entryAfter [ "writeBoundary" ] ''
    HASH_FILE="$HOME/.local/state/oh-my-posh-store-hash"
    CACHE_DIR="$HOME/.cache/oh-my-posh"

    mkdir -p "$(dirname "$HASH_FILE")"

    NEW_HASH="${ohMyPoshCacheHash}"
    OLD_HASH=""
    [ -f "$HASH_FILE" ] && OLD_HASH=$(cat "$HASH_FILE")

    # Only clear cache if the package path changed and this isn't the first run.
    if [ -n "$OLD_HASH" ] && [ "$OLD_HASH" != "$NEW_HASH" ] && [ -d "$CACHE_DIR" ]; then
      echo "oh-my-posh store path changed, clearing cached init scripts..."
      $DRY_RUN_CMD ${pkgs.coreutils}/bin/rm -rf "$CACHE_DIR"
    fi

    # Recreate the cache directory so the next shell startup can repopulate it.
    $DRY_RUN_CMD ${pkgs.coreutils}/bin/mkdir -p "$CACHE_DIR"

    run echo "$NEW_HASH" > "$HASH_FILE"
  '';
}
