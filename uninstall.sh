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
# FIXED 2026-09-27 (audit F97): `systemctl ... | grep -q` is a
# false-negative race under `set -o pipefail` — `grep -q` exits on the
# first match, systemctl takes SIGPIPE (exit 141) if it was still
# writing, and the pipeline reports failure. Measured 4/20 on this
# machine; the consequence here was uninstall silently SKIPPING the
# stop/disable/unit-file removal, leaving a running service behind.
# Capture the list once, then grep the string.
USER_UNIT_FILES=$(systemctl --user list-unit-files 2>/dev/null || true)
for service in $SERVICES; do
    if grep -q "^$service" <<< "$USER_UNIT_FILES"; then
        # FIXED 2026-09-27 (F86): A && B || C is not if-then-else
        # (shellcheck SC2015); the `|| true` swallowed a failure, an
        # if/then says the same explicitly.
        if systemctl --user stop "$service" 2>/dev/null; then
            echo "  ✅ Stopped $service"
        fi
        if systemctl --user disable "$service" 2>/dev/null; then
            echo "  ✅ Disabled $service"
        fi
        if rm "$HOME/.config/systemd/user/$service" 2>/dev/null; then
            echo "  ✅ Removed $service"
        fi
    else
        echo "  ⚠️  $service not found (skipping)"
    fi
done

systemctl --user daemon-reload 2>/dev/null || true

# FIXED 2026-09-27 (audit F82): install.sh runs
# `dracon-warden setup-hooks --global`, which writes ~/.config/git/hooks
# and points the GLOBAL git config's core.hooksPath at it. The binary is
# gone by this point, so every commit and push on the machine would
# invoke a missing executable — and the global config keeps shadowing
# each repo's own .git/hooks. Undo it before the binary disappears.
echo ""
echo "Removing warden git hooks..."
HOOKS_DIR="$HOME/.config/git/hooks"
CURRENT_HOOKS_PATH=""
if command -v git &>/dev/null; then
    CURRENT_HOOKS_PATH=$(git config --global --get core.hooksPath 2>/dev/null || true)
fi
# Only touch core.hooksPath when it actually points at the warden hooks
# dir — a user who set it themselves keeps their setting.
if [ -n "$CURRENT_HOOKS_PATH" ] && [ "$CURRENT_HOOKS_PATH" = "$HOOKS_DIR" ]; then
    git config --global --unset-all core.hooksPath 2>/dev/null || true
    echo "  ✅ Unset global core.hooksPath (was $CURRENT_HOOKS_PATH)"
elif [ -n "$CURRENT_HOOKS_PATH" ]; then
    echo "  ⚠️  global core.hooksPath is $CURRENT_HOOKS_PATH (not warden) — left alone"
fi
if [ -d "$HOOKS_DIR" ]; then
    rm -f "$HOOKS_DIR/pre-commit" "$HOOKS_DIR/pre-push" "$HOOKS_DIR/pre-rebase"
    rmdir "$HOOKS_DIR" 2>/dev/null || true
    echo "  ✅ Removed warden hooks from $HOOKS_DIR"
fi

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
