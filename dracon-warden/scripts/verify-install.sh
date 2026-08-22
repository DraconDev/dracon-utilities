#!/usr/bin/env bash
# scripts/verify-install.sh — post-install / pre-release fixture check for a
# dracon-warden binary.
#
# Guards against the 2026-08-09 v0.113.3/0.3.1 incident class: a binary
# whose clean filter does NOT wire `protected_patterns` from the policy
# (WardenSecurity.managed_patterns left EMPTY -> path_is_protected treats
# empty as "scan everything (legacy)") secret-scans EVERY file — a 6.87 MB
# pi-session HTML took ~16 s of filter regex work, blew the 30 s
# FILTER_TIMEOUT_SECS, and wedged junk-runner's `git add` every cycle.
# The incident was caught only by manual publish-verify (cargo publish
# resolves the registry twin of the `path` dep); this script is the
# automated, behavioral equivalent.
#
# Usage:
#   scripts/verify-install.sh [binary-path]
#
#   binary-path   default: `dracon-warden` (resolved via PATH) — i.e. the
#                 installed binary the operator just put in place.
#
# Exit codes: 0 = fixture clean; 1 = fixture failed (protected_patterns
# wiring broken in the binary under test).
set -euo pipefail

BIN="${1:-dracon-warden}"
if ! command -v "$BIN" >/dev/null 2>&1; then
    echo "✗ binary '$BIN' not found on PATH" >&2
    exit 1
fi

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

# ── fixture policy: ONLY *.pem is protected — everything else must pass ───
# through the clean filter UNTOUCHED (the default-deny posture).
cat > "$TMP/policy.toml" <<EOF
protected_patterns = ["*.pem"]
EOF

# ── fixture files: a scanner-matching secret (OpenAI sk- key, guaranteed
# to match the OpenAI regex used in the existing test suite) in both a
# NON-protected and a protected path. The filter is invoked with RELATIVE
# paths (like git does) — the clean filter refuses absolute/`..` paths
# fail-closed as a security guard.
mkdir -p "$TMP/work"
printf 'sk-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n' > "$TMP/work/notes.txt"
cp "$TMP/work/notes.txt" "$TMP/work/key.pem"
cd "$TMP"

# 1. NON-protected path: the clean filter must pass the file through
#    unchanged (no DRACON_SECRET tag, the sk- key still present).
#    A binary with the wedge bug (empty managed_patterns = scan everything)
#    encrypts it here -> FAIL.
OUT="$(DRACON_WARDEN_POLICY="$TMP/policy.toml" "$BIN" filter-clean work/notes.txt < work/notes.txt 2>/dev/null || true)"
if [[ "$OUT" == *"DRACON_SECRET"* ]] || [[ "$OUT" != *"sk-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"* ]]; then
    echo "✗ FAIL: non-protected notes.txt was secret-scanned/encrypted — protected_patterns not wired (the 2026-08-09 wedge class)." >&2
    exit 1
fi

# 2. Protected path: the filter must STILL work — the sk- key in key.pem
#    gets replaced with a [DRACON_SECRET:...] tag.
#    FIXED 2026-08-12 (audit LOW-MEDIUM, scripts/verify-install.sh:54-60):
#    the old check captured with `|| true`, so a filter binary that ERRORS
#    on protected files (empty OUT2 — e.g. no recipients configured) passed
#    both `[[ ]]` tests (empty is not sk-*) and reported "✓ OK". An errored
#    filter is a FAIL: check the exit code FIRST, then tag presence.
if OUT2="$(DRACON_WARDEN_POLICY="$TMP/policy.toml" "$BIN" filter-clean work/key.pem < work/key.pem 2>/dev/null)"; then
    :
else
    rc=$?
    echo "✗ FAIL: filter-clean errored on the protected file (exit $rc) — filter not functional (e.g. no recipients configured)." >&2
    exit 1
fi
if [[ "$OUT2" != *"DRACON_SECRET"* ]] || [[ "$OUT2" == *"sk-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"* ]]; then
    echo "✗ FAIL: protected key.pem was not encrypted — filter not functional." >&2
    exit 1
fi

echo "✓ OK: $BIN honors protected_patterns (non-protected file untouched, protected file encrypted)"
exit 0
