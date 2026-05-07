#!/usr/bin/env bash
set -euo pipefail

# Dracon Utilities Doctor
# Checks prerequisites and diagnoses common issues

cd "$(dirname "$0")"

PASS=0
FAIL=0
WARN=0

check() {
    local name="$1"
    local cmd="$2"
    local required="${3:-true}"
    
    if eval "$cmd" &>/dev/null; then
        echo "  ✅ $name"
        PASS=$((PASS + 1))
        return 0
    else
        if [ "$required" = true ]; then
            echo "  ❌ $name (REQUIRED)"
            FAIL=$((FAIL + 1))
        else
            echo "  ⚠️  $name (optional)"
            WARN=$((WARN + 1))
        fi
        return 1
    fi
}

echo "🔍 Dracon Utilities Health Check"
echo "================================"
echo ""

echo "📦 Prerequisites"
check "Rust/Cargo installed" "command -v cargo"
check "Git installed" "command -v git"
check "systemctl available" "command -v systemctl"
check "Bash version >= 4.0" "[[ \${BASH_VERSINFO[0]} -ge 4 ]]"

echo ""
echo "📁 Directory Structure"
check "dracon-libs sibling directory" "[ -d ../dracon-libs ]"
check "dracon-libs/services/ai/ exists" "[ -d ../dracon-libs/services/ai ]"
check "dracon-libs/tools/sync/dracon-git/ exists" "[ -d ../dracon-libs/tools/sync/dracon-git ]"

echo ""
echo "🔧 Binaries"
for binary in dracon-sync dracon-system dracon-warden dracon-ai; do
    if [ -f "target/release/$binary" ]; then
        echo "  ✅ $binary (built)"
        ((PASS++))
    elif [ -f "$HOME/.local/bin/$binary" ]; then
        echo "  ✅ $binary (installed)"
        ((PASS++))
    else
        echo "  ⚠️  $binary (not built or installed)"
        ((WARN++))
    fi
done

echo ""
echo "⚙️  Systemd Services"
for service in dracon-sync.service dracon-system-guard.service dracon-warden.service; do
    if systemctl --user list-unit-files 2>/dev/null | grep -q "^$service"; then
        if systemctl --user is-active "$service" &>/dev/null; then
            echo "  ✅ $service (active)"
            ((PASS++))
        else
            echo "  ⚠️  $service (installed but not running)"
            ((WARN++))
        fi
    else
        echo "  ⚠️  $service (not installed)"
        ((WARN++))
    fi
done

echo ""
echo "📂 Configuration"
for config in \
    "$HOME/.dracon/utilities/sync/dracon-sync.toml" \
    "$HOME/.dracon/utilities/system/dracon-system.toml" \
    "$HOME/.dracon/utilities/warden/dracon-warden.toml"; do
    if [ -f "$config" ]; then
        echo "  ✅ $(basename "$config")"
        ((PASS++))
    else
        echo "  ⚠️  $(basename "$config") (not created yet)"
        ((WARN++))
    fi
done

echo ""
echo "🌐 AI Configuration"
check "AI provider config (ai.toml)" "[ -f $HOME/.dracon/utilities/sync/ai.toml ]" false

echo ""
echo "📝 PATH Check"
if [[ ":$PATH:" == *":$HOME/.local/bin:"* ]]; then
    echo "  ✅ ~/.local/bin is in PATH"
    PASS=$((PASS + 1))
else
    echo "  ⚠️  ~/.local/bin is NOT in PATH"
    echo "     Add: export PATH=\"\$HOME/.local/bin:\$PATH\""
    WARN=$((WARN + 1))
fi

echo ""
echo "================================"
echo "Results: $PASS passed, $WARN warnings, $FAIL failures"

if [ $FAIL -gt 0 ]; then
    echo ""
    echo "❌ Some required checks failed. Please fix the issues above."
    echo "   Run ./install.sh after fixing."
    exit 1
elif [ $WARN -gt 0 ]; then
    echo ""
    echo "⚠️  Some optional checks have warnings. You may want to address them."
    exit 0
else
    echo ""
    echo "✅ All checks passed!"
    exit 0
fi
