# Detached-HEAD push refspec bug — 2026-08-09

**Author**: pi-goal-list-loop-audit investigation
**Released**: dracon-sync v0.113.48
**Severity**: medium — self-recovers via HTTPS fallback path, but the
50-minute stall produced a misleading `pushing Xm` indicator and wasted daemon
push retries.

## Summary

Three push sites in the daemon used the bare refspec `HEAD` when
`current_branch(repo)` returned `Some(branch)` and only fell back to
`HEAD:refs/heads/main` when detached. On a detached worktree, `HEAD` is a
commit SHA — git rejects it with `error: The destination you provided is not
a full refname`. The daemon retried every ~2 minutes for the next 50
minutes before the HTTPS fallback (which already used the qualified refspec)
finally succeeded.

## Live incident: pi-goal-loop-audit (2026-08-09)

`pi-goal-loop-audit` had a 197-file commit ahead on 2026-08-09:

| Time (BST) | Event |
|------------|-------|
| 10:05:42   | gitlab SSH push started |
| 10:05:55   | refspec error (`destination is not a full refname`) |
| 10:11 / 10:21 / 10:32 / 10:44 / 10:52 | retries 1-5 — same error each time |
| ~10:55     | HTTPS fallback succeeds with `HEAD:refs/heads/main` |
| 11:00+     | self-recovered, push ledger normalized |

The `pushing 47m` indicator on the repo row misled the operator into thinking
the push was actively in flight. It was actually failing for 47 minutes,
then succeeding at the very last moment.

## Root cause analysis

`current_branch(repo)` reads `.git/HEAD`. Three behaviors:

| HEAD file content | `current_branch` returns |
|-------------------|--------------------------|
| `ref: refs/heads/main` (attached) | `Some("main")` |
| `abc123...` (detached) | falls through to `git rev-parse --abbrev-ref HEAD` which returns `"HEAD"` (literal) — filtered out → `None` |
| missing / error | `None` |

So `current_branch` correctly distinguishes attached vs detached. The bug was
in the consumer: `push_with_transport_fallbacks` and `push_with_retries` both
chose `Some(_) => "HEAD"` — using bare `HEAD` for the ATTACHED case too.

Why didn't this break every push? `git push origin HEAD` with HEAD attached
**does** work — git resolves `HEAD` as the current branch (e.g. `main`) and
pushes it. The bug only fires when HEAD is detached, in which case the bare
`HEAD` is interpreted as a commit SHA. So for 99% of repo states, this is
fine.

The remaining 1% — detached HEADs — is the case we hit. Three sub-cases that
produce detached worktrees:

1. **Agent/migration left it detached**: an interactive session that ran
   `git checkout <sha>` and exited without re-attaching.
2. **Worktree state race**: `.git/HEAD` says "ref: refs/heads/main" but the
   worktree's actual HEAD is a SHA (libgit2 ↔ git CLI disagreement during a
   checkout).
3. **Nested-on-main submodule migration** (2026-07-02): the nested path was
   detached at the parent's gitlink SHA while the migration was in flight.

## The fix

Always use the fully-qualified refspec `HEAD:refs/heads/<branch>` whenever a
branch is known. This works for both attached and detached worktrees —
git pushes the commit pointed at by HEAD to `refs/heads/<branch>`.

```rust
// Before (buggy)
let ssh_refspec = match crate::git::branch::current_branch(repo) {
    Some(_branch) => "HEAD".to_string(),         // ← bare HEAD (bug)
    None => "HEAD:refs/heads/main".to_string(),
};

// After (fixed)
let ssh_refspec = match crate::git::branch::current_branch(repo) {
    Some(branch) => format!("HEAD:refs/heads/{branch}"),
    None => "HEAD:refs/heads/main".to_string(),
};
```

Three sites patched:

- `src/git/push.rs:97` (`push_with_transport_fallbacks`)
- `src/git/push.rs:165` (`push_with_retries`)
- `src/git/multi_remote.rs:609` (`push_to_remote` retry loop — was using bare
  `HEAD`)

## Why the HTTPS fallback worked

`push_https_fallback` always used the fully-qualified refspec (line 137 of
`push.rs`). That's why the daemon eventually recovered — the HTTPS push
attempt ran with `HEAD:refs/heads/main`, which git accepts for both attached
and detached worktrees.

This also explains why `git push origin HEAD:refs/heads/main` is what we
should have been doing all along.

## Deeper issue (out of scope for v0.113.48)

The libgit2 path in `dracon-git`'s `get_status` (`dracon-git/src/lib.rs:218-249`)
under-reports `ahead=0` on a detached HEAD. `head_ref.shorthand()` returns
`"HEAD"` (the literal), so `branch_upstream_name("refs/heads/HEAD")` returns
Err → ahead/behind stay at 0 → the daemon never realizes there are unpushed
commits.

This is a SEPARATE bug. In the production incident, the daemon DID notice
ahead>0 (somewhere upstream detected it — likely an earlier libgit2 path
that resolved the upstream differently, or a fallback to git CLI that
succeeded). The refspec fix is for the case where the daemon DOES notice
ahead>0 and tries to push.

If we want to address the "detached HEAD with unpushed commits that the
daemon doesn't even attempt to push" case, that's a follow-up: change
`get_status` to fall back to `git rev-list --count @{u}..HEAD` when
`head_ref.shorthand()` is `"HEAD"`. Tracked separately; not in v0.113.48.

## Tests

Two new regression tests:

- `test_push_succeeds_with_detached_head` (`src/sync.rs`) — detached HEAD
  + ahead commit + shell-out push with the qualified refspec → assert the
  ahead commit lands on origin.
- `test_refspec_format_is_always_qualified` (`src/sync.rs`) — pins the
  contract that no push refspec is the bare `HEAD` form. A tripwire for
  future reverts.

One existing test updated:

- `test_push_to_named_remote_https_fallback_failure_still_retries_ssh`
  (`src/git/mod.rs`) — pre-fix, this test relied on the retry loop using
  bare `HEAD` to slip past the test's fake-git (which matched only
  `HEAD:refs/heads/<branch>`). Post-fix, the retry loop uses the qualified
  refspec. The test now asserts the new (deterministic) failure mode: the
  retry loop IS REACHED (not short-circuited), it just fails the same way
  as the SSH attempt.

## Verification

| Step                              | Result |
|-----------------------------------|--------|
| `cargo test --workspace --locked` | 1240 passed, 9 ignored (+2) |
| `cargo clippy --workspace --locked --all-targets -- -D warnings` | pre-existing 6 warnings in daemon.rs / report.rs only; 0 new |
| `cargo deny check` | clean |
| Release v0.113.48 | published + tagged + gh-released (v0.113.48) |
| Install + restart daemon | fleet 37/34 clean/3 active, 0 untracked, no freeze marker |

## Operator action

None required. The fix is automatic. Repos with active push backlogs should
see their `pushing Xm` indicators normalize within 1 push cycle (~40-300s).

If you want to manually verify: `dracon-sync repos` should show no `pushing`
indicators stuck past a few minutes. If a push is genuinely stuck, see the
existing diagnostic paths — this fix doesn't change them, it just removes
one stall class.
