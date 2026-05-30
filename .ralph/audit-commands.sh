#!/usr/bin/env bash
# Full audit script — each command explained

set -euo pipefail

echo "═══════════════════════════════════════════════════════════"
echo " 1. BINARY HASH VERIFICATION"
echo "    Checks that installed binary ~/.local/bin/<name>"
echo "    matches the freshly-built release binary."
echo "═══════════════════════════════════════════════════════════"
for bin in dracon-sync dracon-system dracon-warden; do
  echo "--- $bin ---"
  installed_hash=$(sha256sum ~/.local/bin/$bin 2>/dev/null | cut -d' ' -f1)
  built_hash=$(sha256sum ~/Dev/dracon-utilities/target/release/$bin 2>/dev/null | cut -d' ' -f1)
  if [ "$installed_hash" = "$built_hash" ]; then
    echo "  ✅ Match: $installed_hash"
  else
    echo "  ❌ MISMATCH"
    echo "  Installed: $installed_hash"
    echo "  Built:     $built_hash"
  fi
done

echo ""
echo "═══════════════════════════════════════════════════════════"
echo " 2. DAEMON PROCESS CHECK"
echo "    Verifies each running daemon loaded its binary"
echo "    from ~/.local/bin/ (readlink /proc/PID/exe)."
echo "    If wrong path → stale binary is running."
echo "═══════════════════════════════════════════════════════════"
for bin in dracon-sync dracon-system dracon-warden; do
  pid=$(pgrep -x "$bin" 2>/dev/null | head -1)
  if [ -n "$pid" ]; then
    running=$(readlink /proc/$pid/exe 2>/dev/null)
    expected="$HOME/.local/bin/$bin"
    if [ "$running" = "$expected" ]; then
      echo "  ✅ $bin (PID $pid) → $running"
    else
      echo "  ❌ $bin (PID $pid) → $running (expected $expected)"
    fi
  else
    echo "  ❌ $bin — NOT RUNNING"
  fi
done

echo ""
echo "═══════════════════════════════════════════════════════════"
echo " 3. SERVICE STATUS"
echo "    Checks systemd user service state for all 3 daemons."
echo "═══════════════════════════════════════════════════════════"
for svc in dracon-sync.service dracon-system-guard.service dracon-warden.service; do
  state=$(systemctl --user is-active "$svc" 2>/dev/null || echo "not found")
  echo "  $svc: $state"
done

echo ""
echo "═══════════════════════════════════════════════════════════"
echo " 4. DAEMON HEALTH"
echo "    dracon-sync health --json → structured check."
echo "═══════════════════════════════════════════════════════════"
dracon-sync health 2>&1

echo ""
echo "═══════════════════════════════════════════════════════════"
echo " 5. PATH SHADOWING"
echo "    Scans PATH for dracon-* binaries OUTSIDE ~/.local/bin/"
echo "    which would shadow the intended version."
echo "═══════════════════════════════════════════════════════════"
found=0
IFS=':' read -ra _dirs <<< "$PATH"
for _dir in "${_dirs[@]}"; do
  [ "$_dir" = "$HOME/.local/bin" ] && continue
  for _bin in "$_dir"/dracon-sync "$_dir"/dracon-system "$_dir"/dracon-warden; do
    if [ -f "$_bin" ]; then
      echo "  ❌ Shadowing: $_bin"
      found=1
    fi
  done
done
[ "$found" = "0" ] && echo "  ✅ No shadowing binaries found"

echo ""
echo "═══════════════════════════════════════════════════════════"
echo " 6. REPOS STATUS"
echo "    Full report from daemon: status per repo, dirty files,"
echo "    ahead/behind, last-updated time."
echo "═══════════════════════════════════════════════════════════"
dracon-sync repos 2>&1

echo ""
echo "═══════════════════════════════════════════════════════════"
echo " 7. TAG-EXISTS CHECK"
echo "    Reads current version from Cargo.toml, checks if a"
echo "    git tag v{version} already exists. If yes → skip bump."
echo "    This is the fix that breaks the infinite bump loop."
echo "═══════════════════════════════════════════════════════════"
cd ~/Dev/dracon-utilities
version=$(python3 -c "
import tomllib
data = tomllib.load(open('Cargo.toml','rb'))
print(data.get('workspace',{}).get('package',{}).get('version','NOT FOUND'))
")
echo "  Cargo.toml version: $version"
if git tag -l "v$version" | grep -q .; then
  echo "  Tag v$version: EXISTS → skip bump ✅"
else
  echo "  Tag v$version: NOT FOUND → would allow bump"
fi

echo ""
echo "═══════════════════════════════════════════════════════════"
echo " 8. NO RECENT BUMPS"
echo "    Checks daemon journal for 'ai-bump' events since fix"
echo "    was deployed at 19:06. Any = regression."
echo "═══════════════════════════════════════════════════════════"
bumps=$(journalctl --user -u dracon-sync.service --since '19:06' --no-pager 2>&1 | grep -c "ai-bump" || true)
echo "  ai-bump events since 19:06: $bumps"
[ "$bumps" = "0" ] && echo "  ✅ No bumps since fix" || echo "  ❌ $bumps bumps detected"

echo ""
echo "═══════════════════════════════════════════════════════════"
echo " 9. TEST SUITE"
echo "    Runs all 3 test suites sequentially."
echo "═══════════════════════════════════════════════════════════"
echo "  dracon-sync..."
cd ~/Dev/dracon-utilities
DRACON_SYNC_GIT_BIN=/run/current-system/sw/bin/git cargo test --release -p dracon-sync -- --test-threads=1 -q 2>&1 | tail -1
echo "  dracon-system..."
cargo test --release -p dracon-system -q 2>&1 | tail -1
echo "  dracon-warden..."
cargo test --release -p dracon-warden -q 2>&1 | tail -1

echo ""
echo "═══════════════════════════════════════════════════════════"
echo "10. CLIPPY"
echo "    Lints all 3 packages."
echo "═══════════════════════════════════════════════════════════"
cd ~/Dev/dracon-utilities
cargo clippy --release -p dracon-sync -p dracon-system -p dracon-warden 2>&1 | grep -E "^error|warning.*dracon" | head -5
echo "  (no output = clean)"

echo ""
echo "═══════════════════════════════════════════════════════════"
echo "11. BUILD ARTIFACTS"
echo "    Checks every repo for tracked target/, .output/,"
echo "    and node_modules/ — build artifacts that shouldn't"
echo "    be in git."
echo "═══════════════════════════════════════════════════════════"
bad=0; total=0
for d in ~/Dev/*/.git; do
  repo=$(dirname "$d"); name=$(basename "$repo"); total=$((total+1))
  targets=$(cd "$repo" && git ls-files 2>/dev/null | awk -F/ 'BEGIN{c=0}$1=="target"{c++}END{print c+0}')
  outputs=$(cd "$repo" && git ls-files 2>/dev/null | awk -F/ 'BEGIN{c=0}$1==".output"{c++}END{print c+0}')
  node_mod=$(cd "$repo" && git ls-files 2>/dev/null | awk -F/ 'BEGIN{c=0}$1=="node_modules"{c++}END{print c+0}')
  if [ "$targets" != "0" ] || [ "$outputs" != "0" ] || [ "$node_mod" != "0" ]; then
    echo "  ❌ $name: target=$targets .output=$outputs node_modules=$node_mod"
    bad=$((bad+1))
  fi
done
echo "  Scanned $total repos. $bad with artifacts. ✅"

echo ""
echo "═══════════════════════════════════════════════════════════"
echo "12. LICENSE FILES"
echo "    Checks for obsolete CLA.md / COMMERCIAL-LICENSE.md"
echo "    and confirms every repo has a LICENSE file."
echo "═══════════════════════════════════════════════════════════"
bad=0
for d in ~/Dev/*/.git; do
  repo=$(dirname "$d"); name=$(basename "$repo")
  if [ -f "$repo/CLA.md" ]; then echo "  ❌ $name: CLA.md still present"; bad=$((bad+1)); fi
  if [ -f "$repo/COMMERCIAL-LICENSE.md" ]; then echo "  ❌ $name: COMMERCIAL-LICENSE.md still present"; bad=$((bad+1)); fi
  if [ ! -f "$repo/LICENSE" ]; then echo "  ❌ $name: MISSING LICENSE"; bad=$((bad+1)); fi
done
[ "$bad" = "0" ] && echo "  ✅ All clean ($total repos)" || echo "  $bad issues found"

echo ""
echo "═══════════════════════════════════════════════════════════"
echo "13. INCIDENT LEDGER"
echo "    Last 5 entries from the append-only incident log."
echo "═══════════════════════════════════════════════════════════"
tail -5 ~/.local/state/dracon/dracon-sync-incidents.jsonl 2>/dev/null || echo "  (no incidents)"

echo ""
echo "═══════════════════════════════════════════════════════════"
echo "14. STUCK REPOS"
echo "═══════════════════════════════════════════════════════════"
dracon-sync repair stuck-list 2>&1 | head -3

echo ""
echo "═══════════════════════════════════════════════════════════"
echo "15. DAEMON METRICS (key counters)"
echo "═══════════════════════════════════════════════════════════"
dracon-sync metrics 2>&1 | grep -E "stuck|blocked|total|discovered" | grep -v "^#" | head -5

echo ""
echo "═══════════════════════════════════════════════════════════"
echo " DONE"
echo "═══════════════════════════════════════════════════════════"
