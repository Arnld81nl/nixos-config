{
  description = "NixOS configuration with Home Manager, Hyprland, and multi-shell support";

  inputs = {
    # Use nixpkgs-unstable for compatibility
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

    # Home Manager following nixpkgs-unstable
    home-manager = {
      url = "github:nix-community/home-manager";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    # Noctalia Desktop Shell
    noctalia = {
      url = "github:noctalia-dev/noctalia-shell";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    # Illogical Impulse dotfiles (direct from upstream)
    dots-hyprland = {
      url = "github:end-4/dots-hyprland";
      flake = false;
    };

    # Rounded polygon shapes submodule for dots-hyprland
    rounded-polygon-qmljs = {
      url = "github:end-4/rounded-polygon-qmljs";
      flake = false;
    };

    # Quickshell (latest git for IdleInhibitor support)
    quickshell = {
      url = "github:quickshell-mirror/quickshell";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    # Hyprland (git version for bug fixes)
    hyprland = {
      url = "github:hyprwm/Hyprland";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    # Disko for declarative disk partitioning
    disko = {
      url = "github:nix-community/disko";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, home-manager, noctalia, dots-hyprland, rounded-polygon-qmljs, disko, quickshell, hyprland, ... }@inputs:
  let
    system = "x86_64-linux";

    # Overlay to patch xdg-desktop-portal-gtk for Hyprland support
    gtkPortalOverlay = final: prev: {
      xdg-desktop-portal-gtk = prev.xdg-desktop-portal-gtk.overrideAttrs (old: {
        postInstall = (old.postInstall or "") + ''
          substituteInPlace $out/share/xdg-desktop-portal/portals/gtk.portal \
            --replace-fail "UseIn=gnome" "UseIn=gnome;Hyprland"
        '';
      });
    };

    pkgs = import nixpkgs {
      inherit system;
      overlays = [ gtkPortalOverlay ];
    };

    # Custom packages
    plymouth-cybex = pkgs.callPackage ./packages/plymouth-cybex { };
    forge = pkgs.callPackage ./packages/forge { };

    # Home Manager configuration (shell-agnostic - shell comes from osConfig)
    mkHomeManagerConfig = { hostname, username }: {
      home-manager.useGlobalPkgs = true;
      home-manager.useUserPackages = true;
      home-manager.backupFileExtension = "backup";
      home-manager.extraSpecialArgs = { inherit inputs hostname username dots-hyprland rounded-polygon-qmljs quickshell forge; };
      home-manager.users.${username} = import ./home/home.nix;
      # sharedModules removed - external modules now imported conditionally in home.nix
    };

    # Helper to create NixOS configurations with shell specialisations
    # Set useDisko = false for hosts with manual partition setup (e.g., hibernate swap)
    mkNixosSystem = { hostname, username ? "arnold", extraModules ? [], useDisko ? true }:
      nixpkgs.lib.nixosSystem {
        inherit system;
        specialArgs = { inherit inputs plymouth-cybex forge username hyprland; };
        modules = []
        # Disko for declarative disk partitioning (optional)
        ++ (if useDisko then [
          disko.nixosModules.disko
          ./modules/disko/${hostname}.nix
        ] else [])
        ++ [
          ./hosts/${hostname}
          ./modules/common.nix
          ./modules/shell-config.nix
          ./modules/desktop-environments.nix

          # Home Manager
          home-manager.nixosModules.home-manager
          (mkHomeManagerConfig { inherit hostname username; })

          # Shell specialisations (boot menu entries)
          {
            specialisation = {
              illogical.configuration.desktop.shell = "illogical";
            };
          }
        ] ++ extraModules;
      };
  in
  {
    apps.${system} = {
      disko = {
        type = "app";
        program = "${disko.packages.${system}.disko}/bin/disko";
      };
      forge = {
        type = "app";
        program = "${forge}/bin/forge";
      };
      default = {
        type = "app";
        program = "${forge}/bin/forge";
      };
    };

    nixosConfigurations = {
      # Lenovo ThinkPad X1 Yoga Gen 6 (Intel Tiger Lake + Iris Xe)
      # Default: Noctalia | Specialisations: illogical
      x1yoga = mkNixosSystem {
        hostname = "x1yoga";
        username = "arnold";
      };

      # Forge Installer ISO
      # Build: nix build .#nixosConfigurations.iso.config.system.build.isoImage
      iso = nixpkgs.lib.nixosSystem {
        inherit system;
        specialArgs = { inherit inputs plymouth-cybex; };
        modules = [
          ./modules/iso
        ];
      };
    };

    packages.${system} = {
      disko = disko.packages.${system}.disko;
      forge = forge;
      default = forge;
    };
  };
}
