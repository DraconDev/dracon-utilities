#!/usr/bin/env bash
set -euo pipefail

# Dracon Utilities Doctor
# Checks prerequisites and diagnoses common issues

cd "$(dirname "$0")"

PASS=0
FAIL=0
WARN=0

# FIXED 2026-09-27 (audit F77): this function used to end the failure branch
# with `return 1`. Every call site is a bare statement, so under
# `set -euo pipefail` the FIRST failing check killed the whole script —
# the Results summary, the "Some required checks failed" branch, and
# every section after the failure were unreachable, and $FAIL could only
# ever be 0. Failures are now accumulated in the counters and the
# function always returns 0, so a full report is always produced.
check() {
    local name="$1"
    local cmd="$2"
    local required="${3:-true}"

    if eval "$cmd" &>/dev/null; then
        echo "  ✅ $name"
        PASS=$((PASS + 1))
    else
        if [ "$required" = true ]; then
            echo "  ❌ $name (REQUIRED)"
            FAIL=$((FAIL + 1))
        else
            echo "  ⚠️  $name (optional)"
            WARN=$((WARN + 1))
        fi
    fi
    return 0
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
check "workspace manifest" "[ -f Cargo.toml ]"
for utility in dracon-sync dracon-system dracon-warden; do
    check "$utility sources present" "[ -f \"$utility/Cargo.toml\" ]"
done

echo ""
echo "🔧 Binaries"
for binary in dracon-sync dracon-system dracon-warden; do
    # FIXED 2026-09-27 (audit F78): `((VAR++))` is an arithmetic COMMAND,
    # not an assignment — when the pre-increment value is 0 it evaluates
    # to 0, exits 1, and `set -e` aborted the doctor mid-section. Use the
    # assignment form, which always exits 0.
    if [ -f "target/release/$binary" ]; then
        echo "  ✅ $binary (built)"
        PASS=$((PASS + 1))
    elif [ -f "$HOME/.local/bin/$binary" ]; then
        echo "  ✅ $binary (installed)"
        PASS=$((PASS + 1))
    else
        echo "  ⚠️  $binary (not built or installed)"
        WARN=$((WARN + 1))
    fi
done

echo ""
echo "⚙️  Systemd Services"
# FIXED 2026-09-27 (audit F97): the unit-file list was piped straight
# into `grep -q`. `grep -q` exits on the first match, closing the read
# end of the pipe; if systemctl was still writing it takes SIGPIPE and
# exits 141, and `set -o pipefail` turns the whole pipeline into a
# false negative. Measured 4/20 false "not installed" on this machine.
# The list is captured once instead, so the grep reads a string.
USER_UNIT_FILES=$(systemctl --user list-unit-files 2>/dev/null || true)
for service in dracon-sync.service dracon-system-guard.service; do
    if grep -q "^$service" <<< "$USER_UNIT_FILES"; then
        if systemctl --user is-active "$service" &>/dev/null; then
            echo "  ✅ $service (active)"
            PASS=$((PASS + 1))
        else
            echo "  ⚠️  $service (installed but not running)"
            WARN=$((WARN + 1))
        fi
    else
        echo "  ⚠️  $service (not installed)"
        WARN=$((WARN + 1))
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
        PASS=$((PASS + 1))
    else
        echo "  ⚠️  $(basename "$config") (not created yet)"
        WARN=$((WARN + 1))
    fi
done

echo ""
echo "🌐 AI Configuration"
# FIXED 2026-09-27 (audit F79): install.sh now copies
# dracon-sync/ai.example.toml here, so this check can pass. Before that the
# check was a permanent false WARN — install.sh shipped only the three
# dracon-*.example.toml files and never installed an ai.toml.
check "AI provider config (ai.toml)" "[ -f \"$HOME/.dracon/utilities/sync/ai.toml\" ]" false

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
