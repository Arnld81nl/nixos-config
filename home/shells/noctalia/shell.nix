# Noctalia v5 Desktop Shell configuration
#
# v5 splits configuration in two:
#   ~/.config/noctalia/config.toml           declarative base, deployed read-only here
#   ~/.local/state/noctalia/settings.toml    runtime overrides written by the GUI
#
# Noctalia layers settings.toml on top of config.toml, so GUI changes survive
# rebuilds on their own. That replaces v4's copy-and-hash deployment: there is no
# .deployed-hash any more, and nothing in ~/.config/noctalia is overwritten.
#
# To promote a GUI change into the repo, copy the relevant keys from
# ~/.local/state/noctalia/settings.toml into ./config.toml (keeping the
# /home/USER placeholder). Note the state file wins over config.toml, so a key
# set there keeps overriding the repo until it is removed from the state file.
{ config, pkgs, lib, inputs, hostname, username, osConfig ? null, ... }:

let
  noctaliaPackage = inputs.noctalia.packages.${pkgs.stdenv.hostPlatform.system}.default;

  # Keep the repo config public-safe by using a /home/USER placeholder, then
  # render the real home path at build time.
  homePath = "/home/${username}";

  # Same deal for the VPN button labels: the names identify the tenants, so they
  # live only in the gitignored secrets.nix and are substituted in here. Loaded
  # the same way home/home.nix does — by absolute path, because a gitignored file
  # is not part of the flake source.
  secretsPath = "/home/${username}/nixos-config/home/secrets.nix";
  secrets =
    if builtins.pathExists secretsPath
    then import secretsPath
    else { };
  vpnName = key: fallback: secrets.vpn.${key}.name or fallback;

  # @NOCTALIA_TEMPLATES@ resolves to the package's shipped theme templates, so the
  # hyprland palette template is rendered from a pinned path rather than through
  # the builtin template's compositor-probing apply.sh.
  baseConfig = builtins.replaceStrings
    [ "/home/USER" "@NOCTALIA_TEMPLATES@" "@VPN1_NAME@" "@VPN2_NAME@" "@VPN3_NAME@" ]
    [
      homePath
      "${noctaliaPackage}/share/noctalia/assets/templates"
      (vpnName "vpn1" "VPN 1")
      (vpnName "vpn2" "VPN 2")
      (vpnName "vpn3" "VPN 3")
    ]
    (builtins.readFile ./config.toml);

  # Hosts without a battery should not carry the battery widget.
  hostsWithoutBattery = [ "kraken" ];
  hasBattery = !builtins.any (host: lib.hasPrefix host hostname) hostsWithoutBattery;

  withoutBattery = builtins.replaceStrings [ ''"battery", '' ] [ "" ] baseConfig;

  # Wire fingerprint auth into the lock screen when fprintd is enabled.
  # The PAM service itself is selected via NOCTALIA_PAM_SERVICE, exported from
  # home/hyprland/autostart.nix.
  hasFingerprintAuth =
    if osConfig == null then false else osConfig.services.fprintd.enable or false;

  withFingerprint = text:
    if hasFingerprintAuth
    then builtins.replaceStrings [ "fingerprint    = false" ] [ "fingerprint    = true" ] text
    else text;

  settingsToml = withFingerprint (if hasBattery then baseConfig else withoutBattery);
in
{
  # The module is loaded via conditional import in home/home.nix
  programs.noctalia = {
    enable = true;
    package = noctaliaPackage;

    # v5 ships a user unit. It is PartOf hyprland-session.target and carries
    # X-Restart-Triggers on the config, so the shell restarts itself on rebuild
    # and on config changes — this is why home/shells/restart-on-change.nix no
    # longer handles Noctalia.
    systemd.enable = true;

    settings = settingsToml;
  };

  # Local plugins. Noctalia scans this directory unconditionally (it is
  # FileUtils::dataDir()/plugins, the implicit "local" source that outranks every
  # configured git/path source), so nothing has to be declared as a
  # [[plugins.source]] — but a plugin still only runs once its id appears in
  # `[plugins] enabled` in config.toml.
  #
  # ai-usage restores the one v4 CustomButton that polled a command for its
  # label; v5's custom_button cannot do that. See ./plugins/ai-usage.
  xdg.dataFile."noctalia/plugins/ai-usage".source = ./plugins/ai-usage;

  # vpn gives the three shield buttons the status text v5's custom_button cannot
  # render, and is the collapsed face of the `vpn` accordion capsule group.
  xdg.dataFile."noctalia/plugins/vpn".source = ./plugins/vpn;

  # hyprland.conf sources ~/.config/hypr/noctalia-colors.conf for border colors. The
  # theme template rewrites it whenever the palette changes, so it has to exist
  # as a real writable file (Hyprland fails a `source =` on a missing file) and
  # must not be a Home Manager symlink.
  home.activation.noctaliaHyprColors = lib.hm.dag.entryAfter ["writeBoundary"] ''
    file="$HOME/.config/hypr/noctalia-colors.conf"
    mkdir -p "$(dirname "$file")"

    if [ -L "$file" ]; then
      rm -f "$file"
    fi

    touch "$file"
    chmod 0644 "$file"

    # Older Noctalia template runs could drop a hyprland.lua containing nothing
    # but the Noctalia include. That file outranks hyprland.conf and boots a
    # stock desktop, so clear it — but only when it is exactly that stub, never
    # a real Lua config someone wrote on purpose.
    lua="$HOME/.config/hypr/hyprland.lua"
    if [ -f "$lua" ] && [ ! -L "$lua" ] \
       && ! grep -qv -e '^$' -e '^-- For Noctalia Color templates$' \
            -e '^require("noctalia").apply_theme()$' "$lua"; then
      echo "Removing Noctalia-generated ~/.config/hypr/hyprland.lua (would shadow hyprland.conf)"
      rm -f "$lua"
    fi
  '';
}
