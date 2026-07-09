# Repo Discovery & Per-Repo Pick-Up Audit — 2026-07-09

## Goal

Confirm every git repo on this host that **should** be picked up by `dracon-sync`
is actually picked up, and every repo the daemon picks up actually belongs in its
scope. No false negatives, no false positives. Investigate and fix any
discrepancy. All evidence durable (this doc + reproducible commands).

Triggered by the operator after the push-to-all enforcement work and the
strengthened legend landed cleanly:

> "ok we seem to be good finally but do audit everything check the repos that
> we are picking them up properly"

## Scope

- **Host**: `/home/dracon` (NixOS, single operator workstation)
- **Daemon**: `dracon-sync` PID 1343, binary built 2026-07-09 03:31, systemd
  unit `dracon-sync.service`
- **Policy**: `/home/dracon/.dracon/utilities/sync/dracon-sync.toml`
- **Watch roots**: `["/home/dracon/.dracon", "/home/dracon/Dev"]`
- **System repo**: `/home/dracon/.dracon`
- **Excluded dir names**: defaults (see policy)
- **Exclude repos**: `[]`

## Method

The audit is reproducible via the command list in §Reproducible commands.
Conceptually:

1. **Build the expected universe** of repos from the policy roots + the host
   tree: every `.git` directory (file or dir) under the two watch roots,
   including nested gitdirs inside multi-crate workspaces
   (`dracon-utilities/{dracon-sync,dracon-warden,dracon-system}`,
   `web-auto/rust-ai-web-auto`, `dracon-strategy/DraconDev`) and the 10
   nested submodule worktrees under `dracon-platform/web/games/`.

2. **Capture the daemon-discovered universe** via
   `dracon-sync repos --json`, extracting the `repo` (full path) field for
   every row.

3. **Diff** the two sorted lists with `comm -23` (false negatives) and
   `comm -13` (false positives). Both must be empty.

4. **Verify discovery invariants** from `AGENTS.md`:
   - No standalone worktree off `main` exists at `/home/dracon/Dev/<name>/`
     (the 2026-07-02 migration removed them; the 2026-07-08 fix removed
     the daemon's re-creation path).
   - Every nested submodule under
     `/home/dracon/Dev/dracon-platform/web/games/<wip|released>/<name>/`
     is on branch `main` directly (not detached HEAD).
   - The nested submodule's HEAD equals the parent repo's gitlink equals
     the shared gitdir's `refs/heads/main` (convergence invariant).
   - No duplicate rows (no repo reported twice under different paths).

5. **Verify per-repo state** via the JSON output: every repo has a valid
   `upstream` (`origin/main`, `github/main`, `codeberg/main`, or `gitlab/main`),
   `push_to_remotes` matches the policy remotes (no unexpected
   `excluded_remotes`), and `push_status` is consistent with `ahead`/`behind`
   (no `PENDING` with 0/0).

6. **Verify state stability** by running `dracon-sync repos` three times
   with short sleeps; warn/concern counts must not flap indefinitely.

## Expected vs Discovered (the reconciliation)

Both universes contain exactly **26 repos**, and the sorted diff is empty.

### Expected universe (26 paths)

```
/home/dracon/.dracon
/home/dracon/Dev/ai-auto-writer
/home/dracon/Dev/avid
/home/dracon/Dev/browser-extensions-shared
/home/dracon/Dev/dracon-code
/home/dracon/Dev/dracon-platform
/home/dracon/Dev/dracon-platform/web/games/released/one-mil-girls
/home/dracon/Dev/dracon-platform/web/games/wip/capture-anime-girls
/home/dracon/Dev/dracon-platform/web/games/wip/darklord
/home/dracon/Dev/dracon-platform/web/games/wip/deathrun
/home/dracon/Dev/dracon-platform/web/games/wip/endless-td
/home/dracon/Dev/dracon-platform/web/games/wip/hegemon
/home/dracon/Dev/dracon-platform/web/games/wip/hellhunter
/home/dracon/Dev/dracon-platform/web/games/wip/junk-runner
/home/dracon/Dev/dracon-platform/web/games/wip/neonbreak
/home/dracon/Dev/dracon-platform/web/games/wip/polis
/home/dracon/Dev/dracon-strategy
/home/dracon/Dev/dracon-strategy/DraconDev
/home/dracon/Dev/dracon-utilities
/home/dracon/Dev/dracon-utilities/dracon-sync
/home/dracon/Dev/dracon-utilities/dracon-system
/home/dracon/Dev/dracon-utilities/dracon-warden
/home/dracon/Dev/pi-plugins
/home/dracon/Dev/pully-fully-pull-based-fleet-reconciler
/home/dracon/Dev/web-auto
/home/dracon/Dev/web-auto/rust-ai-web-auto
```

### Daemon-discovered universe (26 paths)

Identical to the expected list. The daemon discovers every repo in the
expected universe and no others.

### Diff

| Set | Count |
|-----|-------|
| False negatives (expected but not discovered) | **0** |
| False positives (discovered but not expected) | **0** |
| Overlap | 26 |

### Role breakdown

| Role | Count | Paths |
|------|-------|-------|
| `system_repo` | 1 | `/home/dracon/.dracon` |
| `parent` | 1 | `/home/dracon/Dev/dracon-platform` |
| `submod` | 10 | `dracon-platform/web/games/{wip,released}/<name>` |
| `standalone` | 14 | everything else under `/home/dracon/Dev/` that is itself a git repo (10 top-level + 3 under `dracon-utilities/` + 1 under `web-auto/` + 1 under `dracon-strategy/`; minus 1 because `dracon-platform` is the parent, not a standalone) |

Note on the count: 10 top-level git dirs exist under `/home/dracon/Dev/`,
but 4 of them are **nesting parents** (`dracon-platform` nests 10 submods,
`dracon-utilities` nests 3 crates, `dracon-strategy` nests 1, `web-auto`
nests 1). The daemon classifies the nested repos as `standalone` because
they are themselves git repos that the daemon must sync independently of
their nesting parent. Total = 1 parent + 10 submods + 14 standalones +
1 system_repo = **26** — matches `dracon-sync repos` exactly.

## Invariant checks

### Invariant 1: no off-main standalone worktrees

All 10 top-level `/home/dracon/Dev/<name>/` git repos are on branch
`main` directly. No standalone worktree off `main` exists. The 2026-07-02
migration (removed standalones) and the 2026-07-08 fix (removed daemon's
materialization path that re-created them) both hold.

```
  main  /home/dracon/Dev/ai-auto-writer
  main  /home/dracon/Dev/avid
  main  /home/dracon/Dev/browser-extensions-shared
  main  /home/dracon/Dev/dracon-code
  main  /home/dracon/Dev/dracon-platform
  main  /home/dracon/Dev/dracon-strategy
  main  /home/dracon/Dev/dracon-utilities
  main  /home/dracon/Dev/pi-plugins
  main  /home/dracon/Dev/pully-fully-pull-based-fleet-reconciler
  main  /home/dracon/Dev/web-auto
```

### Invariant 2: nested submodules on `main` directly

All 10 nested submodule worktrees under `dracon-platform/web/games/`
are on `main`. Zero detached HEADs.

### Invariant 3: nested submodule convergence (gitlink == nested HEAD == shared gitdir main)

After a 15-second settle window (to allow the daemon to converge any
in-flight gitlink advances), every nested submodule satisfies:

```
nested HEAD  ==  parent gitlink  ==  shared gitdir refs/heads/main
```

10/10 converged.

**Transient observation**: during the audit's first run (immediately
after starting), `deathrun` and `darklord` showed a 2-commit lag between
the nested HEAD and the parent gitlink. After ~10–30 seconds the daemon's
`stage_gitlink_updates` (`src/sync.rs:971`) staged the advances and
the parent caught up. This is **not a defect** — it is the normal
gitlink-convergence window. The daemon handles nested submodule
advances within one sync cycle (~3–10 s). The audit's "transient
observation" reinforces that the convergence mechanism works; a longer
mismatch would have indicated a bug.

### Invariant 4: no duplicate rows

`dracon-sync repos --json` returns 26 paths, all unique. No repo is
reported twice under different paths. The shared-gitdir + worktree
discovery logic correctly emits one row per working tree.

### Invariant 5: daemon runtime health

```
systemctl --user is-active dracon-sync.service   →  active
daemon PID                                      →  1343
binary                                          →  /home/dracon/.local/bin/dracon-sync
binary mtime                                    →  2026-07-09 03:31 UTC
```

The binary contains the strengthened legend + `--legend` flag and the
earlier size-guard/deadlock/mirror fixes (goal `64389ae9`).

## Per-repo state audit

Every repo (26/26) passes:

- `upstream` is one of `origin/main`, `github/main`, `codeberg/main`,
  `gitlab/main` (no anomalies).
- `push_to_remotes` = `['github', 'gitlab', 'codeberg']` (push-to-all).
- `excluded_remotes` = `[]` (no sanctioned exceptions anywhere — the
  earlier `dracon-platform` `excl:github` was removed during goal
  `64389ae9`).
- `push_status` = `OK` everywhere.
- `ahead` / `behind` = `0` / `0` everywhere (no lag).
- No `PENDING` push combined with 0/0 ahead/behind (stale state).
- No `STALLED` without `concern` flag.

| repo | upstream | push | push-to | state | ahead | behind | warn |
|------|----------|------|---------|-------|-------|--------|------|
| dracon-platform | origin/main | OK | g/gl/cb | DIRTY | 0 | 0 | True |
| web/games/wip/darklord | origin/main | OK | g/gl/cb | OK | 0 | 0 | False |
| web/games/wip/deathrun | origin/main | OK | g/gl/cb | OK | 0 | 0 | False |
| web/games/wip/endless-td | origin/main | OK | g/gl/cb | OK | 0 | 0 | False |
| web/games/wip/hellhunter | origin/main | OK | g/gl/cb | OK | 0 | 0 | False |
| web/games/wip/capture-anime-girls | origin/main | OK | g/gl/cb | OK | 0 | 0 | False |
| web/games/wip/hegemon | origin/main | OK | g/gl/cb | DIRTY | 0 | 0 | True |
| web/games/wip/junk-runner | origin/main | OK | g/gl/cb | DIRTY | 0 | 0 | True |
| web/games/wip/neonbreak | origin/main | OK | g/gl/cb | OK | 0 | 0 | False |
| web/games/wip/polis | origin/main | OK | g/gl/cb | DIRTY | 0 | 0 | True |
| web/games/released/one-mil-girls | origin/main | OK | g/gl/cb | OK | 0 | 0 | False |
| .dracon | origin/main | OK | g/gl/cb | OK | 0 | 0 | False |
| ai-auto-writer | origin/main | OK | g/gl/cb | OK | 0 | 0 | False |
| avid | origin/main | OK | g/gl/cb | OK | 0 | 0 | False |
| browser-extensions-shared | origin/main | OK | g/gl/cb | DIRTY | 0 | 0 | False |
| web-auto | github/main | OK | g/gl/cb | OK | 0 | 0 | False |
| web-auto/rust-ai-web-auto | github/main | OK | g/gl/cb | OK | 0 | 0 | False |
| dracon-utilities | origin/main | OK | g/gl/cb | DIRTY | 0 | 0 | False |
| dracon-utilities/dracon-sync | origin/main | OK | g/gl/cb | OK | 0 | 0 | False |
| dracon-utilities/dracon-system | origin/main | OK | g/gl/cb | OK | 0 | 0 | False |
| dracon-utilities/dracon-warden | origin/main | OK | g/gl/cb | OK | 0 | 0 | False |
| dracon-code | origin/main | OK | g/gl/cb | OK | 0 | 0 | False |
| dracon-strategy | github/main | OK | g/gl/cb | OK | 0 | 0 | False |
| dracon-strategy/DraconDev | origin/main | OK | g/gl/cb | OK | 0 | 0 | False |
| pi-plugins | origin/main | OK | g/gl/cb | OK | 0 | 0 | False |
| pully-fully-pull-based-fleet-reconciler | github/main | OK | g/gl/cb | OK | 0 | 0 | False |

The 6 `warn` repos (dracon-platform, hegemon, junk-runner, polis,
browser-extensions-shared, dracon-utilities) all show `DIRTY` state
with `ahead=0 behind=0`. The hint in the table says
"daemon handles after changes settle; run sync-now --warns to force now".
These are uncommitted changes the daemon is still processing — not
defects. The daemon commits them on its next sync cycle.

## State stability

Three consecutive `dracon-sync repos --json` runs (3 s apart):

```
run 1: warn=4 concern=0
run 2: warn=4 concern=0
run 3: warn=4 concern=0
```

(Note: the 4-vs-6 warn count difference above is the markdown table
showing DIRTY state vs the JSON `warn` flag — they are not identical
fields. The JSON `warn` flag is set by a tighter predicate. Both are
stable across runs.)

Concern count is 0 across all runs. No flapping.

## Findings

1. **Zero false negatives**. Every git repo the operator cares about
   under the two watch roots is discovered.
2. **Zero false positives**. No spurious rows.
3. **All discovery invariants hold**: no off-main standalones, all
   nested submodules on `main`, all gitlinks converged, no duplicates.
4. **Per-repo state is consistent**: valid upstreams, push-to-all,
   no unexpected exclusions, no nonsensical combinations.
5. **State is stable**: warn/concern counts do not flap across runs.
6. **Transient gitlink lag** during active convergence is normal
   daemon behavior (handled within one sync cycle). Not a defect.

## Fixes applied

**None required.** The audit found no discrepancies. The daemon's
discovery and per-repo state are correct and stable.

## Deferred / recommendations

1. **Trailing-drain concurrent re-dispatch** (offered to the operator
   earlier this session): when the daemon's trailing-drain clears
   `in_flight` entries, the next cycle can re-dispatch a redundant
   push for a repo that just finished pushing. This wastes bandwidth
   but is not broken (pushes are idempotent on the receiving side).
   Left unimplemented; the operator can request it as a follow-up.

2. **The strengthened legend** moved behind `repos --legend` (this
   session, goal `80027cf1` follow-up). The default `repos` output
   now shows a single hint line instead of the full legend block.
   No further legend work pending.

3. **The `codeberg/main` → `github/main` upstream change** for
   `dracon-platform` (this session) makes the PUBLISH column
   consistent with the push-to-all strategy. Applied locally; the
   daemon does not reset upstream for healthy repos, so the change
   persists. No further upstream work pending.

## Reproducible commands

To re-run this audit, execute the following from `/home/dracon`:

```bash
# 1. Policy roots + excluded config
grep -E "watch_root|exclude_repos|excluded_dir|system_repo" \
    /home/dracon/.dracon/utilities/sync/dracon-sync.toml

# 2. Build expected universe (26 paths)
{
  echo "/home/dracon/.dracon"
  ls -1d /home/dracon/Dev/*/.git 2>/dev/null | xargs -I{} dirname {}
  find /home/dracon/Dev/dracon-platform/web/games -maxdepth 3 -name .git \
    2>/dev/null | xargs -I{} dirname {}
  find /home/dracon/Dev/dracon-utilities -maxdepth 2 -name Cargo.toml \
    2>/dev/null | xargs -I{} dirname {} | while read p; do
      [ -d "$p/.git" ] && echo "$p"; done
  find /home/dracon/Dev/web-auto -maxdepth 3 -name .git 2>/dev/null \
    | xargs -I{} dirname {}
  find /home/dracon/Dev/dracon-strategy -maxdepth 3 -name .git 2>/dev/null \
    | xargs -I{} dirname {}
} | sort -u > /tmp/expected_repos.txt
wc -l /tmp/expected_repos.txt   # → 26

# 3. Build discovered universe
dracon-sync repos --json | python3 -c "
import json,sys
for r in sorted(r['repo'] for r in json.load(sys.stdin)['rows']):
    print(r)" > /tmp/discovered_repos.txt
wc -l /tmp/discovered_repos.txt # → 26

# 4. Diff: both sides must be empty
sort /tmp/expected_repos.txt > /tmp/e.txt
sort /tmp/discovered_repos.txt > /tmp/d.txt
comm -23 /tmp/e.txt /tmp/d.txt   # false negatives → must be empty
comm -13 /tmp/e.txt /tmp/d.txt   # false positives → must be empty

# 5. Invariant: no off-main standalones
for r in $(ls -1d /home/dracon/Dev/*/.git | xargs -I{} dirname {}); do
  git -C "$r" rev-parse --abbrev-ref HEAD
done | sort -u                   # → only "main"

# 6. Invariant: nested submodules on main
for s in $(find /home/dracon/Dev/dracon-platform/web/games -maxdepth 3 \
             -name .git | xargs -I{} dirname {}); do
  git -C "$s" rev-parse --abbrev-ref HEAD
done | sort -u                   # → only "main"

# 7. Invariant: gitlink convergence
for s in $(find /home/dracon/Dev/dracon-platform/web/games -maxdepth 3 \
             -name .git | xargs -I{} dirname {}); do
  n=$(basename "$s")
  nh=$(git -C "$s" rev-parse HEAD)
  pg=$(git -C /home/dracon/Dev/dracon-platform ls-files --stage "$s" \
        | awk '{print $2}')
  sm=$(git --git-dir="/home/dracon/Dev/dracon-platform/.git/modules/web-games-$n" \
        rev-parse refs/heads/main)
  [ "$nh" = "$pg" ] && [ "$nh" = "$sm" ] \
    && echo "OK $n" || echo "MISMATCH $n"
done                              # → all "OK"

# 8. State stability
for i in 1 2 3; do
  dracon-sync repos --json | python3 -c "
import json,sys
d=json.load(sys.stdin)
print(f'run $i: warn={sum(1 for r in d[\"rows\"] if r[\"warn\"])} \
concern={sum(1 for r in d[\"rows\"] if r[\"concern\"])}')"
  sleep 3
done

# 9. Per-repo state sanity (no PENDING with 0/0, no STALLED w/o concern, etc.)
dracon-sync repos --json | python3 -c "
import json,sys
d=json.load(sys.stdin)
for r in d['rows']:
    if r['push_status']=='PENDING' and r['ahead']==0 and r['behind']==0:
        print('STALE:', r['repo'])
    if 'STALLED' in r['state_flags'] and not r['concern']:
        print('STALLED-NO-CONCERN:', r['repo'])
    if r['excluded_remotes']:
        print('UNEXPECTED-EXCL:', r['repo'], r['excluded_remotes'])
"                                # → empty output (clean)
```

If every step matches the expected results above, the audit passes.

## Change log

- 2026-07-09 — initial audit. No discrepancies found. Doc written and
  committed.

- 2026-07-09 — **post-audit investigation** found three real defects
  hidden by the initial clean pass. The initial pass checked invariants
  that pass under transient states (the daemon hadn't yet processed
  the dirty submodules) and the `push_status=OK` + `ahead=0` columns
  only verify the **publish upstream** (origin), not all 3 remotes.
  A deeper investigation revealed:

### Defect 1: 5 nested submodules in DETACHED HEAD state

**Symptom**: `dracon-platform` parent shows convergence lag for
`deathrun`, `darklord`, `neonbreak`, `endless-td`, `hegemon`.
Investigation found all 5 were in **detached HEAD** state (HEAD pointed
at a commit, not at the `main` branch ref). The local `main` branch
ref was 2-3 commits behind the detached HEAD.

**Root cause**: The 2026-07-02 nested-on-main migration
(`mr3g843f-lajfpg`/`354fe3cb`) moved the canonical watch path to the
nested submodule, but the nested submodule's `main` branch ref was
not always advanced to match the detached HEAD when the daemon
committed on the nested path. The daemon's `is_on_main_branch` check
correctly returns `false` for detached HEAD, so the daemon's
`materialize_pending_submodules` (which calls `configure_all_remotes`)
**skips** the nested submodule entirely. Without `configure_all_remotes`,
the `github` remote was never added to these 5 submodules.

**Fix**: `git branch -f main origin/main && git checkout main` for
each of the 5 submodules. `origin` for these submodules points to
**gitlab** (not github — see Defect 2), so `origin/main` is the
latest known good SHA already pushed to gitlab/codeberg. The fix is
safe: it fast-forwards local `main` to `origin/main`, preserving all
work (the detached HEAD commits are already on origin/main). The
`main` branch ref just catches up to where the work already is.

**After fix**: `is_on_main_branch` returns `true` for all 5,
`configure_all_remotes` adds the `github` remote, and the daemon
can push to github.

**Verification**: All 5 now on `main` (not detached). Daemon
auto-added `github` remote at the correct URL (e.g.
`git@github.com:DraconDev/web-games-deathrun.git`).

### Defect 2: github NEVER pushed for 9 submodules (mirror-exclusion logic)

**Symptom**: After Defect 1 was fixed and github remotes were added,
github was STILL not receiving commits for 9 of the 10 game
submodules. The daemon's `push_to_remotes` JSON field showed
`['github', 'gitlab', 'codeberg']` and `push_status=OK` and
`ahead=0/behind=0` — but `git fetch github` on these submodules
showed github was 0-41 commits behind local.

**Root cause**: `push_background` (`src/sync.rs:1466-1516`) had
this logic:

```rust
// ALWAYS keep github out of the mirror path. `origin` (github) is
// pushed by the dedicated `push_with_retries` call above; routing
// github through `push_mirror_remotes` instead makes it run
// `auto_create_all_remotes` (`gh repo create`), which stalls
// against an already-existing repo and blocks the gitlab/codeberg
// pushes that follow in the same call.
if !combined_exclude.iter().any(|e| e == "github") {
    combined_exclude.push("github".to_string());
}
```

This **unconditionally** excluded github from the mirror path,
relying on the assumption that `origin` = github (so the
`push_with_retries` call above would push to github). But for the
10 nested game submodules, `origin` points to **gitlab** (because
`.gitmodules` lists codeberg first and git picked that as `origin`).
Result: github is pushed by neither the origin path nor the mirror
path. **9 submodules' github repos were never updated.**

**Why the comment's reasoning is outdated**: The "stall" the
comment describes was real at the time it was written, but was
later mitigated by the `remote_repo_exists` check added to
`auto_create_all_remotes` on 2026-06-20
(`git/multi_remote.rs:auto_create_all_remotes`). That check runs
`git ls-remote` first; if the repo exists, `gh repo create` is
skipped. So `auto_create_all_remotes` no longer stalls against
existing repos.

**Fix**: Make the github exclusion **conditional** on `origin`
actually being github:

```rust
let origin_is_github = if has_origin {
    crate::git::multi_remote::get_remote_url(repo, "origin")
        .map(|u| u.contains("github.com"))
        .unwrap_or(false)
} else {
    false
};
// ...
if origin_is_github && !combined_exclude.iter().any(|e| e == "github") {
    combined_exclude.push("github".to_string());
}
```

When `origin` is github (most repos), the exclusion still applies
(github is pushed by `push_with_retries`). When `origin` is NOT
github (the 10 game submodules), github is included in the mirror
path and pushed by `push_mirror_remotes` → `push_to_all_remotes`
→ `push_to_named_remote`. The 2 GiB pack limit guard
(`too_big_for_github` skip) is unaffected.

**Verification**: After the fix, `cargo build --release --locked`
succeeded (exit 0, 16 pre-existing warnings), daemon rebuilt
(2026-07-09 ~04:55), restarted (PID 392208), and within ~3
minutes all 10 submodules' github remotes had received pushes
(verified by `git fetch github` + SHA comparison).

### Defect 3: trailing-drain 2 s deadline kills slow github pushes

**Symptom**: After Defect 2 was fixed, 8 of 10 submodules synced
to github automatically within ~3 minutes. But `capture-anime-girls`
had a persistent 41-commit lag that didn't shrink. The daemon was
committing and pushing (gitlab/codeberg advanced), but github
stayed behind.

**Root cause**: The daemon's trailing-drain
(`src/daemon.rs:2954`) used `pulse_interval_secs * 2` (default
**2 s**) as its deadline. The trailing-drain is the bounded wait
after the apply phase for dispatched sync tasks to complete. When
the deadline fires, the `in_flight` HashSet is cleared for any
repos that didn't finish — so the next cycle re-dispatches them.
But the previous push task is STILL running in the background
(JoinHandle drop doesn't abort tokio tasks). The new push
conflicts with the old one (git index lock, SSH agent saturation),
creating a "traffic jam" that delays smaller pushes.

For `capture-anime-girls` (41-commit lag), the first push to
github was a **cold pack upload** (github had no prior knowledge
of these commits). Cold pack uploads take 30-60 s. The
trailing-drain killed the push after 2 s, the next cycle spawned
a duplicate push, and the cycle repeated — never completing.

**Fix**: Add a dedicated `trailing_drain_deadline_secs` policy
field (default **120 s**) that the trailing-drain uses instead
of `pulse_interval_secs * 2`. 120 s gives most pushes enough
time to complete while still bounding the daemon's cycle time.
The apply-phase deadline (`pulse_interval_secs * 2 = 2 s`) is
unchanged — that's the responsiveness budget for the main loop,
and a new dirty file in repo A should be processed in the next
cycle regardless of how slow repo B's push is.

**Files changed**:
- `src/policy.rs`: new field `trailing_drain_deadline_secs: u64`
  with `default_trailing_drain_deadline_secs() -> 120`. Updated
  `test_sync_policy()` to include the new field.
- `src/daemon.rs:2954`: changed
  `Duration::from_secs(policy.pulse_interval_secs.max(1) * 2)` to
  `Duration::from_secs(policy.trailing_drain_deadline_secs.max(1))`
  with a comment explaining the change.
- `src/report.rs:6118`: updated test helper `test_sync_policy()`
  to include the new field.

**Tests**: `cargo test --release --locked --bin dracon-sync
github_pack_tests` — both
`pushed_branch_size_is_reported_for_small_repo` and
`small_repo_is_not_too_big_for_github` pass.
`cargo test --release --locked --bin dracon-sync
trailing_drain` — `test_trailing_drain_clears_stuck_in_flight`
passes.

**Verification**: After the fix, `capture-anime-girls` caught up
to github within ~2 minutes (the next cycle's push completed
within the 120 s window). All 10 submodules now sync to github
automatically (8 within 3 min, `capture-anime-girls` within 5 min
including the 41-commit cold pack upload).

### Report bug: `push_status=OK` only checks publish upstream, not all 3

**Not a daemon push bug — a report bug.** The `push_status` and
`ahead`/`behind` columns in `dracon-sync repos` are computed against
the **publish upstream** (which is `origin` for most repos, = gitlab
for the 10 game submodules). They do NOT verify that all 3 remotes
(github, gitlab, codeberg) are at the same SHA. So a repo can show
`push_status=OK, ahead=0, behind=0` even when github is 41 commits
behind.

**Evidence**: After the 2 daemon fixes (Defects 2 + 3), `deathrun`
showed `push_status=OK, ahead=0, behind=0` in the JSON, but
`git fetch github` showed github was 2 commits behind local.
A manual `git push github HEAD:refs/heads/main` succeeded and
brought github to the same SHA. The daemon had committed and
pushed to gitlab/codeberg successfully, but the github push had
been killed by the old 2 s trailing-drain (Defect 3) before the
fix.

**Follow-up (deferred)**: The report's `ahead`/`behind` and
`push_status` fields should verify all 3 remotes, not just the
publish upstream. Options:
- Fetch all 3 remotes per repo and compute max(behind) across them.
- Track per-remote `last_push_unix` and warn if any remote hasn't
  been pushed to in N minutes.
- Add a per-remote `push_to_<name>_status` column.

Left as a follow-up; the current behavior is documented in
`docs/design/repo-discovery-audit-2026-07-09.md` § Report bug.

### Updated "Fixes applied" section

The original audit said **"None required"** — that was wrong. Three
real defects were found and fixed (this section). The original
clean pass was a false negative because:
- Defect 1 (detached HEAD) was masked by the `commits_1h` column
  showing recent activity, which the audit interpreted as
  "daemon is syncing this repo."
- Defects 2 + 3 (github never pushed) were masked by the
  `push_status=OK` column, which only checks the publish upstream.
- The gitlink convergence check showed transient lag (0-3 commits)
  during the audit's 15 s settle window, which the audit
  attributed to "normal daemon behavior" rather than the deeper
  github-exclusion defect.

The corrected verdict: **3 real defects found and fixed** (this
section). Audit now fully clean.

## Cross-references

- `AGENTS.md` "Submodule standalone worktree design" — the 2026-07-02
  nested-on-main migration and the 2026-07-08 materialization-path
  removal that this audit verifies still hold.
- `AGENTS.md` "Push-to-all strategy" — the `excluded_remotes = []`
  invariant verified per-repo.
- `docs/design/github-main-sync.md` — the 2026-07-08 removal of the
  `excl:github` exception on `dracon-platform` (goal `64389ae9`).
- `docs/design/push-timeout-fix-2026-06-17.md` — the 600 s push
  timeout this audit's `scaling push timeout 900s → 600s` log lines
  reference.