# Automatic shell restart on store path change
#
# After nixos-rebuild, the running quickshell process may have old store paths
# while IPC commands point to the new path. This activation hook detects when
# the shell package has changed and restarts the shell via hyprctl.
#
# See CLAUDE.md for details on the store path persistence issue.
{ config, pkgs, lib, osConfig, quickshell, ... }:

let
  shell = osConfig.desktop.shell;

  # Hash the relevant package path to detect changes
  # For Noctalia: use the noctalia-shell package
  # For Illogical: use the quickshell package from flake input
  shellPackageHash = builtins.hashString "sha256" (
    if shell == "noctalia"
    then toString config.programs.noctalia-shell.package
    else toString quickshell.packages.x86_64-linux.default
  );

  restartCommand = if shell == "noctalia"
    then "noctalia-shell"
    else "quickshell -c ~/.config/quickshell/ii";
in
{
  home.activation.restartShellOnStorePathChange = lib.hm.dag.entryAfter ["writeBoundary"] ''
    HASH_FILE="$HOME/.local/state/shell-store-hash"
    mkdir -p "$(dirname "$HASH_FILE")"

    NEW_HASH="${shellPackageHash}"
    OLD_HASH=""
    [ -f "$HASH_FILE" ] && OLD_HASH=$(cat "$HASH_FILE")

    # Only restart if hash changed AND we had a previous hash (not first run)
    if [ -n "$OLD_HASH" ] && [ "$OLD_HASH" != "$NEW_HASH" ]; then
      # Check if quickshell is running and Hyprland is available
      if ${pkgs.procps}/bin/pgrep -x quickshell >/dev/null 2>&1; then
        if command -v hyprctl >/dev/null 2>&1 && hyprctl version >/dev/null 2>&1; then
          echo "Shell store path changed, restarting ${shell}..."

          # Kill old processes
          $DRY_RUN_CMD ${pkgs.procps}/bin/pkill -x quickshell || true
          sleep 0.5

          # Restart via hyprctl for proper Wayland integration
          $DRY_RUN_CMD hyprctl dispatch exec "${restartCommand}"
        fi
      fi
    fi

    # Record current hash (use run for dry-run support)
    run echo "$NEW_HASH" > "$HASH_FILE"
  '';
}
