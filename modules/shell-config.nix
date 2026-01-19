# Desktop shell configuration option
# Used by specialisations to switch between shells
{ lib, ... }:

{
  options.desktop.shell = lib.mkOption {
    type = lib.types.enum [ "noctalia" "illogical" ];
    default = "noctalia";
    description = "Active desktop shell environment";
  };
}
