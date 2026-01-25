# Disko configuration for xps9320 (Dell XPS 13 9320)
{ ... }:

{
  imports = [ ./default.nix ];

  disko.devices.disk.main.device = "/dev/nvme0n1";
}
