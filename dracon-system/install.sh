#!/bin/bash
# Install script for dracon-system-guard
# Run this script to install the guard daemon as a user service

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
INSTALL_DIR="$HOME/.local/bin"
SERVICE_DIR="$HOME/.config/systemd/user"

echo "=== Dracon System Guard Installer ==="
echo ""

# Build release binary
echo "Building release binary..."
cd "$SCRIPT_DIR"
cargo build --release

# Create install directory
echo "Creating install directory..."
mkdir -p "$INSTALL_DIR"

# Copy binary
echo "Installing binary to $INSTALL_DIR/dracon-system..."
cp target/release/dracon-system "$INSTALL_DIR/dracon-system"
chmod +x "$INSTALL_DIR/dracon-system"

# Create systemd user directory
echo "Setting up systemd service..."
mkdir -p "$SERVICE_DIR"

# Copy service file
cp dracon-system-guard.service "$SERVICE_DIR/dracon-system-guard.service"

# Reload systemd
echo "Reloading systemd daemon..."
systemctl --user daemon-reload

# Enable and start service
echo ""
echo "Installation complete!"
echo ""
echo "To enable the guard daemon:"
echo "  systemctl --user enable dracon-system-guard"
echo ""
echo "To start it now:"
echo "  systemctl --user start dracon-system-guard"
echo ""
echo "To check status:"
echo "  systemctl --user status dracon-system-guard"
echo ""
echo "To view logs:"
echo "  journalctl --user -u dracon-system-guard -f"