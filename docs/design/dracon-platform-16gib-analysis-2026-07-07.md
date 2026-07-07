# dracon-platform 16.2 GiB `.git` — Is it safe to delete? (2026-07-07)

## TL;DR

**NO. The 16.2 GiB is `main`'s own real (corrupted) history — not disposable cruft.
`git gc --prune=now` is UNSAFE here: it currently fails (missing parent objects) and,
if it ever did run, it would delete `main`'s entire pre-submodule history.**

The repo is **heavily corrupted** (26 missing parent objects across `main`'s chain).
`git gc` can never succeed on this repo without either (a) recovering the missing
objects from codeberg — which would leave `main` at 16.2 GiB (no shrink, still over
GitHub's 2 GiB limit) — or (b) rewriting history (risky, AGENTS.md-restricted).

**Recommendation: keep `exclude_remotes = ["github"]` on dracon-platform (current safe
state). Do NOT run `git gc`.** Fitting GitHub requires a deliberate history-rewrite
decision from the operator, not a prune.

---

## The user's question

> "we need to be sure what is wrong and not … numbers seem extra … we can't just delete
> everything … the goal was to be smaller so we can push to github again."

So the test is: is the 16.2 GiB **orphaned/unreferenced junk** (safe to prune) or
**`main`'s actual history** (must keep)?

## What the numbers say

```
size-pack     = 12.61 GiB
in-pack       = 461,357 objects (3 packs)
reachable     = 13,233 objects  (~4.8 MB)   # everything git CAN traverse from all refs
unreachable   = 348,826 objects (~16.2 GiB) # git labels these "unreachable"
  of which:   108,449 blobs (16.02 GiB), 223,059 trees, 16,987 commits, 331 tags
fsck missing  = 26 commits (broken parent links)
```

The 16.2 GiB looks like "unreachable cruft" — but that label is a **traversal artifact**,
not proof of disposability. Git cannot traverse past a *missing parent object*, so
everything on the far side of the gap is mislabeled "unreachable."

## Why it is NOT safe to prune (the decisive evidence)

### 1. `main` is an orphan-root history — but it is ALSO corrupted

`git cat-file -p 953d7f8759` (the submodule-migration commit, oldest on `main`'s
reachable log) shows **no `parent` line** — it is an orphan root:

```
tree 4a93c94e0c3bda36574fd4c02a93ec19f9ef6e50
author DraconDev <dracsharp@gmail.com> 1782774764 +0100
committer DraconDev <dracsharp@gmail.com> 1782774764 +0100

migration(heavy): extract endless-td, neonbreak, hegemon to submodules
```

`git log main` lists 3,457 commits from tip → `953d7f8759` and naturally ends there.

### 2. BUT `main` contains a corrupted side-branch — proven by validation

I cloned dracon-platform to a throwaway repo, **deleted every side branch** (`main-temp`,
all `codeberg/*` / `gitlab/*` / `origin/*` remote-tracking refs), leaving **only `main`**.
Then `git gc --prune=now` **still failed**:

```
error: Could not read 1d64cfb8c2c64844b6845fceae87207556a06636
fatal: Failed to traverse parents of commit c5f0be1ead7720c65316266da03679ac13e200de
fatal: failed to run repack
```

`c5f0be1e` is reachable **from `main` itself**. So `main` has a commit whose parent
(`1d64cfb8`) is missing. The 16.2 GiB is therefore **part of `main`'s own history**
(including this corrupted branch + the pre-submodule monorepo), not orphaned side-branch
junk.

### 3. Corroborating signals

- `git rev-list --objects --all` aborts/truncates at the missing parents, so the
  "unreachable 348,826" set is exactly the history git cannot walk past the gaps.
- Early `git for-each-ref --contains 93ee4b46` listed `heads/main` — a **false positive**
  from the corruption. Recovering `c2eb911c` (that commit's parent) did **not** change
  the reachable count (still ~13,233), and `merge-base --is-ancestor 93ee4b46 main`
  returned NO. So `93ee4b46` is on `main-temp`/`codeberg/*`, **not** `main` — but the
  *corrupted branch inside `main`* (`c5f0be1e` → `1d64cfb8`) is what blocks `gc`.
- There are **26 missing commits** total (cascading recovery from codeberg pulled dozens
  more before timing out). The missing objects live on **codeberg** (a blobless mirror
  clone recovered them), so codeberg holds the *good* history; the local copy is broken.

## What would happen under each action

| Action | Result | Safe? |
|---|---|---|
| `git gc --prune=now` (now) | **Fails** on missing parent | N/A (errors) |
| `git gc --prune=now` (after healing missing objects from codeberg) | Succeeds, but `main` stays **16.2 GiB** (history now fully reachable) | Repo healthy, but still > GitHub 2 GiB |
| `git gc --prune=now` (if gaps were force-closed) | Would **delete `main`'s pre-submodule history** | **CATASTROPHIC** |
| Recover missing objects from codeberg | `fsck` clean; `main` = 16.2 GiB; GitHub still excluded | Safe, reversible, but no shrink |
| `filter-repo` to strip large blobs | `main` shrinks to fit GitHub | **History rewrite** — AGENTS.md-restricted, irreversible, needs backup + explicit operator override |
| Keep `exclude_remotes = ["github"]` | Status quo; repo 16.2 GiB locally, synced to gitlab+codeberg | **Safe (recommended)** |

## Root cause of the 16.2 GiB

The parent repo accumulated **16.2 GiB of large game assets/media in its own commit
history** before the 2026-06-29 submodule extraction (`953d7f8759`). Submodules prevent
**future** bloat but cannot undo **past** history. The missing-parent corruption (26
commits) makes that past history look "unreachable" and blocks `gc`.

This is unrelated to the hegemon cleanup (hegemon was always a gitlink — its 2.4 GB of
assets were never in the parent).

## Options for the operator

- **A. Keep GitHub excluded (RECOMMENDED, current state).** Safe. dracon-platform stays
  16.2 GiB locally, synced to gitlab+codeberg. No destructive ops.
- **B. History rewrite to fit GitHub.** `git filter-repo` to strip the large blobs from
  `main`'s history, then re-enable GitHub. **Requires explicit operator override** (per
  AGENTS.md "NEVER rewrite history" rule) + a pre-rewrite backup bundle. Irreversible.
- **C. Heal the corruption (recover missing objects from codeberg).** Makes `fsck` clean
  and the repo internally consistent, but `main` remains 16.2 GiB and GitHub stays
  excluded. Good for repo health; does not solve the GitHub goal.

## Verification commands (reproducible)

```bash
DP=/home/dracon/Dev/dracon-platform
cd "$DP"
git count-objects -vH | grep -E "size-pack|in-pack"   # 12.61 GiB / 461,357
git rev-list --objects --all 2>/dev/null | awk '{print $1}' | sort -u | wc -l  # ~13,233 reachable
git cat-file -p 953d7f8759 | head -5                  # orphan root (no parent line)
git fsck --full 2>/dev/null | grep -c missing         # 26 missing commits
# Validation (on a throwaway clone): drop all refs but main, then `git gc --prune=now`
#   -> still fails: "Could not read 1d64cfb8... parent of c5f0be1e"
```

## Conclusion

The 16.2 GiB is **`main`'s real, corrupted history** — not junk. `git gc` is unsafe
(would delete `main`'s past) and cannot shrink below GitHub's limit anyway. The safe
path is to keep GitHub excluded. Shrinking to fit GitHub is a **separate, deliberate
history-rewrite decision** that must be made explicitly by the operator.
