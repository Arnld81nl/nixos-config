# Application Profile Backup/Restore Module
#
# Provides age-encrypted app profile backup to a private GitHub repository.
# Supports 1Password integration for automatic key retrieval across machines.
#
# MINIMAL BACKUP - Only backs up essential files for:
# - Login sessions and cookies
# - Saved passwords (encrypted by browser/app)
# - Firefox Sync / Chrome sync data
# - Current session tabs
#
# Does NOT backup: cache, history, extensions, themes, or other data.
# Typical backup size: <15MB (vs 3GB+ for full profiles)
#
# See CLAUDE.md for setup instructions and troubleshooting.
#
{ config, pkgs, lib, ... }:

with lib;

let
  cfg = config.programs.app-backup;

  # Essential Chrome files for login/session restoration
  # These are the minimum files needed to preserve:
  # - Cookies (session cookies for logged-in sites)
  # - Login Data (saved passwords, encrypted)
  # - Web Data (autofill, form data)
  # - Session data (open tabs)
  # - Preferences and settings
  chromeEssentialFiles = [
    "Default/Cookies"
    "Default/Cookies-journal"
    "Default/Login Data"
    "Default/Login Data-journal"
    "Default/Web Data"
    "Default/Web Data-journal"
    "Default/Preferences"
    "Default/Secure Preferences"
    "Default/Current Session"
    "Default/Current Tabs"
    "Default/Last Session"
    "Default/Last Tabs"
    "Default/Bookmarks"
    "Default/Favicons"
    "Default/Favicons-journal"
    "Local State"
  ];

  # Essential Firefox files for login/session restoration
  # Firefox stores profile data in random-named subdirectories
  # These glob patterns match the essential files in any profile
  firefoxEssentialPatterns = [
    "*.default*/cookies.sqlite"
    "*.default*/cookies.sqlite-wal"
    "*.default*/logins.json"
    "*.default*/key4.db"
    "*.default*/cert9.db"
    "*.default*/prefs.js"
    "*.default*/sessionstore.jsonlz4"
    "*.default*/sessionstore-backups/recovery.jsonlz4"
    "*.default*/signons.sqlite"
    "*.default*/formhistory.sqlite"
    "*.default*/places.sqlite"
    "*.default*/favicons.sqlite"
    "profiles.ini"
    "installs.ini"
  ];

  # The app-backup script
  app-backup = pkgs.writeShellApplication {
    name = "app-backup";
    runtimeInputs = with pkgs; [ coreutils gnutar gzip age git git-lfs findutils libsecret ];
    text = ''
      set -euo pipefail

      # Colors
      RED='\033[0;31m'
      GREEN='\033[0;32m'
      YELLOW='\033[1;33m'
      BLUE='\033[0;34m'
      NC='\033[0m'

      log_info() { echo -e "''${BLUE}[INFO]''${NC} $1"; }
      log_success() { echo -e "''${GREEN}[SUCCESS]''${NC} $1"; }
      log_warn() { echo -e "''${YELLOW}[WARN]''${NC} $1"; }
      log_error() { echo -e "''${RED}[ERROR]''${NC} $1"; exit 1; }

      # Load configuration (check new path first, then legacy)
      CONFIG_FILE="$HOME/.config/app-backup/config"
      if [[ ! -f "$CONFIG_FILE" ]]; then
        CONFIG_FILE="$HOME/.config/browser-backup/config"
      fi
      if [[ ! -f "$CONFIG_FILE" ]]; then
        log_error "Config file not found. Run: nixos-rebuild switch"
      fi
      # shellcheck source=/dev/null
      source "$CONFIG_FILE"

      # Validate required config (support both old and new variable names)
      APP_BACKUP_REPO="''${APP_BACKUP_REPO:-''${BROWSER_BACKUP_REPO:-}}"
      : "''${APP_BACKUP_REPO:?APP_BACKUP_REPO not set in config}"
      : "''${AGE_RECIPIENT:?AGE_RECIPIENT not set in config}"
      : "''${LOCAL_REPO_PATH:=$HOME/.local/share/app-backup}"

      # Parse arguments
      FORCE=false
      PUSH=false
      while [[ $# -gt 0 ]]; do
        case $1 in
          --force|-f) FORCE=true; shift ;;
          --push|-p) PUSH=true; shift ;;
          --help|-h)
            echo "Usage: app-backup [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --force, -f    Force backup even if apps are running"
            echo "  --push, -p     Push encrypted archives to GitHub after backup"
            echo "  --help, -h     Show this help"
            echo ""
            echo "This backs up ONLY essential files for login restoration:"
            echo "  Chrome: cookies, login data, sessions, preferences"
            echo "  Firefox: cookies, logins, sessions, sync data"
            exit 0
            ;;
          *) log_error "Unknown option: $1" ;;
        esac
      done

      # Check if apps are running
      check_apps() {
        local running=""
        pgrep -x "chrome" >/dev/null 2>&1 && running="Chrome"
        pgrep -x "firefox" >/dev/null 2>&1 && running="''${running:+$running, }Firefox"
        pgrep -x "firefox-bin" >/dev/null 2>&1 && running="''${running:+$running, }Firefox"
        pgrep -x ".firefox-wrapped" >/dev/null 2>&1 && running="''${running:+$running, }Firefox"
        if [[ -n "$running" ]]; then
          if [[ "$FORCE" == "true" ]]; then
            log_warn "Apps running ($running) - continuing with --force"
          else
            log_error "Apps are running: $running. Close them or use --force"
          fi
        fi
      }

      # Export Chrome Safe Storage key from GNOME Keyring
      # This key is used to encrypt/decrypt cookies and saved passwords
      export_chrome_key() {
        local keyfile="$1"
        log_info "Exporting Chrome Safe Storage key..."

        # Try to get the Chrome Safe Storage key
        local key
        key=$(secret-tool search --all xdg:schema chrome_libsecret_os_crypt_password_v2 application chrome 2>/dev/null | grep "^secret = " | head -1 | cut -d' ' -f3) || true

        if [[ -z "$key" ]]; then
          log_warn "Chrome Safe Storage key not found in keyring"
          return 1
        fi

        # Save key to file (will be included in encrypted archive)
        echo "$key" > "$keyfile"
        log_success "Chrome Safe Storage key exported"
        return 0
      }

      # Create Chrome archive with only essential files
      backup_chrome() {
        local chrome_dir="$HOME/.config/google-chrome"
        local archive="$TEMP_DIR/chrome-profile.tar.gz"
        local filelist="$TEMP_DIR/chrome-files.txt"
        local staging_dir="$TEMP_DIR/chrome-staging"

        if [[ ! -d "$chrome_dir" ]]; then
          log_warn "Chrome directory not found: $chrome_dir"
          return 1
        fi

        log_info "Backing up Chrome essential files..."

        # Create staging directory for archive contents
        mkdir -p "$staging_dir"

        # Build list of files that exist and copy to staging
        local count=0
        cd "$chrome_dir"
        for f in ${lib.concatMapStringsSep " " (f: ''"${f}"'') chromeEssentialFiles}; do
          if [[ -f "$f" ]]; then
            mkdir -p "$staging_dir/$(dirname "$f")"
            cp "$f" "$staging_dir/$f"
            ((count++)) || true
          fi
        done

        if [[ $count -eq 0 ]]; then
          log_warn "No Chrome files found to backup"
          return 1
        fi

        log_info "Found $count essential Chrome files"

        # Export Chrome Safe Storage key and add to staging
        if export_chrome_key "$staging_dir/.chrome-safe-storage-key"; then
          ((count++)) || true
        fi

        # Create archive from staging directory
        tar --create --gzip --file="$archive" \
          --directory="$staging_dir" \
          --sort=name \
          --mtime='2024-01-01' \
          .

        # Clean up staging
        rm -rf "$staging_dir"

        local size
        size=$(du -h "$archive" | cut -f1)
        log_success "Chrome archive: $size"
        return 0
      }

      # Create Firefox archive with only essential files
      backup_firefox() {
        local firefox_dir="$HOME/.mozilla/firefox"
        local archive="$TEMP_DIR/firefox-profile.tar.gz"
        local filelist="$TEMP_DIR/firefox-files.txt"

        if [[ ! -d "$firefox_dir" ]]; then
          log_warn "Firefox directory not found: $firefox_dir"
          return 1
        fi

        log_info "Backing up Firefox essential files..."

        # Find files matching essential patterns
        cd "$firefox_dir"
        : > "$filelist"
        for pattern in ${lib.concatMapStringsSep " " (p: ''"${p}"'') firefoxEssentialPatterns}; do
          # Use find with -path for glob matching
          find . -path "./$pattern" -type f 2>/dev/null | sed 's|^\./||' >> "$filelist" || true
        done

        if [[ ! -s "$filelist" ]]; then
          log_warn "No Firefox files found to backup"
          return 1
        fi

        local count
        count=$(wc -l < "$filelist")
        log_info "Found $count essential Firefox files"

        # Create archive from file list
        tar --create --gzip --file="$archive" \
          --directory="$firefox_dir" \
          --files-from="$filelist" \
          --sort=name \
          --mtime='2024-01-01'

        local size
        size=$(du -h "$archive" | cut -f1)
        log_success "Firefox archive: $size"
        return 0
      }

      # Encrypt with age (only needs public key - no 1Password needed)
      encrypt_archive() {
        local src="$1" dst="$2"
        log_info "Encrypting: $(basename "$src")"
        age --encrypt --recipient "$AGE_RECIPIENT" --output "$dst" "$src"
        # Securely remove unencrypted archive
        shred -u "$src" 2>/dev/null || rm -f "$src"
        local size
        size=$(du -h "$dst" | cut -f1)
        log_success "Encrypted: $(basename "$dst") ($size)"
      }

      # Check if LFS is needed and set up
      setup_lfs_if_needed() {
        local file="$1"
        local size
        size=$(stat -c%s "$file" 2>/dev/null || echo 0)
        if [[ $size -gt 104857600 ]]; then  # 100MB
          log_info "File exceeds 100MB, setting up Git LFS..."
          if ! git lfs env &>/dev/null; then
            git lfs install
          fi
          if ! grep -qF '*.age filter=lfs' .gitattributes 2>/dev/null; then
            echo '*.age filter=lfs diff=lfs merge=lfs -text' >> .gitattributes
            git add .gitattributes
            log_success "Git LFS configured for .age files"
          fi
        fi
      }

      # Main
      log_info "App Profile Backup (Essential Files Only)"
      echo ""

      check_apps

      # Create secure temp directory
      TEMP_DIR=$(mktemp -d)
      chmod 700 "$TEMP_DIR"
      trap 'rm -rf "$TEMP_DIR"' EXIT INT TERM

      # Chrome backup
      if backup_chrome; then
        encrypt_archive "$TEMP_DIR/chrome-profile.tar.gz" "$TEMP_DIR/chrome-profile.tar.gz.age"
      fi

      # Firefox backup
      if backup_firefox; then
        encrypt_archive "$TEMP_DIR/firefox-profile.tar.gz" "$TEMP_DIR/firefox-profile.tar.gz.age"
      fi

      # Keys backup (encrypted with age key, same as app profiles)
      backup_keys() {
        local staging_dir="$TEMP_DIR/keys-staging"
        mkdir -p "$staging_dir"
        local count=0

        # Add Age key if it exists locally
        if [[ -n "''${AGE_KEY_PATH:-}" ]]; then
          local age_key_path="''${AGE_KEY_PATH/#\~/$HOME}"
          if [[ -f "$age_key_path" ]]; then
            cp "$age_key_path" "$staging_dir/age-key.txt"
            ((count++)) || true
          fi
        fi

        # Add SSH key if it exists locally
        if [[ -n "''${SSH_KEY_PATH:-}" ]]; then
          local ssh_key_path="''${SSH_KEY_PATH/#\~/$HOME}"
          if [[ -f "$ssh_key_path" ]]; then
            cp "$ssh_key_path" "$staging_dir/id_ed25519"
            ((count++)) || true
            if [[ -f "''${ssh_key_path}.pub" ]]; then
              cp "''${ssh_key_path}.pub" "$staging_dir/id_ed25519.pub"
            fi
          fi
        fi

        if [[ $count -eq 0 ]]; then
          log_warn "No local keys found to backup"
          return 1
        fi

        log_info "Backing up $count key(s)..."
        local archive="$TEMP_DIR/keys.tar.gz"
        tar --create --gzip --file="$archive" \
          --directory="$staging_dir" \
          --sort=name \
          --mtime='2024-01-01' \
          .

        # Encrypt with age key (same as app profiles)
        encrypt_archive "$archive" "$TEMP_DIR/keys.tar.gz.age"

        # Cleanup
        rm -rf "$staging_dir"

        log_success "Keys backed up"
        return 0
      }

      # Backup keys if they exist locally
      backup_keys || true

      # Backup configs (secrets.nix, vpn config, app-backup config)
      backup_configs() {
        local staging_dir="$TEMP_DIR/configs-staging"
        mkdir -p "$staging_dir/configs"
        local count=0

        # Add secrets.nix (contains VPN certs, IPs, 1Password paths)
        local secrets_path="/home/arnold/nixos-config/home/secrets.nix"
        if [[ -f "$secrets_path" ]]; then
          cp "$secrets_path" "$staging_dir/configs/secrets.nix"
          ((count++)) || true
          log_info "Added secrets.nix"
        fi

        # Add VPN config
        local vpn_config="$HOME/.config/vpn/config"
        if [[ -f "$vpn_config" ]]; then
          cp "$vpn_config" "$staging_dir/configs/vpn-config"
          ((count++)) || true
          log_info "Added vpn-config"
        fi

        # Add app-backup config
        local app_config="$HOME/.config/app-backup/config"
        if [[ -f "$app_config" ]]; then
          cp "$app_config" "$staging_dir/configs/app-backup-config"
          ((count++)) || true
          log_info "Added app-backup-config"
        fi

        if [[ $count -eq 0 ]]; then
          log_warn "No config files found to backup"
          return 1
        fi

        log_info "Backing up $count config file(s)..."
        local archive="$TEMP_DIR/configs.tar.gz"
        tar --create --gzip --file="$archive" \
          --directory="$staging_dir" \
          --sort=name \
          --mtime='2024-01-01' \
          configs

        encrypt_archive "$archive" "$TEMP_DIR/configs.tar.gz.age"
        rm -rf "$staging_dir"

        log_success "Configs backed up"
        return 0
      }

      # Backup configs
      backup_configs || true

      # Push to GitHub if requested
      if [[ "$PUSH" == "true" ]]; then
        echo ""
        log_info "Pushing to GitHub..."

        # Clone or update repo
        LOCAL_REPO_PATH="''${LOCAL_REPO_PATH/#\~/$HOME}"
        mkdir -p "$(dirname "$LOCAL_REPO_PATH")"
        if [[ ! -d "$LOCAL_REPO_PATH/.git" ]]; then
          log_info "Cloning repository..."
          git clone "$APP_BACKUP_REPO" "$LOCAL_REPO_PATH"
        else
          log_info "Updating repository..."
          # Reset any uncommitted changes from interrupted backups
          git -C "$LOCAL_REPO_PATH" reset --hard HEAD
          git -C "$LOCAL_REPO_PATH" pull --rebase
        fi

        # Copy encrypted files
        cd "$LOCAL_REPO_PATH"
        for age_file in "$TEMP_DIR"/*.age; do
          if [[ -f "$age_file" ]]; then
            cp "$age_file" .
            setup_lfs_if_needed "$(basename "$age_file")"
          fi
        done

        # Commit and push
        git add -A
        if git diff --staged --quiet; then
          log_info "No changes to commit"
        else
          git commit -m "Backup $(date +%Y-%m-%d\ %H:%M)"
          git push
          log_success "Pushed to GitHub"
        fi
      fi

      echo ""
      log_success "Backup complete!"
      if [[ "$PUSH" != "true" ]]; then
        log_info "Encrypted files in: $TEMP_DIR"
        log_info "Use --push to upload to GitHub"
        # Don't clean up temp dir if user might want the files
        trap - EXIT INT TERM
      fi
    '';
  };

  # The app-restore script
  # NOTE: Do NOT include _1password-cli in runtimeInputs - we need the system wrapper
  # at /run/wrappers/bin/op which has permissions to communicate with the desktop app
  app-restore = pkgs.writeShellApplication {
    name = "app-restore";
    runtimeInputs = with pkgs; [ coreutils gnutar gzip age git git-lfs libsecret ];
    text = ''
      set -euo pipefail

      # Colors
      RED='\033[0;31m'
      GREEN='\033[0;32m'
      YELLOW='\033[1;33m'
      BLUE='\033[0;34m'
      NC='\033[0m'

      log_info() { echo -e "''${BLUE}[INFO]''${NC} $1"; }
      log_success() { echo -e "''${GREEN}[SUCCESS]''${NC} $1"; }
      log_warn() { echo -e "''${YELLOW}[WARN]''${NC} $1"; }
      log_error() { echo -e "''${RED}[ERROR]''${NC} $1"; exit 1; }

      # Load configuration (check new path first, then legacy)
      CONFIG_FILE="$HOME/.config/app-backup/config"
      if [[ ! -f "$CONFIG_FILE" ]]; then
        CONFIG_FILE="$HOME/.config/browser-backup/config"
      fi
      if [[ ! -f "$CONFIG_FILE" ]]; then
        log_error "Config file not found. Run: nixos-rebuild switch"
      fi
      # shellcheck source=/dev/null
      source "$CONFIG_FILE"

      # Validate required config (support both old and new variable names)
      APP_BACKUP_REPO="''${APP_BACKUP_REPO:-''${BROWSER_BACKUP_REPO:-}}"
      : "''${APP_BACKUP_REPO:?APP_BACKUP_REPO not set in config}"
      : "''${LOCAL_REPO_PATH:=$HOME/.local/share/app-backup}"
      : "''${BACKUP_RETENTION:=3}"

      # Validate that at least one key source is configured
      if [[ -z "''${AGE_KEY_1PASSWORD:-}" && -z "''${AGE_KEY_PATH:-}" ]]; then
        log_error "Neither AGE_KEY_1PASSWORD nor AGE_KEY_PATH is set in config"
      fi

      # Parse arguments
      FORCE=false
      PULL=false
      while [[ $# -gt 0 ]]; do
        case $1 in
          --force|-f) FORCE=true; shift ;;
          --pull|-p) PULL=true; shift ;;
          --help|-h)
            echo "Usage: app-restore [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --force, -f    Force restore even if apps are running"
            echo "  --pull, -p     Pull latest from GitHub before restoring"
            echo "  --help, -h     Show this help"
            echo ""
            echo "This restores essential files for login restoration:"
            echo "  Chrome: cookies, login data, sessions, preferences"
            echo "  Firefox: cookies, logins, sessions, sync data"
            echo ""
            echo "Files are merged into existing app profiles."
            exit 0
            ;;
          *) log_error "Unknown option: $1" ;;
        esac
      done

      # Check if apps are running
      check_apps() {
        local running=""
        pgrep -x "chrome" >/dev/null 2>&1 && running="Chrome"
        pgrep -x "firefox" >/dev/null 2>&1 && running="''${running:+$running, }Firefox"
        pgrep -x "firefox-bin" >/dev/null 2>&1 && running="''${running:+$running, }Firefox"
        pgrep -x ".firefox-wrapped" >/dev/null 2>&1 && running="''${running:+$running, }Firefox"
        if [[ -n "$running" ]]; then
          if [[ "$FORCE" == "true" ]]; then
            log_warn "Apps running ($running) - continuing with --force"
          else
            log_error "Apps are running: $running. Close them or use --force"
          fi
        fi
      }

      # Get age key (local file first, then 1Password fallback)
      # IMPORTANT: This function outputs ONLY the key to stdout (for piping to age)
      # All other messages must go to stderr
      get_age_key() {
        # Priority 1: Local file
        if [[ -n "''${AGE_KEY_PATH:-}" ]]; then
          local key_path="''${AGE_KEY_PATH/#\~/$HOME}"
          if [[ -f "$key_path" ]]; then
            cat "$key_path"
            return 0
          fi
        fi

        # Priority 2: 1Password
        if [[ -n "''${AGE_KEY_1PASSWORD:-}" ]]; then
          log_info "Local age key not found, retrieving from 1Password..." >&2
          op read "$AGE_KEY_1PASSWORD"
          return 0
        fi

        log_error "No age key available (checked local file and 1Password)"
      }

      # Backup essential files before overwriting
      backup_essential_files() {
        local target_dir="$1" archive_content_dir="$2"
        local timestamp
        timestamp=$(date +%Y%m%d-%H%M%S)
        local backup_dir="$target_dir.backup-essential.$timestamp"

        if [[ ! -d "$target_dir" ]]; then
          return 0
        fi

        # Only backup files that will be overwritten
        mkdir -p "$backup_dir"
        cd "$archive_content_dir"
        find . -type f | while read -r f; do
          local src="$target_dir/$f"
          local dst="$backup_dir/$f"
          if [[ -f "$src" ]]; then
            mkdir -p "$(dirname "$dst")"
            cp "$src" "$dst"
          fi
        done

        local count
        count=$(find "$backup_dir" -type f 2>/dev/null | wc -l)
        if [[ "$count" -gt 0 ]]; then
          log_info "Backed up $count existing files to: $(basename "$backup_dir")"
        else
          rm -rf "$backup_dir"
        fi
      }

      # Prune old essential backups
      prune_essential_backups() {
        local dir="$1" keep="$2"
        local parent
        parent=$(dirname "$dir")
        local base
        base=$(basename "$dir")

        # Find and delete oldest backups beyond retention
        # shellcheck disable=SC2012
        ls -dt "$parent/$base".backup-essential.* 2>/dev/null | tail -n +$((keep + 1)) | while read -r old_backup; do
          log_info "Pruning old backup: $(basename "$old_backup")"
          rm -rf "$old_backup"
        done
      }

      # Import Chrome Safe Storage key into GNOME Keyring
      # This allows Chrome to decrypt cookies from another machine
      import_chrome_key() {
        local keyfile="$1"

        if [[ ! -f "$keyfile" ]]; then
          log_warn "Chrome Safe Storage key not found in backup"
          return 1
        fi

        local key
        key=$(cat "$keyfile")

        if [[ -z "$key" ]]; then
          log_warn "Chrome Safe Storage key is empty"
          return 1
        fi

        log_info "Importing Chrome Safe Storage key..."

        # Check if key already exists and matches
        local existing_key
        existing_key=$(secret-tool search --all xdg:schema chrome_libsecret_os_crypt_password_v2 application chrome 2>/dev/null | grep "^secret = " | head -1 | cut -d' ' -f3) || true

        if [[ "$existing_key" == "$key" ]]; then
          log_info "Chrome Safe Storage key already matches"
          return 0
        fi

        # Store the key in GNOME Keyring
        # This will overwrite any existing key with the same attributes
        echo -n "$key" | secret-tool store --label="Chrome Safe Storage" \
          xdg:schema chrome_libsecret_os_crypt_password_v2 \
          application chrome

        log_success "Chrome Safe Storage key imported"
        return 0
      }

      # Restore Chrome essential files (merge into existing profile)
      restore_chrome() {
        local age_file="$1"
        local chrome_dir="$HOME/.config/google-chrome"

        if [[ ! -f "$age_file" ]]; then
          log_warn "Chrome backup not found: $age_file"
          return 1
        fi

        log_info "Restoring Chrome essential files..."

        # Decrypt
        local tar_file="$TEMP_DIR/chrome-profile.tar.gz"
        local extract_dir="$TEMP_DIR/chrome-extract"
        mkdir -p "$extract_dir"

        get_age_key | age --decrypt --identity - --output "$tar_file" "$age_file"
        tar --extract --gzip --file="$tar_file" --directory="$extract_dir"

        # Import Chrome Safe Storage key BEFORE restoring files
        # This must happen before Chrome reads the cookies
        if [[ -f "$extract_dir/.chrome-safe-storage-key" ]]; then
          import_chrome_key "$extract_dir/.chrome-safe-storage-key"
          # Remove the key file so it doesn't get copied to Chrome dir
          rm -f "$extract_dir/.chrome-safe-storage-key"
        fi

        # Backup files that will be overwritten
        backup_essential_files "$chrome_dir" "$extract_dir"
        prune_essential_backups "$chrome_dir" "$BACKUP_RETENTION"

        # Ensure target directory exists
        mkdir -p "$chrome_dir/Default"

        # Copy files from archive to chrome dir (merge)
        local file_count=0
        cd "$extract_dir"
        find . -type f | while read -r f; do
          local src="$extract_dir/$f"
          local dst="$chrome_dir/$f"
          mkdir -p "$(dirname "$dst")"
          cp "$src" "$dst"
        done
        file_count=$(find "$extract_dir" -type f 2>/dev/null | wc -l)
        cd /

        # Cleanup
        shred -u "$tar_file" 2>/dev/null || rm -f "$tar_file"
        rm -rf "$extract_dir"

        log_success "Restored $file_count Chrome essential files"
        return 0
      }

      # Restore Firefox essential files (merge into existing profile)
      # Firefox profiles have random names (e.g., abc123.default), so we need to:
      # 1. Find the local profile directory
      # 2. Find the backup's profile directory
      # 3. Copy files from backup profile INTO local profile
      restore_firefox() {
        local age_file="$1"
        local firefox_dir="$HOME/.mozilla/firefox"

        if [[ ! -f "$age_file" ]]; then
          log_warn "Firefox backup not found: $age_file"
          return 1
        fi

        log_info "Restoring Firefox essential files..."

        # Decrypt
        local tar_file="$TEMP_DIR/firefox-profile.tar.gz"
        local extract_dir="$TEMP_DIR/firefox-extract"
        mkdir -p "$extract_dir"

        get_age_key | age --decrypt --identity - --output "$tar_file" "$age_file"
        tar --extract --gzip --file="$tar_file" --directory="$extract_dir"

        # Find the backup's profile directory (*.default* pattern)
        local backup_profile=""
        backup_profile=$(find "$extract_dir" -maxdepth 1 -type d -name "*.default*" | head -1)

        if [[ -z "$backup_profile" ]]; then
          log_warn "No Firefox profile found in backup"
          rm -rf "$extract_dir"
          shred -u "$tar_file" 2>/dev/null || rm -f "$tar_file"
          return 1
        fi
        backup_profile=$(basename "$backup_profile")
        log_info "Backup profile: $backup_profile"

        # Find the local profile directory
        # First try to get it from profiles.ini, then fall back to finding *.default*
        local local_profile=""
        if [[ -f "$firefox_dir/profiles.ini" ]]; then
          local_profile=$(grep -E "^Path=" "$firefox_dir/profiles.ini" | head -1 | cut -d= -f2)
        fi
        if [[ -z "$local_profile" ]] || [[ ! -d "$firefox_dir/$local_profile" ]]; then
          local_profile=$(find "$firefox_dir" -maxdepth 1 -type d -name "*.default*" | head -1)
          if [[ -n "$local_profile" ]]; then
            local_profile=$(basename "$local_profile")
          fi
        fi

        if [[ -z "$local_profile" ]]; then
          log_warn "No local Firefox profile found, creating from backup"
          local_profile="$backup_profile"
        fi
        log_info "Local profile: $local_profile"

        # Ensure local profile directory exists
        mkdir -p "$firefox_dir/$local_profile"

        # Copy essential files from backup profile to local profile
        # Skip profiles.ini and installs.ini to preserve local profile config
        local file_count=0
        if [[ -d "$extract_dir/$backup_profile" ]]; then
          cd "$extract_dir/$backup_profile"
          for f in *; do
            if [[ -f "$f" ]]; then
              # Backup existing file if present
              if [[ -f "$firefox_dir/$local_profile/$f" ]]; then
                cp "$firefox_dir/$local_profile/$f" "$firefox_dir/$local_profile/$f.bak" 2>/dev/null || true
              fi
              cp "$f" "$firefox_dir/$local_profile/$f"
              ((file_count++)) || true
            fi
          done
          cd /
        fi

        # Cleanup
        shred -u "$tar_file" 2>/dev/null || rm -f "$tar_file"
        rm -rf "$extract_dir"

        log_success "Restored $file_count Firefox essential files to $local_profile"
        return 0
      }

      # Main
      log_info "App Profile Restore (Essential Files Only)"
      echo ""

      check_apps

      # Pull from GitHub if requested
      LOCAL_REPO_PATH="''${LOCAL_REPO_PATH/#\~/$HOME}"
      if [[ "$PULL" == "true" ]]; then
        log_info "Pulling from GitHub..."
        mkdir -p "$(dirname "$LOCAL_REPO_PATH")"
        if [[ ! -d "$LOCAL_REPO_PATH/.git" ]]; then
          log_info "Cloning repository..."
          git clone "$APP_BACKUP_REPO" "$LOCAL_REPO_PATH"
        else
          log_info "Updating repository..."
          # Reset any uncommitted changes from interrupted backups
          git -C "$LOCAL_REPO_PATH" reset --hard HEAD
          git -C "$LOCAL_REPO_PATH" pull --rebase
        fi
        echo ""
      fi

      if [[ ! -d "$LOCAL_REPO_PATH" ]]; then
        log_error "Local repo not found: $LOCAL_REPO_PATH. Use --pull to clone."
      fi

      # Create secure temp directory
      TEMP_DIR=$(mktemp -d)
      chmod 700 "$TEMP_DIR"
      trap 'rm -rf "$TEMP_DIR"' EXIT INT TERM

      # Restore keys FIRST (so age key is available for app restores)
      restore_keys() {
        local age_file="$LOCAL_REPO_PATH/keys.tar.gz.age"

        if [[ ! -f "$age_file" ]]; then
          log_info "No keys backup found - skipping keys restore"
          return 0
        fi

        log_info "Restoring keys..."

        # Decrypt using age key (local file first, then 1Password)
        local tar_file="$TEMP_DIR/keys.tar.gz"
        local extract_dir="$TEMP_DIR/keys-extract"
        mkdir -p "$extract_dir"

        get_age_key | age --decrypt --identity - --output "$tar_file" "$age_file" || {
          log_warn "Failed to decrypt keys backup - continuing without keys restore"
          return 1
        }

        tar --extract --gzip --file="$tar_file" --directory="$extract_dir"

        local count=0

        # Restore Age key (only if not already present, unless --force)
        if [[ -f "$extract_dir/age-key.txt" && -n "''${AGE_KEY_PATH:-}" ]]; then
          local age_key_path="''${AGE_KEY_PATH/#\~/$HOME}"
          if [[ -f "$age_key_path" && "$FORCE" != "true" ]]; then
            log_info "Age key already exists - skipping"
          else
            mkdir -p "$(dirname "$age_key_path")"
            cp "$extract_dir/age-key.txt" "$age_key_path"
            chmod 600 "$age_key_path"
            log_success "Restored Age key to: $age_key_path"
            ((count++)) || true
          fi
        fi

        # Restore SSH key (only if not already present, unless --force)
        if [[ -f "$extract_dir/id_ed25519" && -n "''${SSH_KEY_PATH:-}" ]]; then
          local ssh_key_path="''${SSH_KEY_PATH/#\~/$HOME}"
          if [[ -f "$ssh_key_path" && "$FORCE" != "true" ]]; then
            log_info "SSH key already exists - skipping"
          else
            mkdir -p "$(dirname "$ssh_key_path")"
            cp "$extract_dir/id_ed25519" "$ssh_key_path"
            chmod 600 "$ssh_key_path"
            log_success "Restored SSH key to: $ssh_key_path"
            ((count++)) || true
            if [[ -f "$extract_dir/id_ed25519.pub" ]]; then
              cp "$extract_dir/id_ed25519.pub" "''${ssh_key_path}.pub"
              chmod 644 "''${ssh_key_path}.pub"
            fi
          fi
        fi

        # Cleanup
        shred -u "$tar_file" 2>/dev/null || rm -f "$tar_file"
        rm -rf "$extract_dir"

        if [[ $count -gt 0 ]]; then
          log_success "Restored $count key(s)"
        fi
        return 0
      }

      # Restore configs (secrets.nix, vpn config, app-backup config)
      restore_configs() {
        local age_file="$LOCAL_REPO_PATH/configs.tar.gz.age"

        if [[ ! -f "$age_file" ]]; then
          log_info "No configs backup found - skipping configs restore"
          return 0
        fi

        log_info "Restoring configs..."

        local tar_file="$TEMP_DIR/configs.tar.gz"
        local extract_dir="$TEMP_DIR/configs-extract"
        mkdir -p "$extract_dir"

        get_age_key | age --decrypt --identity - --output "$tar_file" "$age_file" || {
          log_warn "Failed to decrypt configs backup"
          return 1
        }

        tar --extract --gzip --file="$tar_file" --directory="$extract_dir"

        local count=0

        # Restore secrets.nix
        if [[ -f "$extract_dir/configs/secrets.nix" ]]; then
          local secrets_path="/home/arnold/nixos-config/home/secrets.nix"
          if [[ -f "$secrets_path" && "$FORCE" != "true" ]]; then
            log_info "secrets.nix already exists - skipping (use --force to overwrite)"
          else
            cp "$extract_dir/configs/secrets.nix" "$secrets_path"
            chmod 600 "$secrets_path"
            log_success "Restored secrets.nix"
            ((count++)) || true
          fi
        fi

        # Restore VPN config
        if [[ -f "$extract_dir/configs/vpn-config" ]]; then
          local vpn_config="$HOME/.config/vpn/config"
          mkdir -p "$(dirname "$vpn_config")"
          # VPN config is a symlink managed by Home Manager, copy to a local override location
          log_info "VPN config found (managed by Home Manager - rebuild to apply)"
          ((count++)) || true
        fi

        # Restore app-backup config
        if [[ -f "$extract_dir/configs/app-backup-config" ]]; then
          local app_config="$HOME/.config/app-backup/config"
          if [[ -f "$app_config" && "$FORCE" != "true" ]]; then
            log_info "app-backup config already exists - skipping"
          else
            mkdir -p "$(dirname "$app_config")"
            cp "$extract_dir/configs/app-backup-config" "$app_config"
            chmod 600 "$app_config"
            log_success "Restored app-backup config"
            ((count++)) || true
          fi
        fi

        # Cleanup
        shred -u "$tar_file" 2>/dev/null || rm -f "$tar_file"
        rm -rf "$extract_dir"

        if [[ $count -gt 0 ]]; then
          log_success "Restored $count config file(s)"
          log_info "Run 'sudo nixos-rebuild switch --flake . --impure' to apply secrets.nix"
        fi
        return 0
      }

      # Restore configs first (secrets.nix needed for rebuild)
      restore_configs || true

      # Restore keys
      restore_keys || true

      # Restore Chrome
      restore_chrome "$LOCAL_REPO_PATH/chrome-profile.tar.gz.age" || true

      # Restore Firefox
      restore_firefox "$LOCAL_REPO_PATH/firefox-profile.tar.gz.age" || true

      echo ""
      log_success "Restore complete!"
      log_info "You can now start your apps."
    '';
  };

  # Backward compatibility wrapper for browser-backup
  browser-backup-compat = pkgs.writeShellScriptBin "browser-backup" ''
    echo -e "\033[1;33m[DEPRECATED]\033[0m browser-backup has been renamed to app-backup" >&2
    exec ${app-backup}/bin/app-backup "$@"
  '';

  # Backward compatibility wrapper for browser-restore
  browser-restore-compat = pkgs.writeShellScriptBin "browser-restore" ''
    echo -e "\033[1;33m[DEPRECATED]\033[0m browser-restore has been renamed to app-restore" >&2
    exec ${app-restore}/bin/app-restore "$@"
  '';

  # Keys setup script - retrieves keys from 1Password and stores locally
  keys-setup = pkgs.writeShellApplication {
    name = "keys-setup";
    runtimeInputs = with pkgs; [ coreutils openssh ];
    text = ''
      set -euo pipefail

      # Colors
      RED='\033[0;31m'
      GREEN='\033[0;32m'
      YELLOW='\033[1;33m'
      BLUE='\033[0;34m'
      NC='\033[0m'

      log_info() { echo -e "''${BLUE}[INFO]''${NC} $1"; }
      log_success() { echo -e "''${GREEN}[SUCCESS]''${NC} $1"; }
      log_warn() { echo -e "''${YELLOW}[WARN]''${NC} $1"; }
      log_error() { echo -e "''${RED}[ERROR]''${NC} $1"; exit 1; }

      # Load configuration
      CONFIG_FILE="$HOME/.config/app-backup/config"
      if [[ ! -f "$CONFIG_FILE" ]]; then
        log_error "Config file not found. Run: nixos-rebuild switch"
      fi
      # shellcheck source=/dev/null
      source "$CONFIG_FILE"

      log_info "Key Setup - Retrieving keys from 1Password"
      echo ""

      # Check 1Password CLI is available
      if ! command -v op &>/dev/null; then
        log_error "1Password CLI (op) not found"
      fi

      # Check 1Password is signed in
      if ! op account list &>/dev/null; then
        log_error "Not signed in to 1Password. Open 1Password app and sign in."
      fi

      KEYS_CREATED=0

      # Setup Age key
      if [[ -n "''${AGE_KEY_1PASSWORD:-}" && -n "''${AGE_KEY_PATH:-}" ]]; then
        AGE_KEY_PATH_EXPANDED="''${AGE_KEY_PATH/#\~/$HOME}"

        if [[ -f "$AGE_KEY_PATH_EXPANDED" ]]; then
          log_info "Age key already exists at: $AGE_KEY_PATH_EXPANDED"
          read -rp "Overwrite? [y/N] " REPLY
          if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            log_info "Skipping Age key"
          else
            log_info "Retrieving Age key from 1Password..."
            mkdir -p "$(dirname "$AGE_KEY_PATH_EXPANDED")"
            op read "$AGE_KEY_1PASSWORD" > "$AGE_KEY_PATH_EXPANDED"
            chmod 600 "$AGE_KEY_PATH_EXPANDED"
            log_success "Age key saved to: $AGE_KEY_PATH_EXPANDED"
            ((KEYS_CREATED++)) || true
          fi
        else
          log_info "Retrieving Age key from 1Password..."
          mkdir -p "$(dirname "$AGE_KEY_PATH_EXPANDED")"
          op read "$AGE_KEY_1PASSWORD" > "$AGE_KEY_PATH_EXPANDED"
          chmod 600 "$AGE_KEY_PATH_EXPANDED"
          log_success "Age key saved to: $AGE_KEY_PATH_EXPANDED"
          ((KEYS_CREATED++)) || true
        fi
      else
        log_warn "Age key setup not configured (need both AGE_KEY_1PASSWORD and AGE_KEY_PATH)"
      fi

      # Setup SSH key
      if [[ -n "''${SSH_KEY_1PASSWORD:-}" && -n "''${SSH_KEY_PATH:-}" ]]; then
        SSH_KEY_PATH_EXPANDED="''${SSH_KEY_PATH/#\~/$HOME}"

        if [[ -f "$SSH_KEY_PATH_EXPANDED" ]]; then
          log_info "SSH key already exists at: $SSH_KEY_PATH_EXPANDED"
          read -rp "Overwrite? [y/N] " REPLY
          if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            log_info "Skipping SSH key"
          else
            log_info "Retrieving SSH key from 1Password..."
            mkdir -p "$(dirname "$SSH_KEY_PATH_EXPANDED")"
            op read "$SSH_KEY_1PASSWORD" > "$SSH_KEY_PATH_EXPANDED"
            chmod 600 "$SSH_KEY_PATH_EXPANDED"
            # Generate public key from private key
            ssh-keygen -y -f "$SSH_KEY_PATH_EXPANDED" > "''${SSH_KEY_PATH_EXPANDED}.pub"
            chmod 644 "''${SSH_KEY_PATH_EXPANDED}.pub"
            log_success "SSH key saved to: $SSH_KEY_PATH_EXPANDED"
            log_success "Public key saved to: ''${SSH_KEY_PATH_EXPANDED}.pub"
            ((KEYS_CREATED++)) || true
          fi
        else
          log_info "Retrieving SSH key from 1Password..."
          mkdir -p "$(dirname "$SSH_KEY_PATH_EXPANDED")"
          op read "$SSH_KEY_1PASSWORD" > "$SSH_KEY_PATH_EXPANDED"
          chmod 600 "$SSH_KEY_PATH_EXPANDED"
          # Generate public key from private key
          ssh-keygen -y -f "$SSH_KEY_PATH_EXPANDED" > "''${SSH_KEY_PATH_EXPANDED}.pub"
          chmod 644 "''${SSH_KEY_PATH_EXPANDED}.pub"
          log_success "SSH key saved to: $SSH_KEY_PATH_EXPANDED"
          log_success "Public key saved to: ''${SSH_KEY_PATH_EXPANDED}.pub"
          ((KEYS_CREATED++)) || true
        fi
      else
        log_warn "SSH key setup not configured (need both SSH_KEY_1PASSWORD and SSH_KEY_PATH)"
      fi

      echo ""
      if [[ $KEYS_CREATED -gt 0 ]]; then
        log_success "Setup complete! $KEYS_CREATED key(s) created."
        log_info "Local keys will now be used for backup/restore operations."
      else
        log_info "No keys were created."
      fi
    '';
  };

  # Keys backup script - backs up keys to passphrase-encrypted archive
  keys-backup = pkgs.writeShellApplication {
    name = "keys-backup";
    runtimeInputs = with pkgs; [ coreutils gnutar gzip age git git-lfs ];
    text = ''
      set -euo pipefail

      # Colors
      RED='\033[0;31m'
      GREEN='\033[0;32m'
      YELLOW='\033[1;33m'
      BLUE='\033[0;34m'
      NC='\033[0m'

      log_info() { echo -e "''${BLUE}[INFO]''${NC} $1"; }
      log_success() { echo -e "''${GREEN}[SUCCESS]''${NC} $1"; }
      log_warn() { echo -e "''${YELLOW}[WARN]''${NC} $1"; }
      log_error() { echo -e "''${RED}[ERROR]''${NC} $1"; exit 1; }

      # Load configuration
      CONFIG_FILE="$HOME/.config/app-backup/config"
      if [[ ! -f "$CONFIG_FILE" ]]; then
        log_error "Config file not found. Run: nixos-rebuild switch"
      fi
      # shellcheck source=/dev/null
      source "$CONFIG_FILE"

      # Parse arguments
      PUSH=false
      while [[ $# -gt 0 ]]; do
        case $1 in
          --push|-p) PUSH=true; shift ;;
          --help|-h)
            echo "Usage: keys-backup [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --push, -p     Push encrypted archive to GitHub after backup"
            echo "  --help, -h     Show this help"
            exit 0
            ;;
          *) log_error "Unknown option: $1" ;;
        esac
      done

      log_info "Keys Backup"
      echo ""

      # Create secure temp directory
      TEMP_DIR=$(mktemp -d)
      chmod 700 "$TEMP_DIR"
      trap 'rm -rf "$TEMP_DIR"' EXIT INT TERM

      STAGING_DIR="$TEMP_DIR/keys-staging"
      mkdir -p "$STAGING_DIR"

      COUNT=0

      # Add Age key if it exists locally
      if [[ -n "''${AGE_KEY_PATH:-}" ]]; then
        AGE_KEY_PATH_EXPANDED="''${AGE_KEY_PATH/#\~/$HOME}"
        if [[ -f "$AGE_KEY_PATH_EXPANDED" ]]; then
          cp "$AGE_KEY_PATH_EXPANDED" "$STAGING_DIR/age-key.txt"
          ((COUNT++)) || true
          log_info "Added Age key to backup"
        fi
      fi

      # Add SSH key if it exists locally
      if [[ -n "''${SSH_KEY_PATH:-}" ]]; then
        SSH_KEY_PATH_EXPANDED="''${SSH_KEY_PATH/#\~/$HOME}"
        if [[ -f "$SSH_KEY_PATH_EXPANDED" ]]; then
          cp "$SSH_KEY_PATH_EXPANDED" "$STAGING_DIR/id_ed25519"
          ((COUNT++)) || true
          log_info "Added SSH private key to backup"
          if [[ -f "''${SSH_KEY_PATH_EXPANDED}.pub" ]]; then
            cp "''${SSH_KEY_PATH_EXPANDED}.pub" "$STAGING_DIR/id_ed25519.pub"
            log_info "Added SSH public key to backup"
          fi
        fi
      fi

      if [[ $COUNT -eq 0 ]]; then
        log_error "No keys found to backup. Run 'keys-setup' first."
      fi

      log_info "Creating keys archive..."
      ARCHIVE="$TEMP_DIR/keys.tar.gz"
      tar --create --gzip --file="$ARCHIVE" \
        --directory="$STAGING_DIR" \
        --sort=name \
        --mtime='2024-01-01' \
        .

      # Get passphrase for encryption
      PASSPHRASE=""
      if [[ -n "''${KEYS_PASSPHRASE_1PASSWORD:-}" ]]; then
        log_info "Retrieving keys passphrase from 1Password..."
        PASSPHRASE=$(op read "$KEYS_PASSPHRASE_1PASSWORD")
      else
        read -rsp "Enter passphrase for keys backup: " PASSPHRASE
        echo ""
        read -rsp "Confirm passphrase: " PASSPHRASE2
        echo ""
        if [[ "$PASSPHRASE" != "$PASSPHRASE2" ]]; then
          log_error "Passphrases do not match"
        fi
      fi

      # Encrypt with passphrase
      log_info "Encrypting keys archive..."
      ENCRYPTED="$TEMP_DIR/keys.tar.gz.age"
      echo "$PASSPHRASE" | age --encrypt --passphrase --output "$ENCRYPTED" "$ARCHIVE"

      # Cleanup unencrypted archive
      shred -u "$ARCHIVE" 2>/dev/null || rm -f "$ARCHIVE"
      rm -rf "$STAGING_DIR"

      SIZE=$(du -h "$ENCRYPTED" | cut -f1)
      log_success "Keys archive created: $SIZE"

      # Push to GitHub if requested
      if [[ "$PUSH" == "true" ]]; then
        echo ""
        log_info "Pushing to GitHub..."

        LOCAL_REPO_PATH="''${LOCAL_REPO_PATH/#\~/$HOME}"
        : "''${LOCAL_REPO_PATH:=$HOME/.local/share/app-backup}"
        mkdir -p "$(dirname "$LOCAL_REPO_PATH")"

        if [[ ! -d "$LOCAL_REPO_PATH/.git" ]]; then
          log_info "Cloning repository..."
          git clone "$APP_BACKUP_REPO" "$LOCAL_REPO_PATH"
        else
          log_info "Updating repository..."
          git -C "$LOCAL_REPO_PATH" reset --hard HEAD
          git -C "$LOCAL_REPO_PATH" pull --rebase
        fi

        # Copy encrypted file
        cp "$ENCRYPTED" "$LOCAL_REPO_PATH/"

        # Commit and push
        cd "$LOCAL_REPO_PATH"
        git add -A
        if git diff --staged --quiet; then
          log_info "No changes to commit"
        else
          git commit -m "Keys backup $(date +%Y-%m-%d\ %H:%M)"
          git push
          log_success "Pushed to GitHub"
        fi
      else
        log_info "Encrypted file: $ENCRYPTED"
        log_info "Use --push to upload to GitHub"
        # Keep temp dir if user might want the file
        trap - EXIT INT TERM
      fi

      echo ""
      log_success "Keys backup complete!"
    '';
  };

  # Keys restore script - restores keys from passphrase-encrypted archive
  keys-restore = pkgs.writeShellApplication {
    name = "keys-restore";
    runtimeInputs = with pkgs; [ coreutils gnutar gzip age git git-lfs openssh ];
    text = ''
      set -euo pipefail

      # Colors
      RED='\033[0;31m'
      GREEN='\033[0;32m'
      YELLOW='\033[1;33m'
      BLUE='\033[0;34m'
      NC='\033[0m'

      log_info() { echo -e "''${BLUE}[INFO]''${NC} $1"; }
      log_success() { echo -e "''${GREEN}[SUCCESS]''${NC} $1"; }
      log_warn() { echo -e "''${YELLOW}[WARN]''${NC} $1"; }
      log_error() { echo -e "''${RED}[ERROR]''${NC} $1"; exit 1; }

      # Load configuration
      CONFIG_FILE="$HOME/.config/app-backup/config"
      if [[ ! -f "$CONFIG_FILE" ]]; then
        log_error "Config file not found. Run: nixos-rebuild switch"
      fi
      # shellcheck source=/dev/null
      source "$CONFIG_FILE"

      # Parse arguments
      PULL=false
      FORCE=false
      while [[ $# -gt 0 ]]; do
        case $1 in
          --pull|-p) PULL=true; shift ;;
          --force|-f) FORCE=true; shift ;;
          --help|-h)
            echo "Usage: keys-restore [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --pull, -p     Pull latest from GitHub before restoring"
            echo "  --force, -f    Force overwrite of existing keys"
            echo "  --help, -h     Show this help"
            exit 0
            ;;
          *) log_error "Unknown option: $1" ;;
        esac
      done

      log_info "Keys Restore"
      echo ""

      # Pull from GitHub if requested
      LOCAL_REPO_PATH="''${LOCAL_REPO_PATH/#\~/$HOME}"
      : "''${LOCAL_REPO_PATH:=$HOME/.local/share/app-backup}"

      if [[ "$PULL" == "true" ]]; then
        log_info "Pulling from GitHub..."
        mkdir -p "$(dirname "$LOCAL_REPO_PATH")"
        if [[ ! -d "$LOCAL_REPO_PATH/.git" ]]; then
          log_info "Cloning repository..."
          git clone "$APP_BACKUP_REPO" "$LOCAL_REPO_PATH"
        else
          log_info "Updating repository..."
          git -C "$LOCAL_REPO_PATH" reset --hard HEAD
          git -C "$LOCAL_REPO_PATH" pull --rebase
        fi
        echo ""
      fi

      AGE_FILE="$LOCAL_REPO_PATH/keys.tar.gz.age"
      if [[ ! -f "$AGE_FILE" ]]; then
        log_error "Keys backup not found: $AGE_FILE"
      fi

      # Get passphrase for decryption
      PASSPHRASE=""
      if [[ -n "''${KEYS_PASSPHRASE_1PASSWORD:-}" ]]; then
        log_info "Retrieving keys passphrase from 1Password..."
        PASSPHRASE=$(op read "$KEYS_PASSPHRASE_1PASSWORD")
      else
        read -rsp "Enter passphrase for keys backup: " PASSPHRASE
        echo ""
      fi

      # Create secure temp directory
      TEMP_DIR=$(mktemp -d)
      chmod 700 "$TEMP_DIR"
      trap 'rm -rf "$TEMP_DIR"' EXIT INT TERM

      # Decrypt
      log_info "Decrypting keys archive..."
      TAR_FILE="$TEMP_DIR/keys.tar.gz"
      echo "$PASSPHRASE" | age --decrypt --passphrase --output "$TAR_FILE" "$AGE_FILE" || {
        log_error "Failed to decrypt keys backup (wrong passphrase?)"
      }

      EXTRACT_DIR="$TEMP_DIR/keys-extract"
      mkdir -p "$EXTRACT_DIR"
      tar --extract --gzip --file="$TAR_FILE" --directory="$EXTRACT_DIR"

      COUNT=0

      # Restore Age key
      if [[ -f "$EXTRACT_DIR/age-key.txt" && -n "''${AGE_KEY_PATH:-}" ]]; then
        AGE_KEY_PATH_EXPANDED="''${AGE_KEY_PATH/#\~/$HOME}"
        if [[ -f "$AGE_KEY_PATH_EXPANDED" && "$FORCE" != "true" ]]; then
          log_warn "Age key already exists at $AGE_KEY_PATH_EXPANDED - skipping (use --force to overwrite)"
        else
          mkdir -p "$(dirname "$AGE_KEY_PATH_EXPANDED")"
          cp "$EXTRACT_DIR/age-key.txt" "$AGE_KEY_PATH_EXPANDED"
          chmod 600 "$AGE_KEY_PATH_EXPANDED"
          log_success "Restored Age key to: $AGE_KEY_PATH_EXPANDED"
          ((COUNT++)) || true
        fi
      fi

      # Restore SSH key
      if [[ -f "$EXTRACT_DIR/id_ed25519" && -n "''${SSH_KEY_PATH:-}" ]]; then
        SSH_KEY_PATH_EXPANDED="''${SSH_KEY_PATH/#\~/$HOME}"
        if [[ -f "$SSH_KEY_PATH_EXPANDED" && "$FORCE" != "true" ]]; then
          log_warn "SSH key already exists at $SSH_KEY_PATH_EXPANDED - skipping (use --force to overwrite)"
        else
          mkdir -p "$(dirname "$SSH_KEY_PATH_EXPANDED")"
          cp "$EXTRACT_DIR/id_ed25519" "$SSH_KEY_PATH_EXPANDED"
          chmod 600 "$SSH_KEY_PATH_EXPANDED"
          log_success "Restored SSH key to: $SSH_KEY_PATH_EXPANDED"
          ((COUNT++)) || true
          if [[ -f "$EXTRACT_DIR/id_ed25519.pub" ]]; then
            cp "$EXTRACT_DIR/id_ed25519.pub" "''${SSH_KEY_PATH_EXPANDED}.pub"
            chmod 644 "''${SSH_KEY_PATH_EXPANDED}.pub"
            log_success "Restored SSH public key to: ''${SSH_KEY_PATH_EXPANDED}.pub"
          else
            # Generate public key if not in backup
            ssh-keygen -y -f "$SSH_KEY_PATH_EXPANDED" > "''${SSH_KEY_PATH_EXPANDED}.pub"
            chmod 644 "''${SSH_KEY_PATH_EXPANDED}.pub"
            log_success "Generated SSH public key: ''${SSH_KEY_PATH_EXPANDED}.pub"
          fi
        fi
      fi

      # Cleanup
      shred -u "$TAR_FILE" 2>/dev/null || rm -f "$TAR_FILE"
      rm -rf "$EXTRACT_DIR"

      echo ""
      if [[ $COUNT -gt 0 ]]; then
        log_success "Restored $COUNT key(s)"
      else
        log_info "No keys were restored"
      fi
    '';
  };

  # Keys status script - shows status of local keys
  keys-status = pkgs.writeShellApplication {
    name = "keys-status";
    runtimeInputs = with pkgs; [ coreutils ];
    text = ''
      set -euo pipefail

      # Colors
      RED='\033[0;31m'
      GREEN='\033[0;32m'
      YELLOW='\033[1;33m'
      BLUE='\033[0;34m'
      NC='\033[0m'

      log_info() { echo -e "''${BLUE}[INFO]''${NC} $1"; }
      log_success() { echo -e "''${GREEN}[SUCCESS]''${NC} $1"; }
      log_warn() { echo -e "''${YELLOW}[WARN]''${NC} $1"; }
      log_error() { echo -e "''${RED}[ERROR]''${NC} $1"; }

      # Load configuration
      CONFIG_FILE="$HOME/.config/app-backup/config"
      if [[ ! -f "$CONFIG_FILE" ]]; then
        log_error "Config file not found. Run: nixos-rebuild switch"
        exit 1
      fi
      # shellcheck source=/dev/null
      source "$CONFIG_FILE"

      echo -e "''${BLUE}Keys Status''${NC}"
      echo ""

      # Check Age key
      echo -e "''${BLUE}Age Key:''${NC}"
      if [[ -n "''${AGE_KEY_PATH:-}" ]]; then
        AGE_KEY_PATH_EXPANDED="''${AGE_KEY_PATH/#\~/$HOME}"
        if [[ -f "$AGE_KEY_PATH_EXPANDED" ]]; then
          echo -e "  Local: ''${GREEN}✓''${NC} $AGE_KEY_PATH_EXPANDED"
        else
          echo -e "  Local: ''${YELLOW}✗''${NC} $AGE_KEY_PATH_EXPANDED (not found)"
        fi
      else
        echo -e "  Local: ''${YELLOW}✗''${NC} Not configured"
      fi
      if [[ -n "''${AGE_KEY_1PASSWORD:-}" ]]; then
        echo -e "  1Password: ''${GREEN}✓''${NC} $AGE_KEY_1PASSWORD"
      else
        echo -e "  1Password: ''${YELLOW}✗''${NC} Not configured"
      fi

      echo ""

      # Check SSH key
      echo -e "''${BLUE}SSH Key:''${NC}"
      if [[ -n "''${SSH_KEY_PATH:-}" ]]; then
        SSH_KEY_PATH_EXPANDED="''${SSH_KEY_PATH/#\~/$HOME}"
        if [[ -f "$SSH_KEY_PATH_EXPANDED" ]]; then
          echo -e "  Local: ''${GREEN}✓''${NC} $SSH_KEY_PATH_EXPANDED"
          if [[ -f "''${SSH_KEY_PATH_EXPANDED}.pub" ]]; then
            echo -e "  Public: ''${GREEN}✓''${NC} ''${SSH_KEY_PATH_EXPANDED}.pub"
          fi
        else
          echo -e "  Local: ''${YELLOW}✗''${NC} $SSH_KEY_PATH_EXPANDED (not found)"
        fi
      else
        echo -e "  Local: ''${YELLOW}✗''${NC} Not configured"
      fi
      if [[ -n "''${SSH_KEY_1PASSWORD:-}" ]]; then
        echo -e "  1Password: ''${GREEN}✓''${NC} $SSH_KEY_1PASSWORD"
      else
        echo -e "  1Password: ''${YELLOW}✗''${NC} Not configured"
      fi

      echo ""

      # Check backup
      echo -e "''${BLUE}Keys Backup:''${NC}"
      LOCAL_REPO_PATH="''${LOCAL_REPO_PATH/#\~/$HOME}"
      : "''${LOCAL_REPO_PATH:=$HOME/.local/share/app-backup}"
      KEYS_FILE="$LOCAL_REPO_PATH/keys.tar.gz.age"
      if [[ -f "$KEYS_FILE" ]]; then
        SIZE=$(du -h "$KEYS_FILE" | cut -f1)
        MTIME=$(stat -c %y "$KEYS_FILE" | cut -d. -f1)
        echo -e "  Backup: ''${GREEN}✓''${NC} $KEYS_FILE ($SIZE, $MTIME)"
      else
        echo -e "  Backup: ''${YELLOW}✗''${NC} No backup found"
      fi
    '';
  };

in
{
  options.programs.app-backup = {
    enable = mkEnableOption "app profile backup/restore (browser sessions and logins)";

    repoUrl = mkOption {
      type = types.str;
      description = "Private GitHub repo URL for encrypted backups";
      example = "git@github.com:YOUR_USER/private-settings.git";
    };

    ageRecipient = mkOption {
      type = types.str;
      description = "Age public key for encryption";
      example = "age1xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx";
    };

    ageKey1Password = mkOption {
      type = types.nullOr types.str;
      default = null;
      description = ''
        1Password secret reference for the age private key.
        Format: op://vault/item/field
        Example: op://Private/age-key/private-key

        When set, the key is retrieved from 1Password on-the-fly.
        This takes precedence over ageKeyPath.
      '';
      example = "op://Private/age-key/private-key";
    };

    ageKeyPath = mkOption {
      type = types.nullOr types.str;
      default = null;
      description = ''
        Path to age identity key file for local storage.
        When both ageKeyPath and ageKey1Password are set, local file is checked first.
      '';
      example = "~/.config/age/key.txt";
    };

    sshKey1Password = mkOption {
      type = types.nullOr types.str;
      default = null;
      description = ''
        1Password secret reference for the SSH private key.
        Format: op://vault/item/field
        Example: op://Private/SSH Key (Kraken)/private key
      '';
      example = "op://Private/SSH Key (Kraken)/private key";
    };

    sshKeyPath = mkOption {
      type = types.nullOr types.str;
      default = null;
      description = ''
        Path to SSH private key file for local storage.
        When both sshKeyPath and sshKey1Password are set, local file is checked first.
      '';
      example = "~/.ssh/id_ed25519";
    };

    keysPassphrase1Password = mkOption {
      type = types.nullOr types.str;
      default = null;
      description = ''
        1Password reference for the passphrase to encrypt/decrypt keys backup.
        If not set, will prompt interactively during backup/restore.
      '';
      example = "op://Private/keys-backup-passphrase/password";
    };

    localRepoPath = mkOption {
      type = types.str;
      default = "~/.local/share/app-backup";
      description = "Local clone location for the backup repo";
    };

    backupRetention = mkOption {
      type = types.int;
      default = 3;
      description = "Number of timestamped backups to keep when restoring";
    };
  };

  config = mkIf cfg.enable {
    assertions = [
      {
        assertion = cfg.ageKey1Password != null || cfg.ageKeyPath != null;
        message = "app-backup: Either ageKey1Password or ageKeyPath must be set";
      }
    ];

    # Install the scripts and dependencies
    home.packages = [
      app-backup
      app-restore
      # Keys management
      keys-setup
      keys-backup
      keys-restore
      keys-status
      # Backward compatibility
      browser-backup-compat
      browser-restore-compat
      # Dependencies
      pkgs.age
      pkgs.git
      pkgs.git-lfs
      pkgs._1password-cli
      pkgs.openssh
    ];

    # Generate the example configuration file
    # Users should copy this to config and edit with their values
    xdg.configFile."app-backup/config.example" = {
      text = ''
        # App Backup Configuration
        # Copy this file to ~/.config/app-backup/config and edit with your values
        #
        # Required: Set your private GitHub repo for encrypted backups
        APP_BACKUP_REPO="${cfg.repoUrl}"
        #
        # Required: Your age public key (for encryption)
        AGE_RECIPIENT="${cfg.ageRecipient}"
        #
        # Optional settings
        LOCAL_REPO_PATH="${cfg.localRepoPath}"
        BACKUP_RETENTION=${toString cfg.backupRetention}
      '' + optionalString (cfg.ageKey1Password != null) ''
        #
        # 1Password reference for age private key
        AGE_KEY_1PASSWORD="${cfg.ageKey1Password}"
      '' + optionalString (cfg.ageKeyPath != null) ''
        #
        # Local path to age private key (fallback if 1Password unavailable)
        AGE_KEY_PATH="${cfg.ageKeyPath}"
      '' + optionalString (cfg.sshKey1Password != null) ''
        #
        # 1Password reference for SSH key (GitHub auth)
        SSH_KEY_1PASSWORD="${cfg.sshKey1Password}"
      '' + optionalString (cfg.sshKeyPath != null) ''
        #
        # Local path to SSH key
        SSH_KEY_PATH="${cfg.sshKeyPath}"
      '' + optionalString (cfg.keysPassphrase1Password != null) ''
        #
        # 1Password reference for keys backup passphrase
        KEYS_PASSPHRASE_1PASSWORD="${cfg.keysPassphrase1Password}"
      '';
      force = true;
    };

    # Copy example to config on first activation if config doesn't exist
    home.activation.setupAppBackupConfig = lib.hm.dag.entryAfter [ "writeBoundary" ] ''
      CONFIG_DIR="$HOME/.config/app-backup"
      if [ ! -f "$CONFIG_DIR/config" ] && [ -f "$CONFIG_DIR/config.example" ]; then
        $DRY_RUN_CMD cp "$CONFIG_DIR/config.example" "$CONFIG_DIR/config"
        echo "Created ~/.config/app-backup/config from example - please edit with your values"
      fi
    '';
  };
}
