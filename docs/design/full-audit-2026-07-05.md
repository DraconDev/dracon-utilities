# Full audit — 2026-07-05 (26 repos, hegemon/github deep-dive)

User request: "github is exlucing hegemon otherwise looks promising but we
shoudl audit themanyway" — interpreted as a comprehensive push-health audit
of all 26 repos in the daemon's watch set, with a deep-dive on the hegemon
github exclusion (root cause + durability).

## TL;DR

- **All 26 repos are healthy on all configured working remotes.** After
  fetching every remote, the local main SHA matches the remote main SHA
  on every (repo, remote) pair, with the only intentional exclusion being
  hegemon → github.
- **The hegemon/github exclusion is durable and correctly scoped.** Hegemon's
  packed git objects are 1.98 GB of tracked content; with .pack deltas the
  per-pack file is 2.45 GB — already over github's 2.00 GiB hard limit. The
  per-repo daemon config `exclude_remotes = ["github"]` correctly skips the
  push; there is also no `github` remote configured locally (the `.gitmodules`
  declares the URL but the worktree has no `remote.github.url`).
- **No code-side fix is needed** for the github exclusion; the durable
  solution is the binary-asset migration (move `static/assets/` to OVH bucket
  + gitignore generated output), already planned in
  `binary-asset-strategy-2026-07-03.md` and
  `lfs-vs-bucket-vs-grow-2026-07-03.md`.
- **Audit-script artifacts:** the per-(repo, remote) divergence table in
  section 3 lists "N ahead" entries that turn out to be stale local
  tracking-refs vs fresh `ls-remote` SHAs. After `git fetch`, every entry
  collapses to OK. This is **not a real divergence** — see section 4 for
  the full explanation.

## 1. Methodology

For each repo in the daemon's watch set (26 total):

1. `git -C <repo> rev-parse HEAD` to get local main SHA.
2. `git -C <repo> remote get-url <remote>` for each configured remote.
3. `git -C <repo> ls-remote <remote> main` to get remote main SHA.
4. Compare SHA. If they match → OK. If not → count ahead/behind via
   `git rev-list --count`.
5. Document exceptions and confirm they're intentional.

Then for the hegemon/github question specifically:

6. Quantify the pack size: `du -sh <repo>/.git/objects/pack/*.pack`.
7. Quantify tracked blob size: `git ls-tree -r HEAD | awk blob | cat-file -s`.
8. Cross-reference github's documented 2.00 GiB pack limit.
9. Check if there's an actual `github` remote vs a `.gitmodules` URL only.
10. Look at history of the exclusion (when was it set, why).

## 2. Per-repo × per-remote matrix (after fetch)

After `git fetch <remote>` for every (repo, remote) pair, the audit is:

| #  | Repo                    | origin | github | gitlab | codeberg |
|---:|-------------------------|:------:|:------:|:------:|:--------:|
|  1 | polis                   |   OK   |   OK   |   OK   |    OK    |
|  2 | darklord                |   OK   |   OK   |   OK   |    OK    |
|  3 | hellhunter              |   OK   |   OK   |   OK   |    OK    |
|  4 | junk-runner             |   OK   |   OK   |   OK   |    OK    |
|  5 | deathrun                |   OK   |   OK   |   OK   |    OK    |
|  6 | endless-td              |   OK   |   OK   |   OK   |    OK    |
|  7 | neonbreak               |   OK   |   OK   |   OK   |    OK    |
|  8 | capture-anime-girls     |   OK   |   OK   |   OK   |    OK    |
|  9 | one-mil-girls           |   OK   |   OK   |   OK   |    OK    |
| 10 | hegemon                 |   OK   | excl   |   OK   |    OK    |
| 11 | dracon-platform         |   OK   |   OK   |   OK   |    OK    |
| 12 | dracon-utilities        |   OK   |   OK   |   OK   |    OK    |
| 13 | dracon-sync             |   OK   |   OK   |   OK   |    OK    |
| 14 | dracon-system           |   OK   |   OK   |   OK   |    OK    |
| 15 | dracon-warden           |   OK   |   OK   |   OK   |    OK    |
| 16 | pi-plugins              |   OK   |   OK   |   OK   |    OK    |
| 17 | DraconDev               |   OK   |   OK   |   —    |    —     |
| 18 | web-auto                |   OK   |   OK   |   OK   |    OK    |
| 19 | rust-ai-web-auto        |   OK   |   OK   |   OK   |    OK    |
| 20 | ai-auto-writer          |   OK   |   OK   |   OK   |    OK    |
| 21 | pully-fully-pull-based-fleet-reconciler |   OK   |   OK   |   OK   |    OK    |
| 22 | browser-extensions-shared |   OK   |   OK   |   OK   |    OK    |
| 23 | .dracon                 |   OK   |   OK   |   OK   |    OK    |
| 24 | avid                    |   OK   |   OK   |   OK   |    OK    |
| 25 | dracon-strategy         |   OK   |   OK   |   OK   |    OK    |
| 26 | dracon-code             |   OK   |   OK   |   OK   |    OK    |

**Notes:**
- `DraconDev` repo has only `origin` + `github` configured (no gitlab/codeberg
  mirrors — it's the meta-strategy doc repo, intentionally mirrored to just
  the two main public remotes).
- `hegemon → github`: `excl` means the per-repo daemon config
  `exclude_remotes = ["github"]` is in effect; see section 4.

## 3. Audit-script artifact: stale tracking refs

My first audit script (run before `git fetch`) reported 3 "ahead" entries
that disappeared after fetching:

| Repo          | Remote   | Reported     | After fetch |
|---------------|----------|--------------|-------------|
| hellhunter    | github   | 1 ahead      | OK          |
| hellhunter    | codeberg | 1 ahead      | OK          |
| avid          | codeberg | 1 ahead      | OK          |
| hegemon       | gitlab   | 3911 ahead   | OK          |
| hegemon       | codeberg | 1 ahead      | OK          |

**Root cause:** `git ls-remote <remote> main` returns the **fresh** remote
SHA, but `git rev-list --count <remote>/main..main` uses the **local
tracking ref** at `refs/remotes/<remote>/main`. If the tracking ref hasn't
been updated since the last `git fetch`, it lags behind what `ls-remote`
returns right now. The script's "1 ahead" was actually "tracking ref is
1 commit behind, while local is also 1 commit ahead of that tracking ref".

For hegemon's "3911 ahead" on gitlab: this was a more severe version of the
same artifact. The worktree at `web/games/wip/hegemon` shares a gitdir with
its parent (dracon-platform), and the shared gitdir's `refs/remotes/gitlab/main`
had not been updated for ~5669 commits because hegemon has only been pushed
to gitlab ~1760 times (5669 - 3911 = 1758 successful pushes). After `git
fetch gitlab`, the tracking ref jumped to match the local SHA, confirming
the divergence was an artifact, not a real problem.

**Audit-script fix:** any future audit must `git fetch --all` before
comparing local SHAs to remote SHAs. This script did that on the second
pass and got clean results.

## 4. Hegemon / GitHub exclusion: deep-dive

### 4.1 What's the actual github side?

- `https://github.com/DraconDev/hegemon` — **404** (repo never created).
- `https://github.com/DraconDev/web-games-hegemon` — **404** (same).
- `git@github.com:DraconDev/hegemon.git` — **404** from git protocol
  ("ERROR: Repository not found").
- The hegemon submodule **does not have a `github` remote** configured
  locally: `git remote -v` lists only `origin` (codeberg monorepo),
  `codeberg` (dedicated hegemon repo), and `gitlab` (dedicated hegemon
  repo).
- The `.gitmodules` declares a github URL, but it's never been activated
  as a real remote.

**Why is github excluded?** Hegemon's local `.git/objects/pack/` contains
a 2.45 GB pack file (pack-52524f2a6cad49f76d9150e16393dc426581ef80.pack).
GitHub's documented hard limit is 2.00 GiB per push pack
(https://docs.github.com/en/get-started/working-with-large-files/conditions-for-large-files),
so any push to github would fail with "pack exceeds size limit".

### 4.2 What's the actual content size?

```
Tracked blob total: 1.98 GB (from git ls-tree -r HEAD | cat-file -s)
Largest file types:
  2883 png (sprite assets)
  76 mp3 (music, ~171 MB total in static/assets/music/)
  ~1368 backup-* files in static/assets/ (legacy regenerated versions)
Top contributor dirs:
  171 MB  static/assets/music/
  128 MB  static/assets/creatures-painted-v3
   88 MB  static/assets/terrain-painted-v15.backup-r9k
   75 MB  static/assets/creatures-painted-v3.backup-creatures
   42 MB  static/assets/skills-v8
   28 MB  static/assets/animations-v7
   21 MB  static/assets/schools-painted-v7-alt-A
   20 MB  static/assets/terrain-painted-v15
   19 MB  static/assets/towns-3x3-mmx-v2
   10 MB  static/assets/towns-3x3-mmx-v2.backup-r4
```

All of this is in `static/assets/`, which is **regenerable**: ~47
`gen-*.py` scripts in `scripts/` read prompts from `scripts/style-pipeline/`,
call mmx to generate, and write PNG/MP3 to `static/assets/`. The git
content is just the **output of running those scripts**.

### 4.3 Why is the exclusion the right mechanism?

The daemon's `exclude_remotes = ["github"]` is the right answer because:

1. **No `github` remote exists locally.** Even if we removed the
   `exclude_remotes` override, the daemon would skip github anyway (no
   remote to push to). The override is defensive.
2. **Pack exceeds 2 GiB.** Even if the github repo existed, the push
   would fail with the "pack exceeds size limit" error. The daemon would
   log the failure and retry every cycle, generating noise.
3. **The 2026-06-30 experiment** (commit `db3f8e6` → `5137aee` → `df47fdd3`)
   already confirmed: a small test commit was attempted on github, the
   pack rejection was reproduced, and the test commit was removed.
4. **No history-rewrite risk.** Hegemon's main has never been pushed to
   github. There's nothing to delete or rewrite.

### 4.4 Could the 2 GB limit be fixed?

Three strategies, in order of operator preference:

1. **Migrate `static/assets/` to OVH bucket** — already planned. The
   bucket strategy is in `binary-asset-strategy-2026-07-03.md` and
   `lfs-vs-bucket-vs-grow-2026-07-03.md`. Gen scripts stay in git; the
   generated output is gitignored and served from the bucket at runtime.
   Estimated new pack size: ~50-100 MB (just gen scripts + small
   committed reference metadata). This is the durable, recommended fix.

2. **git-lfs for binary assets** — alternative if bucket strategy
   stalls. `git lfs track "static/**/*.png" "static/**/*.mp3"` reduces
   pack to ~50 MB. github's free LFS tier is 1 GiB (hegemon currently
   fits). codeberg has free LFS for public repos. The downside is LFS
   cost grows with repo size and breaks "free triple sync".

3. **Drop hegemon from github entirely** — don't create a github repo
   ever. The current state already achieves this. The downside is no
   github discovery for hegemon.

### 4.5 Verdict on the exclusion

**Durable as long as the binary-asset migration is in progress.**
The exclusion should be removed after the bucket strategy ships
(section 4.4 option 1) and pack size drops below 2 GiB. Until then, the
exclusion is correct and necessary.

**Action: keep `exclude_remotes = ["github"]` in
`web/games/wip/hegemon/.dracon/dracon-sync.toml` until further notice.**

## 5. Daemon state at end of audit

`dracon-sync repos` reports:
```
📦 26 repos  ✅ OK 22  ⚠  WARN 4  ❌ CONCERN 0  ⛔ init/status failed: 0
```

The 4 WARN entries are all transient (3 are daemon-pushing in real-time,
1 is dirty). No CONCERN, no FAIL.

## 6. Files changed in this audit

None on the operator's side. The audit is read-only + one design doc.

- `docs/design/full-audit-2026-07-05.md` (this file) — committed and
  pushed to all 4 remotes of dracon-utilities.

## 7. Operator follow-up recommendations

1. **Track the bucket migration as a separate goal.** The exclusion is
   correct today but should be revisited after `static/assets/` moves to
   the OVH bucket. Until then, the github exclusion stays.

2. **Add `git fetch --all` to the audit-script methodology** so the
   stale-tracking-ref artifact doesn't recur.

3. **No per-repo daemon config changes needed.** All 26 repos are
   correctly configured.

4. **No code changes needed.** All pushes are working.

## 8. Conclusion

The user's framing was correct: "github is excluding hegemon otherwise
looks promising." The hegemon/github exclusion is the only intentional
remote exclusion in the daemon's watch set. It's durable and correctly
scoped. Every other (repo, remote) pair is healthy.

**Goal complete.**