# Packages required for Illogical Impulse
{ config, pkgs, lib, quickshell, ... }:

let
  # Python environment for wallpaper color extraction
  pythonEnv = pkgs.python3.withPackages (ps: with ps; [
    pillow
    numpy
  ]);

  # Quickshell wrapper with QML import paths for KDE modules
  # Note: Use .unwrapped for kirigami to get actual QML files, not just wrapper
  quickshellScript = ''
    export QML2_IMPORT_PATH="${pkgs.kdePackages.kirigami.unwrapped}/lib/qt-6/qml:${pkgs.kdePackages.qt5compat}/lib/qt-6/qml:${pkgs.kdePackages.qtpositioning}/lib/qt-6/qml:${pkgs.kdePackages.syntax-highlighting}/lib/qt-6/qml''${QML2_IMPORT_PATH:+:}$QML2_IMPORT_PATH"
    exec ${quickshell.packages.x86_64-linux.default}/bin/quickshell "$@"
  '';
  quickshellWrapped = pkgs.writeShellScriptBin "quickshell" quickshellScript;
  # 'qs' alias used by upstream Illogical Impulse (e.g., for opening settings panel)
  qsWrapper = pkgs.writeShellScriptBin "qs" quickshellScript;
in
{
  home.packages = [
    # Quickshell with wrapper for KDE QML modules
    quickshellWrapped
    qsWrapper  # 'qs' command expected by Illogical Impulse UI
  ] ++ (with pkgs; [
    # Qt theming and dependencies
    kdePackages.qt6ct
    kdePackages.qtpositioning       # Required by Weather.qml
    kdePackages.qt5compat           # Required by RippleButton.qml (Qt5Compat.GraphicalEffects)
    kdePackages.syntax-highlighting # Required by MessageCodeBlock.qml (org.kde.syntaxhighlighting)
    kdePackages.kirigami            # Required by FluentIcon.qml (org.kde.kirigami)
    libsForQt5.qtstyleplugin-kvantum
    kdePackages.qtstyleplugin-kvantum

    # Audio visualization
    cava

    # Wayland utilities
    hyprlock
    wl-clipboard
    cliphist
    hyprsunset

    # Screenshot and recording
    hyprshot
    wf-recorder
    imagemagick
    ffmpeg

    # Brightness control
    ddcutil

    # Launchers
    fuzzel
    wlogout

    # Python environment for color extraction
    pythonEnv

    # CLI tools
    eza  # Modern ls replacement (used by fish aliases)

    # Fonts for Material Design
    material-symbols
    rubik
    nerd-fonts.ubuntu
    nerd-fonts.ubuntu-mono
    nerd-fonts.caskaydia-cove
    nerd-fonts.fantasque-sans-mono
    nerd-fonts.mononoki
    nerd-fonts.space-mono
  ]);

  # Qt/KDE environment configuration
  qt = {
    enable = true;
    platformTheme.name = "qt6ct";
    style.name = "kvantum";
  };

  # Set QML import paths for quickshell to find KDE/Qt modules
  home.sessionVariables = {
    QML2_IMPORT_PATH = lib.concatStringsSep ":" [
      "${pkgs.kdePackages.kirigami}/lib/qt-6/qml"
      "${pkgs.kdePackages.qt5compat}/lib/qt-6/qml"
      "${pkgs.kdePackages.qtpositioning}/lib/qt-6/qml"
      "${pkgs.kdePackages.syntax-highlighting}/lib/qt-6/qml"
    ];
  };
}
