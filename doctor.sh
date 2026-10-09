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
echo "⚙️  Systemd Units & Timers"
# FIXED 2026-10-09 (audit F127): install.sh has installed 8 unit files and 3
# watchdog scripts since M8 (2026-10-02), while doctor.sh still listed only
# dracon-sync.service and dracon-system-guard.service — so a failed copy_unit
# or a disabled watchdog timer produced no signal here. uninstall.sh already
# documents the consequence (timers firing every 2 min against removed units),
# and the watchdog timers are the only backstop for a manually stopped or
# disabled guard/sync service. Every unit the installer copies is now
# reported: services must be ACTIVE, timers ENABLED *and* ACTIVE.
#
# FIXED 2026-09-27 (audit F97): the unit-file list is captured ONCE and
# grepped as a string. Piping it into `grep -q` closes the read end on the
# first match, so a still-writing systemctl takes SIGPIPE, exits 141, and
# under `set -o pipefail` the whole pipeline reports "not installed"
# (measured 4/20 false negatives on this machine).
USER_UNIT_FILES=$(systemctl --user list-unit-files 2>/dev/null || true)
for unit in dracon-sync.service dracon-system-guard.service \
    dracon-sync-watchdog.service dracon-sync-watchdog.timer \
    dracon-freeze-watchdog.service dracon-freeze-watchdog.timer \
    dracon-system-guard-watchdog.service dracon-system-guard-watchdog.timer; do
    if ! grep -q "^$unit" <<< "$USER_UNIT_FILES"; then
        echo "  ⚠️  $unit (not installed)"
        WARN=$((WARN + 1))
        continue
    fi
    case "$unit" in
        *.timer)
            # A timer that is installed but not enabled never fires: that is
            # the M8 regression this check exists to surface.
            if systemctl --user is-enabled "$unit" &>/dev/null \
                && systemctl --user is-active "$unit" &>/dev/null; then
                echo "  ✅ $unit (enabled + active)"
                PASS=$((PASS + 1))
            else
                echo "  ⚠️  $unit (installed but not enabled/active — its backstop will never fire)"
                WARN=$((WARN + 1))
            fi
            ;;
        *)
            if systemctl --user is-active "$unit" &>/dev/null; then
                echo "  ✅ $unit (active)"
                PASS=$((PASS + 1))
            else
                echo "  ⚠️  $unit (installed but not running)"
                WARN=$((WARN + 1))
            fi
            ;;
    esac
done

echo ""
echo "🛡️  Watchdog Scripts"
# The three scripts the installer chmod +x into ~/.dracon/{sync,system}-notify.
# A unit whose script is missing or not executable fails at run time with no
# install-time signal, so the doctor checks existence AND the exec bit.
for script in \
    "$HOME/.dracon/sync-notify/dracon-sync-watchdog.sh" \
    "$HOME/.dracon/sync-notify/dracon-freeze-watchdog.sh" \
    "$HOME/.dracon/system-notify/dracon-system-guard-watchdog.sh"; do
    script_name=$(basename "$script")
    if [ -x "$script" ]; then
        echo "  ✅ $script_name (installed + executable)"
        PASS=$((PASS + 1))
    elif [ -f "$script" ]; then
        echo "  ⚠️  $script_name (present but NOT executable — its timer will fail to run it)"
        WARN=$((WARN + 1))
    else
        echo "  ⚠️  $script_name (not installed)"
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
