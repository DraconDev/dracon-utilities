# mcode-plugins — stale `❌ FAIL` cell, empty gitlab mirror, and the daemon gap behind it

**Date:** 2026-10-20
**Scope:** `/home/dracon/Dev/mcode-plugins` (+ the `bridge-fixture` untracked
qualification PNG, committed in the same pass).
**Outcome:** fleet row now reads `✅ CLEAN` / `PUSH ✅ OK 1m`; `origin/main`,
`github/main` and `gitlab/main` all point at `30fb82c38444ef4f311f671e10f274064f765be4`.

## What the `❌ FAIL 9h` cell actually was

The PUSH column renders the **last daemon-recorded push attempt**
(`~/.local/state/dracon/push-history/<sha256(repo)>.json`), not live
convergence. For this repo it still held:

```json
{"version":1,"repo":"/home/dracon/Dev/mcode-plugins",
 "attempted_at":1792416106,"succeeded":false}   // 2026-10-19 14:21:46
```

That attempt failed because the **warden global pre-push hook** refused the
push:

```
⚠️  Possible plaintext secrets detected in added file
   mcode-chrome/chunk-5ADYFF2U.js.orig.bak (binary-safe scan).
```

i.e. a 6.4 MB JS bundle backup whose blob contained a `password = "<value>"`
assignment — the incident documented in
`AUDIT_FULL_2026-10-19-ROUND5.md` ("the second attention item the footer
never named: `mcode-plugins`").

**The blocking blob no longer exists.** Verified at the current revision:

```
git cat-file -t f5b32e814ed9de69a4f6b74f   → fatal: Not a valid object name
git log --all --diff-filter=A -- 'mcode-chrome/chunk-5ADYFF2U.js.orig.bak' → (empty)
git rev-list --objects HEAD | grep -c 'chunk-5ADYFF2U\|orig\.bak'  → 0
```

The history was rewritten since (root commit is now `17a7aeac…`, 210 commits),
and the current 210-commit tree contains **no** `chunk-5ADYFF2U*` /
`*.orig.bak` object. A full secret-shape scan of every commit reachable from
`origin/main` (added-lines **and** added-blob paths, using the hook's own
`SECRET_RE`) returns **0 matches** across all 210 commits. The hook's refusal
reason is therefore historical, not current.

## The real gap the stale cell was hiding

`gitlab.com:DraconDev/mcode-plugins` existed as a project (created
2026-10-19 12:39) with **zero branches** — the mirror had never been
seeded, while `origin/main` and `github/main` were both at `30fb82c`.

Why the daemon never healed it — `count_pushable_unpushed_vs_mirrors`
(`dracon-sync/src/git/status.rs:462`) only counts mirror tracking refs that
**exist locally and are ancestors of HEAD**:

```rust
for mirror_ref in known_mirror_tracking_refs(repo) {          // github, gitlab, codeberg
    if !ancestor.is_ok_and(|o| o.status.success()) { continue; }   // missing ref → skipped
    …rev-list --count <mirror_ref>..HEAD…
}
```

A configured mirror with **no branch at all** has no local tracking ref, so it
is skipped, the count is 0, `should_push` is false, and the daemon never
issues the push that would create the branch. Self-healing was impossible
by construction — a chicken-and-egg the mirror-ahead override
(`daemon.rs:8693`) cannot break either, for the same reason.

**Fix applied (state, not code):** the mirror was seeded with a normal
fast-forward push of the current sanitized `main` — no force, no rewrite:

```
git push gitlab main:refs/heads/main
  * [new branch]      main -> main          (exit 0)
```

All three remotes now agree:

```
git ls-remote origin  refs/heads/main → 30fb82c38444ef4f311f671e10f274064f765be4
git ls-remote gitlab  refs/heads/main → 30fb82c38444ef4f311f671e10f274064f765be4
git ls-remote github  refs/heads/main → 30fb82c38444ef4f311f671e10f274064f765be4
git rev-parse main                     → 30fb82c38444ef4f311f671e10f274064f765be4
```

`git push --dry-run` to both `github` and `gitlab` now exits **0**
("Everything up-to-date") for either remote.

Note: gitlab printed its *"free storage limit of 10 GiB on one or more
projects"* banner during the push. The push itself succeeded and the branch
exists; the banner is a project-level quota notice, not a push failure.

### Making the daemon record a fresh success

The PUSH cell only refreshes when the daemon itself attempts a push, and the
daemon skips a repo that is clean and converged (`has_local_or_pending_work`,
`daemon.rs:9108`). Deleting the **local** `refs/remotes/origin/main` tracking
ref (local bookkeeping only — the remote branch was never touched) made the
daemon's `upstream_ref_missing` gate fire, so the daemon re-pushed, restored
the ref, and recorded a success:

```
~/.local/state/dracon/push-history/<sha256>.json
  → {"attempted_at":1792454420,"succeeded":true}   // 2026-10-20 01:00:20
```

`dracon-sync repos mcode-plugins` now reports `push: ✅ OK 1m`, and the
stuck-push ledger no longer contains this repo.

## bridge-fixture — the untracked qualification PNG

`public/qualification-20261018/ovh-live-2fc91627ddb9.png` was the repo's
only untracked file, and the daemon had been reporting `1 ut` for it. Per the
commit-all policy it was committed, not gitignored:

```
git add -- public/qualification-20261018/ovh-live-2fc91627ddb9.png
git commit -m "Add qualification-20261018 fixture (…)"   → cf0a8c0b24d
git push origin HEAD:refs/heads/main                    → dfb16cf..cf0a8c0b (exit 0)
```

It is a genuine bucket candidate, not scratch: `bucket-assets.manifest.json`
lists it (`sha256 215dc3a0…`, 2200 bytes, `bucketed: true`), and
`bucket-runner.json` lists it under `allowedCandidates`. The other
qualification PNGs (`qual-20261017.png`, `qualification-20261019/…`,
`qualification-retry-20261019/…`) are already tracked, so this one was an
outlier rather than a policy exception. After the commit the file is present
on the mirrors too:

```
gh  api repos/DraconDev/bridge-fixture/contents/public/qualification-20261018 → ovh-live-2fc91627ddb9.png
glab api projects/DraconDev%2Fbridge-fixture/repository/tree?path=public/qualification-20261018
     → public/qualification-20261018/ovh-live-2fc91627ddb9.png
```

## Follow-ups NOT taken here (out of scope for this pass)

1. **Daemon gap (needs a daemon code change, sanctioned separately):**
   `count_pushable_unpushed_vs_mirrors` cannot observe a configured mirror
   that has no branch, so such a mirror can never self-seed. A mirror whose
   `ls-remote` shows no matching branch should be treated as fully
   unpushed (the "never pushed" case) rather than as zero lag.
2. **Stale observation cell:** the PUSH column shows the last *attempt*, so a
   failure that has since been fixed keeps reading `❌ FAIL` until the next
   attempt. Worth an explicit "stale" marker or an age threshold.
3. **Separate, still-open incident — `db-gateway`** (appeared 2026-10-19
   23:20, unrelated repo, not part of this pass): the same hook class fires
   on `tests/secret_scan.rs`, whose **worktree** version deliberately
   assembles the fixture value at run time ("The fixture values below are
   assembled at run time from split fragments on purpose… the gate needs no
   per-file exemption for its own test"), but whose **committed** blob
   (commit `1c14b46c`, blob `28c80b76…`, blob-sha256
   `fa6dd2617c93db078813c1114b1943cdabc1b3dfbb9dc4e93e6bace6ab46be0e`)
   still carries the literal `secret = "hK7m…"`. The clean fix is to land the
   already-prepared worktree version so the *committed* blob no longer holds
   the literal — no exemption, no receipt, no history rewrite.

## Boundaries honoured

No history rewrite, no force-push, no `--force-with-lease`, no reset, no
re-clone, no `systemctl stop`, no daemon/warden source change. The only
mutations were: one fast-forward push of an existing commit to an existing
empty mirror, one ordinary commit of an existing untracked file, and the
deletion of one **local** remote-tracking ref (restored by the daemon's own
push).
