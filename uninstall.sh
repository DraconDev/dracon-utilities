#!/usr/bin/env bash
set -e

echo "Uninstalling dracon utilities..."

# Binaries to remove
BINARIES="dracon-sync dracon-system dracon-warden"

# Service files to remove
SERVICES="dracon-sync.service dracon-system-guard.service dracon-warden.service"

# Remove binaries
echo "Removing binaries from ~/.local/bin/"
for binary in $BINARIES; do
    if [ -f "$HOME/.local/bin/$binary" ]; then
        rm "$HOME/.local/bin/$binary"
        echo "  Removed ~/.local/bin/$binary"
    else
        echo "  ~/.local/bin/$binary not found (skipping)"
    fi
done

# Stop and remove systemd services
echo ""
echo "Stopping and removing systemd services..."
for service in $SERVICES; do
    if systemctl --user list-unit-files | grep -q "^$service"; then
        systemctl --user stop "$service" 2>/dev/null && echo "  Stopped $service" || true
        systemctl --user disable "$service" 2>/dev/null && echo "  Disabled $service" || true
        rm "$HOME/.config/systemd/user/$service" 2>/dev/null && echo "  Removed $service" || true
    else
        echo "  $service not found (skipping)"
    fi
done

systemctl --user daemon-reload 2>/dev/null || true

echo ""
echo "Uninstallation complete."
echo ""
echo "Note: Policy configs in ~/.dracon/ were not removed."
echo "Note: Git remotes and repo data are unaffected."
