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
    local subdir=$3
    local binary=${package%%@*}  # strip version suffix if present

    echo "Building $package..."
    local bin_path="target/release/$binary"

    if [ -n "$features" ]; then
        (cd "$subdir" && cargo build --release --package "$package" --features "$features" 2>/dev/null) || \
        (cd "$subdir" && cargo build --release -p "$package" 2>/dev/null) || \
        (cd "$subdir" && cargo build --release -p "$package")
    else
        (cd "$subdir" && cargo build --release -p "$package")
    fi

    if [ -f "$subdir/$bin_path" ]; then
        cp "$subdir/$bin_path" ~/.local/bin/$binary
        chmod +x ~/.local/bin/$binary
        echo "  Installed ~/.local/bin/$binary"
    else
        echo "  ERROR: Could not find binary for $package"
        return 1
    fi
}

# dracon-sync with scribe and ai-bumper (both on by default)
install_binary dracon-sync "scribe,ai-bumper" "dracon-sync"
install_binary dracon-system "" "dracon-system"
install_binary dracon-warden "" "dracon-warden"

# Check if ~/.local/bin is in PATH
if [[ ":$PATH:" != *":$HOME/.local/bin:"* ]]; then
    echo ""
    echo "⚠️  WARNING: ~/.local/bin is not in your PATH"
    echo "   Add this to your shell config to use dracon utilities:"
    echo '   export PATH="$HOME/.local/bin:$PATH"'
    echo ""
fi

mkdir -p ~/.config/systemd/user
mkdir -p ~/.dracon/utilities/sync

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
echo "AI config: ~/.dracon/utilities/sync/"
echo "  ai.toml          — provider configuration (copy ai.example.toml if new)"
echo "  *.env            — API keys (OPENROUTER_API_KEY, GEMINI_API_KEY, NVIDIA_API_KEY)"

if [ ! -f ~/.dracon/utilities/sync/ai.toml ] && [ -f dracon-sync/ai.example.toml ]; then
    cp dracon-sync/ai.example.toml ~/.dracon/utilities/sync/ai.toml
    echo ""
    echo "✅ Copied ai.example.toml → ~/.dracon/utilities/sync/ai.toml"
    echo "   Edit ~/.dracon/utilities/sync/ai.toml and set your API keys"
fi
