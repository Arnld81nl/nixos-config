{ lib, rustPlatform, pkg-config, makeWrapper, openssl, dbus, nvd, libnotify }:

rustPlatform.buildRustPackage {
  pname = "forge";
  version = "1.0.0";

  src = lib.cleanSource ./.;

  cargoLock.lockFile = ./Cargo.lock;

  nativeBuildInputs = [ pkg-config makeWrapper ];
  buildInputs = [ openssl dbus ];

  postInstall = ''
    # Wrap forge with nvd in PATH
    wrapProgram $out/bin/forge \
      --prefix PATH : ${lib.makeBinPath [ nvd ]}

    # Wrap forge-notify with libnotify in PATH (for notify-send fallback)
    wrapProgram $out/bin/forge-notify \
      --prefix PATH : ${lib.makeBinPath [ libnotify ]}
  '';

  meta = {
    description = "NixOS Configuration Tool - Copyright Cybex B.V.";
    homepage = "https://github.com/Arnld81nl/nixos-config";
    license = lib.licenses.mit;
    mainProgram = "forge";
  };
}
