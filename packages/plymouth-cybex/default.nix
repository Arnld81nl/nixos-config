{ stdenvNoCC, fetchFromGitHub }:

stdenvNoCC.mkDerivation {
  pname = "plymouth-theme-cybex";
  version = "unstable-2024-01-01";

  src = fetchFromGitHub {
    owner = "DigitalPals";
    repo = "omarchy-cybex";
    rev = "e76c323ed69baf0efae12b2e7afb42635920607a";
    sha256 = "sha256-TiyHkaNhgKv9xZcCdBgvUicfjsQtnsVBEW5JdTAw4gI=";
  };

  installPhase = ''
    runHook preInstall

    mkdir -p $out/share/plymouth/themes/cybex
    cp -r config/plymouth/themes/cybex/* $out/share/plymouth/themes/cybex/

    # Fix paths in .plymouth file to point to the nix store
    substituteInPlace $out/share/plymouth/themes/cybex/cybex.plymouth \
      --replace "/usr/share/plymouth/themes/cybex" "$out/share/plymouth/themes/cybex"

    runHook postInstall
  '';

  meta = {
    description = "Cybex Plymouth boot splash theme - Created by DigitalPals (Cybex B.V.)";
    homepage = "https://github.com/DigitalPals/omarchy-cybex";
    maintainers = [];
    # Credits: Plymouth theme created by DigitalPals / Cybex B.V.
    # Source: https://github.com/DigitalPals/omarchy-cybex
  };
}
