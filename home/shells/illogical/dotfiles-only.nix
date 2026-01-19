# Illogical Impulse dotfiles - always imported for file deployment
# This module ensures Quickshell config files are deployed regardless of which
# shell specialisation is active. Programs config (fish, theming) is in the
# conditionally-imported main module.
#
# See CLAUDE.md "Shell Module Import Architecture" for details.
{ config, pkgs, lib, dots-hyprland, rounded-polygon-qmljs, ... }:

let
  # Source paths from dots-hyprland
  configSource = "${dots-hyprland}/dots/.config";

  # Quickshell settings to persist across rebuilds
  quickshellSettings = {
    ai = {
      model = "";
      temperature = 0.5;
    };
    booru = {
      allowNsfw = false;
      provider = "yandere";
    };
    cheatsheet = {
      tabIndex = 0;
    };
    idle = {
      inhibit = false;
    };
    overlay = {
      crosshair = {
        clickthrough = true;
        height = 100;
        pinned = false;
        width = 250;
        x = 827;
        y = 441;
      };
      floatingImage = {
        clickthrough = false;
        height = 0;
        pinned = false;
        width = 0;
        x = 1650;
        y = 390;
      };
      fpsLimiter = {
        clickthrough = false;
        height = 80;
        pinned = false;
        width = 280;
        x = 1570;
        y = 615;
      };
      notes = {
        clickthrough = true;
        height = 330;
        pinned = false;
        width = 460;
        x = 1400;
        y = 42;
      };
      open = [ "crosshair" "recorder" "volumeMixer" "resources" ];
      recorder = {
        clickthrough = false;
        height = 130;
        pinned = false;
        width = 350;
        x = 80;
        y = 80;
      };
      resources = {
        clickthrough = true;
        height = 200;
        pinned = false;
        tabIndex = 0;
        width = 350;
        x = 1500;
        y = 770;
      };
      volumeMixer = {
        clickthrough = false;
        height = 600;
        pinned = false;
        tabIndex = 0;
        width = 350;
        x = 80;
        y = 280;
      };
    };
    sidebar = {
      bottomGroup = {
        collapsed = false;
        tab = 0;
      };
    };
    "options.bar.weather.city" = "Emmen, Drenthe, Netherlands";
    "options.bar.weather.enableGPS" = false;
    "options.screenSnip.savePath" = "/home/john/Pictures/Screenshots";
  };
in
{
  # Copy Quickshell configuration to ~/.config/quickshell/ii/
  # force = true prevents backup conflicts when activation script modifies files
  xdg.configFile."quickshell/ii" = {
    source = "${configSource}/quickshell/ii";
    recursive = true;
    force = true;
  };

  # Create state directories for Material You theming and copy git submodule content
  # Use writeBoundary to ensure this runs AFTER Home Manager symlinks are in place
  home.activation.setupIllogicalDirs = lib.hm.dag.entryAfter [ "writeBoundary" ] ''
    mkdir -p "$HOME/.local/state/quickshell/user/generated/terminal"

    # Copy the shapes submodule (not included in dots-hyprland due to git submodules)
    shapesDir="$HOME/.config/quickshell/ii/modules/common/widgets/shapes"
    if [ ! -f "$shapesDir/qmldir" ]; then
      run rm -rf "$shapesDir" 2>/dev/null || true
      run cp -r "${rounded-polygon-qmljs}" "$shapesDir"
      run chmod -R u+w "$shapesDir"
    fi

    # Apply quickshell settings to states.json (merges with existing, preserving session data)
    statesFile="$HOME/.local/state/quickshell/states.json"
    mkdir -p "$(dirname "$statesFile")"
    if [ -f "$statesFile" ]; then
      run ${pkgs.jq}/bin/jq '. * ${builtins.toJSON quickshellSettings}' "$statesFile" > "$statesFile.tmp" && run mv "$statesFile.tmp" "$statesFile"
    else
      run echo '${builtins.toJSON quickshellSettings}' > "$statesFile"
    fi
    run chmod 644 "$statesFile"

    # Fix screenshot command being killed when panel closes (upstream bug)
    # The issue: snipProc.startDetached() + immediate dismiss() kills the process
    # The fix: Use Quickshell.execDetached() which is component-independent
    regionSelectionFile="$HOME/.config/quickshell/ii/modules/ii/regionSelector/RegionSelection.qml"
    if [ -f "$regionSelectionFile" ] && grep -q "snipProc.startDetached" "$regionSelectionFile"; then
      run ${pkgs.gnused}/bin/sed -i 's/snipProc.startDetached();/Quickshell.execDetached(command);/' "$regionSelectionFile"
      run ${pkgs.gnused}/bin/sed -i 's/snipProc.command = command;//' "$regionSelectionFile"
    fi
  '';
}
