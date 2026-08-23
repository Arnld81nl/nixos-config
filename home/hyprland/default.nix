# Hyprland window manager configuration
# Generates modular config for multi-shell environment
{ config, pkgs, lib, hostname, osConfig, ... }:

let
  # Get shell from NixOS config (set by specialisations)
  shell = osConfig.desktop.shell;
  # Import config generators (only used for noctalia)
  monitorsConfig = import ./monitors.nix { inherit hostname lib; };
  inputConfig = import ./input.nix {};
  looknfeelConfig = import ./looknfeel.nix { inherit hostname lib; };
  # Generate bindings for all shells (each specialisation sources its own)
  bindingsNoctalia = import ./bindings.nix { shell = "noctalia"; };
  bindingsIllogical = import ./bindings.nix { shell = "illogical"; };
  # Generate autostart for all shells (each specialisation sources its own)
  hasFprintd = osConfig.services.fprintd.enable;
  autostartNoctalia = import ./autostart.nix { shell = "noctalia"; inherit hasFprintd; };
  autostartIllogical = import ./autostart.nix { shell = "illogical"; inherit hasFprintd; };

  # Laptops that should blank the internal panel when an external display
  # (XDR, XREAL, AORUS, etc.) is attached. hyprctl-only detection is unreliable
  # during hotplug — we walk /sys/class/drm/card*-*/status instead.
  isExternalDisplayLaptop = lib.hasPrefix "G1a" hostname || lib.hasPrefix "xps" hostname;

  externalMonitorFunctions = ''
    physical_external_monitor_connected() {
      for status in /sys/class/drm/card*-*/status; do
        [ -e "$status" ] || continue
        case "$status" in
          *-eDP-*/status) continue ;;
        esac

        if [ "$(<"$status")" = "connected" ]; then
          return 0
        fi
      done

      return 1
    }

    apply_monitor_state() {
      if ! physical_external_monitor_connected; then
        ${pkgs.hyprland}/bin/hyprctl keyword monitor eDP-1,preferred,0x0,auto || true
        return 0
      fi

      monitors="$(${pkgs.hyprland}/bin/hyprctl monitors -j 2>/dev/null)" || return 0

      if printf '%s\n' "$monitors" | ${pkgs.jq}/bin/jq -e '
        .[]
        | select(
            (.model // "") == "XREAL One Pro"
            or (.model // "") == "Nreal XREAL One Pro"
            or (.model // "") == "Studio XDR"
            or (.model // "") == "Pro Display XDR"
            or (.model // "") == "AORUS FO32U2"
          )
          | select((.disabled // false) | not)
      ' > /dev/null 2>&1; then
        if printf '%s\n' "$monitors" | ${pkgs.jq}/bin/jq -e '.[] | select(.name == "eDP-1" and ((.disabled // false) | not))' > /dev/null 2>&1; then
          ${pkgs.hyprland}/bin/hyprctl keyword monitor eDP-1,disable || true
        fi

        printf '%s\n' "$monitors" | ${pkgs.jq}/bin/jq -r '
          map(select(
            ((.model // "") == "Studio XDR" or (.model // "") == "Pro Display XDR")
            and ((.disabled // false) | not)
          ))
          | group_by(.serial // "")
          | map(
            select(length > 1)
            | sort_by((.width * .height), .refreshRate)
            | .[0:-1][]
            | .name
          )
          | .[]
        ' | while read -r output; do
          if [ -n "$output" ]; then
            ${pkgs.hyprland}/bin/hyprctl keyword monitor "$output,disable" || true
          fi
        done
      else
        if ! printf '%s\n' "$monitors" | ${pkgs.jq}/bin/jq -e '.[] | select(.name == "eDP-1" and ((.disabled // false) | not) and .x == 0 and .y == 0)' > /dev/null 2>&1; then
          ${pkgs.hyprland}/bin/hyprctl keyword monitor eDP-1,preferred,0x0,auto || true
        fi
      fi

      return 0
    }
  '';

  externalMonitorApply = pkgs.writeShellScript "external-monitor-apply" ''
    ${externalMonitorFunctions}
    apply_monitor_state
  '';

  # Listens for Hyprland monitor hotplug events and reapplies state after a
  # short settle delay (XREAL connect/disconnect emits transient add/remove
  # events while Hyprland is reconfiguring).
  externalMonitorToggle = pkgs.writeShellScript "external-monitor-toggle" ''
    ${externalMonitorFunctions}

    if [ -z "''${HYPRLAND_INSTANCE_SIGNATURE:-}" ]; then
      exit 0
    fi

    ${pkgs.socat}/bin/socat -U - "UNIX-CONNECT:$XDG_RUNTIME_DIR/hypr/$HYPRLAND_INSTANCE_SIGNATURE/.socket2.sock" | while read -r line; do
      case "$line" in
        monitoradded*|monitorremoved*)
          sleep 2
          apply_monitor_state
          ;;
      esac
    done
  '';

  # Shell-specific Hyprland configuration
  # CLEAN APPROACH: Use our ENTIRE Hyprland config for both shells
  # Only difference is autostart (which shell to launch) and bindings (shell-specific launchers)
  # Illogical's Hyprland configs are incompatible with Hyprland 0.52+
  hyprlandExtraConfig = ''
    # Modular Hyprland configuration
    source = ~/.config/hypr/monitors.conf
    source = ~/.config/hypr/input.conf
    # Named <shell>-bindings.conf, not bindings-<shell>.conf: Noctalia's builtin
    # hyprland template undo hook strips any line matching
    # `source = .*noctalia\.conf`, which the old name matched.
    source = ~/.config/hypr/${shell}-bindings.conf
    source = ~/.config/hypr/looknfeel.conf
    source = ~/.config/hypr/${shell}-autostart.conf
  '' + lib.optionalString isExternalDisplayLaptop ''
    source = ~/.config/hypr/external-monitor-toggle.conf
  '' + lib.optionalString (shell == "noctalia") ''
    # Palette written by Noctalia v5's hyprland theme template
    # (declared in home/shells/noctalia/config.toml).
    source = ~/.config/hypr/noctalia-colors.conf
  '';

in {
  imports = [
    (import ./hypridle.nix { inherit shell; })
  ];

  wayland.windowManager.hyprland = {
    enable = true;
    settings = {};
    extraConfig = hyprlandExtraConfig;
  };

  # Modular config files in ~/.config/hypr/
  # Generate all shell-specific configs (each specialisation sources its own)
  xdg.configFile."hypr/monitors.conf".text = monitorsConfig;
  xdg.configFile."hypr/input.conf".text = inputConfig;
  xdg.configFile."hypr/noctalia-bindings.conf".text = bindingsNoctalia;
  xdg.configFile."hypr/illogical-bindings.conf".text = bindingsIllogical;
  xdg.configFile."hypr/looknfeel.conf".text = looknfeelConfig;
  xdg.configFile."hypr/noctalia-autostart.conf".text = autostartNoctalia;
  xdg.configFile."hypr/illogical-autostart.conf".text = autostartIllogical;

  # External-monitor toggle (only sourced when isExternalDisplayLaptop).
  # apply-once gets initial state right; toggle daemon handles hotplug.
  xdg.configFile."hypr/external-monitor-toggle.conf".text =
    lib.optionalString isExternalDisplayLaptop ''
      exec-once = ${externalMonitorApply}
      exec-once = ${externalMonitorToggle}
    '';
}
