{ lib, rustPlatform, fetchFromGitHub, pkg-config, gtk4, dbus, polkit }:

rustPlatform.buildRustPackage {
  pname = "badged";
  version = "0.1.0";

  src = fetchFromGitHub {
    owner = "jfernandez";
    repo = "badged";
    rev = "076acc91";
    hash = "sha256-HuUBz11MDnt5zRiJ1k9+2AOw1sci4avsFPxved0t3sU=";
  };

  cargoHash = "sha256-PQ60GeBBDIPgJ2swcCQpwgTCVhQoL0P0qu2/NQoipGU=";

  nativeBuildInputs = [ pkg-config ];
  buildInputs = [ gtk4 dbus polkit ];

  # Patch hardcoded FHS path to polkit helper for NixOS
  postPatch = ''
    substituteInPlace src/agent.rs \
      --replace-fail '/usr/lib/polkit-1/polkit-agent-helper-1' \
                     '/run/wrappers/bin/polkit-agent-helper-1'
  '';

  meta = {
    description = "A polkit authentication agent for Linux window managers with fingerprint support";
    homepage = "https://github.com/jfernandez/badged";
    license = lib.licenses.mit;
    mainProgram = "badged";
  };
}
