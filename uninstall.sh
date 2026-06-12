#!/usr/bin/env bash
set -euo pipefail

# Dracon Utilities Uninstaller
# Usage: ./uninstall.sh [OPTIONS]
#
# Options:
#   --help, -h     Show this help message
#   --force        Skip confirmation prompts
#   --configs      Also remove config files in ~/.dracon/
#   --logs         Also remove log files in ~/.local/state/dracon/
#   --purge        Remove everything including configs and logs
#
# Examples:
#   ./uninstall.sh              # Remove binaries and services only
#   ./uninstall.sh --purge      # Remove everything (full cleanup)

# Parse arguments
FORCE=false
REMOVE_CONFIGS=false
REMOVE_LOGS=false

for arg in "$@"; do
    case "$arg" in
        --help|-h)
            sed -n '3,16p' "$0" | sed 's/^# //; s/^#$//'
            exit 0
            ;;
        --force)
            FORCE=true
            ;;
        --configs)
            REMOVE_CONFIGS=true
            ;;
        --logs)
            REMOVE_LOGS=true
            ;;
        --purge)
            REMOVE_CONFIGS=true
            REMOVE_LOGS=true
            ;;
        *)
            echo "Unknown option: $arg"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

echo "Uninstalling dracon utilities..."

# Confirm unless --force
if [ "$FORCE" = false ]; then
    echo ""
    read -p "Are you sure? This will remove binaries and systemd services. [y/N] " -n 1 -r
    echo ""
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "Aborted."
        exit 0
    fi
fi

# Binaries to remove
BINARIES="dracon-sync dracon-system dracon-warden"

# Service files to remove
SERVICES="dracon-sync.service dracon-system-guard.service"

# Remove binaries
echo ""
echo "Removing binaries from ~/.local/bin/"
for binary in $BINARIES; do
    if [ -f "$HOME/.local/bin/$binary" ]; then
        rm "$HOME/.local/bin/$binary"
        echo "  ✅ Removed ~/.local/bin/$binary"
    else
        echo "  ⚠️  ~/.local/bin/$binary not found (skipping)"
    fi
done

# Stop and remove systemd services
echo ""
echo "Stopping and removing systemd services..."
for service in $SERVICES; do
    if systemctl --user list-unit-files 2>/dev/null | grep -q "^$service"; then
        systemctl --user stop "$service" 2>/dev/null && echo "  ✅ Stopped $service" || true
        systemctl --user disable "$service" 2>/dev/null && echo "  ✅ Disabled $service" || true
        rm "$HOME/.config/systemd/user/$service" 2>/dev/null && echo "  ✅ Removed $service" || true
    else
        echo "  ⚠️  $service not found (skipping)"
    fi
done

systemctl --user daemon-reload 2>/dev/null || true

# Remove configs if requested
if [ "$REMOVE_CONFIGS" = true ]; then
    echo ""
    echo "Removing configuration files..."
    if [ -d "$HOME/.dracon/utilities" ]; then
        rm -rf "$HOME/.dracon/utilities"
        echo "  ✅ Removed ~/.dracon/utilities/"
    fi
fi

# Remove logs if requested
if [ "$REMOVE_LOGS" = true ]; then
    echo ""
    echo "Removing log files..."
    if [ -d "$HOME/.local/state/dracon" ]; then
        rm -rf "$HOME/.local/state/dracon"
        echo "  ✅ Removed ~/.local/state/dracon/"
    fi
fi

echo ""
echo "✅ Uninstallation complete."

if [ "$REMOVE_CONFIGS" = false ] && [ "$REMOVE_LOGS" = false ]; then
    echo ""
    echo "Note: Policy configs in ~/.dracon/ were preserved."
    echo "Note: Log files in ~/.local/state/dracon/ were preserved."
    echo ""
    echo "To remove everything including configs and logs:"
    echo "  ./uninstall.sh --purge"
fi
