# NixOS Configuration Notes

Configuration details and solutions to issues in this NixOS setup.

## Configuration Structure

```
~/nixos-config/                     # Symlinked from /etc/nixos
├── flake.nix                       # Main flake with host definitions
├── hosts/
│   └── x1yoga/                     # Lenovo ThinkPad X1 Yoga Gen 6 (Intel)
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
| `x1yoga` | ThinkPad X1 Yoga Gen 6 (Intel) | Default (Noctalia), illogical |

```bash
# Rebuild (includes all shell specialisations)
sudo nixos-rebuild switch --flake .#x1yoga

# Or use hostname (auto-detected)
sudo nixos-rebuild switch --flake .
```

### Rebuilding with Active Specialisation

**IMPORTANT:** When rebuilding, always check which specialisation is currently active and re-activate it after the rebuild. A plain `nixos-rebuild switch` activates the **default** configuration, which will switch you out of any active specialisation.

```bash
# Check which specialisation is active (if any)
# Look at DESKTOP_SHELL environment variable or check the runtime file
cat /run/user/$(id -u)/desktop-shell 2>/dev/null || echo "default"

# Standard rebuild (activates default configuration)
sudo nixos-rebuild switch --flake .

# If you were in a specialisation, re-activate it:
sudo /run/current-system/specialisation/illogical/bin/switch-to-configuration switch
```

**For Claude:** Before running `nixos-rebuild switch`, always:
1. Check the active shell: `cat /run/user/$(id -u)/desktop-shell 2>/dev/null`
2. If it returns "illogical" (or another specialisation name), re-activate after rebuild:
   ```bash
   sudo nixos-rebuild switch --flake . && \
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

**Properties without new syntax equivalent:**
- `scrollInput` - No equivalent in `windowrule`; must use legacy `windowrulev2` format

**Implementation:** Window rules are split across files:
- `home/hyprland/looknfeel.nix` - Most window rules (using new syntax)
- `home/hyprland/input.nix` - `scrollInput` rule (must use legacy `windowrulev2`)

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

## Shell Restart on Store Path Change

**Problem:** After `nixos-rebuild switch`, the running quickshell process has old `/nix/store/...` paths baked in, while IPC commands (like `qs -c noctalia-shell ipc call launcher toggle`) reference the new path. This causes IPC failures with "No running instances" errors.

**Root cause:** Quickshell processes embed their store path at startup. When the package updates, the symlink at `~/.config/quickshell/noctalia-shell` points to the new path, but the running process still has the old path.

**Solution:** Home Manager activation hook that automatically restarts the shell when store paths change.

**Implementation:** `home/shells/restart-on-change.nix`

The hook:
1. Hashes the current shell package path (noctalia-shell or quickshell)
2. Compares to previously stored hash in `~/.local/state/shell-store-hash`
3. If changed and quickshell is running, kills old process and restarts via `hyprctl dispatch exec`
4. Records new hash for next comparison

**Behavior:**
- **First run**: No restart (no previous hash to compare), just records hash
- **Package unchanged**: No restart, hash matches
- **Package updated**: Automatic restart via hyprctl

**Edge cases handled:**
- First run after adding the hook (no restart)
- No Hyprland running (gracefully skipped)
- No quickshell running (gracefully skipped)
- Dry-run mode (respects `$DRY_RUN_CMD`)

**Files:**
- `home/shells/restart-on-change.nix` - Shared activation hook
- `home/shells/noctalia/default.nix` - Imports restart module
- `home/shells/illogical/default.nix` - Imports restart module

**Manual restart** (if needed):
```bash
pkill -x quickshell && hyprctl dispatch exec noctalia-shell
# Or for Illogical:
pkill -x quickshell && hyprctl dispatch exec "quickshell -c ~/.config/quickshell/ii"
```

## Noctalia Settings (Hybrid Management)

Noctalia settings use a hybrid approach that allows GUI changes while preserving reproducibility across machines.

### How It Works

Settings are stored in `~/.config/noctalia/` as regular files (not symlinks). A hash file tracks when the repo version was last deployed:

- **First run**: Configs are copied from repo to `~/.config/noctalia/`
- **GUI changes**: Saved locally, persist across reboots and rebuilds
- **Repo updated**: When you pull updated configs from another machine and rebuild, the hash changes and local files are overwritten

Implementation: `home/shells/noctalia/shell.nix:44-70`

### Syncing Settings to Another Machine

When you've made GUI changes you want to sync to the repo:

1. **Ask Claude**: "Sync my Noctalia settings to the repo"
   - Claude copies `~/.config/noctalia/*.json` → `home/shells/noctalia/`
2. **Commit and push** the changes
3. **On other machine**: Pull and rebuild → hash changes → local files updated

### Config Files

| File | Purpose |
|------|---------|
| `settings.json` | Main shell settings (bar widgets, layouts) |
| `gui-settings.json` | GUI-specific preferences |
| `colors.json` | Color scheme |
| `plugins.json` | Plugin configuration |
| `.deployed-hash` | Tracks repo version (auto-managed) |

### Forcing a Re-sync

To force re-deployment from repo (discarding local changes):
```bash
rm ~/.config/noctalia/.deployed-hash
sudo nixos-rebuild switch --flake .
```

## 1Password SSH Agent

SSH keys are managed through 1Password's SSH agent (`home/1password-secrets.nix`). After rebuild:

1. Open 1Password GUI
2. Settings → Developer → Enable "Integrate with 1Password CLI"
3. Settings → Developer → Enable "Use the SSH agent"
4. Add/import SSH keys to 1Password

SSH commands will automatically use keys from 1Password after a single unlock.

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
   - Create a new item in 1Password (e.g., "age-key" in Private vault)
   - Add a field called "private-key" with the `AGE-SECRET-KEY-1...` value
   - The 1Password reference will be: `op://Private/age-key/private-key`

3. Configure in `home/home.nix`:
   ```nix
   programs.app-backup = {
     enable = true;
     # Repo is pre-configured to: git@github.com:Arnld81nl/private-settings.git
     ageRecipient = "age1...your-public-key...";
     ageKey1Password = "op://Private/age-key/private-key";
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
