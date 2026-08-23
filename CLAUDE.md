# NixOS Configuration Notes

Configuration details and solutions to issues in this NixOS setup.

## Configuration Structure

```
~/nixos-config/                     # Symlinked from /etc/nixos
├── flake.nix                       # Main flake with host definitions
├── hosts/
│   ├── G1a/                        # HP ZBook Ultra G1a (AMD Strix Halo) — current daily driver
│   ├── x1yoga/                     # Lenovo ThinkPad X1 Yoga Gen 6 (Intel) — retired hardware
│   └── xps9320/                    # Dell XPS 13 9320
├── modules/
│   ├── boot/limine-plymouth.nix    # Bootloader + Plymouth config
│   ├── common.nix                  # Shared system config
│   ├── shell-config.nix            # Desktop shell option (specialisations)
│   ├── desktop-environments.nix
│   ├── gaming.nix
│   ├── disko/                      # Disk partitioning configs
│   ├── iso/                        # Forge installer ISO config
│   └── hardware/
│       ├── nvidia.nix              # NVIDIA driver config
│       └── intel.nix               # Intel GPU config (unused)
├── home/
│   ├── home.nix                    # Main Home Manager config
│   ├── ghostty.nix                 # Terminal config
│   ├── neovim.nix
│   ├── 1password-secrets.nix       # 1Password SSH agent integration
│   ├── app-backup/                 # App profile backup/restore (browsers)
│   │   └── default.nix
│   ├── hyprland/                   # Hyprland WM config (modular)
│   │   ├── default.nix
│   │   ├── bindings.nix
│   │   ├── monitors.nix
│   │   └── ...
│   └── shells/                     # Desktop shell options
│       ├── noctalia/               # AGS-based shell
│       └── illogical/              # Illogical Impulse shell
└── packages/
    ├── forge/                      # Rust TUI configuration tool
    ├── plymouth-cybex/             # Custom Plymouth theme
    └── hyprland-sessions/          # Session desktop entries
```

## Forge - NixOS Configuration Tool

Forge is a Rust TUI application for managing NixOS installations and updates.

### Running Forge

```bash
# From installed system
forge                    # Interactive TUI menu
forge update            # Update flake + rebuild + CLI tools
forge apps backup       # Backup browser profiles
forge apps restore      # Restore app profiles

# From NixOS ISO (fresh install)
nix run github:Arnld81nl/nixos-config
```

### Commands

| Command | Description |
|---------|-------------|
| `forge` | Interactive TUI with main menu |
| `forge install [hostname] [disk]` | Fresh NixOS installation |
| `forge create-host [hostname]` | Create a new host configuration |
| `forge update` | Update flake, rebuild, update CLI tools |
| `forge apps backup` | Backup + push app profiles |
| `forge apps restore` | Pull + restore app profiles |
| `forge apps status` | Check for profile updates |

Note: `forge browser` is still supported as an alias for `forge apps`.

### Fresh Installation from ISO

1. Boot the NixOS minimal ISO
2. Connect to WiFi: `nmtui`
3. Run Forge: `nix run github:Arnld81nl/nixos-config`
4. Select "Install NixOS", choose host and disk
5. Enter LUKS passphrase when prompted
6. Reboot and select a shell from the boot menu

### Building the Forge ISO

Build a custom ISO that boots directly into Forge:

```bash
nix build .#nixosConfigurations.iso.config.system.build.isoImage
```

The ISO will be at `result/iso/NixOS-Cybex-<version>.iso`. Flash to USB:

```bash
sudo dd if=result/iso/NixOS-Cybex-*.iso of=/dev/sdX bs=4M status=progress
```

The ISO automatically:
1. Boots with Plymouth cybex theme
2. Auto-logins and checks internet connectivity
3. Opens `nmtui` if WiFi needed
4. Launches Forge installer from GitHub

## Rebuilding the System

| Config | Host | Specialisations |
|--------|------|-----------------|
| `G1a` | HP ZBook Ultra G1a (AMD Strix Halo) — current daily driver | Default (Noctalia), illogical |
| `x1yoga` | ThinkPad X1 Yoga Gen 6 (Intel) — retired hardware | Default (Noctalia), illogical |

```bash
# Rebuild (includes all shell specialisations)
# IMPORTANT: Use --impure to allow access to gitignored secrets.nix
sudo nixos-rebuild switch --flake .#G1a --impure

# Or use hostname (auto-detected)
sudo nixos-rebuild switch --flake . --impure
```

**Note:** The `--impure` flag is required because `secrets.nix` is gitignored and flakes run in pure evaluation mode by default. Without `--impure`, the build will use placeholder values for VPN configs and other secrets.

### Rebuilding with Active Specialisation

**IMPORTANT:** When rebuilding, always check which specialisation is currently active and re-activate it after the rebuild. A plain `nixos-rebuild switch` activates the **default** configuration, which will switch you out of any active specialisation.

```bash
# Check which specialisation is active (if any)
# Look at DESKTOP_SHELL environment variable or check the runtime file
cat /run/user/$(id -u)/desktop-shell 2>/dev/null || echo "default"

# Standard rebuild (activates default configuration)
sudo nixos-rebuild switch --flake . --impure

# If you were in a specialisation, re-activate it:
sudo /run/current-system/specialisation/illogical/bin/switch-to-configuration switch
```

**For Claude:** Before running `nixos-rebuild switch`, always:
1. Check the active shell: `cat /run/user/$(id -u)/desktop-shell 2>/dev/null`
2. If it returns "illogical" (or another specialisation name), re-activate after rebuild:
   ```bash
   sudo nixos-rebuild switch --flake . --impure && \
   sudo /run/current-system/specialisation/illogical/bin/switch-to-configuration switch
   ```

## Switching Desktop Shells

Desktop shells are switched via the **boot menu** (Limine):

1. Reboot your system
2. In Limine, select your generation
3. Choose from the sub-menu:
   - **Default** - Noctalia (AGS-based shell)
   - **illogical** - Illogical Impulse (Material Design 3)

The selected shell persists for that boot session. To change shells, reboot and select a different specialisation.

**Note:** Each rebuild builds both shell variants. The boot menu shows all options for each generation.

## Hyprland 0.53+ Changes

### Startup

**Change:** Hyprland 0.53 introduced `start-hyprland` as the required launcher, replacing direct `Hyprland` invocation.

**Implementation:** The session wrapper scripts in `packages/hyprland-sessions/default.nix` use `exec start-hyprland -- "$@"` to launch Hyprland properly.

**Benefits:**
- Crash recovery - Hyprland can recover from crashes without losing your session
- Safe mode - Allows booting into a minimal config if the main config is broken

**Optional dependency:** `hyprland-guiutils` enhances safe mode and provides a welcome app for new users. Not yet available in nixpkgs as of December 2025.

### Window Rules Syntax

**Change:** Hyprland 0.53 completely overhauled window rules syntax. The old `windowrulev2` format is deprecated.

**Old syntax (deprecated):**
```
windowrulev2 = float, class:^(firefox)$
windowrulev2 = center, class:^(firefox)$
windowrulev2 = size 800 600, class:^(firefox)$
windowrulev2 = suppressevent maximize, class:.*
windowrulev2 = noscreenshare, class:^(1password)$
```

**New syntax (0.53+):**
```
# IMPORTANT: match clauses MUST come first, then effects
windowrule = match:class firefox, float on, center on, size 800 600
windowrule = match:class .*, suppress_event maximize
windowrule = match:class 1[pP]assword, no_screen_share on
```

**Key differences:**
- `windowrulev2` → `windowrule`
- `class:^(pattern)$` → `match:class pattern` (regex simplified, no anchors needed)
- `title:^(pattern)$` → `match:title pattern`
- **Match clauses must come FIRST**, before any effects
- Actions use `on/off` suffix: `float` → `float on`, `center` → `center on`
- Property names use underscores: `suppressevent` → `suppress_event`, `noscreenshare` → `no_screen_share`
- Multiple actions can be combined in one rule

**Common properties:**
- `float on/off` - Float the window
- `center on` - Center the window
- `size W H` or `size W% H%` - Set window size
- `opacity X Y` - Set active/inactive opacity (0.0-1.0)
- `suppress_event maximize/fullscreen/activate` - Ignore window events
- `no_screen_share on` - Hide window from screen sharing

**Scroll properties (split in 0.54+):**
- `scrollInput` → `scroll_mouse` (mouse wheel) and `scroll_touchpad` (touchpad) — separate controls

**Implementation:** Window rules are split across files:
- `home/hyprland/looknfeel.nix` - Most window rules
- `home/hyprland/input.nix` - Scroll rules for terminal

**Documentation:** https://wiki.hypr.land/Configuring/Window-Rules/

## Intel i915 Power-Saving Crashes (Tiger Lake)

**Problem:** Random system crashes/freezes on Intel graphics (Tiger Lake, 11th Gen and newer).

**Symptom:** System freezes randomly during normal use, requiring a hard power cycle (battery removal). No errors in system logs - the system just stops.

**Root cause:** Intel power-saving features (PSR, FBC) and CPU C-states can cause system instability on Tiger Lake hardware. The CPU enters deep sleep states it cannot properly wake from.

**Solution:** Disable i915 power features and limit CPU C-states via kernel parameters:
```nix
# In modules/hardware/intel.nix (shared)
boot.kernelParams = [
  "i915.enable_fbc=0"  # Disable Frame Buffer Compression
];

# In hosts/x1yoga/default.nix (per-host)
boot.kernelParams = [
  "i915.enable_psr=0"        # Disable Panel Self Refresh
  "i915.enable_guc=0"        # Disable GuC firmware (causes Wayland GPU hangs)
  "i915.enable_dc=0"         # Disable display C-states
  "intel_idle.max_cstate=1"  # Limit CPU C-states (prevents deep sleep freezes)
];
```

**Implementation:**
- PSR, GuC, DC, max_cstate: `hosts/x1yoga/default.nix`
- FBC: `modules/hardware/intel.nix`

**Affected hosts:** x1yoga (ThinkPad X1 Yoga Gen 6, Intel Iris Xe)

**Timeline:**
- January 2026: Disabled PSR - reduced crash frequency
- February 2026: Disabled FBC - crashes continued
- February 2026: Disabled GuC and DC - crashes continued
- February 2026: Added `intel_idle.max_cstate=1` - limits CPU to shallow sleep states

**Alternative options if issues persist:**
- `intel_iommu=off` - Disable Intel VT-d (may help with GPU hangs)

## G1a Webcam (works since kernel 7.2.0)

The HP ZBook Ultra G1a webcam was unusable on Linux through kernel 7.1.x and
**works from 7.2.0** (verified 2026-08-23). **Nothing in this repo is needed for
it** — no module, firmware or option was added; it works on the stock kernel.

What landed in 7.2 is `CONFIG_VIDEO_AMD_ISP4_CAPTURE`, the AMD ISP V4L2 capture
bridge. `amd_isp4_capture` binds to `amd_isp_capture.0.auto` under amdgpu and
creates `/dev/video0` (card "Preview", `amd_isp41_mdev`) and `/dev/media0`.
PipeWire picks it up on its own as a `[v4l2]` device and source.

| | |
|---|---|
| Formats | **NV12 only**, 640x360 - 2880x1620, all 30fps |
| Controls | none — `v4l2-ctl -L` is empty (no brightness/exposure) |

**Testing gotcha:** auto-exposure starts fully dark and takes ~2-3s to converge,
so a single-frame grab looks black and reads as "still broken". Capture ~90
frames and keep the last one:

```bash
v4l2-ctl -d /dev/video0 --set-fmt-video=width=1280,height=720,pixelformat=NV12 \
  --stream-mmap --stream-count=90 --stream-to=stream.nv12
ffmpeg -f rawvideo -pix_fmt nv12 -s 1280x720 -i stream.nv12 \
  -vf 'select=eq(n\,89)' -frames:v 1 out.png
```

Still missing, neither of which the RGB camera needs: the `ov05c10` sensor driver
(the i2c device exists from ACPI `OMNI5C10` with **no driver bound** — the ISP
drives the sensor itself) and the `HIMX1092` IR camera used for face unlock,
which gets no device node at all.

If an app cannot see the camera, suspect the NV12-only, media-controller-centric
(`I/O MC`) device rather than the driver. And note this is an **all-AMD** host:
the Intel `ipu7-camera-*` / `icamerasrc` packages are irrelevant here.

## Global LD_LIBRARY_PATH vs. Hyprland (GLIBCXX)

**Problem:** After a flake update, Hyprland refuses to start and
`xdg-desktop-portal-hyprland.service` fails during activation:

```
libstdc++.so.6: version `GLIBCXX_3.4.36' not found (required by libhyprutils.so.13)
```

**Root cause:** `modules/common.nix` puts the nix-ld library directory on the
**global** `environment.sessionVariables.LD_LIBRARY_PATH` (so NixOS-compiled
binaries can dlopen those libs, e.g. ONNX runtime via fastembed). That forces
*that* directory's `libstdc++` on every process — including Hyprland. When the
Hyprland stack is built with a newer GCC than `pkgs.stdenv.cc` (gcc 16 vs 15 in
August 2026), the older libstdc++ shadows the correct one and nothing in the
Hyprland stack can load.

**Solution:** pin the nix-ld libstdc++ to the newest GCC in nixpkgs rather than
the default stdenv one — newer libstdc++ is backward compatible, so FHS binaries
keep working:

```nix
programs.nix-ld.libraries = with pkgs; [
  gcc16.cc.lib   # NOT stdenv.cc.cc.lib
  ...
];
```

**Diagnosing it:** the failure is invisible until you reboot, because the running
compositor is the old one. Test the new binary directly before rebooting:

```bash
D=$(readlink -f /run/current-system/sw/bin/Hyprland | xargs dirname)
LD_LIBRARY_PATH=/run/current-system/sw/share/nix-ld/lib ldd "$D/.Hyprland-wrapped" | grep "not found"
# any output = the compositor will not start after reboot
```

Re-check this whenever GCC in nixpkgs moves ahead of the version Hyprland is
built with.

## Plymouth Resolution on Limine

**Problem:** Plymouth displays at low resolution (~1080p) regardless of native display.

**Root cause:** NixOS Limine module doesn't expose per-entry `resolution:` option. The `interface.resolution` only affects the menu, not the Linux framebuffer.

**Status:** Accepted limitation. Consider filing nixpkgs feature request for `boot.loader.limine.resolution`.

## Home Manager Backup File Conflicts

**Problem:** `programs.ghostty.themes.*` creates regular files that cause backup conflicts on each activation.

**Solution:** Use `xdg.configFile` with `force = true` instead (`home/ghostty.nix:77-80`).

## Shell Module Import Architecture

**Problem:** Conditional Home Manager imports (`if shell == "illogical" then ...`) don't work correctly with NixOS specialisations. Home Manager is evaluated at build time with the default configuration, so the non-default shell's dotfiles are never deployed.

**Symptom:** Settings menu (and other UI elements) don't work in the non-default shell because critical files like `settings.qml` are missing from `~/.config/quickshell/ii/`.

**Solution:** Separate shell modules into two parts:
1. **Dotfiles module** (`dotfiles-only.nix`) - Always imported, deploys config files
2. **Programs module** (main shell module) - Conditionally imported, sets fish/starship/theming

**Files:**
- `home/shells/illogical/dotfiles-only.nix` - Always imported (xdg.configFile, activation script)
- `home/shells/illogical/` - Conditionally imported (packages, fish, theming)
- `home/shells/noctalia/` - Conditionally imported (works with Noctalia Home Manager module)

**Implementation:** `home/home.nix` imports `./shells/illogical/dotfiles-only.nix` unconditionally, ensuring Quickshell files exist regardless of which shell is selected at boot.

**Important:** When adding new shell configurations:
1. Create a `dotfiles-only.nix` that only handles file deployment (xdg.configFile, activation)
2. Import it unconditionally in `home/home.nix`
3. Keep program configs (fish, starship, theming) in the conditionally-imported module to avoid conflicts

## Shell Restart on Store Path Change (Illogical Impulse only)

**Problem:** After `nixos-rebuild switch`, a running quickshell process still has
old `/nix/store/...` paths baked in while IPC commands reference the new path,
producing "No running instances" errors.

**Solution:** `home/shells/restart-on-change.nix` hashes the quickshell package
path, compares it to `~/.local/state/shell-store-hash`, and restarts the shell
via `hyprctl dispatch exec` when it changed.

**Noctalia is no longer handled here.** v5 ships a systemd user unit
(`programs.noctalia.systemd.enable`) that is `PartOf hyprland-session.target` and
carries `X-Restart-Triggers` on its config, so systemd restarts it on rebuild and
on config changes.

```bash
systemctl --user restart noctalia     # Noctalia v5
pkill -f 'bin/quickshell' && hyprctl dispatch exec "quickshell -c ~/.config/quickshell/ii"   # Illogical
```

Note the Illogical process is named `.quickshell-wra` (a wrapper), so
`pkill -x quickshell` matches nothing — match the binary path instead, and run it
from an interactive shell (a script whose own command line contains
`bin/quickshell` would be matched by `pkill -f` and kill itself).

## Noctalia v5

**The `noctalia` flake input is pinned to a v5 tag** (`v5.0.0-beta.9`) in
`flake.nix`. v5 has **no stable release yet** — every v5 tag upstream is a beta —
so the pin is deliberate: a branch-tracking input would ship config-schema
changes straight to the desktop. Bump it consciously and re-validate the config.

Upstream also renamed the repo `noctalia-shell` → `noctalia`; v4 lives on a
frozen `legacy-v4` branch.

### What changed from v4

| | v4 | v5 |
|---|---|---|
| Implementation | Quickshell QML config | standalone C++/meson binary |
| HM module | `programs.noctalia-shell` | `programs.noctalia` |
| Config | `settings.json` + `gui-settings.json` + `colors.json` | `config.toml` |
| IPC | `noctalia-shell ipc call launcher toggle` | `noctalia msg panel-toggle launcher` |
| Lock | `... ipc call lockScreen lock` | `noctalia msg session lock` |
| Startup | `exec-once = noctalia-shell` | systemd user unit |
| PAM | `NOCTALIA_PAM_SERVICE` env var | authenticates via the `login` PAM service |

Upstream publishes builds to **`noctalia.cachix.org`** (added to
`nix.settings.substituters` in `modules/common.nix`), so rebuilds do not compile
the shell from source.

### Configuration layering

| File | Owner |
|------|-------|
| `~/.config/noctalia/config.toml` | Home Manager, read-only symlink, generated from `home/shells/noctalia/config.toml` |
| `~/.local/state/noctalia/settings.toml` | Noctalia itself — every change made in the Settings GUI |

Noctalia layers the state file **on top of** config.toml, so GUI tweaks persist
across rebuilds by themselves. This replaces v4's copy-and-hash deployment;
there is no `.deployed-hash` any more and nothing in `~/.config/noctalia` is
overwritten.

Consequence worth remembering: **a key present in the state file keeps winning
over the repo.** If a change to `config.toml` seems to have no effect, check
whether the GUI (or the first-run setup wizard) already wrote that key to
`~/.local/state/noctalia/settings.toml`, and delete it there.

To promote a GUI change into the repo, copy the keys from the state file into
`home/shells/noctalia/config.toml`, keeping the `/home/USER` placeholder that
`shell.nix` substitutes. Validate before rebuilding — the Home Manager module
also validates at build time and fails the build on a bad config:

```bash
noctalia config validate home/shells/noctalia/config.toml
```

### Theme templates — handle with care

Noctalia renders theme templates into *other applications'* config files, which
collides with anything Home Manager manages. Two rules learned the hard way:

1. **A disabled builtin template still runs its `undo.sh` on every startup.**
   Those undo hooks delete files and rewrite configs. Enabling the gtk3/gtk4/qt
   templates made Home Manager activation fail on a clobbered
   `~/.config/gtk-4.0/gtk.css`; leaving `ghostty` disabled made its undo hook
   delete `~/.config/ghostty/themes/noctalia`, which `home/ghostty.nix` seeds and
   `theme = noctalia` references. Only `ghostty` is enabled, and its apply hook
   is a no-op while that line is already present.

2. **The builtin `hyprland` template is not used.** Its `apply.sh` probes the
   running compositor and writes `~/.config/hypr/hyprland.lua` whenever it reads
   as Lua-config mode — a stray `hyprland.lua` outranks `hyprland.conf` and boots
   a stock desktop (see *Hyprland 0.56+ Lua Config*). It also appends a `source`
   line to `hyprland.conf`, which is a read-only Home Manager symlink. Instead,
   `[theme.templates.user.hyprland]` in `config.toml` renders the same shipped
   template to a fixed path, and `home/hyprland/default.nix` sources it
   declaratively.

   Its output is named `noctalia-colors.conf`, **not** `noctalia.conf`, because
   the builtin template's undo hook deletes `~/.config/hypr/noctalia.conf` and
   strips any line matching `source = .*noctalia\.conf` from `hyprland.conf`.
   That same regex is why the per-shell includes were renamed
   `bindings-noctalia.conf` → `noctalia-bindings.conf` (and likewise for
   `autostart`): the old names matched, so the undo hook tried to strip the
   keybind include.

### Command-output bar widgets need a plugin

v4's `CustomButton` could poll a command and render its stdout
(`textCommand` + `parseJson`). **v5's `custom_button` has no equivalent** — its
settings are `glyph`, `label`, `tooltip` and click/scroll actions only, and there
is no builtin widget that renders command output (`text` is fixed text).

That affected four widgets, and both are now local Luau plugins:

- `ai-usage` → `home/shells/noctalia/plugins/ai-usage` (see *AI Subscription
  Usage → Bar widget*).
- all four VPN buttons → `home/shells/noctalia/plugins/vpn`, which has **two**
  widget entries. `status` polls `vpn-status --bar` and is the collapsed face of
  the `vpn` accordion capsule (see *Bar capsule groups*); `toggle` polls
  `vpn-status --bar <N>` and renders one VPN, and there are three instances of
  it inside the accordion.

`vpn-status --bar [N]` (in `home/home.nix`, next to the raw state map the script
already emitted) supplies the names, which identify tenants and so come from the
gitignored `secrets.nix`. Because the plugin fetches them at runtime, **nothing
tenant-identifying is needed in `config.toml` at all** — there is no name
placeholder and no substitution for it in `shell.nix`.

Two things this plugin demonstrates that are worth reusing:

1. **Per-instance settings.** A `[[widget.setting]]` block in `plugin.toml`
   (`key`, `type`, `default`, and `label_key`/`description_key` — a literal
   `label` is a lint *error*, and the key must exist in
   `translations/en.json`) becomes a valid key in `[widget.<name>]`, read back
   with `noctalia.getConfig("vpn")`. That is why three bar buttons need only one
   entry file. It is also the reason a plugin widget rejects `capsule = true`:
   a plugin widget accepts only the settings its manifest declares.
2. **Click feedback.** `~/.local/bin/vpn-<N>` reports progress and errors on
   stdout, which goes nowhere when the shell spawns it — a connection takes
   10-30s, so a plain `exec` action looks like it did nothing, and a failure is
   completely silent. `toggle.luau` runs it through `runAsync` instead and turns
   the last non-empty output line into `noctalia.notify` / `notifyError`, while
   painting a `shield-bolt` "working" state and pausing its own polling
   (`busy`) so the 5s tick cannot repaint over it.

### Local plugins

A plugin is a directory holding `plugin.toml` plus its `.luau` entry files.
Noctalia scans `~/.local/share/noctalia/plugins` unconditionally — that is the
implicit "local" source, and it outranks every configured git/path source, so a
drop-in there needs no `[[plugins.source]]` entry. Discovery is not activation:
the plugin runs only once its id appears in `[plugins] enabled` in `config.toml`.

`home/shells/noctalia/shell.nix` deploys `./plugins/*` there with
`xdg.dataFile` (`ai-usage` and `vpn`). **New plugin files must be `git add`ed
before rebuilding** —
the flake only copies tracked files, and an untracked directory fails evaluation
with `path '/nix/store/...-source/home/shells/noctalia/plugins/x' does not exist`.

Widget scripts run against these globals:

| | |
|---|---|
| `noctalia.setUpdateInterval(ms)` | poll cadence; drives repeated `update()` calls |
| `noctalia.runAsync(argv, cb, timeoutMs)` | `cb` gets `{exitCode, stdout, stderr, timedOut, …}` |
| `noctalia.json.decode(s)` | returns a table, or `nil, err` |
| `barWidget.setText/setGlyph/setTooltip/setColor/setGlyphColor` | colors are palette role names (`primary`, `tertiary`, `error`, `on_surface`, …) |
| `update()`, `onClick()`, `onRightClick()`, `onScroll()` | globals the host calls |

Two things that cost time:

1. **A configured gesture binding beats the script's own handler.** `Widget`
   installs the `[widget.<name>.actions]` bindings on an outer input area and
   strips those buttons from the plugin's inner area, so a `left = "exec …"`
   entry means `onClick` never fires. Pick one.
2. **Every paint must set every field.** A patch that omits `setColor` leaves the
   previous value on screen, so an error state keeps the last good color.

Check the work before rebuilding:

```bash
noctalia plugins lint home/shells/noctalia/plugins/ai-usage
noctalia config validate <rendered config.toml>   # warns "unrecognized widget
                                                  # type" until it is installed
```

### Bar capsule groups

`[bar.main] capsule_group` wraps several widgets in one shared pill, which is
what keeps a long bar readable. **A group is itself a lane item**: the lane lists
`group:<id>` where the group sits, and the members are named only in the group
entry.

```toml
start = ["launcher", "group:sysmon", "clock"]
end   = ["group:vpn", "group:ai", "group:tools", "control-center"]

capsule_group = [
  { id = "sysmon", members = ["cpu", "cpu-temp", "ram"] },
  { id = "vpn", members = ["vpn-status", "vpn1", "vpn2", "vpn3"],
    accordion = true, accordion_direction = "end" },
]
```

Listing the members in the lane instead is the trap: it **validates clean and
renders nothing** — the widgets appear ungrouped, with no capsule and no
accordion, and nothing is logged. Putting the group id in a lane without the
`group:` prefix is only slightly louder: `[shell] widget factory: unknown widget
"vpn"`, and a name that happens to collide with a widget *type* (`sysmon`) even
draws a stray default widget instead.

Group keys: `id`, `members`, `enabled`, `accordion`, `accordion_direction`
(`start`/`end` only), `fill`, `border`, `foreground`, `opacity`, `padding`,
`radius`, `widget_spacing`. The GUI's drawer-mode options are not in the 5.0.0
schema even though its translations mention them.

`accordion = true` draws only the first member and unfolds the rest on hover —
the cheapest way to keep a cluster of buttons off the bar without losing them.

Single widgets take `capsule = true` directly, **but plugin widgets do not**: a
plugin only accepts the settings its `plugin.toml` declares, so
`noctalia config validate` flags `widget.ai-usage.capsule` as an unknown setting.
Give a plugin widget a one-member `capsule_group` instead.

Useful de-cluttering options on the builtin widgets, all verified against 5.0.0:
`volume.show_label`, `notifications.hide_when_no_unread`,
`battery.hide_when_full` / `hide_when_plugged` / `label_content`
(`percent`/`time`/`rate`), `sysmon.label_min_width` (stops the capsule resizing
as digits come and go) / `visualization` (`graph`/`gauge`/`none`) / `show_glyph`
/ `show_value`, `tray.drawer` + `drawer_columns` + `drawer_item_size`.

There is no reference doc for most of this; `noctalia config export full` prints
every widget with its defaults, and `noctalia config validate` on a scratch file
names unknown keys and rejects bad enum values, which is how the above was
established.

### Other v5 notes

- The bar is `[bar.main]` with `start`/`center`/`end` lists of widget names;
  named `[widget.<name>]` entries add config. `sysmon` shows **one** stat per
  instance, so v4's combined CPU/temp/RAM widget is three widgets.
- v5 floats the bar by default: `margin_ends = 100` insets it 100px from each
  end and `margin_edge` lifts it off the screen edge. `config.toml` zeroes both
  for the v4 edge-to-edge bar. With `margin_ends = 0` the default
  `concave_edge_corners` carves the two inner corners instead of rounding the
  outer ones, so `radius` is left alone.
- Running under systemd means the process has no logind session, so Noctalia logs
  `failed to resolve logind session` and disables its own brightness control and
  lock-on-suspend monitor. Neither matters here: brightness keys call
  `brightnessctl` directly (`home/hyprland/bindings.nix`) and lock-before-sleep
  comes from hypridle's `before_sleep_cmd`.
- The first run opens a setup wizard which writes to the state file — on this
  machine it set `theme.source = "wallpaper"`, overriding the repo's Kanagawa.
  Fix with `noctalia msg color-scheme-set builtin Kanagawa`.

## Rebuilding While the Screen Is Locked

**Problem:** `nixos-rebuild switch` restarts `noctalia.service`, and Noctalia v5
owns the lock screen. If hypridle has locked the session (5 minutes idle,
`home/hyprland/hypridle.nix`), the rebuild kills the lock surface and Hyprland
falls into its "Oopsie daisy, it looks like you locked your screen but the
lockscreen app died" screen.

**Why that is a trap here:** the on-screen instructions say to run
`hyprctl eval 'hl.clear_crashed_lockscreen()'`, and that fails with *"eval is
only supported with the lua config manager"* — this config is deliberately on
the legacy `hyprland.conf` parser (see *Hyprland 0.56+ Lua Config*). There is no
dispatcher equivalent; `clear_crashed_lockscreen` exists only as a Lua binding.

**Recovery** (from a TTY, over SSH, or from a shell in the stuck session):

```bash
hyprctl keyword misc:allow_session_lock_restore true   # runtime only, not persisted
noctalia msg session lock                              # a fresh lock takes over
```

`hyprctl locked` reports the crashed state as `true`, and the lock surface is
**not** a layer-shell layer, so `hyprctl layers` never lists it — that is not
evidence the lock failed.

**Do not put `misc:allow_session_lock_restore` in the config.** It reads like a
crash-recovery option ("allow you to restart a lockscreen app in case it
crashes") but the compositor never checks whether the existing lock is actually
dead. In `src/managers/SessionLockManager.cpp` (0.56.2):

```cpp
if (PROTO::sessionLock->isLocked() && !*PALLOWRELOCK && ...) {
    pLock->sendDenied();        // the only thing protecting a LIVE lock
    return;
}
if (m_sessionLock && !clientDenied() && !clientLocked())
    return;                     // only bails while the old lock is in limbo
m_sessionLock = makeUnique<SSessionLock>();   // otherwise the new lock TAKES OVER
```

With the flag on, any client that can bind `ext_session_lock_manager_v1` on the
Wayland socket can lock, wait for the `locked` event (sent unconditionally by a
5s fallback timer), then `unlock_and_destroy` — which clears `m_locked` and
refocuses the desktop with **no PAM authentication at all**. Combined with
`security.sudo.wheelNeedsPassword = false` that is passwordless root from any
sandboxed app in the session, so the flag stays a momentary, manual recovery
step and never a persistent setting.

The real fix is not to rebuild while locked; the exposure window is only as long
as the session is left locked with a rebuild running.

## 1Password SSH Agent

SSH keys are managed through 1Password's SSH agent (`home/1password-secrets.nix`). After rebuild:

1. Open 1Password GUI
2. Settings → Developer → Enable "Integrate with 1Password CLI"
3. Settings → Developer → Enable "Use the SSH agent"
4. Add/import SSH keys to 1Password

SSH commands will automatically use keys from 1Password after a single unlock.

## AI Subscription Usage (`ai-usage`)

Shows how much of the Claude Code and Codex CLI subscription limits are used —
the same numbers Claude Code's `/usage` and Codex's `/status` screens report.

**Script:** `home/home.nix` → `.local/bin/ai-usage`

```bash
ai-usage                      # report for both providers in the terminal
ai-usage --bar                # JSON for the Noctalia CustomButton
ai-usage --refresh            # bypass the response cache
AI_USAGE_DEBUG=1 ai-usage     # show why a response failed to parse
```

### How it works

It reads (read-only) the OAuth tokens the CLIs already keep on disk and calls the
endpoints those CLIs use themselves:

| Provider | Credentials | Endpoint |
|----------|-------------|----------|
| Claude Code | `~/.claude/.credentials.json` (or `$CLAUDE_CONFIG_DIR`) | `GET https://api.anthropic.com/api/oauth/usage` |
| Codex CLI | `~/.codex/auth.json` (or `$CODEX_HOME`) | `GET https://chatgpt.com/backend-api/wham/usage` |

Notes:
- Tokens are **never refreshed** by the script. Refresh tokens rotate on use, so
  refreshing here would invalidate the CLI's own session. An expired token is
  reported as `token expired - run claude` / `run codex` and fixes itself the
  next time that CLI is used.
- The Anthropic request must send `User-Agent: claude-code/...` and
  `anthropic-beta: oauth-2025-04-20`; other user agents land in a much more
  aggressively rate-limited bucket.
- Responses are cached in `$XDG_RUNTIME_DIR/ai-usage/` for 120s so the bar widget
  polling every minute doesn't hammer the endpoints.
- Which windows exist depends on the plan: Claude Max reports a 5-hour and a
  weekly window (plus per-model ones when present), a ChatGPT Team plan only a
  weekly one.
- Both endpoints are internal and may change without notice.

Credit: the approach is lifted from [`lumen-model-usage`](https://github.com/DigitalPals/Lumen)
(the `model-usage` bar module in DigitalPals' Lumen shell).

### Bar widget (Noctalia)

`home/shells/noctalia/plugins/ai-usage` is a local Noctalia plugin that polls
`ai-usage --bar` every 60s and paints its `text`, `tooltip` and `textColor`
straight onto a bar widget. `config.toml` wires it up as:

```toml
[plugins]
enabled = ["arnold/ai-usage"]

[widget.ai-usage]
type = "arnold/ai-usage:usage"
```

Left click re-runs the script with `--refresh`, bypassing the response cache.
That is handled by the script's `onClick`, which is exactly why the widget
declares **no** `[widget.ai-usage.actions]` — a configured binding would take the
click first (see *Noctalia v5 → Local plugins*).

`C` is Claude, `X` is Codex; the number is the highest used percentage across
that provider's windows. `-` means not signed in / token expired, `?` means rate
limited. The text turns `tertiary` at 75% used and `error` at 90%.

Colors in JSON output must be one of Noctalia's palette roles —
`primary`, `secondary`, `tertiary`, `error`, `none` — anything else is ignored.
The tooltip is rendered as HTML, so runs of spaces collapse; the script drops its
column padding in tooltip mode instead of trying to align with spaces.

This replaced the v4 **CustomButton** (`textCommand` + `parseJson` +
`textIntervalMs`), which v5 has no equivalent for. The script's `--bar` output
contract did not change across the move.

### Gotcha: `label` is reserved in jq

The Codex filter originally defined `def label($secs)`, which is a syntax error —
`label` is a jq keyword (`label $out | ...`). The filter never compiled and every
call fell through to `err error "unexpected response"`. It stayed invisible while
the Codex token was expired (the request 401'd before reaching the parse) and only
surfaced after signing in again. `AI_USAGE_DEBUG=1` exists to make that class of
failure visible.

## App Profile Backup/Restore

Encrypted app profile backup system using Age encryption and a private GitHub repository. Supports 1Password integration for automatic key retrieval across machines.

### Supported Applications

- **Chrome**: Cookies, login data, sessions, preferences
- **Firefox**: Cookies, logins, sessions, sync data

### Setup with 1Password (Recommended)

1. Generate an Age keypair locally:
   ```bash
   age-keygen
   # Output:
   # Public key: age1xxxxxxxxxx...
   # AGE-SECRET-KEY-1XXXXXXXXXX...
   ```

2. Store the private key in 1Password:
   - Create a new item in 1Password (e.g., "age-key" in your private vault)
   - Add a field called "private-key" with the `AGE-SECRET-KEY-1...` value
   - The 1Password reference will look like: `op://VAULT/age-key/private-key`

3. Configure in `home/home.nix`:
   ```nix
   programs.app-backup = {
     enable = true;
     repoUrl = "git@github.com:YOUR_USER/private-settings.git";
     ageRecipient = "age1...your-public-key...";
     ageKey1Password = "op://VAULT/age-key/private-key";
   };
   ```

4. Rebuild: `sudo nixos-rebuild switch --flake .`

### Alternative: File-based Key

If not using 1Password, you can use a file-based key:
```nix
programs.app-backup = {
  enable = true;
  ageRecipient = "age1...";
  ageKeyPath = "~/.config/age/key.txt";  # Fallback if ageKey1Password not set
};
```

### Commands

Via Forge TUI (recommended):
```bash
forge apps backup        # Backup + push profiles to GitHub
forge apps restore       # Restore profiles from GitHub
forge apps status        # Check for remote updates
forge apps               # Interactive menu

# Backward compatibility alias
forge browser backup     # Same as forge apps backup
```

Via standalone scripts (after Home Manager activation):
```bash
app-backup --push          # Backup + push
app-restore --pull         # Pull + restore

# Deprecated aliases (still work)
browser-backup --push      # Same as app-backup
browser-restore --pull     # Same as app-restore
```

### New Machine Bootstrap

1. Install NixOS with Forge: `nix run github:Arnld81nl/nixos-config`
2. Sign in to 1Password desktop app (unlocks the CLI)
3. Run `forge apps restore`
4. Open browsers - sessions restored (Chrome, Firefox)

The age key is retrieved from 1Password on-the-fly - no manual key management needed!

### Troubleshooting

- **"Apps are running"**: Close Chrome/Firefox or use `--force`
- **"1Password not unlocked"**: Open 1Password app and sign in
- **"op: command not found"**: Rebuild to install 1Password CLI
- **"Git push failed"**: Check SSH key is in 1Password agent
- **"Config not found"**: Enable `programs.app-backup` and rebuild

### Security Notes

Profile archives contain session cookies, auth tokens, and potentially saved passwords. The archives are encrypted with Age before being pushed to GitHub.

- Age private key is stored in 1Password, never on disk
- Key is retrieved on-the-fly and never written to filesystem
- LUKS disk encryption (enabled by default) provides additional protection
- Decrypted archives are only created in temp directories and shredded after use

## Hyprland 0.56+ Lua Config (why Hyprland comes from nixpkgs)

**Problem:** After a flake update (August 2026), Hyprland booted into a stock
desktop: no Noctalia bar, default wallpaper and theme, no keybinds, nothing from
`autostart-noctalia.conf` running. Nothing crashed and nothing was logged as an
error.

**Root cause:** Hyprland replaced the hyprlang `hyprland.conf` format with a Lua
config (`hyprland.lua`, `hl.*` API). Config discovery
(`src/config/supplementary/jeremy/Jeremy.cpp`) prefers `hyprland.lua` and only
falls back to `hyprland.conf`; **on git master the legacy parser was removed
entirely** (`src/config/ConfigManager.cpp` always constructs
`Lua::CConfigManager`). With no `hyprland.lua` present, Hyprland *generates a
default one* and boots that — silently. The `hyprland` flake input tracked
master, so `nix flake update` pulled this in.

Tell them apart in `$XDG_RUNTIME_DIR/hypr/<sig>/hyprland.log`:

```
[cfg] Regular config at /home/arnold/.config/hypr/hyprland.lua   # BAD: Lua manager
WARN ]: No config file found; attempting to generate.            # BAD: stock config
[cfg] Lua config not found, using legacy config at .../hyprland.conf   # GOOD
```

Another tell: `hyprctl dispatch exec foo` fails with
`attempt to call a nil value (global 'exec')` — the Lua manager is active.

**Solution:** `modules/desktop-environments.nix` uses `pkgs.hyprland` (0.56.2 at
the time of writing, which still parses `hyprland.conf`) instead of the
`hyprwm/Hyprland` flake input, and that input was **removed from `flake.nix`** so
it can't come back on the next update. nixpkgs is also cached, so no source build.

**This is a deferral, not a fix.** When nixpkgs moves to a release without the
legacy parser (0.57+), the same breakage returns. Migrating means rewriting
`home/hyprland/*.nix` to emit `hyprland.lua`.

The Noctalia side of that migration is already in place: v5 ships **both**
`hyprland.conf` and `hyprland.lua` variants of its palette template, so
`[theme.templates.user.hyprland]` in `home/shells/noctalia/config.toml` only
needs to point at the `.lua` input and write a `.lua` output (see *Noctalia v5 →
Theme templates*).

**Verify before rebooting** (the running compositor is still the old one):

```bash
rm -f ~/.config/hypr/hyprland.lua   # a stray autogenerated one always wins
nix build --impure --no-link --print-out-paths \
  .#nixosConfigurations.G1a.config.programs.hyprland.package
<store-path>/bin/Hyprland --verify-config   # must say "legacy config" + "config ok"
```

If it prints `Regular config at .../hyprland.lua` instead, the desktop will come
up stock after reboot.
