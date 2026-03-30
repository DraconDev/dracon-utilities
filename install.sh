#!/usr/bin/env bash
set -e

cd "$(dirname "$0")"

echo "Installing dracon utilities to ~/.local/bin/"

# Check for required dracon-libs sibling directory
DRACON_LIBS="../dracon-libs"
if [ ! -d "$DRACON_LIBS" ]; then
    echo "ERROR: dracon-libs not found at ../dracon-libs"
    echo ""
    echo "This project requires dracon-libs as a sibling directory:"
    echo "  git clone https://github.com/your-org/dracon-libs.git ../dracon-libs"
    echo ""
    exit 1
fi

restart_service() {
    local service=$1
    if systemctl --user list-unit-files | grep -q "^$service"; then
        echo "Restarting $service..."
        systemctl --user restart "$service" 2>/dev/null && echo "  ✅ $service restarted" || echo "  ⚠️ Could not restart $service (may need sudo)"
    else
        echo "  ⚠️ $service not found (not installed as user service)"
    fi
}

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
echo "Restarting services..."
restart_service dracon-sync.service
restart_service dracon-system-guard.service
restart_service dracon-warden.service

echo ""
echo "AI config: ~/.dracon/ai/"
echo "  ai.toml          — provider configuration (copy ai.example.toml if new)"
echo "  secrets/*.env    — API keys (OPENROUTER_API_KEY, GEMINI_API_KEY, NVIDIA_API_KEY)"

if [ ! -f ~/.dracon/ai.toml ] && [ -f dracon-sync/ai.example.toml ]; then
    mkdir -p ~/.dracon
    cp dracon-sync/ai.example.toml ~/.dracon/ai.toml
    echo ""
    echo "✅ Copied ai.example.toml → ~/.dracon/ai.toml"
    echo "   Edit ~/.dracon/ai.toml and set your API keys"
