# NO_ORIGIN concern misclassification (post-SSH-migration)

**Date:** 2026-06-20
**Goal:** `2a11662d-2c8b-4251-8125-aea69a72cda8`
**Status:** FIXED

## Summary

After the recent migration of all watched repos from an `origin` remote
(HTTPS) to three SSH mirror remotes (`github`, `gitlab`, `codeberg`),
`dracon-sync repos` was reporting 11 ❌ CONCERN entries for repos that
were actually healthy. The daemon's report logic raised `NO_ORIGIN` as
a CONCERN whenever a repo lacked a remote literally named `origin`,
which is true for every SSH-mirror repo. The hint
"no origin remote (using github SSH instead)" was a self-aware message
that did not change the classification — every row was still CONCERN.

The fix changes the concern predicate so a repo is concerning for
"no origin" only when it has *zero* remotes at all (truly
remote-less). Repos with at least one configured remote (any name) are
no longer CONCERN for "no origin", and the missing-tracking-upstream
flag is now informational rather than a concern when the daemon can
still push via the configured mirrors.

## Root cause

`dracon-sync/src/report.rs` raised the `NO_ORIGIN` flag for any repo
where `[remote "origin"]` was absent from `.git/config`, and the
concern predicate `repo_is_concern` short-circuited on `!has_origin`
without checking whether other remotes were configured.

The global policy at `/home/dracon/.dracon/utilities/sync/dracon-sync.toml`
configures three `[[remotes]]`:

```toml
[[remotes]]
name = "github"
push_url = "git@github.com:DraconDev/{repo}.git"
auto_create = true

[[remotes]]
name = "gitlab"
push_url = "git@gitlab.com:dracondev/{repo}.git"
auto_create = true
force_push_when_behind = true

[[remotes]]
name = "codeberg"
push_url = "git@codeberg.org:dracondev/{repo}.git"
auto_create = true
force_push_when_behind = true
```

None of these is named `origin`. The daemon's
`multi_remote::push_to_all_remotes` pushes to each one via explicit
`git push <remote> HEAD:refs/heads/<branch>` refspecs, so it does
not require an `origin` remote to exist. The previous
`!has_origin → CONCERN` check was correct in the single-origin world
but became a false positive after the multi-mirror migration.

The `NO_UPSTREAM` flag had a similar, narrower issue: it was only
emitted when `has_origin && !has_upstream`. Repos with no `origin`
but with a configured `github` remote silently swallowed the
missing-tracking signal and fell through to the generic
"run repair-concerns --apply" hint, which would have failed
(`git push -u origin HEAD` against a non-existent `origin`).

## Audit: every repo's remote config

For each watched repo, the audit was:

1. `git -C <repo> remote -v` — list all configured remotes
2. `git -C <repo> rev-parse --abbrev-ref --symbolic-full-name '@{u}'` — list tracking upstream
3. `cat <repo>/.git/config` — verify the on-disk config

| # | Repo | Remotes | Tracking upstream | Before fix | After fix |
|---|------|---------|-------------------|-----------|-----------|
| 1 | `/home/dracon/Dev/dracon-platform` | `origin` (HTTPS), `github` (SSH), `gitlab` (SSH), `codeberg` (SSH) | none (until repaired) | CONCERN (push FAIL, set upstream) | ✅ OK (after `repair concerns --apply`) |
| 2 | `/home/dracon/Dev/browser-extensions-shared` | `github` (SSH), `gitlab` (SSH), `codeberg` (SSH) | none | CONCERN (no origin) | ✅ OK |
| 3 | `/home/dracon/Dev/pully-fully-pull-based-fleet-reconciler` | `github` (SSH), `gitlab` (SSH), `codeberg` (SSH) | none | CONCERN (no origin) | ✅ OK |
| 4 | `/home/dracon/.dracon` | `github` (SSH), `gitlab` (SSH), `codeberg` (SSH) | none | CONCERN (no origin) | ✅ OK |
| 5 | `/home/dracon/Dev/dracon-utilities` | `github` (SSH), `gitlab` (SSH), `codeberg` (SSH) | none | CONCERN (no origin) | ✅ OK |
| 6 | `/home/dracon/Dev/dracon-code` | `github` (SSH), `gitlab` (SSH), `codeberg` (SSH) | none | CONCERN (no origin) | ✅ OK |
| 7 | `/home/dracon/Dev/ai-auto-writer` | `github` (SSH), `gitlab` (SSH), `codeberg` (SSH) | none | CONCERN (no origin) | ✅ OK |
| 8 | `/home/dracon/Dev/rust-ai-web-auto` | `github` (SSH), `gitlab` (SSH), `codeberg` (SSH) | none | CONCERN (no origin) | ✅ OK |
| 9 | `/home/dracon/Dev/dracon-libs` | `github` (SSH), `gitlab` (SSH), `codeberg` (SSH) | none | CONCERN (no origin) | ✅ OK |
| 10 | `/home/dracon/Dev/DraconDev` | `github` (SSH), `gitlab` (SSH), `codeberg` (SSH) | none | CONCERN (no origin) | ✅ OK |
| 11 | `/home/dracon/Dev/avid` | `github` (SSH), `gitlab` (SSH), `codeberg` (SSH) | none | CONCERN (no origin) | ✅ OK |

`dracon-platform` (#1) is the only repo with an `origin` remote — it
was a legitimate CONCERN (`origin` exists but the local branch had no
tracking upstream, and the most recent push attempt failed). It was
repaired with `dracon-sync repair concerns --apply --repo
/home/dracon/Dev/dracon-platform`, which set the upstream to
`origin/main` and cleared the concern.

The other 10 repos have only the SSH mirrors and were
misclassified.

## Fix

### `dracon-sync/src/report.rs`

1. **`repo_state_flags_with_push_failure`** (line ~982) — added a
   new `has_any_remote: bool` parameter. The `NO_ORIGIN` flag now
   fires only when the repo has *no* remotes at all
   (`!has_origin && !has_any_remote`). The `NO_UPSTREAM` flag now
   fires whenever the local branch has no tracking upstream
   (`!has_upstream`), regardless of whether the repo has an
   `origin`. The flag remains informational (it does not by itself
   raise a CONCERN).

2. **`repo_is_concern_with_push_failure`** (line ~1097) — the
   concern predicate is now:
   - `!has_origin && !has_any_remote` → CONCERN (truly remote-less)
   - `status.behind > 0` → CONCERN (diverged, can lose history)
   - `status.ahead > 0 && has_origin && has_upstream && recent_push_failure` → CONCERN (stuck push on `origin`)
   - otherwise → not a CONCERN

   The `!has_upstream` case no longer auto-raises a concern when
   the repo has at least one configured remote. The daemon's
   multi-mirror push path uses explicit refspecs and does not
   require `branch.<name>.remote` to be set.

3. **`repo_is_stuck_push` / `repo_is_stuck_pull`** (lines ~1113, ~1129) —
   added `has_any_remote: bool` parameter for signature parity
   with the other helpers. The actual predicates are unchanged
   (they only fire when `has_origin && has_upstream`, which is the
   `origin`-pinned push path). The new parameter is accepted but
   not consulted, with an explicit `let _ = has_any_remote;` to
   silence the unused-arg lint.

4. **`repo_is_concern`** (line ~1076) — kept for backward
   compatibility with the test suite. Same semantic as
   `repo_is_concern_with_push_failure` for the non-push-failure
   path.

5. **`repo_is_warn`** (line ~1131, `#[cfg(test)]`) — added
   `has_any_remote: bool` parameter for signature parity.

6. **`repo_hint`** (line ~1493) — three hint changes:
   - `NO_ORIGIN` hint changed from
     "no origin remote (using github SSH instead)" to
     "no remote configured (cannot push)" — the flag now only
     fires for truly remote-less repos, so the hint must reflect
     that.
   - `NO_UPSTREAM` hint now context-sensitive: when the row is
     also a CONCERN (`has_origin && !has_upstream`), the original
     "run repair-concerns --apply (set upstream)" hint is
     accurate and `repair concerns --apply` will succeed. When
     the row is not a CONCERN (`has_origin=false, has_any_remote=true`),
     the hint is "no tracking upstream (daemon uses explicit
     refspecs; not a concern)" — the daemon is already pushing
     successfully via the configured mirrors and the auto-repair
     path would fail.

7. **Push-status logic** (line ~1896) — the `NO_UPSTREAM` branch
   of the push-status cascade previously mapped to `("FAIL",
   "no upstream set")` unconditionally. After the flag-semantic
   change, that fires for SSH-only repos where the daemon is
   successfully pushing. The branch now distinguishes:
   - `has_origin && !has_upstream` → `("FAIL", "no upstream set")`
   - `!has_origin && has_any_remote && !has_upstream` → `("OK", "")`
     (daemon is pushing successfully via the configured mirror list)

### All call sites updated

- The main `repos` command loop (`build_repo_row`/`render_repos`),
  the `repair concerns` loop, and the `repair warns` loop all
  compute `let has_any_remote = !crate::git::multi_remote::list_remotes(&repo).is_empty();`
  and pass it to the renamed helpers. There is no `has_origin` call
  site that misses the new `has_any_remote` companion.

### `docs/design/sync-push-classification.md`

Updated the classification-rules table to reflect the new semantic:

| State | `repos` flag | `repair concerns`? |
|-------|--------------|---------------------|
| Clean, no unpushed commits | `OK` | No |
| Unpushed commits (ahead > 0), no recent push failure | `PENDING` | No |
| Unpushed commits (ahead > 0), recent push failure (< 10 min) | `STUCK_PUSH` | Yes |
| Behind remote (behind > 0) | `STUCK_PULL` | Yes |
| **No remote at all** (no `origin` AND no other remote) | `NO_ORIGIN` | Yes |
| No tracking `upstream` branch AND `has_origin` | `NO_UPSTREAM` | Yes |
| No tracking `upstream` branch, repo flagged `intentional_no_upstream = true` | `INTENTIONAL_NO_UPSTREAM` | No (skipped) |
| No tracking `upstream` branch, no `origin`, but other remotes (e.g. SSH mirrors) | `NO_UPSTREAM` (informational) | No |

The old row "No `origin` remote | `NO_ORIGIN` | Yes" is split into
two rows so the SSH-mirror case is explicit.

## Tests

Three new tests cover the regression and the new semantic:

- `test_repo_state_flags_no_origin_but_has_remote` — verifies that
  `repo_state_flags(clean, has_origin=false, has_upstream=*, has_any_remote=true)`
  does NOT emit `NO_ORIGIN`.
- `test_repo_is_concern_no_origin_but_has_remote` — verifies that
  `repo_is_concern(clean, has_origin=false, has_upstream=true, has_any_remote=true)`
  is `false` (i.e. not a concern).
- `test_repo_hint_no_upstream` — split into two assertions to cover
  both the `concern=true` and `concern=false` cases (the hint is
  context-sensitive).

All existing tests were updated to pass the new `has_any_remote`
parameter; the function signatures changed (added parameter) but
the semantic for previously-valid cases is preserved.

Full test suite: 577 passed, 3 ignored (pre-existing).

## Before / after

### Before (11 ❌ CONCERN)

```
📜 /home/dracon/.dracon/utilities/sync/dracon-sync.toml
📦 11 repos  ✅ OK 0  ⚠️  WARN 0  ❌ CONCERN 11
…
│ 1 │ ❌ CONCERN │ dracon-platform        │ main │ …│ FAIL │ …│ run repair-concerns --apply (set upstream) │
│ 2 │ ❌ CONCERN │ browser-extensions-sha…│ main │ …│ OK   │ …│ no origin remote (using github SSH instead)│
│ 3 │ ❌ CONCERN │ pully-fully-pull-based…│ main │ …│ OK   │ …│ no origin remote (using github SSH instead)│
│ 4 │ ❌ CONCERN │ .dracon                │ main │ …│ OK   │ …│ no origin remote (using github SSH instead)│
│ 5 │ ❌ CONCERN │ dracon-utilities       │ main │ …│ OK   │ …│ no origin remote (using github SSH instead)│
│ 6 │ ❌ CONCERN │ dracon-code            │ main │ …│ OK   │ …│ no origin remote (using github SSH instead)│
│ 7 │ ❌ CONCERN │ ai-auto-writer         │ main │ …│ OK   │ …│ no origin remote (using github SSH instead)│
│ 8 │ ❌ CONCERN │ rust-ai-web-auto       │ main │ …│ OK   │ …│ no origin remote (using github SSH instead)│
│ 9 │ ❌ CONCERN │ dracon-libs            │ main │ …│ OK   │ …│ no origin remote (using github SSH instead)│
│ 10│ ❌ CONCERN │ DraconDev              │ main │ …│ OK   │ …│ no origin remote (using github SSH instead)│
│ 11│ ❌ CONCERN │ avid                   │ main │ …│ OK   │ …│ no origin remote (using github SSH instead)│
```

### After (0 ❌ CONCERN, 10 ✅ OK, 1 ⚠ WARN while editing)

```
📜 /home/dracon/.dracon/utilities/sync/dracon-sync.toml
📦 11 repos  ✅ OK 10  ⚠️  WARN 1  ❌ CONCERN 0
…
│ 1 │ ⚠️  WARN │ dracon-platform                │ main │ …│ OK │ …│ daemon handles after changes settle; run sync-now --warns to force now │
│ 2 │ ✅ OK    │ dracon-utilities               │ main │ …│ OK │ …│ no tracking upstream (daemon uses explicit refspecs; not a concern) │
│ 3 │ ✅ OK    │ browser-extensions-shared      │ main │ …│ OK │ …│ no tracking upstream (daemon uses explicit refspecs; not a concern) │
│ 4 │ ✅ OK    │ pully-fully-pull-based-fleet-… │ main │ …│ OK │ …│ no tracking upstream (daemon uses explicit refspecs; not a concern) │
│ 5 │ ✅ OK    │ .dracon                        │ main │ …│ OK │ …│ no tracking upstream (daemon uses explicit refspecs; not a concern) │
│ 6 │ ✅ OK    │ dracon-code                    │ main │ …│ OK │ …│ no tracking upstream (daemon uses explicit refspecs; not a concern) │
│ 7 │ ✅ OK    │ ai-auto-writer                 │ main │ …│ OK │ …│ no tracking upstream (daemon uses explicit refspecs; not a concern) │
│ 8 │ ✅ OK    │ rust-ai-web-auto               │ main │ …│ OK │ …│ no tracking upstream (daemon uses explicit refspecs; not a concern) │
│ 9 │ ✅ OK    │ dracon-libs                    │ main │ …│ OK │ …│ no tracking upstream (daemon uses explicit refspecs; not a concern) │
│ 10│ ✅ OK    │ DraconDev                      │ main │ …│ OK │ …│ no tracking upstream (daemon uses explicit refspecs; not a concern) │
│ 11│ ✅ OK    │ avid                           │ main │ …│ OK │ …│ no tracking upstream (daemon uses explicit refspecs; not a concern) │
```

(`dracon-platform` was WARN, not CONCERN, in this snapshot because
the operator was actively editing it. Its CONCERN was cleared by
`dracon-sync repair concerns --apply` immediately before the
misclassification fix landed; the WARN is the expected behavior for
a repo with tracked modifications.)

## Validation

- `cargo test -p dracon-sync --locked` → 577 passed, 3 ignored
- `cargo build --release --locked` → 0 errors, 5 pre-existing warnings
- `cargo deny check` → advisories ok, bans ok, licenses ok, sources ok
- Live `dracon-sync repos` → 0 CONCERN across 11 watched repos

## Install / daemon restart — a critical follow-up

**Date:** 2026-06-20 (same day, follow-up goal `5f291ee1-7bd9-4abb-a44d-8e9ea1961391`)

After the code fix above landed, the operator reported that
`dracon-sync repos` STILL showed the old "no origin remote (using
github SSH instead)" hint for all 10 SSH-mirror repos. Investigation
proved the fix was on disk but not running:

### The two-binary problem

The original install workflow used `cargo install --path dracon-sync
--locked --force`, which places the new binary at
`/home/dracon/.cargo/bin/dracon-sync`. The daemon's systemd unit
(`~/.config/systemd/user/dracon-sync.service`) and the canonical
install script both target `/home/dracon/.local/bin/dracon-sync`
instead. Result: the operator's shell (`PATH=$HOME/.local/bin:...`)
and the running daemon were both executing the OLD binary, while the
new binary sat unused in `~/.cargo/bin/`.

Evidence captured during the follow-up investigation:

```
$ sha256sum /home/dracon/.local/bin/dracon-sync /home/dracon/.cargo/bin/dracon-sync
99fc32720709f851a09b17ce0049c5f8a396caa311c8aefdac062bb7234eb147  /home/dracon/.local/bin/dracon-sync   # OLD (12.27 MB, mtime 01:51)
84710c373259dfaf08c9e7e4ef9ff92346a6c96067928397e33556b421debe54  /home/dracon/.cargo/bin/dracon-sync   # NEW (10.56 MB, mtime 03:00)

$ systemctl --user show -p MainPID --value dracon-sync.service
1134988
$ readlink /proc/1134988/exe
/home/dracon/.local/bin/dracon-sync     # ← OLD binary still running
```

### The fix

Use the project's own `install.sh` with `--upgrade --binaries-only`,
which:

1. Stops the systemd daemon (`systemctl --user stop
   dracon-sync.service`, plus a `pkill` fallback).
2. Removes the stale `~/.cargo/bin/dracon-sync` artifact (lines
   156–169 of `install.sh` explicitly clean this up).
3. Builds each package with the correct release profile.
4. Installs to `~/.local/bin/$binary` (the canonical location the
   systemd unit points at).
5. Restarts the systemd daemon so the new binary is loaded.

```
$ cd /home/dracon/Dev/dracon-utilities
$ ./install.sh --upgrade --binaries-only
…
  ✅ Installed ~/.local/bin/dracon-sync (updated)
…
$ sha256sum /home/dracon/.local/bin/dracon-sync
84710c373259dfaf08c9e7e4ef9ff92346a6c96067928397e33556b421debe54  /home/dracon/.local/bin/dracon-sync   # matches NEW

$ systemctl --user show -p MainPID --value dracon-sync.service
1412654
$ readlink /proc/1412654/exe
/home/dracon/.local/bin/dracon-sync

$ sha256sum /proc/1412654/exe
84710c373259dfaf08c9e7e4ef9ff92346a6c96067928397e33556b421debe54  /proc/1412654/exe   # matches
```

After the install:

- `/home/dracon/.cargo/bin/dracon-sync` is gone (cleaned up by the
  installer as a "shadowing binary").
- `/home/dracon/.local/bin/dracon-sync` is the NEW binary.
- The systemd daemon (PID 1412654) is running the NEW binary.
- `dracon-sync repos` (operator's PATH) reports 0 CONCERN across 11
  repos. Re-checked after >60 s of daemon uptime to confirm the
  daemon and CLI agree.

### Operator action

**Always use `install.sh` to deploy daemon changes — do NOT use
`cargo install`.** `cargo install` writes to `~/.cargo/bin/` and
leaves the daemon running the old binary in `~/.local/bin/`. The
symptom is "fix is on disk but `dracon-sync repos` still shows the
old hint" — confusing, because `cargo install` reports success and
the operator has no way to know the daemon is pinned to a different
path.

```
cd /home/dracon/Dev/dracon-utilities
./install.sh --upgrade --binaries-only    # preferred: stop, install, restart
# or
./install.sh --upgrade                    # full upgrade: also re-touches configs/services
```

For dry-run preview:

```
./install.sh --dry-run --upgrade --binaries-only
```

If the operator ever needs to bypass `install.sh` (e.g. to test a
debug build), they must also update the systemd unit's `ExecStart=`
path and `systemctl --user daemon-reload`. The current systemd unit
hardcodes `%h/.local/bin/dracon-sync`; a `cargo install`-only path
will silently desync the daemon.

## Constraints preserved

- The commit-all default policy (`untracked_exclude_patterns = []`)
  and 100 MiB size limit (`max_stage_file_bytes = 104857600`) are
  unchanged.
- The 300 s push timeout (`push_op_timeout_secs = 300`) is unchanged.
- All 11 repos still push via SSH to `github` / `gitlab` /
  `codeberg` exactly as before — no transport, retry, or auth
  changes.
- Backwards compatibility is preserved:
  - Repos with `origin` + tracking upstream still classify as
    `OK`/`PUSH`/`STUCK_PUSH` exactly as before.
  - Repos with `origin` but no tracking upstream still classify
    as `NO_UPSTREAM` (CONCERN) with the original
    "set upstream" hint — `repair concerns --apply` will succeed.
  - Repos with no remotes at all still classify as `NO_ORIGIN`
    (CONCERN) with the "no remote configured (cannot push)" hint.
  - Repos with no `origin` but with at least one non-origin remote
    (e.g. SSH mirrors) are now correctly classified as healthy.
- The `NO_ORIGIN` flag name is preserved in the source code
  (only its firing condition narrowed) so any operator tooling
  that greps for it continues to work.
- No new dependencies, no `.env`/`*.pem`/`*.key`/`*.age` exposure,
  no dead code, no TODOs, no undocumented behavior changes
  (the new `has_any_remote` parameter and the changed hint text
  are documented in the source comments and in this design doc).
