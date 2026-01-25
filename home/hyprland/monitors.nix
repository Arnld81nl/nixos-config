# Monitor configuration
# Host-specific display setup
{ hostname, lib ? builtins }:

let
  # Check base hostname (handles -illogical suffix)
  isKraken = lib.hasPrefix "kraken" hostname;
  isX1yoga = lib.hasPrefix "x1yoga" hostname;
  isXps9320 = lib.hasPrefix "xps9320" hostname;
  monitorConfig = if isKraken then ''
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
    # XPS 9320: Built-in display (FHD+ 1920x1200 or UHD+ 3840x2400)
    # Using auto-detection with 1.25x for FHD+ or 2x for UHD+
    monitor = eDP-1,preferred,0x0,auto
    # Fallback for external monitors
    monitor = ,preferred,auto,auto
    env = GDK_SCALE,1
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
