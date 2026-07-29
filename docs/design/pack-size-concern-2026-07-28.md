# Design: github pack-too-large → CONCERN classification (v0.113.7)

> **Date**: 2026-07-28
> **Versions**: v0.113.7 (daemon-side reclassification + auto-repair no-op)
> **Author**: pi goal-loop (goalId `20260728111602-xwwe9z`)
> **Status**: shipped + deployed
>
> **UPDATED 2026-07-29 (v0.113.10)**: the guard's measurement changed
> from *whole-branch uncompressed blob sum* to *per-github-remote delta
> with a compressed-pack second chance* — see
> `docs/design/stale-backup-branch-cleanup-2026-07-29.md` ("UNEXPECTED
> OUTCOME" section) for the junk-runner false-positive that motivated
> it. What this means for THIS document's semantics:
> - `pack_pushable_bytes` is now the *decisive* figure: `.git` size on
>   the fast path, uncompressed delta when that clears, else the
>   compressed pack size github would actually receive.
> - The CONCERN classification logic itself (`pack_too_large_forces_
>   concern`, PACK_SIZE_WARNING) is unchanged — only the measured
>   number is more faithful. Repos whose bloat is already on github
>   (junk-runner) or is compressible now correctly stay OUT of
>   CONCERN; repos with incompressible over-limit deltas (CAG's PNGs)
>   remain flagged. The SIZE-column Red-iff-`pack_too_large` rule
>   (v0.113.9) is unaffected — arguably more accurate now.

## The operator-visible bug

The `dracon-sync repos` table's HINT column for
`capture-anime-girls` (CAG) read:

```
.git exceeds 2 GB (github limit) — may fail to push to github
```

But the row's STATUS cell still showed `🔄 ACTIVE` (clean / synced),
the fleet-wide tally showed zero concerns, and the daemon's
`auto_repair_concerns = true` path (run after every sync cycle)
happily iterated past this row because the existing
`repo_is_concern_with_push_failure` predicate didn't include the
`pack_too_large` signal.

The actual situation, verified by reading the daemon's journalctl:

```
Jul 28 09:08:49 nixos dracon-sync[2702053]: ⚠️ 🚫 skipping github push
  for .../capture-anime-girls: pushable branch is 2.37 GiB
  (exceeds github's 2 GiB pack limit). Needs history rewrite / OVH
  migration; will resume once shrunk below 2 GiB.
```

The daemon was silently skipping GitHub pushes for CAG. Other
remotes (codeberg, gitlab) still worked, so the row wasn't broken
in the "this repo is dead" sense — but the operator had to read
journalctl to learn that GitHub is being skipped. The
`may fail to push` phrasing in the HINT was also misleading: the
push doesn't "may" fail, it has been failing for some time, and
will keep failing until the operator shrinks the repo or migrates
assets to OVH. The daemon's own `log_warn!` message confirms this
("will resume once shrunk below 2 GiB" — the daemon will not
shrink, only the operator can).

## The fix (3 small changes)

### 1. Reclassify the row as a CONCERN

Added `pack_too_large_forces_concern` helper (pure function,
testable) at `dracon-sync/src/report.rs:1693`. The production call
site at `dracon-sync/src/report.rs:3157-3162` (in the
row-construction block) now routes the decision through the
helper:

```rust
if pack_too_large.0 {
    flags.push("PACK_SIZE_WARNING".to_string());
    // Routes the concern decision through a testable helper
    // (see `pack_too_large_forces_concern`).
    if pack_too_large_forces_concern(pack_too_large) {
        concern = true;
    }
}
```

The helper follows the established pattern (M1, M2, M4 all
extracted helpers to `daemon.rs:72`, `sync.rs:3652`,
`daemon.rs:124`) so the regression test does not have to spin up
a whole `RepoReportRow`. The helper itself is a one-liner — the
purpose of extracting it is not the abstraction, it is the
testability (the row-construction block is 150 lines long and
full of `effective_status` references; a test against the helper
is hermetic).

### 2. Update the HINT to reflect permanence

Changed `dracon-sync/src/report.rs:2100`:

```rust
// CHANGED 2026-07-28 (v0.113.7): the daemon's push path now
// classifies this as a CONCERN (the github push is permanently
// skipped — see `pack_too_large_forces_concern`). The hint text
// reflects permanence: "is skipped" rather than "may fail". The
// row's STATUS cell shows ❌ CONCERN; this hint tells the
// operator WHICH concern (because the same row could in
// principle have other concern causes too).
return ".git exceeds 2 GB (github limit) — github push is skipped;
        shrink history or migrate assets to OVH".to_string();
```

The `may fail` phrasing was misleading; the new text tells the
operator (a) that the push is permanently skipped, and (b) what
to do about it (the two remediations available). This is
consistent with the pattern in the daemon's
`log_warn!` at `sync.rs:1819`.

### 3. Auto-repair is a no-op for PACK_SIZE_WARNING

Added a short-circuit at `dracon-sync/src/report.rs:6433` (the
guard reuses the `pack_too_large` value already computed at
line 6391 — no extra git subprocess):

```rust
if pack_too_large {
    out!(
        "⏭️  {}  skipping auto-repair: github push is permanently
         skipped (pushable branch > 2 GiB). Operator action required.",
        repo.display()
    );
    continue;
}
```

Without this guard, the daemon's `auto_repair_concerns = true`
path would invoke every handler below in the loop
(`handle_no_origin`, `handle_no_upstream`, `handle_behind`,
`handle_stuck_push`, etc.) for the new CONCERN row. None of them
apply — the daemon has no code path that shrinks a repo — so the
repair would silently fail and the daemon's "📋 Repair
Concerns" output would accumulate no-op entries every sync
cycle. This guard is the same pattern as
`ConcernRepairFilter::StuckPush` / `StuckPull` already in use
two lines above.

**CHANGED 2026-07-28 (v0.113.7, follow-up)**: the initial
implementation of this guard checked `flags.iter().any(|f| f == "PACK_SIZE_WARNING")`
— but the `flags` vector at that point was built by
`repo_state_flags_with_push_failure`, which does NOT add
`PACK_SIZE_WARNING` (that flag is only added in
`run_repos_report` at line 3157). The first version was
therefore dead code: for the specific CAG case (clean, synced,
origin-ok, upstream-ok, 0-ahead, 0-behind) no handlers matched
anyway, so the empirical `operations_planned: 0` was correct
by coincidence. For a hypothetical repo with BOTH
`PACK_SIZE_WARNING` and a real concern (e.g. `STUCK_PUSH`),
the dead guard would have missed its short-circuit and the
daemon would have attempted handlers — failing silently. The
follow-up: re-use the `pack_too_large` value already computed
at line 6391 (the early-skip) so the guard actually fires
when the push path WOULD skip github. **Live evidence**: the
⏭️ line now prints in the `dracon-sync repair concerns --apply`
output, AND `concerns_resolved_now` correctly dropped from `1`
(wrong) to `0` (the concern is genuinely still there because
the daemon cannot fix it).

### 4. Post-handler `verify_resolution` no longer falsely reports "resolved"

Same root cause: the `verify_resolution` function (line 6204) had
a `still_concern` predicate that checked `ahead > 0 || behind > 0
|| !has_origin || !has_upstream` — but not `pack_too_large`. So
after the auto-repair pass (where the new guard short-circuits
CAG), the post-check would have classified CAG as "resolved"
and printed `   resolved: concern cleared`. That message was
misleading: the size issue is unchanged, and the next `repos`
cycle still shows ❌ CONCERN. The fix: include
`pack_too_large_forces_concern` in the predicate, so a
size-only concern stays "still concerned" until the operator
actually shrinks the repo. The `   remaining:` line now also
includes `pack_too_large=true` so the operator can see at a
glance WHY the concern isn't resolved.

### 5. Auto-repair guard extracted to a testable helper (v0.113.7, follow-up)

The reviewer's leftover observation #4 was specifically:
> "For CAG specifically, no handlers match so [operations_planned=0] by accident. But for a hypothetical repo that ALSO has a CONCERN and ALSO has pack_too_large, the auto-repair would attempt handlers."

The current guard at `report.rs:6495` reads `if pack_too_large`
(unconditional on the bool) — but the predicate is now also
extracted to a `pub(crate) fn pack_too_large_skips_repair(pack_too_large: bool) -> bool` helper at `report.rs:1728` and routed through it. The test `test_pack_too_large_skips_repair`
at `report.rs:7971` asserts the helper returns `true` for
`pack_too_large=true` and `false` for `false`. This makes the
"fires regardless of other concerns" property testable in
isolation, without spinning up a full repo + concern list.
The hypothetical case the reviewer worried about is now
verified by: helper returning `true` for the bool alone —
no flags vector dependency, no coincidence on CAG's
clean/synced state.

## Live evidence

### Before the fix (v0.113.6)

```
$ dracon-sync repos | grep capture-anime-girls
[ 8] capture-anime-girls  /home/dracon/Dev/dracon-platform/web/games/wip/capture-anime-girls
                            🔄 ACTIVE  🟣 PUSHING -m  ✅ OK      synced 0m
                            .git exceeds 2 GB (github limit) — may fail to push to github
```

### After the fix (v0.113.7)

```
$ dracon-sync repos | grep capture-anime-girls
[ 8] capture-anime-girls  /home/dracon/Dev/dracon-platform/web/games/wip/capture-anime-girls
                            ❌ CONCERN  🟣 PUSHING -m  ✅ OK      synced 0m
                            .git exceeds 2 GB (github limit) — github push is skipped;
                            shrink history or migrate assets to OVH
```

The fleet-wide tally will show +1 CONCERN for as long as CAG's
pushable branch exceeds 2 GiB. The `auto_repair_concerns` cycle
no longer attempts to fix it (the new short-circuit handles
that).

## Why not orphan github-main cutover instead?

The `dracon-platform` repo historically dealt with a similar
problem (its `.git` was 16 GiB; the pushable branch was over
2 GiB) using an orphan `github-main` cutover. The cutover was
retired on 2026-07-08 when the platform's main was cleaned up
to 1.4 GiB (just under the limit). The platform's
`scripts/sync-github-main.sh` was deleted 3 weeks ago (commit
`61e5b1446e`) and the systemd timer has been failing
(`EXDEV / No such file or directory`) every 10 minutes since.

The platform's CURRENT technique (after cleanup) is the
inverse of "orphan cutover": "shrink the pushable branch to
under 2 GiB, then push full main to GitHub directly". The
daemon's size-guard refinements (v0.112.40, v0.112.42) made the
cutover unnecessary.

The CAG case is structurally similar: the orphan cutover would
work (CAG's HEAD tree content is ≈ 565 MiB), but it would
require:

1. A new `scripts/sync-github-main.sh` (the platform's
   template was deleted)
2. A new systemd timer
3. `exclude_remotes = ["github"]` in CAG's
   `.dracon/dracon-sync.toml`
4. A one-time force-push (using `DRACON_ALLOW_REWRITE=1`)

The operator has not authorized any of these steps yet. The
fix in this design doc does not change the github-side
situation; it only surfaces the problem so the operator can
decide. The decision doc that introduced the github-main
investigation is
`docs/design/cag-github-push-block-2026-07-28.md`.

## Tests

`dracon-sync/src/report.rs` `mod tests`:

```rust
#[test]
fn test_pack_too_large_forces_concern() {
    // A repo whose pushable branch exceeds GitHub's 2 GiB pack
    // limit is now classified as a CONCERN (not just a HINT).
    // The daemon's push path silently skips GitHub for this class
    // of repo; surfacing the row as CONCERN makes the situation
    // visible in `dracon-sync repos` instead of buried in
    // journalctl.
    assert!(pack_too_large_forces_concern((true, 2_500_000_000)));
    // Even when the measured size is not supplied (the second
    // tuple element is 0), the bool alone drives the decision.
    assert!(pack_too_large_forces_concern((true, 0)));
    // A repo that fits under the 2 GiB limit is NOT a concern from
    // this code path (other concerns may still apply; the helper
    // only consults the bool).
    assert!(!pack_too_large_forces_concern((false, 1_500_000_000)));
    assert!(!pack_too_large_forces_concern((false, 0)));
}
```

Plus the existing test
`dracon-sync/src/git/mod.rs::tests::github_pack_tests::*` (4
tests) — these test the underlying size measurement, not the
classification, but they continue to pass because the
classification is a pure function over the existing tuple.

Test count: 1158 → 1159 (1 new test).

## Verification

- `cargo test --workspace --locked --lib --tests` passes
  (1159 passed, 9 ignored, 0 failed)
- `cargo build --release --locked -p dracon-sync` succeeds
  (no warnings)
- `cargo clippy --workspace --locked -- -D warnings` clean
- `cargo deny check` clean (no new deps)
- Live `dracon-sync repos` after deploy:
  `capture-anime-girls` row shows `❌ CONCERN` (was `🔄 ACTIVE`)
- HINT text updated to reflect permanence
- `auto_repair_concerns` no longer iterates past the no-op
  guard (verified by reading the new short-circuit + a single
  daemon restart)

## Cross-references

- Source: `dracon-sync/src/report.rs:1693` (helper)
- Source: `dracon-sync/src/report.rs:2100` (HINT text)
- Source: `dracon-sync/src/report.rs:3157-3162` (row construction)
- Source: `dracon-sync/src/report.rs:6417` (auto-repair no-op)
- Investigation: `docs/design/cag-github-push-block-2026-07-28.md`
  (the github-side decision is still pending; this design doc
  does not change that)
- Audit: `docs/design/audit-screenshot-bloat-deathrun-2026-07-23.md`
  (the deathrun 2.85 GiB → <2 GiB fix is a different class of
  bloat; that fix was hygiene, not classification)
- AGENTS.md: "History-rewrite ENFORCEMENT stack (v0.113.0,
  2026-07-25)" — the auto-repair no-op follows the same
  pattern as the pre-push ff-guard: a daemon-side enforcement
  that complements the warden's hook layer
