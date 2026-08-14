# Installed binary silently dropped `[patch.crates-io]` → phantom untracked counts (2026-08-08)

## Symptom

`dracon-sync repos` showed **persistent untracked counts** that never got
committed and never changed across scans:

- endless-td **294**, dracon-platform **48**, hellhunter **16**, polis **8**, deathrun **4**

State classification degraded to `⚪ untracked-only` for endless-td,
hellhunter, polis, deathrun. Meanwhile `git ls-files --others
--exclude-standard` and `git status --porcelain` in the same repos
returned **0 untracked** — the files existed, were gitignored, and the
daemon was (correctly) never committing them.

## Root cause

1. **The published crate drops `[patch.crates-io]`.** The workspace
   `Cargo.toml` patches `dracon-git` to
   `git+https://github.com/DraconDev/dracon-libs?tag=v94.7.2`
   (workaround for the libgit2 ssh-agent bug, since 2026-07-25).
   `cargo publish` normalizes the manifest and **strips the patch
   section** — verified by diffing
   `~/.cargo/registry/cache/*/dracon-sync-0.113.45.crate` against the
   workspace. The published manifest declares
   `dracon-git = "94.7.0"` with no patch.

2. **`cargo install dracon-sync --version X --locked` therefore builds
   against crates.io dracon-git 94.7.0**, not the patched git version.
   crates.io has no dracon-git 94.7.2 (the documented follow-up —
   publish it — was never done), so the patch cannot resolve to a
   registry version.

3. **crates.io dracon-git 94.7.0's `get_status` lacks the
   `git ls-files --others --exclude-standard -z` override** that git
   dracon-git added 2026-06-20 (goal 38142891). It counts raw libgit2
   `is_wt_new()` entries.

4. **libgit2 1.9.x disagrees with git CLI 2.51.2 on a handful of ignore
   rules in these repos.** Empirically (probe via a git2-based test
   binary): libgit2 fails to apply `docs/screenshots/` (line 27),
   `.pi/` (31), `static/assets/png/_v15-candidates/` (73),
   `static/assets/png/_pre-v15-backup/` (74) in endless-td's
   `.gitignore`, while correctly applying `node_modules/`, `build/`,
   `docs/audit/`, `test-results/`, `.bucket-cache/`, etc. `git
   check-ignore -v` confirms git CLI honors all of them. The 294 count
   == exactly the 248 files under `.pi/` + 46 files under the two
   `static/assets/png/_v*` dirs.

   The override exists precisely to compensate for this class of
   libgit2 quirk (plus untracked-dir collapsing); without it the
   deployed binary shows the raw libgit2 view.

## Impact

- **Display/classification noise only.** The dirty-scan and commit
  paths use `git status --porcelain` (CLI) and always agreed with git:
  the files were never committed anywhere, commits/pushes were
  unaffected, and no alert fired from this.
- Every binary installed via `cargo install` since the patch was
  introduced (2026-07-25) silently carried unpatched dracon-git 94.7.0
  — including the v0.113.44/v0.113.45 installs. (It also lacked the
  v94.7.2 ssh-agent fix the patch exists for, though the daemon shells
  out to git CLI for network ops.)

## Fix applied

1. `cargo build --release --locked -p dracon-sync` (workspace build —
   honors the lock + patch → git dracon-git v94.7.2).
2. Verified behavior on the live repo: workspace build reports
   `untracked: 0` for endless-td (deployed binary reported 294).
3. `rm -f ~/.local/bin/dracon-sync ~/.cargo/bin/dracon-sync && cp` the
   workspace build to both, `systemctl --user restart
   dracon-sync.service`.
4. Verified `repos --json`: all five flagged repos now
   `untracked: 0`, state `synced`/`idle`.

## Verification trap

`dracon-sync --version` shows 0.113.45 both before and after — the
version does not distinguish the dependency graph. The decisive checks
are behavioral: run `repos --json` on any repo with a gitignored `.pi/`
dir (must show 0), or diff the published crate's Cargo.toml against the
workspace (published manifest must contain `[patch.crates-io]`).

## Follow-up status (updated 2026-08-14)

1. **Resolved**: `dracon-git v94.7.2` was published to crates.io on
   2026-08-08. The workspace now depends on `dracon-git = "94.7.2"`
   directly, with no `[patch.crates-io]` workaround or git-source
   allowlist. This closes the published-binary dependency gap.
2. **Resolved**: `dracon-sync/scripts/release.sh` now packages the crate,
   installs that packaged directory with fresh dependency resolution, and
   runs `scripts/verify-install.sh` against the resulting binary before it
   creates the release tag. The fixture scans only its scratch repository and
   asserts that a gitignored `.pi/` directory produces `untracked=0`, so a
   future published-artifact dependency regression blocks the release before
   publication to the operator's users.

   After a manual `cargo install`, operators can still run
   `dracon-sync/scripts/verify-install.sh` as a local smoke check.
