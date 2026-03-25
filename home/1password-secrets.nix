# 1Password SSH Agent Integration
#
# This module configures SSH to try local key files first, then fall back
# to the 1Password SSH agent if available. SSH works even when 1Password
# is locked (using local keys).
#
# === MANUAL ONE-TIME SETUP REQUIRED ===
#
# After rebuilding, open 1Password GUI and configure:
#
# 1. Settings -> Developer -> Enable "Integrate with 1Password CLI"
# 2. Settings -> Developer -> Enable "Use the SSH agent"
# 3. Add your SSH key(s) to 1Password (or import existing keys)
#
# The SSH agent socket will be available at ~/.1password/agent.sock
#
{ config, pkgs, lib, ... }:

{
  # SSH client configuration
  # Uses local key files first, 1Password agent as conditional fallback.
  # This means SSH works even when 1Password is locked.
  programs.ssh = {
    enable = true;
    enableDefaultConfig = false;
    matchBlocks = {
      "*" = {
        # Try local key first (no agent needed)
        identityFile = "~/.ssh/id_ed25519";
        extraOptions.IdentitiesOnly = "yes";
      };
      # GitHub SSH over HTTPS (port 443) - bypasses firewalls blocking port 22
      "github.com" = {
        hostname = "ssh.github.com";
        port = 443;
        user = "git";
      };
    };
    extraConfig = ''
      # Security defaults
      StrictHostKeyChecking accept-new
      HashKnownHosts yes

      # Fallback to 1Password SSH agent when the socket exists
      # This allows SSH to work without 1Password being unlocked
      Match host * exec "test -S %d/.1password/agent.sock"
        IdentityAgent ~/.1password/agent.sock
    '';
  };

}
