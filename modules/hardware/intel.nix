# Intel GPU configuration (iGPU and Xe discrete)
{ config, pkgs, lib, ... }:

{
  # Intel GPU drivers are built into Mesa, no explicit videoDrivers needed
  # Unlike NVIDIA, Intel uses the kernel's i915/xe modules directly

  # Load Intel GPU modules in initrd for early KMS (Plymouth support)
  boot.initrd.kernelModules = [
    "i915"        # Intel integrated graphics (Gen 12 and earlier)
    # "xe"        # Intel Xe discrete GPUs (uncomment if using Arc/Battlemage)
  ];

  # Enable Intel-specific VA-API for hardware video acceleration
  hardware.graphics = {
    extraPackages = with pkgs; [
      intel-media-driver    # iHD driver for Broadwell+ (VA-API)
      intel-vaapi-driver    # i965 driver for older Intel GPUs (VA-API)
      intel-compute-runtime # OpenCL support
    ];
    extraPackages32 = with pkgs.pkgsi686Linux; [
      intel-media-driver
      intel-vaapi-driver
    ];
  };

  # Intel-specific kernel parameters for better performance
  boot.kernelParams = [
    "i915.modeset=1"            # Enable kernel modesetting
    "i915.enable_psr=1"         # Panel Self Refresh (power saving on laptops)
    "i915.enable_fbc=1"         # Frame Buffer Compression
  ];

  # Environment variables for Intel Wayland
  environment.sessionVariables = {
    LIBVA_DRIVER_NAME = "iHD";  # Use Intel Media Driver for VA-API
    NIXOS_OZONE_WL = "1";       # Electron apps use Wayland
  };
}
