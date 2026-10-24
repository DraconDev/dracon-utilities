#!/usr/bin/env bash
# guard-sudo-cleanup.sh — the three cleanups that need root.
#
# WHY THESE NEED ROOT (measured on this host, 2026-10-24):
#
#   1. The coredumps are written by systemd-coredump as root.
#   2. The db-gateway target dir contains fingerprints a root-run cargo
#      build wrote.
#   3. Restoring a nice value needs CAP_SYS_NICE. The guard's unit HAS
#      that capability (CapEff 0000000000800000) but only inside its NESTED
#      user namespace (user:[4026532749]), and the processes it would
#      restore live in the INITIAL one (user:[4026531837]) — a namespaced
#      capability never applies to an ancestor namespace.
#      An ordinary user cannot do it either: on this kernel (7.1.2)
#      setpriority() returns EACCES for ANY nice reduction, even on a
#      process the caller owns and spawned itself. (Verified: spawn a
#      process, renice it to 5, try to set it back to 0 -> EACCES.)
#
# WHAT IT RECLAIMS: ~4.3 GB on this host.
#
# Run it as:  sudo bash scripts/guard-sudo-cleanup.sh
#
# Every step is idempotent and reports what it did. Nothing here is
# destructive of live state: the coredumps are crash diagnostics, the
# cargo target dir is regenerable, and the nice values are being restored
# to the default they had before the guard's old one-way ratchet stranded
# them.
set -euo pipefail

echo "== 1. restoring the priority the guard stranded =="
# Every process this user owns that is sitting at nice 5, back to nice 0.
# The guard's old namespace-blind renice could LOWER a priority but never
# RAISE it again, so everything it ever touched is still degraded. On this
# host that is 197 processes, including the operator's entire Chrome tree
# (65 processes) — the browser has been running at nice 5 ever since.
#
# FIXED 2026-10-24 (the first run of this script restored exactly ONE
# process): the filter asked for `id -un`, and under `sudo` that is ROOT —
# so it selected root's ksmd and none of the operator's 197. The processes
# to restore are the ones owned by the user who INVOKED sudo, which is
# what $SUDO_USER carries.
TARGET_USER="${SUDO_USER:-$(id -un)}"
if [ "$(id -un)" != "$TARGET_USER" ]; then
    echo "   (invoked via sudo as '$TARGET_USER'; restoring that user's processes, not root's)"
fi
restored=0
for pid in $(ps -eo pid=,ni=,user= | awk -v u="$TARGET_USER" '$2 == 5 && $3 == u {print $1}'); do
    if renice -n 0 -p "$pid" >/dev/null 2>&1; then
        restored=$((restored + 1))
    fi
done
echo "   restored $restored process(es) to nice 0"
echo "   still at nice 5: $(ps -eo ni= | awk '$1 == 5' | wc -l) total across the host"

echo
echo "== 1b. restoring the OOM score the guard stranded =="
# The guard's OOM steering writes oom_score_adj=250 onto a process that is
# holding too much memory, so the kernel prefers killing something else. A
# CHILD forked while its parent was biased INHERITED that value, and the
# guard only restores the processes it has a record of — so those children
# stay pinned at 250, which makes them MORE likely to be an OOM victim than
# a process at the default 0.
#
# On this host that set includes the operator's own Chrome session: the
# browser inherited 250 from a parent the guard had biased, and the guard
# will not touch it again, because Chrome is on the exempt list — exemption
# forbids mutation, and it cannot forbid an inherited value it never set.
# With memory this tight (2% available, PSI critical) that is worth undoing
# deliberately rather than leaving to chance.
#
# The processes the guard is steering RIGHT NOW are left alone: the ledger
# says which ones those are, and it owns them.
TARGET_HOME="$(getent passwd "$TARGET_USER" 2>/dev/null | cut -d: -f6 || true)"
LEDGER="${TARGET_HOME:-/home/$TARGET_USER}/.dracon/guard-mitigations.json"
tracked=" "
if [ -r "$LEDGER" ]; then
    tracked=$(python3 - "$LEDGER" <<'PY' || echo " "
import json
import sys

try:
    with open(sys.argv[1], encoding="utf-8") as handle:
        ledger = json.load(handle)
except Exception:
    print(" ")
    raise SystemExit(0)
pids = sorted(
    {
        record["pid"]
        for record in ledger.get("records", [])
        if record.get("original_oom_adj") is not None
    }
)
print(" " + " ".join(str(pid) for pid in pids) + " ")
PY
)
fi
oom_reset=0
oom_skipped=0
for pid in $(ps -eo pid=,user= | awk -v u="$TARGET_USER" '$2 == u {print $1}'); do
    adj_file="/proc/$pid/oom_score_adj"
    [ -r "$adj_file" ] || continue
    adj=$(cat "$adj_file" 2>/dev/null || echo "")
    [ "$adj" = "250" ] || continue
    case "$tracked" in
        *" $pid "*) oom_skipped=$((oom_skipped + 1)); continue ;;
    esac
    if echo 0 >"$adj_file" 2>/dev/null; then
        oom_reset=$((oom_reset + 1))
    fi
done
echo "   restored $oom_reset process(es) from oom_score_adj 250 to 0"
if [ "$oom_skipped" -gt 0 ]; then
    echo "   left $oom_skipped alone: the guard is steering them right now"
fi

echo
echo "== 2. clearing the root-owned coredumps =="
# 494 entries, 4.1 GB, all written by systemd-coredump as root. The ten
# newest are kept, in case a crash still needs looking at.
before=$(du -sm /var/lib/systemd/coredump 2>/dev/null | cut -f1 || echo 0)
ls -1t /var/lib/systemd/coredump/core.* 2>/dev/null | tail -n +11 | while read -r f; do
    rm -f "$f"
done
after=$(du -sm /var/lib/systemd/coredump 2>/dev/null | cut -f1 || echo 0)
echo "   coredump dir: ${before:-0} MiB -> ${after:-0} MiB (10 newest kept)"

echo
echo "== 3. clearing the root-owned cargo target files =="
# db-gateway's .cargo/target: 198 MiB of fingerprints a root-run cargo
# build wrote. The rest of it (1.9 GiB) was already removed as this user.
target=/home/dracon/Dev/db-gateway/.cargo/target
if [ -d "$target" ]; then
    rm -rf "$target"
    echo "   removed $target"
else
    echo "   $target already gone"
fi

echo
echo "== 4. clearing the root-owned /tmp scratch =="
# /tmp/sd-root169: 77 days old, a systemd path unit's copy of the cups
# library tree. Everything in it is this user's except ONE root-owned
# directory inside it, which is why the guard reported "Permission
# denied" on it every pass and could never remove it.
scratch=/tmp/sd-root169
if [ -d "$scratch" ]; then
    rm -rf "$scratch"
    echo "   removed $scratch (111 MiB)"
else
    echo "   $scratch already gone"
fi

echo
echo "== done =="
df -h / | tail -1
echo
echo "The guard itself will now refuse to apply any NEW priority it could"
echo "not take back, so this cannot recur: see the 'renice mitigation"
echo "skipped' lines in the guard's journal."
