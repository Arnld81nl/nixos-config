# VPN Buttons Debug Progress

## Current Status
- VPN scripts work perfectly from CLI terminal
- VPN buttons in Noctalia bar trigger but fail with "Failed to get credentials from 1Password"
- Status indicators (circles) are now stable (no more random changes)
- User mentioned previously a reboot fixed the button issue

## What Works
1. `~/.local/bin/vpn-toggle RSG` - works from terminal
2. `~/.local/bin/vpn-status-rsg` - returns correct JSON
3. 1Password CLI (`op read`) - works from terminal

## The Problem
When Quickshell executes the script via CustomButton click, 1Password CLI cannot connect to the desktop app. Error: "could not read secret... error initializing client: connecting to desktop app: read: connection reset"

## Root Cause Analysis
1Password CLI needs to connect to the 1Password desktop app via a socket. When running from Quickshell, the environment is different and the connection fails.

## What We Tried
1. Added `DBUS_SESSION_BUS_ADDRESS` environment variable - didn't help
2. Added environment vars to wrapper scripts (vpn-rsg, vpn-dnv, vpn-esdal) - didn't help
3. Restored original `systemctl --user show-environment` approach - current state

## Current Script State (vpn-toggle)
```bash
# Ensure we have the proper environment for 1Password CLI
export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}"

# Import systemd user environment (for 1Password socket access)
if command -v systemctl &>/dev/null; then
  eval "$(systemctl --user show-environment 2>/dev/null | sed 's/^/export /')"
fi
```

## Quickshell Environment (from /proc/PID/environ)
Has these relevant vars:
- HOME=/home/arnold
- XDG_RUNTIME_DIR=/run/user/1000
- DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/1000/bus
- PATH includes /run/wrappers/bin (where `op` lives)

## Test That Proves Environment Is The Issue
This works:
```bash
env -i HOME=/home/arnold XDG_RUNTIME_DIR=/run/user/1000 DBUS_SESSION_BUS_ADDRESS=unix:path=/run/user/1000/bus PATH="/run/wrappers/bin:/etc/profiles/per-user/arnold/bin:/run/current-system/sw/bin" bash -c 'op read "op://Vault/VPN-Item/username" --account my-account'
# Returns: username
```

This fails (minimal env):
```bash
env -i HOME="$HOME" PATH="..." bash -c 'op read ...'
# Error: connection reset
```

## Next Steps To Try
1. **Reboot** - user said this fixed buttons before
2. Check if `systemctl` is in Quickshell's PATH
3. Try running `systemctl --user show-environment` from within Quickshell context
4. Consider alternative: store credentials in a secure file instead of 1Password
5. Check 1Password app settings - "Integrate with 1Password CLI" must be enabled

## Files Modified
- `/home/arnold/nixos-config/home/home.nix` - VPN scripts and status scripts

## Settings.json CustomButton Config
Located at `~/.config/noctalia/settings.json`, buttons configured as:
```json
{
    "id": "CustomButton",
    "textCommand": "~/.local/bin/vpn-status-rsg",
    "leftClickExec": "~/.local/bin/vpn-rsg",
    "parseJson": true,
    "textIntervalMs": 2000,
    "leftClickUpdateText": true,
    "showIcon": false
}
```

## VPN Config
Located at `~/.config/vpn/config`:
- OP_ACCOUNT="my-account"
- RSG: 0.0.0.0:10443 (working)
- DNV: 0.0.0.0:443 (working)
- Esdal: 0.0.0.0:443 (needs separate certs - TODO later)
