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
    hash = "sha256-8yM7/KTlGPmtBMrGVEWHsMPv/m9Ngk60ZGAr3uPklkE=";
    stripRoot = false;
  };

  installPhase = ''
    runHook preInstall

    install -Dm444 Conthrax-SemiBold.otf \
      "$out/share/fonts/opentype/Conthrax-SemiBold.otf"
    # dafont repackages this zip periodically and renames the EULA by year
    # (2023 -> 2026 in Aug 2026), so match whatever PDF ships with it.
    for eula in *.pdf; do
      install -Dm444 "$eula" "$out/share/doc/conthrax/$eula"
    done

    runHook postInstall
  '';

  meta = {
    description = "Conthrax futuristic sans-serif font";
    homepage = "https://www.dafont.com/conthrax.font";
    license = lib.licenses.unfreeRedistributable;
    platforms = lib.platforms.all;
  };
}
