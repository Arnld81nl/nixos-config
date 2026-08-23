# Automatic shell restart on store path change (Illogical Impulse only)
#
# After nixos-rebuild, a running quickshell process still has the old
# /nix/store paths baked in, while IPC commands reference the new path. This
# activation hook detects a changed shell package and restarts it via hyprctl.
#
# Noctalia is NOT handled here any more: v5 ships a systemd user unit
# (programs.noctalia.systemd.enable) that is PartOf hyprland-session.target and
# carries X-Restart-Triggers on its config, so systemd restarts it on rebuild.
#
# See CLAUDE.md for details on the store path persistence issue.
{ config, pkgs, lib, osConfig, quickshell, ... }:

let
  shell = osConfig.desktop.shell;
  isIllogical = shell == "illogical";

  shellPackageHash = builtins.hashString "sha256" (
    toString quickshell.packages.x86_64-linux.default
  );
in
{
  home.activation.restartShellOnStorePathChange =
    lib.mkIf isIllogical (lib.hm.dag.entryAfter ["writeBoundary"] ''
      HASH_FILE="$HOME/.local/state/shell-store-hash"
      mkdir -p "$(dirname "$HASH_FILE")"

      NEW_HASH="${shellPackageHash}"
      OLD_HASH=""
      [ -f "$HASH_FILE" ] && OLD_HASH=$(cat "$HASH_FILE")

      # Only restart if hash changed AND we had a previous hash (not first run)
      if [ -n "$OLD_HASH" ] && [ "$OLD_HASH" != "$NEW_HASH" ]; then
        if ${pkgs.procps}/bin/pgrep -f 'bin/quickshell' >/dev/null 2>&1; then
          if command -v hyprctl >/dev/null 2>&1 && hyprctl version >/dev/null 2>&1; then
            echo "Shell store path changed, restarting ${shell}..."

            $DRY_RUN_CMD ${pkgs.procps}/bin/pkill -f 'bin/quickshell' || true
            sleep 0.5

            $DRY_RUN_CMD hyprctl dispatch exec "quickshell -c ~/.config/quickshell/ii"
          fi
        fi
      fi

      run echo "$NEW_HASH" > "$HASH_FILE"
    '');
}
