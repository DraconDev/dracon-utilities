# auto-private-repo feature fix — 2026-07-15 (goal `1de90bd2`)

## Symptom (reported)

`dracon-sync repos` flagged a freshly `git init`'d repo as 🚫 **unowned
(`untrusted_author`)**:

```
│ ... ┆ 🚫 unowned ┆ wezterm-config ┆ ... ┆ dracon <dracon@localhost> ┆ ... ┆ healthy │
```

The operator reported the long-standing "git init a folder → we
automatically make a private repo" feature appeared broken: wezterm-config
had a github remote but its **codeberg** (and gitlab) private repos were
never created, and it "still had problems."

## Root cause

Two daemon behaviors combine into a chicken-and-egg that only bites
repos whose **first commit uses the machine-default git identity**
(`dracon <dracon@localhost>` / `dracon <dracon@local>`) before the repo's
local `user.name`/`user.email` are set to the canonical
`DraconDev <dracsharp@gmail.com>`:

1. **`auto_skip_unowned` defaults to `true`** (`policy.rs:567`,
   `#[serde(default = "default_true")]`). When a repo is classified
   `Unowned`, the daemon `continue`s at `daemon.rs:2393` — it performs
   **no auto-commit and no auto-push** for that repo.
2. **Private mirror creation happens only during a push**
   (`push_mirror_remotes` → `auto_create_all_remotes`, gated by
   `auto_create = true` on each remote). Because the unowned repo is
   skipped, `auto_create` never runs, so its codeberg/gitlab repos are
   never created.

For **owned** repos (e.g. `dracon-utilities`) the feature works fine —
the codeberg repo was created on first push. wezterm-config's only
commit was authored by the machine-default `dracon <dracon@localhost>`,
so it was classified `unowned` → skipped → its codeberg/gitlab repos
were never auto-created. github existed because that commit had been
pushed manually / before the guard tightened.

The ownership guard (`ownership.rs` `classify_ownership`) is working
**as designed** — it correctly refuses to auto-sync a repo whose HEAD
author is not a trusted identity. The "broken feature" was not a daemon
bug but a **trust configuration gap** for the operator's own
machine-default identity.

## Fix

Trust the operator's machine-default identities so a freshly `git init`'d
repo (first commit by `dracon <dracon@localhost>` or `dracon <dracon@local>`)
is classified `Owned` and the daemon creates + syncs its private mirrors.

In `/home/dracon/.dracon/utilities/sync/dracon-sync.toml`:

```toml
trusted_emails = ["dracsharp@gmail.com", "audit@dracon-code", "dracon@localhost", "dracon@local"]
trusted_authors = ["DraconDev", "dracon"]
```

(The two email variants come from git's hostname-based default identity on
different machines/configs: `dracon@localhost` vs `dracon@local`.)

This is a **config change only** — no Rust code was modified, so no
`cargo build/test/deny` rebuild was required. The daemon picks it up on
SIGHUP (the SIGHUP handler at `daemon.rs:2211` also calls
`activity.clear()`, which flushes the cached per-repo ownership verdict).

## Verification

- `systemctl --user kill -s SIGHUP dracon-sync.service` → daemon log
  "policy reloaded on SIGHUP", `activity.clear()` flushes ownership cache.
- `dracon-sync ownership --explain --repo /home/dracon/Dev/wezterm-config`:
  `✓ owned (trusted_email)`, no override, HEAD author
  `DraconDev <dracsharp@gmail.com>`, origin `github.com/DraconDev/wezterm-config.git`.
- `dracon-sync repos`: wezterm-config → **✅ OK / healthy**,
  PUSH-TO `github,gitlab,codeberg`, state synced.
- `git -C wezterm-config ls-remote --heads` for github(origin)/gitlab/codeberg
  all return `72894f8… refs/heads/main` (local `main` matches).
- `dracon-sync health`: healthy, policy valid, 29 repos.
- Tally improved: **OK 27 (was 25), CONCERN 0 (was 1)**, no regression in
  OK/WARN/CONCERN classification logic.

### Note on the new HEAD commit

`sync-now` of wezterm-config committed 2 previously-untracked files,
producing a new HEAD commit `72894f8` authored by
`DraconDev <dracsharp@gmail.com>` (the repo's now-correct local config).
This further solidifies the owned verdict (the old `12a6843` commit by
`dracon@localhost` remains in history but is now also trusted via the
`trusted_emails` entry).

## Remaining WARN / CONCERN items (investigated, daemon-handled)

From the original report (WARN ×3, CONCERN ×1):

| Repo | State | Disposition |
|------|-------|-------------|
| **wezterm-config** | 🚫 unowned (`untrusted_author`) | **RESOLVED** — now ✅ OK, all 3 mirrors synced (this fix) |
| **hegemon** | ❌ CONCERN (1 ahead / 1 behind) | **RESOLVED** — now ✅ OK / healthy / synced (daemon reconciled the divergence) |
| **dracon-platform** | ⚠️ WARN (dirty, 2 files in `web/`) | transient submodule-pointer changes; daemon commits them ("daemon handles after changes settle") |
| **deathrun** | ⚠️ WARN (dirty, 9 BIN in `docs/`) | transient; daemon commits them |
| **.dracon** | ⚠️ WARN (dirty, `utilities/sync/repos-size-cache.json`) | the perf-fix cache file; daemon commits it to its own config repo |
| **darklord** | ⚠️ WARN (pushing 1 ahead, in original report) | resolved by the time of fix (no longer in WARN list) |

All remaining WARNs are transient dirty states the daemon is actively
committing (each shows "Xs ago sync_commit" + "daemon handles after
changes settle"). None require manual intervention.

## Residual behavior (not a bug)

Private **mirror** repos (codeberg/gitlab) are created on the **first
push**, not on initial discovery. A repo that is clean after its first
commit is only on github until a subsequent change triggers a push (or an
explicit `sync-now`). wezterm-config's codeberg/gitlab were created by an
explicit `dracon-sync sync-now` after the trust fix. This matches the
daemon's design (auto-create is part of the push path) and is not a
regression.
