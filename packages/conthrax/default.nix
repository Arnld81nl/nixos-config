{
  lib,
  stdenvNoCC,
  fetchzip,
}:

stdenvNoCC.mkDerivation {
  pname = "conthrax";
  version = "2024-12-10";

  src = fetchzip {
    url = "https://dl.dafont.com/dl/?f=conthrax";
    extension = "zip";
    hash = "sha256-DsB89WLlESpkzOV2knADBt555SpM7OFOTHfzcDZbDKw=";
    stripRoot = false;
  };

  installPhase = ''
    runHook preInstall

    install -Dm444 Conthrax-SemiBold.otf \
      "$out/share/fonts/opentype/Conthrax-SemiBold.otf"
    install -Dm444 "Typodermic Desktop EULA 2023.pdf" \
      "$out/share/doc/conthrax/Typodermic Desktop EULA 2023.pdf"

    runHook postInstall
  '';

  meta = {
    description = "Conthrax futuristic sans-serif font";
    homepage = "https://www.dafont.com/conthrax.font";
    license = lib.licenses.unfreeRedistributable;
    platforms = lib.platforms.all;
  };
}
