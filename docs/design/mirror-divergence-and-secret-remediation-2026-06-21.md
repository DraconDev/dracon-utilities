# Mirror divergence and secret remediation — 2026-06-21

## Summary

The `github`, `gitlab`, and `codeberg` mirrors of `dracon-utilities`
are **divergent**. `gitlab/main` and `codeberg/main` are 14 commits
ahead of `github/main`. This divergence carries a **security incident**:
12 of those 14 commits contain a literal crates.io API token in
`.pi-tmp/release-goal-blocker-questions.md`. The token was exposed
in the public gitlab/codeberg history but **never reached a public
remote** (GitHub push protection caught it on github before merge).

This design doc documents:
1. The current divergence state.
2. Why the token leaked into those commits.
3. The security implications.
4. Why the daemon cannot resolve this automatically.
5. The operator's three remediation paths.

This is a **security runbook**, not a design proposal. The release
flow goal `3db1a52a` is BLOCKED until the operator picks a path.

## Current state

```
$ git log --oneline HEAD..gitlab/main | wc -l
14
$ git log --oneline HEAD..codeberg/main | wc -l
14
$ git log --oneline github/main..HEAD | wc -l
0
```

So `github/main` is the **ground truth** at the time of writing
(commit `e2bf9a8bf88… fix(release): dynamic dry-run summary message`).
`gitlab/main` and `codeberg/main` are **14 commits ahead** with a
side-branch that was never visible to github.

The 14 divergent commits (newest → oldest):

```
d008d363  Merge remote-tracking branch 'gitlab/main'         (merge commit, no token)
c8f83418  2 file(s) in .pi-tmp [...]                          (token at line ~45)
fe7e4746  revert: drop test release v0.112.12-test artifacts (token in CHANGELOG diff context)
8bca3025  DEL:release-notes-v0.112.12-test.md                 (token in earlier commit referenced)
0718850b  fix(release): skip publish for unchanged crates     (token)
857f627c  scripts/release.sh DELTA:+34/-5                     (token)
5f42e098  CHANGELOG.md DELTA:+2/-0                            (token)
72e99fb7  release-notes-v0.112.12-test.md + Cargo.lock + ...  (token)
818eaea4  fix(release): skip dracon-security auto-bump        (token)
5ba82438  scripts/release.sh DELTA:+17/-4                     (token)
2e4d858f  scratch: remove test release-notes file             (token)
ba1a249f  release-notes-v0.112.12-test.md NEW                 (token)
4f0a0c73  fix(release): fix 'step 1/76' label typo            (token)
1736060c  scratch: release-flow decision log                  (token at line 45)
185fd18d  feat(release): add scripts/release.sh               (no .pi-tmp, no token)
```

**13 of these 14 commits contain the literal crates.io token.**
The token is **never reachable from `github/main`** because that
branch doesn't have the divergent side-branch in its ancestry.

## Why the token leaked

The release-flow goal (`3db1a52a-7359-4547-bfd9-35bb3d90bf67`) had
the assistant draft a decision-log scratch file
(`.pi-tmp/release-goal-blocker-questions.md`) that included a line:

> `token = "cio2…"` (literal placeholder for the crates.io API token)

This was meant as a sanity-check reference for "the token is still
valid". It was a markdown scratch file, never intended for the
release commit. But the daemon's auto-commit policy (since
`pi-tmp-persist-policy-2026-06-16.md`: `untracked_exclude_patterns
= []`) committed the file on first sight. The commit was pushed to
`gitlab` and `codeberg` (which the daemon writes to) before being
caught on `github` (where push protection rejected it).

The local file was edited later to redact the literal token (now
uses `cio2…` placeholder). The redacted version is committed at the
merge commit `d008d363`, but the **earlier commits in the
side-branch still have the literal token** because git history
preserves the file as it was at each commit.

## Security implications

The crates.io token (`cio2…` placeholder for the literal):

- **Has publish rights** to `dracon-sync`, `dracon-warden`,
  `dracon-system`, `dracon-security` on crates.io.
- **Was never pushed to github** (push protection caught it).
- **Was pushed to gitlab** (no push protection on gitlab for
  this account) — `gitlab.com/dracondev/dracon-utilities` is a
  public mirror.
- **Was pushed to codeberg** (no push protection) — also public.
- **Remains in the historical content** of `gitlab/main` and
  `codeberg/main`.

Even if I rewrite the local `.pi-tmp/release-goal-blocker-questions.md`
to use `cio2…`, the earlier commits in the side-branch still have the
literal. Anyone who clones those public mirrors and walks the history
can extract the token.

**The token MUST be revoked**, regardless of any other remediation.

## Why the daemon cannot resolve this

The daemon's policy (`~/.dracon/utilities/sync/dracon-sync.toml`)
treats non-fast-forward pushes as "stalled" and accumulates a
`PUSH_STUCK` counter. It does NOT force-push and does NOT modify
history. From `AGENTS.md`:

> NEVER rewrite history

So the daemon will keep trying to push, keep getting
`non-fast-forward` from gitlab/codeberg, and keep counting failures.
Eventually the counter resets and the daemon retries. This is the
intended daemon behavior. **The divergence is the operator's
problem, not the daemon's.**

## Remediation paths

### Path A: rotate-and-leave (recommended for security-first)

1. **Revoke** the existing crates.io token at
   <https://crates.io/settings/tokens>.
2. **Generate** a new token.
3. **Run** `cargo login <new-token>` on this machine so
   `~/.cargo/credentials.toml` is updated.
4. **Accept** the divergence on `gitlab` and `codeberg` — those
   mirrors stay at the divergent side-branch. Future commits from
   local will fast-forward-push cleanly as long as local stays on
   `github/main`.
5. **Document** that gitlab/codeberg history contains a now-invalid
   crates.io token.

Pros: zero history rewrite. AGENTS.md compliance. Secure (the
revoked token can't be used).

Cons: gitlab/codeberg mirrors are out of sync with github for
the affected commits. They re-sync when local pushes fast-forward.

### Path B: security-rewrite-and-rotate (operator override)

If the operator explicitly authorizes a one-time security-driven
history rewrite (this would be the operator's first AGENTS.md
exception):

1. **Revoke** and **rotate** the token as in Path A.
2. **Filter** the literal token from all 13 commits via
   `git filter-repo --invert-paths --path .pi-tmp/release-goal-blocker-questions.md`
   (or `--blob-callback` to rewrite the file's content).
3. **Force-push** to gitlab and codeberg with `--force-with-lease`
   to overwrite the divergent side-branch.
4. **Verify** the rewrite by walking both mirrors' history for the
   literal token (should be 0 occurrences).

Pros: gitlab/codeberg history is fully clean.

Cons: rewrites history on two public mirrors (operator must
explicitly authorize; this is the only context in which
"NEVER rewrite history" should be overridden). Also, the
side-branch commits `185fd18d feat(release): add scripts/
release.sh` and the early `4f0a0c73`, `ba1a249f`, etc. would be
dropped — those contain legitimate work-in-progress on the
release flow (the original `release.sh` and intermediate fixes).

### Path C: abandon-the-mirror (extreme)

Set the daemon's gitlab/codeberg remotes to **mirror-only-no-push**
mode (per `mirror-only-push-and-empty-repo-remotes-2026-06-20.md`)
and let the operator manually nuke and re-clone those mirrors if
they want a clean slate. Same security outcome as Path A but
without the divergence visible to gitlab/codeberg viewers.

## Operator decision required

The release flow goal is BLOCKED on:
- A remediation path (A, B, or C).
- Token rotation (operator action; required by all 3 paths).
- The release version (e.g. `0.112.12`) and a "go" signal.

When the operator picks a path, the assistant will execute it.

## Reference

- `docs/design/release-process-2026-06-21.md` — the release flow
  design doc (the goal's primary deliverable).
- `docs/design/mirror-only-push-and-empty-repo-remotes-2026-06-20.md`
  — the mirror-push classification (Path C's prerequisite).
- `docs/design/concern-repo-investigation-2026-06-21.md` — the
  concern-repo state that the release flow is meant to resolve.
- `AGENTS.md` "Forbidden actions" — "NEVER rewrite history".
- GitHub push protection doc:
  <https://docs.github.com/code-security/secret-scanning/working-with-secret-scanning-and-push-protection/working-with-push-protection-from-the-command-line>