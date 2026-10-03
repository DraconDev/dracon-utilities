#!/usr/bin/env bash
set -euo pipefail

# Dracon Utilities Installer
# Usage: ./install.sh [OPTIONS]
#
# Options:
#   --help, -h         Show this help message
#   --dry-run          Show what would be installed without making changes
#   --force            Overwrite existing configs (normally skipped)
#   --upgrade          Stop services, install, restart (default: only restart if running)
#   --verbose, -v      Show more output
#   --no-restart       Don't restart services after install
#   --binaries-only    Only install binaries, skip configs and services
#
# Note: a full install also sets `git config --global init.defaultBranch main`
# (skipped under --binaries-only; --dry-run only reports what would change).
#
# Examples:
#   ./install.sh                    # First install
#   ./install.sh --upgrade          # Update existing installation
#   ./install.sh --dry-run          # Preview what would happen
#   ./install.sh --force            # Overwrite existing configs

cd "$(dirname "$0")"

# Parse arguments
DRY_RUN=false
FORCE=false
UPGRADE=false
VERBOSE=false
NO_RESTART=false
BINARIES_ONLY=false

# Captured once and reused by the unit-file lookups below. See the
# comment at the `restart_service` call site (audit F97) for why the
# list is never piped into `grep -q`.
USER_UNIT_FILES=""
if command -v systemctl &>/dev/null; then
    USER_UNIT_FILES=$(systemctl --user list-unit-files 2>/dev/null || true)
fi

# ADDED 2026-09-28 (D4): which services were RUNNING before this install
# touched anything. `--upgrade` stops them early, so a later live query
# would report "not running" for a daemon this script itself stopped and
# then refuse to bring it back. The "was it running" fact has to be
# captured before the first mutation, exactly like `USER_UNIT_FILES`.
PRE_ACTIVE_SERVICES=""
if command -v systemctl &>/dev/null; then
    for _svc in dracon-sync.service dracon-system-guard.service; do
        if systemctl --user is-active "$_svc" &>/dev/null; then
            PRE_ACTIVE_SERVICES="$PRE_ACTIVE_SERVICES $_svc"
        fi
    done
    unset _svc
fi

# True when the service was running before this install ran.
service_was_running() {
    [[ " $PRE_ACTIVE_SERVICES " == *" $1 "* ]]
}

for arg in "$@"; do
    case "$arg" in
        --help|-h)
            sed -n '3,23p' "$0" | sed 's/^# //; s/^#$//'
            exit 0
            ;;
        --dry-run)
            DRY_RUN=true
            echo "🔍 DRY RUN MODE - No changes will be made"
            echo ""
            ;;
        --force)
            FORCE=true
            ;;
        --upgrade)
            UPGRADE=true
            ;;
        --verbose|-v)
            VERBOSE=true
            ;;
        --no-restart)
            NO_RESTART=true
            ;;
        --binaries-only)
            BINARIES_ONLY=true
            ;;
        *)
            echo "Unknown option: $arg"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

log() {
    if [ "$VERBOSE" = true ] || [ "$DRY_RUN" = true ]; then
        echo "$@"
    fi
}

# Check prerequisites
# FIXED 2026-09-27 (F86): this was `for cmd in cargo` — a loop that can
# only ever run once (shellcheck SC2043). The list is a single prerequisite
# today; keep the loop shape for the day it grows, but silence the lint at
# the source rather than in CI.
# shellcheck disable=SC2043
for cmd in cargo; do
    if ! command -v "$cmd" &> /dev/null; then
        echo "ERROR: Required command '$cmd' not found"
        exit 1
    fi
done

if [ "$BINARIES_ONLY" != true ] && ! command -v systemctl &> /dev/null; then
    echo "ERROR: Required command 'systemctl' not found"
    echo "Use --binaries-only when installing on a system without user systemd."
    exit 1
fi

# This repo is a monorepo; utility source lives in the tracked
# dracon-sync/, dracon-system/, dracon-warden/ directories. Check their
# manifests before starting a build so a partial checkout gets an actionable
# error instead of Cargo's path error.
for utility in dracon-sync dracon-system dracon-warden; do
    if [ ! -f "$utility/Cargo.toml" ]; then
        echo "ERROR: utility directory '$utility' is missing its Cargo.toml"
        echo "You may have a partial checkout — re-clone https://github.com/DraconDev/dracon-utilities.git;"
        echo "see AGENTS.md for the monorepo layout."
        exit 1
    fi
done

# FIXED 2026-10-03 (audit R4-M-06): setting the global default branch is
# config work, not binary installation — a --binaries-only run must not
# rewrite the operator's global git config as a side effect. (Documented
# in --help above.)
if [ "$BINARIES_ONLY" != true ]; then
    # Set git default branch to main (consistent with GitHub convention)
    CURRENT_DEFAULT=$(git config --global init.defaultBranch 2>/dev/null || echo "")
    if [ "$CURRENT_DEFAULT" != "main" ]; then
        if [ "$DRY_RUN" = true ]; then
            echo "Would set git default branch to main (currently: ${CURRENT_DEFAULT:-master})"
        else
            git config --global init.defaultBranch main
            echo "✅ Set git default branch to main (was: ${CURRENT_DEFAULT:-master})"
        fi
    fi
fi

# Stop services if upgrading.  `--no-restart` deliberately leaves the
# currently running image alone; replacing the on-disk executable is safe on
# Unix because the running process holds the old inode open, and the next
# sanctioned restart will load the new binary.
if [ "$UPGRADE" = true ] && [ "$NO_RESTART" != true ]; then
    echo "Stopping services for upgrade..."
    for service in dracon-sync.service dracon-system-guard.service; do
        # Map service name to binary name
        _bin=""
        case "$service" in
            dracon-sync.service)   _bin=dracon-sync ;;
            dracon-system-guard.service) _bin=dracon-system ;;
        esac

        if [ "$DRY_RUN" = true ]; then
            if systemctl --user is-active "$service" &>/dev/null; then
                echo "  Would stop $service (systemctl)"
            fi
            if pgrep -x "$_bin" &>/dev/null; then
                echo "  Would kill $_bin (pkill fallback)"
            fi
        else
            # Stop via systemd (clean shutdown)
            if systemctl --user is-active "$service" &>/dev/null; then
                # FIXED 2026-09-27 (F86): A && B || C is not if-then-else
                # (shellcheck SC2015). The `|| true` existed to swallow a
                # stop failure; an if/then says the same thing explicitly.
                if systemctl --user stop "$service" 2>/dev/null; then
                    echo "  Stopped $service"
                fi
            fi
            # Catch any remaining processes (manual runs, stale)
            if pgrep -x "$_bin" &>/dev/null; then
                pkill -x "$_bin" 2>/dev/null || true
                echo "  Killed $_bin (non-systemd process)"
            fi
        fi
    done
    # Wait for all processes to exit
    sleep 1
    echo ""
fi

echo "Installing dracon utilities to ~/.local/bin/"
# FIXED 2026-09-27 (audit F80): `--dry-run` promises "No changes will be
# made" but these six directory creations ran unconditionally, so a dry
# run still mutated $HOME. Guarded now.
if [ "$DRY_RUN" = true ]; then
    echo "  Would create ~/.local/bin/"
else
    mkdir -p ~/.local/bin
fi

# Clean up orphaned binaries from previous architectures
ORPHANS=(
    dracon-system-guard
    dracon-security-daemon-guard
)
for orphan in "${ORPHANS[@]}"; do
    if [ -f ~/.local/bin/"$orphan" ]; then
        if [ "$DRY_RUN" = true ]; then
            echo "  Would remove orphan: ~/.local/bin/$orphan"
        else
            rm -f ~/.local/bin/"$orphan"
            echo "  🧹 Removed orphan: ~/.local/bin/$orphan"
        fi
    fi
done

# Clean up stale binaries from ~/.cargo/bin (leftover from cargo install)
# These get shadowed by ~/.local/bin but can cause confusion if PATH order varies
CARGO_BIN_STALE=(
    dracon-sync
    dracon-system
    dracon-warden
)
for stale in "${CARGO_BIN_STALE[@]}"; do
    if [ -f ~/.cargo/bin/"$stale" ]; then
        if [ "$DRY_RUN" = true ]; then
            echo "  Would remove stale ~/.cargo/bin/$stale (outdated cargo install artifact)"
        else
            rm -f ~/.cargo/bin/"$stale"
            echo "  🧹 Removed stale ~/.cargo/bin/$stale (outdated cargo install artifact)"
        fi
    fi
done

# Clean up stale .bak files
for bak in ~/.local/bin/dracon-*.bak*; do
    [ -f "$bak" ] || continue
    if [ "$DRY_RUN" = true ]; then
        echo "  Would remove stale backup: $bak"
    else
        rm -f "$bak"
        echo "  🧹 Removed stale backup: $(basename "$bak")"
    fi
done

# Scan PATH for shadowing binaries — any dracon-* in a directory
# other than ~/.local/bin will take priority depending on PATH order.
# This catches stale installs in /usr/local/bin, ~/bin, etc.
#
# HARDENED 2026-09-28 (incident while shipping D4): this scan deleted the
# LIVE `~/.local/bin/dracon-warden` when the script ran with a HOME that
# was not the operator's (a sandbox/test HOME, a service account, `sudo`
# without `-H`). The install target `$HOME/.local/bin` then did not match
# the real home's bin directory, so the operator's own installed binaries
# looked like "shadowing" and were removed — which broke every filtered
# git operation fleet-wide ("dracon-warden: command not found", daemon
# classification streaks into the teens). Two guards, both cheap:
#
#   1. the REAL home's `~/.local/bin` (from the passwd entry, not `$HOME`)
#      is never a shadowing target;
#   2. a binary that a live user unit executes is never removed, whatever
#      directory it lives in.
IFS=':' read -ra _path_dirs <<< "$PATH"
REAL_HOME=""
if command -v getent &>/dev/null; then
    REAL_HOME=$(getent passwd "$(id -un)" 2>/dev/null | cut -d: -f6)
fi
# Absolute ExecStart paths of the live user units, one per line.
# Parsed with `path=\([^;]*\) ;` rather than `grep -oE '/[^ ;{}]*'`: a
# systemd `--value` line is `{ path=<exe> ; argv[]=<exe> <args> ; … }`, and
# the old `[^ ;]*` class silently TRUNCATED a path containing a space to a
# prefix that then matched nothing — the guard failed open and deleted the
# very binary it exists to protect. `[^;]*` keeps spaces and still stops at
# the field separator; a literal `;` inside an ExecStart path is not
# representable in systemd's own output anyway.
UNIT_EXEC_PATHS=""
for _u in dracon-sync.service dracon-system-guard.service; do
    if command -v systemctl &>/dev/null; then
        UNIT_EXEC_PATHS="$UNIT_EXEC_PATHS$(systemctl --user show "$_u" -p ExecStart --value 2>/dev/null | sed -n 's/.*path=\([^;]*\) ;.*/\1/p' || true)
"
    fi
done
unset _u
for _dir in "${_path_dirs[@]}"; do
    [ "$_dir" = "$HOME/.local/bin" ] && continue
    [ -n "$REAL_HOME" ] && [ "$_dir" = "$REAL_HOME/.local/bin" ] && continue
    for _stale in "$_dir"/dracon-sync "$_dir"/dracon-system "$_dir"/dracon-warden; do
        [ -f "$_stale" ] || continue
        if grep -qxF "$_stale" <<< "$UNIT_EXEC_PATHS"; then
            echo "  ⚠️  Skipped $_stale — a live systemd unit runs it (remove it by hand if it is stale)"
            continue
        fi
        if [ "$DRY_RUN" = true ]; then
            echo "  Would remove shadowing binary: $_stale"
        else
            # FIXED 2026-09-27 (audit F81): a bare `rm -f` under
            # `set -e` aborted the WHOLE installer with a bare
            # "Permission denied" when a PATH directory was not
            # writable (a root-owned /usr/local/bin/dracon-sync, for
            # instance) — nothing got installed and the message never
            # said which path failed. Warn and continue instead; the
            # binary we are about to install still wins whenever
            # ~/.local/bin precedes the shadowing directory in PATH.
            if rm -f "$_stale" 2>/dev/null; then
                echo "  🧹 Removed shadowing binary: $_stale"
            else
                echo "  ⚠️  Could not remove shadowing binary: $_stale (not writable)"
                echo "     If the installed version looks stale, remove it manually or"
                echo "     check the PATH order (this one shadows ~/.local/bin)."
            fi
        fi
    done
done
echo ""

# Build with release and install manually for feature control
RESTARTED_SERVICES=""

install_binary() {
    local package=$1
    local features=$2
    local subdir=$3
    local binary=${package%%@*}  # strip version suffix if present

    echo "Building $package..."
    local bin_path="target/release/$binary"

    if [ "$DRY_RUN" = true ]; then
        echo "  Would build $package → ~/.local/bin/$binary"
        return 0
    fi

    # FIXED 2026-10-02 (audit M11): every build is --locked. A floating
    # build bypasses the Cargo.lock/deny pin chain CI enforces, so the
    # installed binary drifts from the tested one (the
    # installed-binary-drops-patch incident class). A stale lock fails
    # loudly: --locked errors instead of silently resolving, and the last
    # attempt in each chain runs without 2>/dev/null so the error is
    # visible. Regenerate the lock explicitly (cargo update -w) instead
    # of installing past it.
    if [ -n "$features" ]; then
        (cd "$subdir" && cargo build --locked --release --package "$package" --features "$features" 2>/dev/null) || \
        (cd "$subdir" && cargo build --locked --release -p "$package" 2>/dev/null) || \
        (cd "$subdir" && cargo build --locked --release -p "$package")
    else
        (cd "$subdir" && cargo build --locked --release -p "$package")
    fi

    local resolved=""
    if [ -f "$subdir/$bin_path" ]; then
        resolved="$subdir/$bin_path"
    elif [ -f "$bin_path" ]; then
        resolved="$bin_path"
    fi

    if [ -n "$resolved" ]; then
        local installed=~/.local/bin/$binary
        local new_hash
        new_hash=$(md5sum "$resolved" | cut -d' ' -f1)

        # FIXED 2026-10-03 (audit R3-L20): the success echo used to
        # print HERE, before the cp/mv below — a copy failure left a
        # stale `.<binary>.$$` dotfile AND a log line already claiming
        # success. Record the kind now, echo only after `mv` lands.
        local install_kind="new"
        if [ -f "$installed" ]; then
            local old_hash
            old_hash=$(md5sum "$installed" | cut -d' ' -f1)
            if [ "$new_hash" = "$old_hash" ]; then
                echo "  ⏭️  ~/.local/bin/$binary unchanged (same hash)"
                return 0
            else
                install_kind="updated"
            fi
        fi

        # A running Unix process keeps its executable inode open, so an
        # in-place path replacement does not require a stop.  Stop/restart
        # only when the caller requested a live upgrade; --no-restart must
        # never quiesce a service as an incidental side effect.
        local svc_name=""
        case "$binary" in
            dracon-sync)   svc_name=dracon-sync.service ;;
            dracon-system) svc_name=dracon-system-guard.service ;;
            # Warden has no daemon — hooks are the primary enforcement layer
        esac

        # CHANGED 2026-09-28 (D4): this block used to be gated on
        # `NO_RESTART != true` alone, so a plain `./install.sh` — the
        # first-install and routine-update path — stopped every service,
        # pkill-ed its binary, and started it again. The help text promises
        # `--upgrade  Stop services, install, restart (default: only
        # restart if running)`, so a plain install was quietly doing the
        # one thing the operator must not get by accident: resurrecting a
        # service they deliberately stopped.
        #
        # A plain install therefore touches NO service state. Only
        # `--upgrade` quiesces, and even then the restart is gated on
        # `systemctl --user is-active` captured BEFORE the stop, so a
        # service the operator had stopped stays stopped.
        local svc_was_active=false
        if [ "$UPGRADE" = true ] && [ "$NO_RESTART" != true ] && [ -n "$svc_name" ]; then
            if service_was_running "$svc_name"; then
                svc_was_active=true
                systemctl --user stop "$svc_name" 2>/dev/null || true
            else
                echo "  ⏭️  $svc_name was not running before this install — left stopped"
            fi
        fi
        if [ "$UPGRADE" = true ] && [ "$NO_RESTART" != true ]; then
            pkill -x "$binary" 2>/dev/null || true
            # Wait for process to fully exit (up to 3s)
            for _ in $(seq 1 6); do
                pgrep -x "$binary" &>/dev/null || break
                sleep 0.5
            done
        fi

        # Install atomically (FIXED 2026-10-03, audit L12): the old
        # `rm -f` + `cp` left a window where a concurrent git filter
        # spawn exec-ing the live binary failed (missing/half-written
        # file), wedging add/checkout mid-install. Same-dir rename is
        # atomic; mirrors the warden hook installer's temp-then-rename.
        tmp_bin=~/.local/bin/."$binary".$$
        # R3-L20: any step failing removes the temp dotfile (no stale
        # hidden file) and aborts loud — never a premature success line.
        if cp "$resolved" "$tmp_bin" \
            && chmod +x "$tmp_bin" \
            && mv -f "$tmp_bin" ~/.local/bin/"$binary"; then
            echo "  ✅ Installed ~/.local/bin/$binary ($install_kind)"
        else
            rm -f "$tmp_bin"
            echo "  ❌ Failed to install ~/.local/bin/$binary" >&2
            return 1
        fi

        # Restart only what `--upgrade` found running. An operator-stopped
        # service is never resurrected here; the final `restart_service`
        # block applies the same rule.
        if [ "$UPGRADE" = true ] && [ "$NO_RESTART" != true ] && [ "$svc_was_active" = true ]; then
            systemctl --user start "$svc_name" 2>/dev/null || true
            RESTARTED_SERVICES="$RESTARTED_SERVICES $svc_name"
        fi

        # Warn if debug build is newer than release — developer may have uninstalled changes
        local debug_path=""
        if [ -f "$subdir/target/debug/$binary" ]; then
            debug_path="$subdir/target/debug/$binary"
        elif [ -f "target/debug/$binary" ]; then
            debug_path="target/debug/$binary"
        fi
        if [ -n "$debug_path" ] && [ "$debug_path" -nt "$resolved" ]; then
            echo "  ⚠️  WARNING: target/debug/$binary is NEWER than target/release/$binary"
            echo "     You may have code changes that aren't in this release build."
            echo "     Run './install.sh' again after your changes to pick them up."
        fi
    else
        echo "  ❌ ERROR: Could not find binary for $package (checked $subdir/$bin_path and $bin_path)"
        return 1
    fi
}

# Install all binaries
install_binary dracon-sync "" "dracon-sync"
install_binary dracon-system "" "dracon-system"
install_binary dracon-warden "" "dracon-warden"

echo ""

# Check if ~/.local/bin is in PATH
if [[ ":$PATH:" != *":$HOME/.local/bin:"* ]]; then
    echo "⚠️  WARNING: ~/.local/bin is not in your PATH"
    echo "   Add this to your shell config to use dracon utilities:"
    # shellcheck disable=SC2016  # intentional: show literal $HOME in advice to user
    echo '   export PATH="$HOME/.local/bin:$PATH"'
    echo ""
fi

if [ "$BINARIES_ONLY" = true ]; then
    echo "✅ Binaries installed. Skipping configs and services (--binaries-only)."
    exit 0
fi

# Install warden git hooks (pre-commit + pre-push enforcement)
if [ "$DRY_RUN" = true ]; then
    echo "Would install warden git hooks via: dracon-warden setup-hooks --global"
else
    if command -v dracon-warden &>/dev/null || [ -f ~/.local/bin/dracon-warden ]; then
        ~/.local/bin/dracon-warden setup-hooks --global 2>/dev/null || \
            echo "⚠️  Could not install warden hooks (run manually: dracon-warden setup-hooks)"
    else
        echo "⚠️  dracon-warden not found, skipping hook installation"
    fi
fi

# Install systemd service files
# FIXED 2026-09-27 (audit F80): these four mkdir -p calls ran before the
# DRY_RUN branch below, so a dry run created them anyway.
if [ "$DRY_RUN" = true ]; then
    echo "Would install systemd services to ~/.config/systemd/user/"
    echo "Would install watchdog timers + scripts and enable the timers"
    echo "Would create config directories under ~/.dracon/utilities/"
else
    mkdir -p ~/.config/systemd/user
    mkdir -p ~/.dracon/utilities/sync
    mkdir -p ~/.dracon/utilities/system
    mkdir -p ~/.dracon/utilities/warden
    # FIXED 2026-10-03 (audit R3-L32): every copy below used to swallow
    # its failure (`2>/dev/null || true`) while the block printed
    # success regardless — a full disk or bad perms surfaced only
    # obliquely via the timer-enable warning, or never (main
    # services). Track each failure and abort loud, naming them.
    copy_failures=""
    copy_unit() {
        cp "$1" "$2" 2>/dev/null || copy_failures="$copy_failures
  ❌ $1 -> $2"
    }
    copy_unit dracon-sync/dracon-sync.service ~/.config/systemd/user/dracon-sync.service
    copy_unit dracon-system/dracon-system-guard.service ~/.config/systemd/user/dracon-system-guard.service
    # ADDED 2026-10-02 (audit M8): the watchdog timers + scripts were
    # live-only, so fresh installs silently lacked restart-if-stopped
    # and freeze auto-clear. Ship them like the services.
    mkdir -p ~/.dracon/sync-notify ~/.dracon/system-notify
    copy_unit dracon-sync/dracon-sync-watchdog.service ~/.config/systemd/user/dracon-sync-watchdog.service
    copy_unit dracon-sync/dracon-sync-watchdog.timer ~/.config/systemd/user/dracon-sync-watchdog.timer
    copy_unit dracon-sync/dracon-freeze-watchdog.service ~/.config/systemd/user/dracon-freeze-watchdog.service
    copy_unit dracon-sync/dracon-freeze-watchdog.timer ~/.config/systemd/user/dracon-freeze-watchdog.timer
    copy_unit dracon-system/dracon-system-guard-watchdog.service ~/.config/systemd/user/dracon-system-guard-watchdog.service
    copy_unit dracon-system/dracon-system-guard-watchdog.timer ~/.config/systemd/user/dracon-system-guard-watchdog.timer
    copy_unit dracon-sync/scripts/dracon-sync-watchdog.sh ~/.dracon/sync-notify/dracon-sync-watchdog.sh
    copy_unit dracon-sync/scripts/dracon-freeze-watchdog.sh ~/.dracon/sync-notify/dracon-freeze-watchdog.sh
    copy_unit dracon-system/scripts/dracon-system-guard-watchdog.sh ~/.dracon/system-notify/dracon-system-guard-watchdog.sh
    chmod +x ~/.dracon/sync-notify/dracon-sync-watchdog.sh ~/.dracon/sync-notify/dracon-freeze-watchdog.sh ~/.dracon/system-notify/dracon-system-guard-watchdog.sh 2>/dev/null || copy_failures="$copy_failures
  ❌ chmod +x watchdog scripts"
    if [ -n "$copy_failures" ]; then
        echo "  ❌ unit/script install failed:$copy_failures" >&2
        echo "  fix the cause above (disk full? permissions?) and re-run install.sh" >&2
        exit 1
    fi
    systemctl --user daemon-reload 2>/dev/null || true
    # Wait for systemd to settle after daemon-reload
    sleep 1
    echo "✅ Systemd services installed"
    if systemctl --user enable --now dracon-sync-watchdog.timer dracon-freeze-watchdog.timer dracon-system-guard-watchdog.timer 2>/dev/null; then
        echo "✅ Watchdog timers enabled and started"
    else
        echo "  ⚠️ Could not enable watchdog timers (enable manually: systemctl --user enable --now dracon-sync-watchdog.timer dracon-freeze-watchdog.timer dracon-system-guard-watchdog.timer)"
    fi
fi

# Copy example configs
copy_config() {
    local src="$1"
    local dest="$2"
    
    if [ ! -f "$src" ]; then
        log "  Source not found: $src"
        return 0
    fi
    
    if [ -f "$dest" ] && [ "$FORCE" = false ]; then
        log "  Skipping $dest (already exists, use --force to overwrite)"
        return 0
    fi
    
    if [ "$DRY_RUN" = true ]; then
        echo "  Would copy $(basename "$src") → $dest"
    else
        cp "$src" "$dest"
        echo "  ✅ Copied $(basename "$src") → $dest"
    fi
}

echo ""
echo "Installing example configs..."
copy_config "dracon-sync/dracon-sync.example.toml" "$HOME/.dracon/utilities/sync/dracon-sync.toml"
copy_config "dracon-system/dracon-system.example.toml" "$HOME/.dracon/utilities/system/dracon-system.toml"
copy_config "dracon-warden/dracon-warden.example.toml" "$HOME/.dracon/utilities/warden/dracon-warden.toml"
# FIXED 2026-09-27 (audit F79): doctor.sh checks for this file, but
# nothing ever installed it, so the check was a permanent false WARN.
copy_config "dracon-sync/ai.example.toml" "$HOME/.dracon/utilities/sync/ai.toml"


# Create secrets directories with correct permissions
# FIXED 2026-09-27 (audit F80): also mutated $HOME under --dry-run.
if [ "$DRY_RUN" = true ]; then
    echo "  Would create ~/.dracon/utilities/sync/secrets (mode 700)"
else
    mkdir -p "$HOME/.dracon/utilities/sync/secrets"
    chmod 700 "$HOME/.dracon/utilities/sync/secrets" 2>/dev/null || true
fi

if [ "$NO_RESTART" = true ]; then
    echo ""
    echo "✅ Installation complete. Services NOT restarted (--no-restart)."
    echo "   Run 'systemctl --user restart dracon-sync.service' etc. to start."
    exit 0
fi

# Restart services
echo ""
echo "Restarting services..."

restart_service() {
    local service=$1

    # Skip if already restarted during binary install
    if [[ " $RESTARTED_SERVICES " == *" $service "* ]]; then
        if [ "$VERBOSE" = true ]; then
            echo "  ⏭️  $service already restarted during install"
        fi
        return 0
    fi

    if [ "$DRY_RUN" = true ]; then
        echo "  Would restart $service"
        return 0
    fi

    # CHANGED 2026-09-28 (D4): this block restarted every ENABLED unit, so
    # a plain `./install.sh` resurrected a service the operator had
    # deliberately stopped — the exact opposite of the help text's
    # "(default: only restart if running)". The rule now:
    #
    #   * unit existed and was RUNNING         -> restart it (the
    #     "only restart if running" promise);
    #   * unit existed and was NOT running     -> leave it stopped, and say
    #     so. A stopped daemon is a deliberate operator state; resurrecting
    #     it is the surprise this removes.
    #   * unit is NEW (fresh install)          -> enable --now it (R3-L30,
    #     2026-10-03). No prior state exists to preserve; leaving it
    #     disabled contradicts D4 (it only came up via the M8 backstop).
    #
    # "Was running" comes from the pre-install snapshot, not a live query:
    # `--upgrade` stops the service itself before reaching here, and a
    # live `is-active` would then read "stopped" and strand a daemon this
    # script quiesced two minutes earlier.
    local unit_existed=false
    if grep -q "^$service" <<< "$USER_UNIT_FILES"; then
        unit_existed=true
    fi

    # FIXED 2026-10-03 (audit R3-L30): a FRESH install (unit absent from
    # the pre-snapshot) has no "deliberately stopped" state to preserve
    # (D4), so enable + start. The old code fell through to a "not
    # found" line and left the service disabled — it only came up ~2.5
    # min later via the M8 watchdog backstop, contradicting the D4
    # model (and "not found" misreported a unit this script just
    # installed). Enable failure stays loud, never silent.
    if [ "$unit_existed" = false ]; then
        if systemctl --user enable --now "$service" 2>/dev/null; then
            echo "  ✅ $service enabled and started (fresh install)"
        else
            echo "  ⚠️ Could not enable --now $service (enable manually: systemctl --user enable --now $service)"
        fi
        return 0
    fi

    if ! service_was_running "$service"; then
        echo "  ⏭️  $service was not running before this install — left as-is (start it with: systemctl --user start $service)"
        return 0
    fi

    # FIXED 2026-09-27 (audit F97): this used to pipe a live
    # `systemctl --user list-unit-files` into `grep -q`, which is a
    # false-negative race under `set -o pipefail` (grep -q exits on the
    # first match → systemctl takes SIGPIPE → exit 141 → the pipeline
    # reports "not found" even when the unit exists). The unit list is
    # captured once at the top of the script and grepped as a string
    # (see `unit_existed` above, which also settled the fresh-install
    # arm) — reaching here means the unit existed AND was running, so
    # restart unconditionally.
    systemctl --user restart "$service" 2>/dev/null && echo "  ✅ $service restarted" || echo "  ⚠️ Could not restart $service"
}

restart_service dracon-sync.service
restart_service dracon-system-guard.service
# Warden has no daemon — git hooks (pre-commit + pre-push) are the primary
# enforcement layer and are installed above via `dracon-warden setup-hooks --global`.

echo ""
echo "✅ Installation complete!"
echo ""
echo "Binaries:"
ls -la ~/.local/bin/dracon-* 2>/dev/null || true
echo ""
echo "Checksums:"
for bin in ~/.local/bin/dracon-*; do
    # FIXED 2026-09-27 (F86): A && B || C (shellcheck SC2015); the
    # `|| true` swallowed a missing-file race, an if/then does too.
    if [ -f "$bin" ]; then
        sha256sum "$bin" 2>/dev/null || true
    fi
done

# Verify running daemons are using the installed binary
VERIFY_OK=true
for bin in dracon-sync dracon-system dracon-warden; do
    pid=$(pgrep -x "$bin" 2>/dev/null | head -1 || true)
    [ -n "$pid" ] || continue
    running=$(readlink "/proc/$pid/exe" 2>/dev/null)
    expected="$HOME/.local/bin/$bin"
    if [ -n "$running" ] && [ "$running" != "$expected" ]; then
        echo "⚠️  WARNING: $bin (PID $pid) running from $running, not $expected"
        echo "   This means a stale version is still active. Restart the service:"
        svc=""
        # FIXED 2026-09-27 (F86): shellcheck parsed `$bin[` as an array
        # expansion (SC1087, error class). Brace the variable so the
        # literal `[` is unambiguously part of the grep pattern.
        svc=$(systemctl --user list-units --type=service --state=running | grep -oE "${bin}[^ ]*\.service" | head -1 || true)
        if [ -n "$svc" ]; then
            echo "   systemctl --user restart $svc"
        else
            echo "   systemctl --user restart <service-name>"
        fi
        VERIFY_OK=false
    fi
done
if [ "$VERIFY_OK" = true ]; then
    echo "✅ All running daemons verified at ~/.local/bin/"
fi
echo ""
echo "Next steps:"
echo "  1. Warden hooks are installed globally (pre-commit + pre-push)"
echo "  2. Add registry tokens to ~/.dracon/utilities/sync/secrets/*.env (crates.io, npm, etc.)"
echo "  3. Check 'dracon-sync status' to verify sync is working"
echo "  4. Check 'dracon-system status' to verify guard is working"
