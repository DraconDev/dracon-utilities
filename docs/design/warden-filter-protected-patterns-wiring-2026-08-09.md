# Warden filter protected-patterns wiring — 2026-08-09

> **Status**: FIXED in `dracon-warden v0.113.3` + `dracon-security v0.3.1`
> (v0.113.4 re-released the same day with a test-only helper gated
> behind `#[cfg(test)]` so the crate builds warning-free). Installed
> and verified live; the wedged repo recovered within one daemon cycle.

## Symptom

`junk-runner` wedged on 2026-08-09 18:01–18:23: every daemon cycle
logged:

```
⚠️ /home/dracon/Dev/dracon-platform/web/games/wip/junk-runner git add
   failed ... exit status: 128: dracon-warden: filter-clean timed out
   after 30s, exiting (parent likely gone)
```

`git add` exited 128 → the daemon could neither commit new work nor
push (11 commits ahead, `🟣 PENDING` forever), plus a "Changes Piling
Up" alert (83 committable entries, oldest 7925s old).

## Root cause

Two compounding defects:

1. **`protected_patterns` was dead code in the filter process**
   (`dracon-security`). The clean filter's "default-deny" gate
   (`smart_clean_with_path` step 0a) skips scanning for files that do
   NOT match a protected pattern — but the gate read
   `WardenSecurity.managed_patterns`, which the production
   constructor (`WardenSecurity::new`) initializes EMPTY, and
   `path_is_protected` treats an empty list as "scan everything
   (legacy)". The config's `protected_patterns` were wired into
   `.gitignore`/`.gitattributes` block generation and `scrub_markers`,
   but NEVER into the filter process — `with_managed_patterns` was
   only exercised by a test.

2. **`junk-runner/.gitattributes` uses `* filter=dracon`** (hand-edited;
   the current warden writes per-pattern filter lines). So EVERY file
   went through `filter-clean`, and the empty-patterns bug meant every
   one of them was fully secret-scanned.

Measured impact: a 6.87 MB `pi-session-*.html` export took **16.3 s of
CPU** in `filter-clean` (regex scan). Git runs multiple filters
concurrently in one `git add` batch, so the wall-clock 30 s
`FILTER_TIMEOUT_SECS` blew on a busy batch → filter exits 1 → `git
add` 128 → daemon wedge. With the fix the same file filters in
**12–13 ms** (~1250×).

## Fix

- `dracon-security v0.3.1`: process-wide `set_managed_patterns()`
  override + `WardenSecurity::apply_managed_patterns_override()`
  applied inside `get_or_init()`, so the filter's gate honors the
  policy. New unit tests: override round-trip and a 7 MiB
  non-protected passthrough regression test.
- `dracon-warden v0.113.3`: `run_filter` now calls
  `wire_managed_patterns_from_policy()` (policy resolved via
  `resolve_policy_path_local()`) before touching content. New
  binary-level test proves the policy's patterns reach the gate.
- The `* filter=dracon` line in junk-runner's `.gitattributes` was
  left as the operator shaped it (with the documented `.pi-glla/**`
  opt-out); the wiring fix makes it cheap again. Converting it to the
  warden-managed per-pattern block is an optional follow-up.

## Release note (the dracon-git lesson, again)

`dracon-warden` depends on `dracon-security` via
`{ package = "dracon-security", version = "0.3", path = "src/security" }`.
`cargo publish` ignores `path` and resolves the REGISTRY version —
v0.3.0 lacked the new functions, so the publish verify failed
(`E0432: no set_managed_patterns`). Fix order matters: publish the
security crate FIRST (`dracon-security v0.3.1`), bump the dep, then
release the warden. The local workspace build masked this (path dep);
only the publish verify caught it.

## Verification

- 104 warden tests pass (+2), clippy `-D warnings` clean.
- `filter-clean` on the 6.87 MB HTML: 12–13 ms (was 16.3 s); protected
  `.env` fixtures still encrypt (`[DRACON_SECRET:…age…]`).
- Live: after `cargo install dracon-warden --version 0.113.4` (real
  crates.io artifact) + `rm -f`/`cp` to `~/.local/bin`, the daemon's
  next cycle committed all 83 entries (commit `66494733`, 67 files
  incl. the HTML) and pushed to github + gitlab; repo `✅ CLEAN ·
  🟢 synced · healthy`, fleet `🟡 0`.
- Tags `v0.113.3` + `v0.113.4` on github (with gh releases), gitlab,
  and codeberg; crates.io max stable `dracon-warden 0.113.4`,
  `dracon-security 0.3.1`.
