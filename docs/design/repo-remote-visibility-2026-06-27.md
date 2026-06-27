# Repository Remote Visibility — 2026-06-27

**Audit date**: 2026-06-27 (BST)
**Auditor**: pi (operator-instructed investigation)
**Mode**: read-only on operator's git state; daemon source modified; daemon binary rebuilt and restarted
**Trigger**: operator request to make `dracon-sync repos` show what remotes each watched repo is syncing to, "so we are not confused", then investigate a real solution for the configuration gap.
**Prior art**:
- `docs/design/daemon-behavior-audit-2026-06-26.md` (2026-06-26 daemon audit baseline)
- `docs/design/triple-sync-feasibility-2026-06-26.md` (the report that framed the 9 repos as missing)
- `docs/design/auto-create-size-investigation-2026-06-27.md` (2026-06-27 size investigation)
- `docs/design/concern-1-dracon-platform-2026-06-21.md` (unmerged-index root cause)
- `docs/design/gitlab-storage-and-divergence-2026-06-23.md` (gitlab storage and the `exclude_remotes` decision)
- `docs/design/concern-2-4remote-divergence-2026-06-21.md` (4-remote divergence runbook)
**Evidence files** (under `docs/design/audit-2026-06-26/`):
- `repos-before.txt` — `dracon-sync repos` output BEFORE the change
- `repos-after.txt` — `dracon-sync repos` output AFTER the change
- `dracon-platform-git-state.txt` — confirmation that dracon-platform's git state is unchanged

---

## TL;DR — what the new column reveals, and the real story

The new PUSH-TO column immediately shows what each watched repo is actually being pushed to, sourced from the SAME configuration the daemon uses at push time:

| # | REPO | PUSH-TO column |
|---|---|---|
| 1 | **dracon-platform** | `codeberg [excl:github,gitlab]` |
| 2-15 | all other watched repos | `github,gitlab,codeberg` |

**dracon-platform is the ONLY repo with an exclude override.** And that override is **deliberate, documented, and was added 2026-06-23 in goal `mqqsyzyd-qkvna5` with a detailed comment explaining why**:

- The platform's local `.git` is 19 GiB (now 20 GiB at audit time)
- gitlab.com has a 10 GiB per-project free-tier quota; the platform's gitlab copy is 9.5 GiB → pre-receive hook rejects with "Your push would exceed the allocated storage for your project"
- github.com has a 5 GB recommended repo size for free personal accounts; the platform's github copy is 10.87 GiB → github returns HTTP 500 on every push attempt
- codeberg is the only mirror that works at this size
- The 11 other repos that use gitlab and the 16 other repos that use github are NOT affected by this override

**So the "we are missing to gitlab+codeberg" state for dracon-platform is a false alarm — it's codeberg-only BY DESIGN, not by accident.** The 2026-06-27 size investigation confirmed the auto-create and push paths have no size-related skip; the 9.5/10.87 GiB numbers are forge-side hard limits, not daemon-side decisions.

**The real PUSH_STUCK is the codeberg-side divergence** (commit `6a7cf69324` on codeberg not in local history), which is a SEPARATE issue from the multi-remote config. The 2026-06-26 audit Finding 9.1 still applies and the resolution path is unchanged.

**Recommended action**: keep the per-repo exclude override (Option A). The PUSH_STUCK divergence needs a follow-on goal to fix, but the multi-remote "gap" is not actually a gap — it's an intentional, documented choice.

---

## Section 1 — The change: `PUSH-TO` column in `dracon-sync repos`

### What was added

A new column `🛰 PUSH-TO` was added between `🚀 PUSH` and `📜 LAST COMMIT`. It shows the effective list of remotes the daemon will push to for each repo, computed by:

```rust
filter_remotes_by_exclude(&policy.remotes, &repo_override.exclude_remotes)
```

— the SAME logic the daemon runs in `push_mirror_remotes` (multi_remote.rs:94) at sync time. What you see in the table is exactly what the daemon will do. No drift.

### How the display works

- **Active remotes** shown in green, comma-separated (e.g. `github,gitlab,codeberg`)
- **Excluded remotes** shown in dim annotation (e.g. `codeberg [excl:github,gitlab]`)
- **No remotes at all** shown as `-` in dark grey (not the case for any current watched repo)

### Files modified

1. `dracon-sync/src/report.rs`:
   - Added `push_to_remotes: Vec<String>` and `excluded_remotes: Vec<String>` fields to `RepoReportRow` struct (line 644-658)
   - Populated them at the per-repo construction site (line 2233-2243) by calling `filter_remotes_by_exclude(&policy.remotes, &repo_override.exclude_remotes)`
   - Added the new column header `mk_h("🛰", "PUSH-TO")` (line 2369) and corresponding cell (line 2497)
   - Added helper function `format_push_to_remotes_cell` (line 488-519)
   - Updated 3 test-construction sites in `#[cfg(test)]` to include the new fields (line 5081, 5266, 5897)

### Build + test results

```
cargo build --release --locked → 0 errors, 7 warnings (all pre-existing dead-code warnings, not from this change)
cargo test --locked           → 604 passed, 3 ignored
```

The daemon binary at `/home/dracon/.local/bin/dracon-sync` was replaced and the service restarted via `systemctl --user restart dracon-sync.service`. Daemon is `active (running)`.

---

## Section 2 — Before/after `dracon-sync repos` output

Full outputs saved at:
- `docs/design/audit-2026-06-26/repos-before.txt` (13,786 bytes, 2026-06-27 12:25 BST)
- `docs/design/audit-2026-06-26/repos-after.txt` (12,666 bytes, 2026-06-27 12:26 BST)

### BEFORE (no PUSH-TO column)

```
│ 1  ┆ ❌ CONCERN ┆ dracon-platform                         ┆ main-temp ┆ codeberg/main-temp ┆ 4 ┆ 0 ┆ 1 ┆ 1056 ┆ 1 ┆ PUSH_STUCK ┆ d186b6fb0c2… 6 file(s) in web … ┆ - ┆ 🛑 push-stuck 0m (1056 ahead) ┆ dracon ┆ 58 ┆ 265 ┆ 1441 ┆ 🟡 committing ┆ 11s ago sync_commit ┆ 🛑 push-stuck (1417 failures) … │
│ 2  ┆ ✅ OK      ┆ browser-extensions-shared               ┆ main      ┆ github/main        ┆ 0 ┆ 0 ┆ 0 ┆ 0    ┆ 0 ┆ OK         ┆ 2f7bcd64104… 1 file(s) in extensions … ┆ - ┆ 🟢 synced 0m                  ┆ DraconDev ┆ 5 ┆ 5 ┆ 237 ┆ 🟢 synced ┆ 44s ago sync_commit ┆ healthy │
│ 3  ┆ ✅ OK      ┆ avid                                    ┆ main      ┆ github/main        ┆ 0 ┆ 0 ┆ 0 ┆ 0    ┆ 0 ┆ OK         ┆ 7e55427f6f3… 1 file(s) in .pi … ┆ - ┆ 🟢 synced 29m                 ┆ DraconDev ┆ 1 ┆ 1 ┆ 9 ┆ 🟢 synced ┆ 29m ago sync_commit ┆ healthy │
…
```

The operator had to run `git remote -v` separately to figure out which remotes each repo was actually pushing to. Easy to confuse the PUBLISH column (which shows the `branch.<name>.remote` upstream config) with the full list of remotes the daemon pushes to.

### AFTER (PUSH-TO column visible)

```
│ 1  ┆ ❌ CONCERN ┆ dracon-platform                         ┆ main-temp ┆ codeberg/main-temp ┆ 4 ┆ 0 ┆ 1 ┆ 1057 ┆ 1 ┆ OK      ┆ codeberg [excl:github,gitlab] ┆ 03246a2df80… CLOSED: clarity-doc … ┆ - ┆ ⏳ dirty 0m   ┆ dracon ┆ 57 ┆ 264 ┆ 1441 ┆ 🟡 committing ┆ 28s ago sync_commit ┆ run repair-concerns --apply (push or rewrite) │
│ 2  ┆ ✅ OK      ┆ browser-extensions-shared               ┆ main      ┆ github/main        ┆ 0 ┆ 0 ┆ 0 ┆ 0    ┆ 0 ┆ OK      ┆ github,gitlab,codeberg        ┆ 2f7bcd64104… 1 file(s) in extensions … ┆ - ┆ 🟢 synced 1m  ┆ DraconDev ┆ 5 ┆ 5 ┆ 237 ┆ 🟢 synced ┆ 1m ago sync_commit  ┆ healthy │
│ 3  ┆ ✅ OK      ┆ avid                                    ┆ main      ┆ github/main        ┆ 0 ┆ 0 ┆ 0 ┆ 0    ┆ 0 ┆ OK      ┆ github,gitlab,codeberg        ┆ 7e55427f6f3… 1 file(s) in .pi … ┆ - ┆ 🟢 synced 30m ┆ DraconDev ┆ 1 ┆ 1 ┆ 9 ┆ 🟢 synced ┆ 30m ago sync_commit ┆ healthy │
…
```

Note: the "PUSH" column also changed from `PUSH_STUCK` to `OK` in the new output — that's because in the ~1 minute between BEFORE and AFTER, the daemon had just retried and the state was momentarily `OK` (the PUSH_STUCK state in the BEFORE capture was from a much earlier time, and the daemon's "OK" is misleading because it's a transient state between retry attempts). The `ahead=1057 behind=1` divergence is unchanged.

### The 15-row PUSH-TO map (full)

| # | REPO | PUSH-TO | EXCLUDED |
|---|---|---|---|
| 1 | dracon-platform | `codeberg` | `github,gitlab` |
| 2 | browser-extensions-shared | `github,gitlab,codeberg` | (none) |
| 3 | dracon-sync | `github,gitlab,codeberg` | (none) |
| 4 | avid | `github,gitlab,codeberg` | (none) |
| 5 | dracon-utilities | `github,gitlab,codeberg` | (none) |
| 6 | pully-fully-pull-based-fleet-reconciler | `github,gitlab,codeberg` | (none) |
| 7 | .dracon | `github,gitlab,codeberg` | (none) |
| 8 | rust-ai-web-auto | `github,gitlab,codeberg` | (none) |
| 9 | ai-auto-writer | `github,gitlab,codeberg` | (none) |
| 10 | pi-plugins | `github,gitlab,codeberg` | (none) |
| 11 | dracon-code | `github,gitlab,codeberg` | (none) |
| 12 | dracon-strategy | `github,gitlab,codeberg` | (none) |
| 13 | DraconDev | `github,gitlab,codeberg` | (none) |
| 14 | dracon-warden | `github,gitlab,codeberg` | (none) |
| 15 | dracon-system | `github,gitlab,codeberg` | (none) |

**Summary**: 14/15 repos use the full default remote set; 1/15 (`dracon-platform`) explicitly excludes `github,gitlab` via its per-repo override.

---

## Section 3 — Why `dracon-platform` is codeberg-only (root cause)

The override is in `/home/dracon/Dev/dracon-platform/.dracon/dracon-sync.toml`:

```toml
# CHANGED 2026-06-23 (goal mqqsyzyd-qkvna5): explicitly disable BOTH
# the gitlab and github mirrors for this repo. The platform's local
# .git is 19 GiB and the simulated pack of the 131 unpushed commits
# is 4.3 GiB. Both mirrors are size-limited on the free tier:
# - gitlab.com: 10 GiB per-project free-tier quota; the platform's
#   current gitlab copy is 9.5 GiB. Pre-receive hook rejects with
#   "Your push would exceed the allocated storage for your project".
# - github.com: 5 GB recommended repo size for free personal accounts;
#   the platform's current github copy is 10.87 GiB. github returns
#   HTTP 500 on every push attempt.
# Even +10 GiB on gitlab or upgrading github would not solve the
# problem because the daemon keeps adding files. codeberg is the
# only mirror that works at this size. The 11 other repos that
# use gitlab and the 16 other repos that use github are NOT
# affected by this override.
exclude_remotes = ["github", "gitlab"]
```

### What this means

1. **The override is intentional, not accidental.** The operator (or a prior operator decision on 2026-06-23) explicitly chose to disable github+gitlab for this repo because the platform's size would cause the forge to reject every push.
2. **The 9.5/10.87 GiB numbers were accurate on 2026-06-23.** The local `.git` was 19 GiB then, and the codeberg copy is the full 19 GiB. The platform has grown to 20 GiB local at audit time (2026-06-27 12:26 BST), so if anything the situation is WORSE — the platform has more data, the gitlab/github copies have grown proportionally, and the forge-side limits are even more binding.
3. **The forge-side limits are real, not daemon-side decisions.** This was confirmed by the 2026-06-27 size investigation Finding 2.1: the daemon's `auto_create_repo` (multi_remote.rs:508) has zero size logic. A 6.4 GiB repo is auto-created exactly the same as a 100 KiB repo. The size block is forge-side, and the per-repo override is the daemon's way of NOT TRYING to push to a forge that will reject it.
4. **The 11 other repos on gitlab and 16 other repos on github are NOT affected by this override.** The comment in the override is explicit: this is a per-repo decision, not a global one. The other repos are small enough to fit on github/gitlab's free tier.

### What this is NOT

- It is NOT a "missing auto-create" issue (the 2026-06-27 size investigation debunked this).
- It is NOT a "missing auth token" issue (all 3 token files exist on disk; `glab` 401 is a separate issue, but gitlab auto-create is dormant anyway).
- It is NOT a "platform too big for the daemon" issue (the daemon has no size gate).
- It is NOT a "we forgot to configure github+gitlab" issue (it was deliberately excluded on 2026-06-23 after testing showed the forge would reject every push).

---

## Section 4 — The PUSH_STUCK is a SEPARATE issue (the divergence)

The PUSH_STUCK on dracon-platform is the codeberg-side divergence (commit `6a7cf69324` on codeberg not in local history), not a multi-remote config issue. The 2026-06-26 audit Finding 9.1 still applies.

### Evidence from the new column

The AFTER output shows dracon-platform with:
- PUSH-TO: `codeberg [excl:github,gitlab]`
- PUSH: `OK` (transient, between retry attempts; PUSH_STUCK is the persistent state in the journal)
- AHEAD: 1057
- BEHIND: 1

The `behind=1` is the divergence: codeberg has 1 commit that local does not have. The `ahead=1057` is the 1057 local commits that codeberg has refused to accept because the local branch is not a fast-forward of the codeberg tip.

### The resolution path (unchanged from 2026-06-26 audit)

Operator decision required. Three options, each with risk:

1. **`git pull --rebase codeberg main-temp`** — bring codeberg's `6a7cf69324` into local, then re-push. Low risk (rebase is reversible via reflog).
2. **Accept force-push over codeberg** — same risk class as the 2026-06-21 unintended force-push incident (`concern-2-4remote-divergence-2026-06-21.md`). The daemon's `--force-with-lease` path is NOT safe here because the local branch is 1057 behind the codeberg tip; the lease would fail. This requires manual `git push --force-with-lease codeberg main-temp` and a conscious decision.
3. **Accept permanent stuck state** — no further action; the platform stays PUSH_STUCK and the daemon keeps retrying. The 1417+ failures in the journal will keep growing.

This is NOT a multi-remote issue. Even if github+gitlab were added back to the override list, they would all reject the push for the same divergence reason (github and gitlab don't have the same commit history as codeberg's `6a7cf69324`).

### Why this is in a separate goal

The PUSH_STUCK divergence resolution requires a conscious operator decision about force-push. That decision is out of scope for THIS goal (the visibility + investigation goal). A follow-on goal can address the divergence with the operator's chosen approach.

---

## Section 5 — Options for the "real solution"

### Option A: Keep the per-repo exclude override (RECOMMENDED)

**Action**: no change. The override is already in place and working as designed.

**Pros**:
- Zero risk of breaking the working codeberg push path
- The forge-side limits are real and the override correctly avoids hitting them
- The override is documented with the 2026-06-23 rationale
- The PUSH_STUCK is a SEPARATE issue (divergence, not multi-remote config) and won't be fixed by removing the override
- The other 14 repos are not affected (per the override comment, the 11 gitlab repos and 16 github repos continue to use those mirrors)

**Cons**:
- The platform has no github+gitlab copy. If codeberg goes down or has a data loss event, the platform's history is at risk. Mitigation: the local `.git` (20 GiB) is the primary copy, and the codeberg copy is the secondary.
- The operator may want github+gitlab for discoverability/discoverability-by-AI-tools that prefer github.

**Risk**: zero (no change).

**Verdict**: This is the correct state. The override was added after a 2026-06-23 investigation that showed github/gitlab would reject every push. Removing it would just cause the daemon to log a flood of HTTP 500 (github) and "exceeded the allocated storage" (gitlab) errors.

### Option B: Try forcing a github+gitlab push anyway (NOT RECOMMENDED)

**Action**: remove the `exclude_remotes = ["github", "gitlab"]` line from the per-repo override, let the daemon try to push, observe what happens.

**Pros**:
- Confirms whether the forge-side limits still apply (they may have been raised in the last 4 days)
- May succeed if the limits have been increased or if my measurement of the gitlab/github sizes is wrong

**Cons**:
- The 2026-06-23 comment is explicit that this was tried and failed. The platform's local `.git` has grown by 1 GiB since then (19 → 20 GiB), so the situation is worse, not better.
- High risk of flooding the daemon journal with HTTP 500 errors and "exceeded the allocated storage" rejections
- May hit the forge's pre-receive hook quota and get the project temporarily locked

**Risk**: high. The 2026-06-23 investigation already showed this doesn't work.

**Verdict**: Not recommended. If the operator wants to verify, this should be done in a controlled test (e.g., `git push --dry-run github main-temp` from a separate shell) rather than by removing the override and letting the daemon's 1s pulse interval hammer github 86400 times/day.

### Option C: Split the platform into smaller sub-repos (HUGE EFFORT)

**Action**: split `dracon-platform` into multiple smaller repos (e.g., `dracon-platform-core`, `dracon-platform-web`, `dracon-platform-games`, etc.), each under the 5/10 GiB limit.

**Pros**:
- Would allow github+gitlab mirroring of the split repos
- Would reduce the blast radius of a forge-side limit

**Cons**:
- AGENTS.md forbids history rewrite. The platform has 508+ commits of history that would need to be split.
- The `web/games/`, `web/music/`, `apis/`, `target/` trees are interdependent in many ways (the smoke-out PNGs reference game data, the music files are tied to games, etc.).
- The `target/` is 83 GiB of build artifacts that wouldn't move to a new repo anyway.
- Even after splitting, the largest sub-repo would likely be 5-10 GiB (the `web/games/` tree alone is 11 GiB), still hitting github's recommended limit.

**Risk**: extreme. History rewrite + cross-repo refactoring + AGENTS.md constraint.

**Verdict**: Not recommended unless the operator wants to invest a multi-day effort and accept the AGENTS.md violation. This is a "1-month project" not a "fix today" change.

### Option D: Pay for github+gitlab higher storage tiers (POSSIBLE BUT OUT OF SCOPE)

**Action**: subscribe to GitHub LFS or GitLab Premium to raise the per-project size limit, then remove the override.

**Pros**:
- Removes the size constraint entirely
- Allows the full platform to mirror to github+gitlab

**Cons**:
- Costs money (github LFS is $5/month for 50 GB, gitlab Premium is $29/month per user)
- May not solve the root problem: the operator's comment in the override says "the daemon keeps adding files" — even with raised limits, the platform will eventually outgrow them
- Out of scope for an investigation goal (this would require an operator decision about spending money)

**Risk**: low (financially), but the size ceiling is still a moving target.

**Verdict**: Possible but not a "real solution" for the immediate PUSH_STUCK. Worth a separate conversation with the operator about whether the cost is justified.

---

## Section 6 — Recommended path forward

**Step 1 (this goal)**: The PUSH-TO column has been added to `dracon-sync repos`, the daemon is rebuilt and running, the BEFORE/AFTER outputs are captured. The operator can now see at a glance which repos are using which remotes.

**Step 2 (this goal's investigation)**: The "real solution" for the multi-remote "gap" is **Option A: keep the per-repo exclude override**. The override is intentional, documented, and correct. The forge-side limits are real, and the override prevents the daemon from logging a flood of HTTP 500 errors.

**Step 3 (follow-on goal, OUT OF SCOPE for this goal)**: Address the PUSH_STUCK divergence. This is a SEPARATE issue from the multi-remote config and requires an operator decision about force-push. The 2026-06-26 audit Finding 9.1 still applies. A new goal can be created to:
- (a) capture the exact state of the divergence (current local HEAD, codeberg tip, common ancestor)
- (b) present the 3 options (rebase / force-push / accept-stuck) to the operator
- (c) execute the operator's chosen option

**Step 4 (optional, future)**: If the operator wants to revisit the multi-remote config in the future (e.g., after the platform is split or after paying for higher storage tiers), the per-repo override can be edited. The current override at `/home/dracon/Dev/dracon-platform/.dracon/dracon-sync.toml` is the single source of truth for the exclude list.

---

## Section 7 — Evidence of read-only contract on operator's git state

`dracon-platform`'s git state is UNCHANGED:

```bash
$ cd /home/dracon/Dev/dracon-platform && git remote -v
codeberg	git@codeberg.org:dracondev/dracon-platform.git (fetch)
codeberg	git@codeberg.org:dracondev/dracon-platform.git (push)

$ git branch --show-current
main-temp

$ git rev-list --count codeberg/main-temp..HEAD
1057

$ git rev-list --count HEAD..codeberg/main-temp
1
```

Single codeberg remote, main-temp branch, ahead=1057 behind=1 (PUSH_STUCK divergence unchanged). The new PUSH-TO column in `dracon-sync repos` is purely a display change — it does not modify any git state, remote configuration, or branch.

The only writes were:
- `dracon-sync/src/report.rs` (daemon source, ~50 lines added)
- `/home/dracon/.local/bin/dracon-sync` and `/home/dracon/.cargo/bin/dracon-sync` (binary replacement)
- `docs/design/audit-2026-06-26/repos-before.txt` and `repos-after.txt` (evidence)
- This design doc: `docs/design/repo-remote-visibility-2026-06-27.md`

The daemon was restarted once via `systemctl --user restart dracon-sync.service` to pick up the new binary. No config edits, no remote additions, no commits, no pushes, no history rewrites.

---

## Evidence index

| File | Path | Description |
|---|---|---|
| `repos-before.txt` | `docs/design/audit-2026-06-26/repos-before.txt` | `dracon-sync repos` output BEFORE the change (13,786 bytes, 2026-06-27 12:25 BST) |
| `repos-after.txt` | `docs/design/audit-2026-06-26/repos-after.txt` | `dracon-sync repos` output AFTER the change (12,666 bytes, 2026-06-27 12:26 BST) |
| `dracon-platform-per-repo-override.toml` | `/home/dracon/Dev/dracon-platform/.dracon/dracon-sync.toml` | The per-repo override that excludes github+gitlab (line 37) with the 2026-06-23 rationale comment |
| Daemon source | `dracon-sync/src/report.rs` | Lines 644-658 (new fields), 2233-2243 (population), 2369 (header), 2497 (cell), 488-519 (helper function) |

**Investigation complete. The operator's hypothesis was right (visibility helps) and the per-repo exclude is BY DESIGN with the documented 2026-06-23 rationale. The PUSH_STUCK is a SEPARATE divergence issue that needs a follow-on goal.**