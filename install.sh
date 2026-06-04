#!/usr/bin/env bash
set -euo pipefail

# Dracon Utilities Installer
# Usage: ./install.sh [OPTIONS]
#
# Options:
#   --help, -h         Show this help message
#   --dry-run          Show what would be installed without making changes
#   --force            Overwrite existing configs (normally skipped)
#   --upgrade          Stop services, install, restart (default: only restart if running)
#   --verbose, -v      Show more output
#   --no-restart       Don't restart services after install
#   --binaries-only    Only install binaries, skip configs and services
#
# Examples:
#   ./install.sh                    # First install
#   ./install.sh --upgrade          # Update existing installation
#   ./install.sh --dry-run          # Preview what would happen
#   ./install.sh --force            # Overwrite existing configs

cd "$(dirname "$0")"

# Parse arguments
DRY_RUN=false
FORCE=false
UPGRADE=false
VERBOSE=false
NO_RESTART=false
BINARIES_ONLY=false

for arg in "$@"; do
    case "$arg" in
        --help|-h)
            sed -n '3,17p' "$0" | sed 's/^# //; s/^#$//'
            exit 0
            ;;
        --dry-run)
            DRY_RUN=true
            echo "🔍 DRY RUN MODE - No changes will be made"
            echo ""
            ;;
        --force)
            FORCE=true
            ;;
        --upgrade)
            UPGRADE=true
            ;;
        --verbose|-v)
            VERBOSE=true
            ;;
        --no-restart)
            NO_RESTART=true
            ;;
        --binaries-only)
            BINARIES_ONLY=true
            ;;
        *)
            echo "Unknown option: $arg"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

log() {
    if [ "$VERBOSE" = true ] || [ "$DRY_RUN" = true ]; then
        echo "$@"
    fi
}

# Check for required dracon-libs sibling directory
DRACON_LIBS="../dracon-libs"
if [ ! -d "$DRACON_LIBS" ]; then
    echo "ERROR: dracon-libs not found at ../dracon-libs"
    echo ""
    echo "This project requires dracon-libs as a sibling directory:"
    echo "  git clone https://github.com/DraconDev/dracon-libs.git ../dracon-libs"
    echo ""
    exit 1
fi

# Check prerequisites
for cmd in cargo systemctl; do
    if ! command -v "$cmd" &> /dev/null; then
        echo "ERROR: Required command '$cmd' not found"
        exit 1
    fi
done

# Set git default branch to main (consistent with GitHub convention)
CURRENT_DEFAULT=$(git config --global init.defaultBranch 2>/dev/null || echo "")
if [ "$CURRENT_DEFAULT" != "main" ]; then
    if [ "$DRY_RUN" = true ]; then
        echo "Would set git default branch to main (currently: ${CURRENT_DEFAULT:-master})"
    else
        git config --global init.defaultBranch main
        echo "✅ Set git default branch to main (was: ${CURRENT_DEFAULT:-master})"
    fi
fi

# Stop services if upgrading
if [ "$UPGRADE" = true ]; then
    echo "Stopping services for upgrade..."
    for service in dracon-sync.service dracon-system-guard.service dracon-warden.service; do
        # Map service name to binary name
        _bin=""
        case "$service" in
            dracon-sync.service)   _bin=dracon-sync ;;
            dracon-system-guard.service) _bin=dracon-system ;;
            dracon-warden.service)  _bin=dracon-warden ;;
        esac

        if [ "$DRY_RUN" = true ]; then
            if systemctl --user is-active "$service" &>/dev/null; then
                echo "  Would stop $service (systemctl)"
            fi
            if pgrep -x "$_bin" &>/dev/null; then
                echo "  Would kill $_bin (pkill fallback)"
            fi
        else
            # Stop via systemd (clean shutdown)
            if systemctl --user is-active "$service" &>/dev/null; then
                systemctl --user stop "$service" 2>/dev/null && echo "  Stopped $service" || true
            fi
            # Catch any remaining processes (manual runs, stale)
            if pgrep -x "$_bin" &>/dev/null; then
                pkill -x "$_bin" 2>/dev/null || true
                echo "  Killed $_bin (non-systemd process)"
            fi
        fi
    done
    # Wait for all processes to exit
    sleep 1
    echo ""
fi

echo "Installing dracon utilities to ~/.local/bin/"
mkdir -p ~/.local/bin

# Clean up orphaned binaries from previous architectures
ORPHANS=(
    dracon-system-guard
    dracon-security-daemon-guard
)
for orphan in "${ORPHANS[@]}"; do
    if [ -f ~/.local/bin/"$orphan" ]; then
        if [ "$DRY_RUN" = true ]; then
            echo "  Would remove orphan: ~/.local/bin/$orphan"
        else
            rm -f ~/.local/bin/"$orphan"
            echo "  🧹 Removed orphan: ~/.local/bin/$orphan"
        fi
    fi
done

# Clean up stale binaries from ~/.cargo/bin (leftover from cargo install)
# These get shadowed by ~/.local/bin but can cause confusion if PATH order varies
CARGO_BIN_STALE=(
    dracon-sync
    dracon-system
    dracon-warden
)
for stale in "${CARGO_BIN_STALE[@]}"; do
    if [ -f ~/.cargo/bin/"$stale" ]; then
        if [ "$DRY_RUN" = true ]; then
            echo "  Would remove stale ~/.cargo/bin/$stale (outdated cargo install artifact)"
        else
            rm -f ~/.cargo/bin/"$stale"
            echo "  🧹 Removed stale ~/.cargo/bin/$stale (outdated cargo install artifact)"
        fi
    fi
done

# Clean up stale .bak files
for bak in ~/.local/bin/dracon-*.bak*; do
    [ -f "$bak" ] || continue
    if [ "$DRY_RUN" = true ]; then
        echo "  Would remove stale backup: $bak"
    else
        rm -f "$bak"
        echo "  🧹 Removed stale backup: $(basename "$bak")"
    fi
done

# Scan PATH for shadowing binaries — any dracon-* in a directory
# other than ~/.local/bin will take priority depending on PATH order.
# This catches stale installs in /usr/local/bin, ~/bin, etc.
IFS=':' read -ra _path_dirs <<< "$PATH"
for _dir in "${_path_dirs[@]}"; do
    [ "$_dir" = "$HOME/.local/bin" ] && continue
    for _stale in "$_dir"/dracon-sync "$_dir"/dracon-system "$_dir"/dracon-warden; do
        [ -f "$_stale" ] || continue
        if [ "$DRY_RUN" = true ]; then
            echo "  Would remove shadowing binary: $_stale"
        else
            rm -f "$_stale"
            echo "  🧹 Removed shadowing binary: $_stale"
        fi
    done
done
echo ""

# Build with release and install manually for feature control
RESTARTED_SERVICES=""

install_binary() {
    local package=$1
    local features=$2
    local subdir=$3
    local binary=${package%%@*}  # strip version suffix if present

    echo "Building $package..."
    local bin_path="target/release/$binary"

    if [ "$DRY_RUN" = true ]; then
        echo "  Would build $package → ~/.local/bin/$binary"
        return 0
    fi

    if [ -n "$features" ]; then
        (cd "$subdir" && cargo build --release --package "$package" --features "$features" 2>/dev/null) || \
        (cd "$subdir" && cargo build --release -p "$package" 2>/dev/null) || \
        (cd "$subdir" && cargo build --release -p "$package")
    else
        (cd "$subdir" && cargo build --release -p "$package")
    fi

    local resolved=""
    if [ -f "$subdir/$bin_path" ]; then
        resolved="$subdir/$bin_path"
    elif [ -f "$bin_path" ]; then
        resolved="$bin_path"
    fi

    if [ -n "$resolved" ]; then
        local installed=~/.local/bin/$binary
        local new_hash
        new_hash=$(md5sum "$resolved" | cut -d' ' -f1)

        if [ -f "$installed" ]; then
            local old_hash
            old_hash=$(md5sum "$installed" | cut -d' ' -f1)
            if [ "$new_hash" = "$old_hash" ]; then
                echo "  ⏭️  ~/.local/bin/$binary unchanged (same hash)"
                return 0
            else
                echo "  ✅ Installed ~/.local/bin/$binary (updated)"
            fi
        else
            echo "  ✅ Installed ~/.local/bin/$binary (new)"
        fi

        # Always scrap the old daemon before installing new binary.
        # Prevents "Text file busy" and stale binary issues.
        local svc_name=""
        case "$binary" in
            dracon-sync)   svc_name=dracon-sync.service ;;
            dracon-system) svc_name=dracon-system-guard.service ;;
            dracon-warden)  svc_name=dracon-warden.service ;;
        esac

        # Stop service + kill all processes — always, no check
        if [ -n "$svc_name" ]; then
            systemctl --user stop "$svc_name" 2>/dev/null || true
        fi
        pkill -x "$binary" 2>/dev/null || true
        # Wait for process to fully exit (up to 3s)
        for _ in $(seq 1 6); do
            pgrep -x "$binary" &>/dev/null || break
            sleep 0.5
        done

        # Remove old binary and install new
        rm -f ~/.local/bin/"$binary"
        cp "$resolved" ~/.local/bin/"$binary"
        chmod +x ~/.local/bin/"$binary"

        # Restart the service and track it for the final restart block
        if [ -n "$svc_name" ]; then
            systemctl --user start "$svc_name" 2>/dev/null || true
            RESTARTED_SERVICES="$RESTARTED_SERVICES $svc_name"
        fi

        # Warn if debug build is newer than release — developer may have uninstalled changes
        local debug_path=""
        if [ -f "$subdir/target/debug/$binary" ]; then
            debug_path="$subdir/target/debug/$binary"
        elif [ -f "target/debug/$binary" ]; then
            debug_path="target/debug/$binary"
        fi
        if [ -n "$debug_path" ] && [ "$debug_path" -nt "$resolved" ]; then
            echo "  ⚠️  WARNING: target/debug/$binary is NEWER than target/release/$binary"
            echo "     You may have code changes that aren't in this release build."
            echo "     Run './install.sh' again after your changes to pick them up."
        fi
    else
        echo "  ❌ ERROR: Could not find binary for $package (checked $subdir/$bin_path and $bin_path)"
        return 1
    fi
}

# Install all binaries
install_binary dracon-sync "" "dracon-sync"
install_binary dracon-system "" "dracon-system"
install_binary dracon-warden "" "dracon-warden"
# dracon-ai removed — superseded by dracon-code (when available)

echo ""

# Check if ~/.local/bin is in PATH
if [[ ":$PATH:" != *":$HOME/.local/bin:"* ]]; then
    echo "⚠️  WARNING: ~/.local/bin is not in your PATH"
    echo "   Add this to your shell config to use dracon utilities:"
    # shellcheck disable=SC2016  # intentional: show literal $HOME in advice to user
    echo '   export PATH="$HOME/.local/bin:$PATH"'
    echo ""
fi

if [ "$BINARIES_ONLY" = true ]; then
    echo "✅ Binaries installed. Skipping configs and services (--binaries-only)."
    exit 0
fi

# Install warden git hooks (pre-commit + pre-push enforcement)
if [ "$DRY_RUN" = true ]; then
    echo "Would install warden git hooks via: dracon-warden setup-hooks --global"
else
    if command -v dracon-warden &>/dev/null || [ -f ~/.local/bin/dracon-warden ]; then
        ~/.local/bin/dracon-warden setup-hooks --global 2>/dev/null || \
            echo "⚠️  Could not install warden hooks (run manually: dracon-warden setup-hooks)"
    else
        echo "⚠️  dracon-warden not found, skipping hook installation"
    fi
fi

# Install systemd service files
mkdir -p ~/.config/systemd/user
mkdir -p ~/.dracon/utilities/sync
mkdir -p ~/.dracon/utilities/system
mkdir -p ~/.dracon/utilities/warden

if [ "$DRY_RUN" = true ]; then
    echo "Would install systemd services to ~/.config/systemd/user/"
    echo "Would create config directories under ~/.dracon/utilities/"
else
    cp dracon-sync/dracon-sync.service ~/.config/systemd/user/dracon-sync.service 2>/dev/null || true
    cp dracon-system/dracon-system-guard.service ~/.config/systemd/user/dracon-system-guard.service 2>/dev/null || true
    cp dracon-warden/dracon-warden.service ~/.config/systemd/user/dracon-warden.service 2>/dev/null || true
    systemctl --user daemon-reload 2>/dev/null || true
    # Wait for systemd to settle after daemon-reload
    sleep 1
    echo "✅ Systemd services installed"
fi

# Copy example configs
copy_config() {
    local src="$1"
    local dest="$2"
    
    if [ ! -f "$src" ]; then
        log "  Source not found: $src"
        return 0
    fi
    
    if [ -f "$dest" ] && [ "$FORCE" = false ]; then
        log "  Skipping $dest (already exists, use --force to overwrite)"
        return 0
    fi
    
    if [ "$DRY_RUN" = true ]; then
        echo "  Would copy $(basename "$src") → $dest"
    else
        cp "$src" "$dest"
        echo "  ✅ Copied $(basename "$src") → $dest"
    fi
}

echo ""
echo "Installing example configs..."
copy_config "dracon-sync/dracon-sync.example.toml" "$HOME/.dracon/utilities/sync/dracon-sync.toml"
copy_config "dracon-system/dracon-system.example.toml" "$HOME/.dracon/utilities/system/dracon-system.toml"
copy_config "dracon-warden/dracon-warden.example.toml" "$HOME/.dracon/utilities/warden/dracon-warden.toml"


# Create secrets directories with correct permissions
mkdir -p "$HOME/.dracon/utilities/sync/secrets"
chmod 700 "$HOME/.dracon/utilities/sync/secrets" 2>/dev/null || true
mkdir -p "$HOME/.dracon/utilities/sync/ai/secrets"
chmod 700 "$HOME/.dracon/utilities/sync/ai/secrets" 2>/dev/null || true

if [ "$NO_RESTART" = true ]; then
    echo ""
    echo "✅ Installation complete. Services NOT restarted (--no-restart)."
    echo "   Run 'systemctl --user restart dracon-sync.service' etc. to start."
    exit 0
fi

# Restart services
echo ""
echo "Restarting services..."

restart_service() {
    local service=$1

    # Skip if already restarted during binary install
    if [[ " $RESTARTED_SERVICES " == *" $service "* ]]; then
        if [ "$VERBOSE" = true ]; then
            echo "  ⏭️  $service already restarted during install"
        fi
        return 0
    fi

    if [ "$DRY_RUN" = true ]; then
        echo "  Would restart $service"
        return 0
    fi

    if systemctl --user is-enabled "$service" &>/dev/null || systemctl --user list-unit-files 2>/dev/null | grep -q "^$service"; then
        systemctl --user restart "$service" 2>/dev/null && echo "  ✅ $service restarted" || echo "  ⚠️ Could not restart $service"
    else
        echo "  ⚠️ $service not found"
    fi
}

restart_service dracon-sync.service
restart_service dracon-system-guard.service
# Warden daemon is optional — hooks are the primary enforcement layer.
# Only restart if the service is already running.
if systemctl --user is-active dracon-warden.service &>/dev/null; then
    restart_service dracon-warden.service
fi

echo ""
echo "✅ Installation complete!"
echo ""
echo "Binaries:"
ls -la ~/.local/bin/dracon-* 2>/dev/null || true
echo ""
echo "Checksums:"
for bin in ~/.local/bin/dracon-*; do
    [ -f "$bin" ] && sha256sum "$bin" 2>/dev/null || true
done

# Verify running daemons are using the installed binary
VERIFY_OK=true
for bin in dracon-sync dracon-system dracon-warden; do
    pid=$(pgrep -x "$bin" 2>/dev/null | head -1)
    [ -n "$pid" ] || continue
    running=$(readlink /proc/$pid/exe 2>/dev/null)
    expected="$HOME/.local/bin/$bin"
    if [ -n "$running" ] && [ "$running" != "$expected" ]; then
        echo "⚠️  WARNING: $bin (PID $pid) running from $running, not $expected"
        echo "   This means a stale version is still active. Restart the service:"
        echo "   systemctl --user restart $(systemctl --user list-units --type=service --state=running | grep -o "$bin[^ ]*\.service" | head -1)"
        VERIFY_OK=false
    fi
done
if [ "$VERIFY_OK" = true ]; then
    echo "✅ All running daemons verified at ~/.local/bin/"
fi
echo ""
echo "Next steps:"
echo "  1. Warden hooks are installed globally (pre-commit + pre-push)"
echo "  2. Add API keys to ~/.dracon/utilities/sync/ai/secrets/*.env"
echo "  3. Add registry tokens to ~/.dracon/utilities/sync/secrets/*.env (crates.io, npm, etc.)"
echo "  4. Check 'dracon-sync status' to verify sync is working"
echo "  5. Check 'dracon-system status' to verify guard is working"
