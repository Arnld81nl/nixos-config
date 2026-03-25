# Monitor configuration
# Host-specific display setup
{ hostname, lib ? builtins }:

let
  # Check base hostname (handles -illogical suffix)
  isKraken = lib.hasPrefix "kraken" hostname;
  isG1a = lib.hasPrefix "G1a" hostname;
  isX1yoga = lib.hasPrefix "x1yoga" hostname;
  isXps9320 = lib.hasPrefix "xps9320" hostname;
  monitorConfig = if isG1a then ''
    # G1a: Built-in 2880x1800 at 2x
    monitor = eDP-1,preferred,0x0,2
    # External LG 4K monitor at 1.33x, positioned to the right
    monitor = desc:LG Electronics LG HDR 4K,preferred,1440x0,1.33
    # Fallback for other external monitors
    monitor = ,preferred,auto,1.33
    env = GDK_SCALE,2
  '' else if isKraken then ''
    # Kraken: 4K display at 165Hz with 1.5x scaling
    monitor = ,3840x2160@165,auto,1.5
    env = GDK_SCALE,1.5
  '' else if isX1yoga then ''
    # X1 Yoga: Built-in 4K display at 2x
    monitor = eDP-1,preferred,0x0,2
    # External 4K monitors at 1.33x, positioned to the right
    monitor = desc:LG Electronics LG HDR 4K,preferred,1920x0,1.33
    # Fallback for other external monitors
    monitor = ,preferred,auto,2
    env = GDK_SCALE,2
  '' else if isXps9320 then ''
    # XPS 9320: Built-in 3456x2160 display at 2x scaling
    monitor = eDP-1,preferred,0x0,2
    # External LG 4K monitor at 1.33x, positioned to the right
    monitor = desc:LG Electronics LG HDR 4K,preferred,1728x0,1.33
    # Fallback for other external monitors
    monitor = ,preferred,auto,1.5
    env = GDK_SCALE,2
  '' else ''
    # Laptop: Auto-detect with native scaling
    monitor = ,preferred,auto,auto
    env = GDK_SCALE,2
  '';
in ''
  # See https://wiki.hyprland.org/Configuring/Monitors/
  # List current monitors and resolutions: hyprctl monitors
  # Format: monitor = [port], resolution, position, scale

  ${monitorConfig}
''
