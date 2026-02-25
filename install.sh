#!/usr/bin/env bash
set -e

cd "$(dirname "$0")"

echo "Installing dracon utilities to ~/.local/bin/"

cargo install --path dracon-sync --root ~/.local --force
cargo install --path dracon-system --root ~/.local --force
cargo install --path dracon-warden --root ~/.local --force
cargo install --path dracon-ai --root ~/.local --force

echo ""
echo "Installed:"
ls -la ~/.local/bin/dracon-sync ~/.local/bin/dracon-system ~/.local/bin/dracon-warden ~/.local/bin/dracon-ai 2>/dev/null || true

echo ""
echo "Restart services with:"
echo "  systemctl --user restart dracon-sync.service dracon-system-guard.service dracon-warden.service"
