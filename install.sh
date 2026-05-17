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

# Stop services if upgrading
if [ "$UPGRADE" = true ]; then
    echo "Stopping services for upgrade..."
    for service in dracon-sync.service dracon-system-guard.service dracon-warden.service; do
        if systemctl --user is-active "$service" &>/dev/null; then
            if [ "$DRY_RUN" = true ]; then
                echo "  Would stop $service"
            else
                systemctl --user stop "$service" 2>/dev/null && echo "  Stopped $service" || true
            fi
        fi
    done
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
echo ""

# Build with release and install manually for feature control
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
        local new_hash=$(md5sum "$resolved" | cut -d' ' -f1)

        if [ -f "$installed" ]; then
            local old_hash=$(md5sum "$installed" | cut -d' ' -f1)
            if [ "$new_hash" = "$old_hash" ]; then
                echo "  ⏭️  ~/.local/bin/$binary unchanged (same hash)"
                return 0
            else
                echo "  ✅ Installed ~/.local/bin/$binary (updated)"
            fi
        else
            echo "  ✅ Installed ~/.local/bin/$binary (new)"
        fi

        cp "$resolved" ~/.local/bin/$binary
        chmod +x ~/.local/bin/$binary

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
# dracon-sync with scribe and ai-bumper (both on by default)
install_binary dracon-sync "scribe,ai-bumper" "dracon-sync"
install_binary dracon-system "" "dracon-system"
install_binary dracon-warden "" "dracon-warden"
# dracon-ai is not in the workspace (depends on unpublished dracon-libs/services/ai)
# Install separately from ./dracon-ai/ if needed

echo ""

# Check if ~/.local/bin is in PATH
if [[ ":$PATH:" != *":$HOME/.local/bin:"* ]]; then
    echo "⚠️  WARNING: ~/.local/bin is not in your PATH"
    echo "   Add this to your shell config to use dracon utilities:"
    echo '   export PATH="$HOME/.local/bin:$PATH"'
    echo ""
fi

if [ "$BINARIES_ONLY" = true ]; then
    echo "✅ Binaries installed. Skipping configs and services (--binaries-only)."
    exit 0
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
# dracon-ai config not installed (crate not in workspace)

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
    
    if [ "$DRY_RUN" = true ]; then
        echo "  Would restart $service"
        return 0
    fi
    
    if systemctl --user list-unit-files | grep -q "^$service"; then
        systemctl --user restart "$service" 2>/dev/null && echo "  ✅ $service restarted" || echo "  ⚠️ Could not restart $service"
    else
        echo "  ⚠️ $service not found"
    fi
}

restart_service dracon-sync.service
restart_service dracon-system-guard.service
restart_service dracon-warden.service

echo ""
echo "✅ Installation complete!"
echo ""
echo "Binaries:"
ls -la ~/.local/bin/dracon-* 2>/dev/null || true
echo ""
echo "Next steps:"
echo "  1. Add API keys to ~/.dracon/utilities/sync/ai/secrets/*.env"
echo "  2. Add registry tokens to ~/.dracon/utilities/sync/secrets/*.env (crates.io, npm, etc.)"
echo "  3. Check 'dracon-sync status' to verify sync is working"
echo "  4. Check 'dracon-system status' to verify guard is working"
