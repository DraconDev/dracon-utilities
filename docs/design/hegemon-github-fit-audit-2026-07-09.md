# Hegemon GitHub-fit audit — 2026-07-09

**Question:** "Didn't we clean up hegemon so it can go on GitHub?"

**Answer:** Hegemon *was* cleaned up and *was* on GitHub (95 MiB history) on 2026-07-06.
But it **re-bloated today (2026-07-09)** via a single commit (`b281256`) that
re-added ~2.4 GiB of binary audit screenshots. It is **currently over GitHub's
hard 2 GiB/pack limit again**, so the daemon skips the github push. The prior
cleanup did NOT achieve a durable < 2 GiB state — the anti-rebloat `.gitignore`
rules that protected it are gone, so the daemon's commit-all policy
re-committed audit outputs.

**Getting it back on GitHub requires a history rewrite (drop `b281256` + restore
`.gitignore` + force-push codeberg/gitlab). That was NOT executed — it needs
explicit operator approval (see §6). This doc is the investigation + remediation
plan only.**

---

## 1. Truth — hegemon is currently OVER the limit

Three independent measurements of `main`'s pushable content:

| Method | Result | Over 2 GiB? |
|--------|--------|-------------|
| `git rev-list --objects main \| git pack-objects --stdout` (compressed pack GitHub would receive) | **2.208 GiB** (2,370,773,927 bytes) | ✅ over |
| `git cat-file --batch-check` sum of reachable blob content (uncompressed) | **2.402 GiB** (4,606 blobs) | ✅ over |
| Daemon `github_pack_too_large` (journal, live) | **2.41 GiB** ("pushable branch … exceeds github's 2 GiB pack limit") | ✅ over |

GitHub enforces a **hard 2 GiB per-pack limit** server-side. All three
measurements exceed it, so GitHub rejects the push. (The daemon's
`GITHUB_PACK_LIMIT_BYTES` just mirrors this hard limit; it is not configurable
away.)

---

## 2. Prior cleanup (2026-07-06) — it DID work, briefly

`docs/design/hegemon-github-push-fix-2026-07-06.md` records a real cleanup:

- **Blob inventory (pre-rewrite):** `static/assets/**` = 2,857 MiB, `.pi/**`
  (gitignored junk) = 1,962 MiB, real source ≈ 250 MiB. Shared gitdir 5.15 GiB.
- **Rewrite:** `git filter-repo --invert-paths --path-glob 'static/assets/**'
  --path-glob '.pi/**'` → pack **3.98 GiB → 95.5 MiB**. Force-pushed to
  origin/codeberg/**github** (github was empty → now 95 MiB). New `main` =
  `a36b158`.
- **Anti-rebloat `.gitignore` added** (claimed): `static/assets/**/*.png|jpg|…
  ` and `.pi/` ignored "everywhere (0 tracked)".
- **Daemon guard added:** `is_pack_too_large` + proactive github skip in
  `push_background` (`src/sync.rs`), so a permanent pack-too-large fails fast
  instead of re-packing 4 GB every cycle.
- **Final state claimed (2026-07-07):** hegemon `.git` = 96 MiB, "GitHub push ✅
  synced (95 MiB history)".

So as of 2026-07-07, hegemon **was** on GitHub. The cleanup targeted the
REACHABLE history (what GitHub receives), not just local `.git` dangling bloat.
But the protection was not durable (see §3).

---

## 3. Re-bloat (2026-07-09) — the cleanup was undone

A single commit re-bloated hegemon:

```
b281256  CLOSED: r9-t1, … +96more | 2692 file(s) in .pi,.svelte-kit,screenshots
         [.svelte-kit/output/server/chunks/…] DELTA:+34273/-1 | BIN:2295
         NEW:audit-v2/audit-default.png, crops/view-1-map-bl.png, +2681more
2026-07-09 11:25:34 +0100
```

- Added **2,295 binary files** (audit screenshots/crops from a visual-audit
  run): `audit-v2/`, `crops/`, `screenshots/`, `.pi/chrome-screenshots`,
  `.pi/investigation`, `.pi/audit-v3`, `.pi/audit-v4`, `.pi/visual-audit`.
- **`b281256` is an ancestor of current `main` (85f6ac4)** → its binaries are
  in history.
- **`b281256` is the SOLE binary-adding commit** — `git log b281256..main --
  'audit-v2/' 'crops/' 'screenshots/' '.pi/'` is **empty**. So dropping it
  removes 100% of the bloat with no other commit touching those dirs.

**Why it happened (root cause of the re-bloat):** hegemon's current
`.gitignore` only contains `.svelte-kit/` and `!*.png` (line 86). The cleanup's
anti-rebloat rules (`.pi/`, `static/assets/**` binaries) are **gone**. So when
the audit produced screenshots in untracked dirs, the daemon's commit-all policy
auto-committed them → `b281256`. This is exactly the "daemon re-committed
binaries" failure mode the cleanup doc warned about ("Re-bloat recovery (the
`471d6ea` pattern)").

**Current tracked-binary inventory on `main` (2,106 image/audio files):**
| Dir | Files |
|-----|-------|
| `.pi/chrome-screenshots` | 1,161 |
| `static/assets` | 355 |
| `.pi/investigation` | 256 |
| `docs/refs-hom3` | 164 |
| `.pi/audit-v4` | 68 |
| `.pi/audit-v3` | 49 |
| `.pi/visual-audit` | 33 |
| `.pi/audit-v2` | 5 |
| `screenshots/` | 12 |
| `crops/`, `static/favicon.png`, etc. | rest |

Total reachable blob content = **2.402 GiB / 4,606 blobs** (top blob 8.4 MiB).

---

## 4. GitHub push reality (non-destructive confirmation)

- `git ls-remote git@github.com:DraconDev/hegemon.git refs/heads/main` →
  **`18707862`** (clean history, the 95 MiB cleanup plus a couple of later
  `src` commits — *no binaries*).
- Local `main` = **`85f6ac4`** (contains `b281256`). → **DIVERGED** (github
  behind local by the binary commit + subsequent commits).
- `git merge-base --is-ancestor 18707862 main` → **YES**: github's history is a
  prefix of local, so a binary-free rebase of local would **fast-forward** to
  github (no force needed for github).
- codeberg `main` = gitlab `main` = **`7e21a28`**. Both **contain `b281256`**
  (the daemon pushed it to codeberg/gitlab, which have no 2 GiB limit). So
  pushing a binary-free rebase there is **non-fast-forward → force-push
  required**.
- Daemon journal (every cycle, incl. current binary `1199252` @ 13:37:07):
  `⚠️ 🚫 skipping github push for …/hegemon: pushable branch is 2.41 GiB
  (exceeds github's 2 GiB pack limit). Needs history rewrite / OVH migration`.
  GitHub's actual error when a push is attempted: `remote: fatal: pack exceeds
  maximum allowed size (2.00 GiB)`.

**Conclusion:** hegemon currently cannot reach GitHub. Its github remote already
holds the clean history; only the local (and codeberg/gitlab) history picked up
the binaries.

---

## 5. Why a non-rewrite fix does NOT work

- `git revert b281256` (keep history, add a deletion commit): the binaries
  remain in the ancestry between github's `18707862` and the revert tip, so the
  push still sends them → GitHub still rejects. ✗
- Git LFS: requires `git filter-repo`/smudge-rewrite to move blobs into LFS →
  also a history rewrite. ✗
- Migrate assets to OVH bucket: requires removing the binaries from the repo →
  history rewrite. ✗

Only removing `b281256` (and its blobs) from history shrinks the pushable pack
below 2 GiB. That is a history rewrite.

---

## 6. Path forward (remediation — REQUIRES OPERATOR APPROVAL, not executed)

Minimal, safe plan (drops only the one binary commit; github fast-forwards):

1. **Stop the daemon** so it cannot re-commit binaries during the operation.
2. **Drop the binary commit:**
   `git -C /home/dracon/Dev/dracon-platform/web/games/wip/hegemon rebase --onto c935a71 b281256 main`
   (`c935a71` = `b281256^`; this replays everything after `b281256` onto its
   clean parent, removing all 2.4 GiB of binaries).
3. **Restore anti-rebloat `.gitignore`** (the cleanup's rules, currently missing):
   ignore `.pi/`, `screenshots/`, `audit-v2/`, `crops/`, `docs/refs-hom3/`,
   `static/assets/**` binaries; **remove the `!*.png` line** (it un-ignores
   PNGs). This prevents the daemon's commit-all policy from re-committing audit
   outputs.
4. **Push:**
   - `git push github main` — **fast-forward** (github `18707862` is an
     ancestor; safe, no force).
   - `git push --force-with-lease codeberg main` — **force-push** (codeberg
     holds `b281256`; history rewrite).
   - `git push --force-with-lease gitlab main` — **force-push** (gitlab holds
     `b281256`; per `.dracon/dracon-sync.toml` gitlab `main` may need
     unprotecting first).
5. `git -C /home/dracon/Dev/dracon-platform/.git/modules/web-games-hegemon gc
   --prune=now` to drop the now-unreachable binary objects.
6. **Restart the daemon**; it will resume pushing hegemon to all 3 remotes.

Alternative (more thorough): full `git filter-repo` over all refs/tags to
guarantee no binary lingers in any branch/tag. Longer-term per the cleanup doc:
migrate `static/assets/` to an OVH bucket so asset commits are no longer in-repo.

**Approval needed:** explicit operator sign-off to **force-push codeberg + gitlab
main** (history rewrite of those remotes). I have NOT executed any of the above.
Until approved, hegemon stays on codeberg/gitlab (which accept the 2.4 GiB pack)
and is intentionally skipped for github by the daemon's size guard.

---

## 8. Resolution (executed 2026-07-09, operator-approved)

Operator approved the **full rewrite** (drop `b281256` + restore `.gitignore` + force-push codeberg/gitlab/origin + fast-forward github). Executed:

1. **Stopped the daemon** (avoid races during the rewrite).
2. **Dropped `b281256`** via `git rebase --onto c935a71 b281256 main` (`c935a71` = `b281256^`). The rebase hit a modify/delete conflict on `.svelte-kit/ambient.d.ts` (a build artifact added by `b281256`, modified by later commits); resolved each conflict by accepting the incoming (post-`b281256`) version. `b281256` is no longer an ancestor of `main` (new HEAD `c7c9560`).
3. **Restored anti-rebloat `.gitignore`** as a USER section appended after the warden managed block (so warden preserves it — editing the managed block's `!*.png` is futile, warden regenerates it). Rules: `.pi/`, `screenshots/`, `crops/`, `audit-v2/`, `docs/refs-hom3/`, `static/assets/**`. The parent-dir excludes override the warden `!*.png`/`!*.jpg` negations for those paths, so audit binaries stay ignored.
4. **Force-pushed the rewritten `main`** to all 4 remotes (`github`, `codeberg`, `gitlab`, `origin`) — `--force-with-lease` (GitHub needed an explicit lease; the rest accepted directly). All remotes advanced to `39c2beb9` (the `.gitignore` commit on top of `c7c9560`).
5. **Pruned the binaries**: `git fetch --all` + `git reflog expire --expire=now --all` + `git gc --prune=now`. Shared gitdir dropped **4.9 GiB → 164 MiB**; reachable blob content **2.402 GiB → 0.343 GiB**; `b281256` object gone.
6. **Restarted the daemon** — it committed the `.gitignore` change and resumed syncing hegemon to all 3 remotes, **including GitHub** (no more `skipping github` guard; pack is 0.158 GiB, under the 2 GiB limit).

**Final state (verified):** hegemon `main` = `39c2beb9` on github/codeberg/gitlab/origin; GitHub pack 0.158 GiB; `dracon-sync repos` shows hegemon `OK`, healthy. Hegemon is back on GitHub.

**Lesson:** the 2026-07-06 cleanup worked, but its anti-rebloat `.gitignore` rules were not durable (absent from hegemon's `.gitignore`). The daemon's commit-all policy then re-committed audit screenshots. The fix is a *user-section* `.gitignore` (outside the warden managed block) so warden preserves it.

## 9. Side note — prior sync-stall fix was never deployed

While investigating, found that the earlier sync-stall fix (which stops hegemon's oversized GitHub push from hanging) had **never been deployed**: `cargo build` wrote to `target/release/` but the service runs `~/.local/bin/dracon-sync`, which was the old 10:12 binary. The orphaned hegemon GitHub push had returned. Fixed by stopping the daemon, copying `target/release/dracon-sync` → `~/.local/bin/dracon-sync`, and restarting. Verified 0 orphaned pushes over a full 4-min cycle. (Recorded in `sync-stall-audit-2026-07-09.md`.)

---

## 7. Evidence index

- Pack size: `git rev-list --objects main | git pack-objects --stdout | wc -c`
  → 2,370,773,927 bytes (2.208 GiB).
- Blob content: `git cat-file --batch-check` sum → 2.402 GiB / 4,606 blobs.
- Daemon: journal `github_pack_too_large` → 2.41 GiB (every cycle).
- Re-bloat commit: `git show b281256` → 2,295 BIN files, 2026-07-09 11:25.
- Ancestry: `git merge-base --is-ancestor 18707862 main` → YES (github
  fast-forward possible); codeberg/gitlab `main` = `7e21a28` holds `b281256`.
- `.gitignore`: only `.svelte-kit/` + `!*.png`; cleanup's binary rules absent.
- Prior cleanup: `docs/design/hegemon-github-push-fix-2026-07-06.md`.
