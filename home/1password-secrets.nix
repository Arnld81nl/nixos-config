# 1Password SSH Agent Integration
#
# This module configures SSH to use 1Password's SSH agent, making SSH keys
# stored in 1Password available after a single unlock.
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
  # Point SSH_AUTH_SOCK to 1Password's agent socket
  home.sessionVariables = {
    SSH_AUTH_SOCK = "$HOME/.1password/agent.sock";
  };

  # SSH client configuration
  # Note: enableDefaultConfig = false is recommended by Home Manager (defaults are being deprecated)
  # We explicitly set the defaults we want in matchBlocks and extraConfig
  programs.ssh = {
    enable = true;
    enableDefaultConfig = false;
    matchBlocks = {
      "*" = {
        # Try local key first, then fall back to 1Password agent
        identityFile = "~/.ssh/id_ed25519";
        identityAgent = "~/.1password/agent.sock";
      };
      # GitHub SSH over HTTPS (port 443) - bypasses firewalls blocking port 22
      "github.com" = {
        hostname = "ssh.github.com";
        port = 443;
        user = "git";
      };
    };
    extraConfig = ''
      # Security defaults (previously provided by Home Manager)
      StrictHostKeyChecking accept-new
      HashKnownHosts yes
      # Auto-add keys to agent on first use
      AddKeysToAgent yes
    '';
  };

}
