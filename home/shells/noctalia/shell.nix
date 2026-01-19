# Noctalia Desktop Shell configuration
#
# Settings are managed with a hybrid approach:
# - Configs are seeded from repo on first run
# - GUI changes persist locally across rebuilds
# - When repo configs are updated (hash changes), local files are overwritten
# - To sync local changes back to repo, ask Claude to copy the files
{ config, pkgs, lib, inputs, hostname, ... }:

let
  # Load base settings from JSON
  baseSettings = builtins.fromJSON (builtins.readFile ./settings.json);

  # Filter out Battery widget for hosts without a battery (desktop PCs)
  hostsWithoutBattery = [ "kraken" ];
  hasBattery = !builtins.any (host: lib.hasPrefix host hostname) hostsWithoutBattery;

  # Generate host-specific settings
  settings = baseSettings // {
    bar = baseSettings.bar // {
      widgets = baseSettings.bar.widgets // {
        right = builtins.filter
          (widget: hasBattery || widget.id != "Battery")
          baseSettings.bar.widgets.right;
      };
    };
  };

  settingsJson = pkgs.writeText "noctalia-settings.json" (builtins.toJSON settings);
in
{
  # Noctalia Desktop Shell
  # The module is loaded via conditional import in home/home.nix
  programs.noctalia-shell = {
    enable = true;

    # Disable automatic systemd service - we control startup via Hyprland autostart
    systemd.enable = false;
  };

  # Noctalia configuration files - hybrid approach
  # Seeds from repo on first run, preserves local GUI changes,
  # but overwrites when repo configs are updated (hash changes)
  home.activation.noctaliaConfig = lib.hm.dag.entryAfter ["writeBoundary"] ''
    NOCTALIA_DIR="$HOME/.config/noctalia"
    HASH_FILE="$NOCTALIA_DIR/.deployed-hash"

    mkdir -p "$NOCTALIA_DIR"

    # Calculate hash of repo configs
    REPO_HASH=$(cat ${settingsJson} ${./gui-settings.json} ${./colors.json} ${./plugins.json} | ${pkgs.coreutils}/bin/sha256sum | cut -d' ' -f1)

    # Check if we should deploy (first run OR repo updated)
    SHOULD_DEPLOY=false
    if [ ! -f "$NOCTALIA_DIR/settings.json" ]; then
      SHOULD_DEPLOY=true
    elif [ -f "$HASH_FILE" ] && [ "$(cat "$HASH_FILE")" != "$REPO_HASH" ]; then
      SHOULD_DEPLOY=true
      echo "Noctalia: Repo configs updated, syncing..."
    fi

    if [ "$SHOULD_DEPLOY" = true ]; then
      cp ${settingsJson} "$NOCTALIA_DIR/settings.json"
      cp ${./gui-settings.json} "$NOCTALIA_DIR/gui-settings.json"
      cp ${./colors.json} "$NOCTALIA_DIR/colors.json"
      cp ${./plugins.json} "$NOCTALIA_DIR/plugins.json"
      chmod 644 "$NOCTALIA_DIR"/*.json
      echo "$REPO_HASH" > "$HASH_FILE"
    fi
  '';

  # Noctalia-specific packages
  home.packages = with pkgs; [
    quickshell  # For Noctalia IPC commands
  ];

  # Create symlink for Quickshell IPC config lookup
  # This allows "qs -c noctalia-shell ipc ..." commands to find the running instance
  home.activation.noctaliaQuickshellSymlink = lib.hm.dag.entryAfter ["writeBoundary"] ''
    QS_CONFIG_DIR="$HOME/.config/quickshell"
    mkdir -p "$QS_CONFIG_DIR"
    rm -f "$QS_CONFIG_DIR/noctalia-shell"
    ln -s "${config.programs.noctalia-shell.package}/share/noctalia-shell" "$QS_CONFIG_DIR/noctalia-shell"
  '';
}
