# Sync-stall audit — 2026-07-09

**Trigger:** `dracon-sync repos` showed a cluster of repos stuck in `WARN` /
`PENDING pushing` (neonbreak, junk-runner, polis, dracon-platform, darklord,
hellhunter, capture-anime-girls all WARN; dracon-code + pully PENDING). The
operator reported "many with changes that are not staging or committing."

**Outcome:** one root-cause daemon defect found and fixed. After the fix, every
previously-stalled repo resumed normal commit/push and converged to all three
remotes (github/gitlab/codeberg). Two residual non-OK rows are expected,
separate situations (documented in §6), not the stall.

---

## 1. Root cause (single defect)

**`src/sync.rs` `push_background` — the `too_big_for_github` guard logs
"skipping github push" but does NOT actually exclude `github` from the mirror
push.**

```rust
// origin-github path: ACTUALLY skips (omits push_with_retries)
if too_big_for_github && origin_is_github {
    // skip origin push (github)
} else {
    push_with_retries(...)   // push origin
}
...
if too_big_for_github {
    // Record the skip ...
    if let Some(rf) = remote_failures.as_deref_mut() { *rf.entry("github"...)+=1; }
    if !github_already_flagged {
        log_warn!("🚫 skipping github push for {}: pushable branch is 2.41 GiB ...");
    }
    // ← BUG: "github" is never added to `combined_exclude`
}
let push_results = push_mirror_remotes(
    repo, &policy.remotes, policy.push_op_timeout_secs, policy.push_retries,
    private, &combined_exclude,   // github NOT in here → still pushed
).await;
```

For a repo whose **origin is NOT github** (e.g. hegemon: `origin` = codeberg,
`github` is a mirror remote), `too_big_for_github` is `true` (hegemon's
pushable branch is 2.41 GiB, over GitHub's 2 GiB/pack hard limit), but the
`if origin_is_github` block does not add `"github"` to `combined_exclude`, and
the `if too_big_for_github` block only *logs* the skip — it never adds
`"github"` to `combined_exclude`. So `push_mirror_remotes` →
`filter_remotes_by_exclude` keeps `github` → `push_to_all_remotes` spawns
`git push --no-verify github HEAD` (and the URL form
`https://github.com/DraconDev/hegemon.git` via `auto_create_all_remotes`).

GitHub rejects the 2.41 GiB pack (`pack exceeds maximum allowed size (2.00
GiB)`), but the `git push` process hangs uploading the oversized pack for the
full scaled timeout (capped 600 s) before being killed. The daemon re-dispatches
`hegemon`'s `sync_repo` every scan cycle (~1 s pulse + 120 s trailing drain), so
a fresh hung `git push github` is spawned every ~1–3 minutes, leaking orphaned
git processes:

```
1188379 13:10 git push --no-verify github
1189921 13:11 git push --no-verify github
1191900 13:14 git push --no-verify https://github.com/DraconDev/hegemon.git
```

**Why this starved the other repos:** the 2.41 GiB upload saturated the outbound
link and the daemon's push capacity every cycle. The other repos'
(`neonbreak`/`junk-runner`/`pully`/etc.) pushes to github/gitlab/codeberg
contended with hegemon's giant upload and crawled — hence "pushing 5m / 16m /
43m" with commits never reaching the remotes. The daemon was effectively stuck
in hegemon's re-dispatch loop (the orphaned process outlives the `JoinHandle`,
which the apply/trailing drain drops without killing the child — see §5).

---

## 2. Fix

`src/sync.rs` `push_background` — in the `if too_big_for_github` block, actually
exclude `github` from the mirror push (the mirror-path counterpart of the
origin-github skip that already works):

```rust
if too_big_for_github {
    // ACTUALLY exclude github from the mirror push. The log message below
    // says "skipping github push", but unless we add it to `combined_exclude`
    // the `push_mirror_remotes` call below still routes github through
    // `auto_create_all_remotes` + `push_to_all_remotes`, spawning a
    // `git push github` that github rejects (2 GiB pack limit). That hangs
    // uploading the oversized pack and leaks an orphaned git process the
    // daemon re-dispatches every cycle.
    if !combined_exclude.iter().any(|e| e == "github") {
        combined_exclude.push("github".to_string());
    }
    if let Some(rf) = remote_failures.as_deref_mut() {
        *rf.entry("github".to_string()).or_insert(0) += 1;
    }
    ...
}
```

Build + restart:

```bash
cd /home/dracon/Dev/dracon-utilities/dracon-sync
cargo build --release --locked
systemctl --user restart dracon-sync.service
```

The orphaned hegemon `git push github` processes were `kill -9`'d before the
restart.

---

## 3. Verification (after fix)

- **Orphaned hegemon github pushes: 0** (was 3, re-spawned every cycle).
- **hegemon** now shows `OK` + `PACK_SIZE_WARNING`; github is intentionally
  skipped (2.41 GiB > 2 GiB) and gitlab/codeberg converge (no hung push).
- **Previously-stalled repos all recovered:**

| repo | before | after |
|------|--------|-------|
| neonbreak | pushing 5m (11 ahead) | OK (converged github/gitlab/codeberg) |
| junk-runner | pushing 16m (3 ahead) | OK (`c0a098b2` on all 3) |
| polis | WARN DIRTY | OK |
| dracon-platform | WARN DIRTY | OK (`fc09705d` on origin=github/gitlab/codeberg) |
| darklord | WARN DIRTY | OK |
| hellhunter | WARN DIRTY | OK |
| capture-anime-girls | WARN DIRTY | OK |
| pully | PENDING pushing 43m | OK (converged) |
| dracon-code | PENDING | AHEAD:2 PENDING (see §6) |

The 5 "DIRTY / not committing" repos (polis, dracon-platform, darklord,
hellhunter, capture-anime-girls) were simply starved of daemon cycles by the
hegemon loop — they committed + pushed normally once the loop cleared.

---

## 4. Why the 2 GiB guard matters (hegemon)

hegemon's `main` pushable branch is **2.41 GiB** (history with large assets).
GitHub enforces a **hard 2 GiB/pack** limit server-side; the daemon's
`GITHUB_PACK_LIMIT_BYTES` merely mirrors it. The `is_pack_too_large` /
`github_pack_too_large` backstop is correct and intended: hegemon's github push
must stay skipped until the history is rewritten below 2 GiB (or migrated to
OVH). The defect was only that the mirror-path guard *logged* the skip without
*enforcing* it. With the fix, hegemon's github stays skipped and the daemon no
longer leaks hung pushes.

---

## 5. Related observation (not changed this round)

The apply/trailing-drain deadlines (`inactivity_push_delay_secs=2`,
`trailing_drain_deadline_secs=120`) `timeout_at(in_flight_tasks.next())` drop the
`JoinHandle` of an over-deadline task but never kill the underlying git child
process. The git runner (`src/git/ops.rs`) does kill the child on its own
per-operation timeout (`kill_on_drop(true)` + idle-timeout), so orphaned pushes
are bounded (killed after the scaled timeout, capped 600 s) — but until they are
killed they consume the uplink. This is why a single hung push (hegemon) had
repo-wide blast radius. With Bug A fixed there is no hung push, so this latent
inefficiency is currently dormant. Tracked as a future hardening item (global
push concurrency limit + shorter per-push ceiling), not required for this stall.

---

## 6. Residual non-OK rows (expected, separate)

- **dracon-code — AHEAD:2, push=PENDING.** The 2 unpushed commits are authored
  by `audit-agent <audit@dracon-code>` — an **untrusted author** per the daemon's
  trust model, so the daemon correctly refuses to auto-push until the author is
  trusted/approved. This is a safety gate, not a sync bug. Operator decision:
  trust the author (policy) or push manually.
- **dracon-utilities — DIRTY (mod=0).** `git status` shows `??
  dracon-sync/ dracon-system/ dracon-warden/` — these are **nested git repos**
  (separately watched repos) that git will not auto-add into `dracon-utilities`
  (nested-repo guard). `.gitmodules` is empty and `git submodule status` is
  empty, so they are not submodules. The daemon committed `dracon-utilities`
  itself 49 min ago (result `ok`); the untracked nested dirs are a pre-existing
  config matter (register as submodules vs gitignore) and are out of scope for
  this stall.

---

## 7. Commits

- `dracon-sync/src/sync.rs`: add `"github"` to `combined_exclude` when
  `too_big_for_github` (mirror-path skip enforcement). Auto-committed by the
  daemon to `dracon-sync` (all 3 remotes).
- This design doc: committed to `dracon-utilities` (all 3 remotes) by the
  daemon.

---

## 8. Deferred / follow-ups

- **D-stall-1:** hegemon github 2.41 GiB history — rewrite below 2 GiB or OVH
  migration (operator decision; same deferred item as the 2026-07-09 full audit).
- **D-stall-2:** dracon-code untrusted-author trust decision (operator).
- **D-stall-3:** dracon-utilities nested-repo untracked dirs — submodule vs
  gitignore (operator).
- **D-stall-4:** harden push path so an over-deadline task's git child is killed
  immediately (not just on its own 600 s ceiling), preventing any single hung
  push from saturating the uplink.
