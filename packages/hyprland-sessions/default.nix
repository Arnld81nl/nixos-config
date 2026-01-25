{ pkgs }:

let
  # Wrapper script for Hyprland with Noctalia Desktop Shell
  hyprland-noctalia-bin = pkgs.writeShellScriptBin "hyprland-noctalia" ''
    # Required environment variables for Wayland session
    # XDG_SESSION_TYPE must be set early (Hyprland 0.47+ regression fix)
    export XDG_SESSION_TYPE=wayland
    export XDG_CURRENT_DESKTOP=Hyprland
    export DESKTOP_SHELL=noctalia

    # Create runtime directory and mark desktop shell
    mkdir -p "$XDG_RUNTIME_DIR"
    echo "noctalia" > "$XDG_RUNTIME_DIR/desktop-shell"

    # Launch Hyprland via start-hyprland (required since Hyprland 0.53)
    # start-hyprland provides crash recovery and safe mode
    exec start-hyprland -- "$@"
  '';

  # Wrapper script for Hyprland with Illogical Impulse Desktop Shell
  hyprland-illogical-bin = pkgs.writeShellScriptBin "hyprland-illogical" ''
    # Required environment variables for Wayland session
    export XDG_SESSION_TYPE=wayland
    export XDG_CURRENT_DESKTOP=Hyprland
    export DESKTOP_SHELL=illogical

    # Create runtime directory and mark desktop shell
    mkdir -p "$XDG_RUNTIME_DIR"
    echo "illogical" > "$XDG_RUNTIME_DIR/desktop-shell"

    # Launch Hyprland via start-hyprland (required since Hyprland 0.53)
    # start-hyprland provides crash recovery and safe mode
    exec start-hyprland -- "$@"
  '';

  # Session package with .desktop file for Noctalia
  hyprland-noctalia-session = pkgs.stdenvNoCC.mkDerivation {
    pname = "hyprland-noctalia-session";
    version = "1.0.0";
    dontUnpack = true;

    passthru.providedSessions = [ "hyprland-noctalia" ];

    installPhase = ''
      mkdir -p $out/share/wayland-sessions
      mkdir -p $out/bin

      # Symlink the wrapper script
      ln -s ${hyprland-noctalia-bin}/bin/hyprland-noctalia $out/bin/hyprland-noctalia

      # Create .desktop file
      cat > $out/share/wayland-sessions/hyprland-noctalia.desktop << EOF
      [Desktop Entry]
      Name=Hyprland (Noctalia)
      Comment=Hyprland with Noctalia Desktop Shell
      Exec=$out/bin/hyprland-noctalia
      Type=Application
      DesktopNames=Hyprland
      EOF
    '';
  };

  # Session package with .desktop file for Illogical Impulse
  hyprland-illogical-session = pkgs.stdenvNoCC.mkDerivation {
    pname = "hyprland-illogical-session";
    version = "1.0.0";
    dontUnpack = true;

    passthru.providedSessions = [ "hyprland-illogical" ];

    installPhase = ''
      mkdir -p $out/share/wayland-sessions
      mkdir -p $out/bin

      # Symlink the wrapper script
      ln -s ${hyprland-illogical-bin}/bin/hyprland-illogical $out/bin/hyprland-illogical

      # Create .desktop file
      cat > $out/share/wayland-sessions/hyprland-illogical.desktop << EOF
      [Desktop Entry]
      Name=Hyprland (Illogical Impulse)
      Comment=Hyprland with Illogical Impulse Desktop Shell
      Exec=$out/bin/hyprland-illogical
      Type=Application
      DesktopNames=Hyprland
      EOF
    '';
  };

in {
  # Session packages for display manager registration
  noctalia = hyprland-noctalia-session;
  illogical = hyprland-illogical-session;

  # All session packages as a list
  sessions = [ hyprland-noctalia-session hyprland-illogical-session ];

  # Wrapper scripts for PATH
  noctaliaScript = hyprland-noctalia-bin;
  illogicalScript = hyprland-illogical-bin;

  # Default script (backwards compatibility)
  script = hyprland-noctalia-bin;
}
