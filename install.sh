#!/usr/bin/env bash
set -e

cd "$(dirname "$0")"

echo "Installing dracon utilities to ~/.local/bin/"

# Build with release and install manually for feature control
install_binary() {
    local package=$1
    local features=$2
    local binary=$package
    
    echo "Building $package..."
    if [ -n "$features" ]; then
        cargo build --release --package $package --features "$features" 2>/dev/null || \
        cargo build --release -p $package 2>/dev/null || \
        cargo build --release -p $package
    else
        cargo build --release -p $package
    fi
    
    # Find the binary in target/release
    local bin_path="target/release/$binary"
    if [ ! -f "$bin_path" ]; then
        bin_path=$(find target/release -name "$binary" -type f 2>/dev/null | head -1)
    fi
    
    if [ -n "$bin_path" ] && [ -f "$bin_path" ]; then
        cp "$bin_path" ~/.local/bin/$binary
        chmod +x ~/.local/bin/$binary
        echo "  Installed ~/.local/bin/$binary"
    else
        echo "  ERROR: Could not find binary for $package"
        return 1
    fi
}

# dracon-sync with scribe and ai-bumper (both on by default)
install_binary dracon-sync "scribe,ai-bumper"

# dracon-system and dracon-warden
install_binary dracon-system ""
install_binary dracon-warden ""

mkdir -p ~/.config/systemd/user
mkdir -p ~/.dracon/ai/secrets

cp dracon-sync/dracon-sync.service ~/.config/systemd/user/dracon-sync.service 2>/dev/null || true
cp dracon-system/dracon-system-guard.service ~/.config/systemd/user/dracon-system-guard.service 2>/dev/null || true
cp dracon-warden/dracon-warden.service ~/.config/systemd/user/dracon-warden.service 2>/dev/null || true
systemctl --user daemon-reload 2>/dev/null || true

echo ""
echo "Installed:"
ls -la ~/.local/bin/dracon-sync ~/.local/bin/dracon-system ~/.local/bin/dracon-warden 2>/dev/null || true

echo ""
echo "AI config: ~/.dracon/ai/"
echo "  ai.toml          — provider configuration"
echo "  secrets/*.env    — API keys (OPENROUTER_API_KEY, GEMINI_API_KEY, NVIDIA_API_KEY)"
echo ""
echo "Restart services with:"
echo "  systemctl --user restart dracon-sync.service dracon-system-guard.service dracon-warden.service"
