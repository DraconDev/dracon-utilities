#!/usr/bin/env bash
set -e

cd "$(dirname "$0")"

echo "Installing dracon utilities to ~/.local/bin/"

# Install dracon-sync with optional scribe feature (on by default)
SCRIBE_FEATURE=""
if [ "$1" = "--no-scribe" ] || [ "$DRACON_SCRIBE" = "0" ]; then
    SSCRIBE_FEATURE="--no-default-features"
    echo "  (without AI scribe support)"
fi

cargo install --path dracon-sync --root ~/.local --force $SCRIBE_FEATURE
cargo install --path dracon-system --root ~/.local --force
cargo install --path dracon-warden --root ~/.local --force
cargo install --path dracon-ai --root ~/.local --force

mkdir -p ~/.config/systemd/user
mkdir -p ~/.dracon/ai/secrets

cp dracon-sync/dracon-sync.service ~/.config/systemd/user/dracon-sync.service
cp dracon-system/dracon-system-guard.service ~/.config/systemd/user/dracon-system-guard.service
cp dracon-warden/dracon-warden.service ~/.config/systemd/user/dracon-warden.service
systemctl --user daemon-reload

echo ""
echo "Installed:"
ls -la ~/.local/bin/dracon-sync ~/.local/bin/dracon-system ~/.local/bin/dracon-warden ~/.local/bin/dracon-ai 2>/dev/null || true

echo ""
echo "AI config: ~/.dracon/ai/"
echo "  routing-policy.json  — providers, models, lanes"
echo "  secrets/*.env        — API keys"
echo ""
echo "Restart services with:"
echo "  systemctl --user restart dracon-sync.service dracon-system-guard.service dracon-warden.service"
