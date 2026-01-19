# Disko configuration for x1yoga (Lenovo ThinkPad X1 Yoga Gen 6)
{ ... }:

{
  imports = [ ./default.nix ];

  disko.devices.disk.main.device = "/dev/nvme0n1";
}
