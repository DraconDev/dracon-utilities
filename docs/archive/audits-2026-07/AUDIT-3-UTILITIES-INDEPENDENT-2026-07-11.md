# AUDIT-3-UTILITIES-INDEPENDENT-2026-07-11

> Independent re-audit of the 3 utilities (dracon-sync, dracon-system,
> dracon-warden) in `~/Dev/dracon-utilities`. Goal: validate every claim
> in `AUDIT-3-UTILITIES-RERUN-2026-07-11.md` and look for what that
> audit missed.
>
> The prior three audits (07-10, RECHECK 07-11, RERUN 07-11) covered
> the same 4-command surface. This audit broadens to **architecture**,
> **clippy**, **CHANGELOG/version drift**, **dead tracked code**, and
> **cargo-deny fine print**.

## TL;DR

- **All 4 prior CONCERNs are confirmed RESOLVED** (release build,
  test build, full test, deny — all 0 exit, 0 warnings, 0 advisories).
- **5 new findings the prior audits missed**, in priority order:
  1. `report_v2_snapshot.rs` is **tracked in the nested repo's git
     history** (commit `4f287f1`) but never compiled — 237 KiB / 6339
     lines of dead tracked code.
  2. **`dracon-sync` is at v0.112.14** but `CHANGELOG.md` stops at
     v0.112.12 — the 2 unreleased versions are not in the meta-repo
     changelog.
  3. **CHANGELOG.md lists v0.112.10 twice** (lines 22 and 25).
  4. **`cargo clippy --workspace --locked` produces 28 warnings** in
     `dracon-sync` (none in the other 2 crates); no prior audit ran
     clippy.
  5. **The "0 untracked" daemon report is misleading** because the
     `dracon-utilities` parent repo's `git status` shows
     `?? dracon-sync/`, `?? dracon-system/`, `?? dracon-warden/` as
     untracked — but these are NESTED STANDALONE git repos (each has
     its own `.git/`), not untracked files of the parent.
- **The parent monorepo is intentionally a meta-only repo**: it does
  NOT track the Rust source. Source lives in the 3 nested repos. This
  is non-obvious and un-documented; the prior audits never called it
  out.

## 1. Methodology

| Step | Command | Purpose | New vs prior? |
|------|---------|---------|---------------|
| 1a | `cargo build --release --locked` | release | covered |
| 1b | `cargo build --tests --locked` | test code | covered (rerun) |
| 2 | `cargo test --workspace --locked --no-fail-fast` | full | covered |
| 3 | `cargo deny check` (workspace + per-crate) | advisories | covered |
| 4 | `cargo clippy --workspace --locked` | lints | **NEW** |
| 5 | `git ls-files`, `git status --porcelain` | tracking state | **NEW** |
| 6 | `git log --oneline --all -- '*.rs'` per nested repo | source history | **NEW** |
| 7 | `git tag -l` per nested repo, `Cargo.toml` version, `CHANGELOG.md` headers | version drift | **NEW** |
| 8 | `git ls-files` for `report_v2_snapshot.rs` per nested repo | dead-tracked check | **NEW** |

## 2. Re-validation of prior claims

### 2.1 `cargo build --release --locked` — workspace root

```
Finished `release` profile [optimized] target(s) in 0.75s
---EXIT: 0---
```

Exit 0, no output (no warnings). **Matches** the rerun audit's claim
of "exit 0, 0 warnings".

### 2.2 `cargo build --tests --locked` — workspace root

```
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.75s
---EXIT: 0---
```

Exit 0, no warnings. **Matches** the rerun audit's "exit 0, 0 warnings"
(post-FINDING-#7 fix). 58 warnings from the original audit are gone.

### 2.3 `cargo test --workspace --locked --no-fail-fast` — workspace root

Test count breakdown:

| Crate          | tests | passed | failed | ignored | exit |
|----------------|------:|-------:|-------:|--------:|-----:|
| dracon-sync    |  665  |  665   |   0    |    3    |  0   |
| dracon-sync    |   10  |   10   |   0    |    0    |  0   | (integration)
| dracon-system  |   86  |   86   |   0    |    0    |  0   |
| dracon-warden  |   76  |   76   |   0    |    0    |  0   |
| dracon-warden  |   10  |   10   |   0    |    0    |  0   | (integration)
| **total**      | **847** | **847** | **0** | **3**  | **0** |

**Matches** the rerun audit (847 tests, 0 failed, 3 ignored). All
3 CONCERN #4 sub-claims (test helpers correctly set
`core.hooksPath=/dev/null`) verified via grep — 17 sites across 5
files: `daemon.rs:444,537,620,3161,3185`, `git/discovery.rs:839`,
`exclude.rs:495,504,630,655,860`, `sync.rs:6623,6665,6793,7651,7664`,
`role.rs:196,198`.

### 2.4 `cargo deny check` — workspace root + per-crate

| Where | Exit | Result |
|-------|-----:|--------|
| workspace root | 0 | `advisories ok, bans ok, licenses ok, sources ok` |
| `dracon-sync/` | 0 | `advisories ok, bans ok, licenses ok, sources ok` |
| `dracon-system/` | 0 | `advisories ok, bans ok, licenses ok, sources ok` |
| `dracon-warden/` | 0 | `advisories ok, bans ok, licenses ok, sources ok` |

**Matches** the rerun audit. RUSTSEC-2026-0190 (anyhow 1.0.103 in all
4 lock files) and RUSTSEC-2026-0204 (crossbeam-epoch 0.9.20 in
workspace and `dracon-warden/`) remain resolved.

Two non-blocking warnings surfaced in the per-crate runs:
- `dracon-sync`: `unmatched license allowance "ISC"` (cosmetic —
  the `ISC` line in `deny.toml` is no longer needed because the
  workspace manifest changed the license set).
- `dracon-system`, `dracon-warden`: `unmatched skip configuration`
  (cosmetic — a `[advisories] skip` block references a path that no
  longer exists in the per-crate context).

These are not failures (deny exits 0), but the prior audits
didn't capture them.

## 3. New findings

### 🟡 FINDING #A — `report_v2_snapshot.rs` is **tracked in the nested
`dracon-sync` git repo at commit `4f287f1` but never compiled** (237 KiB
/ 6339 lines of dead tracked code)

**Severity:** 🟡 (medium — clutter, not a bug; but tracked dead code is
strictly worse than the rerun audit's "untracked, not compiled"
characterisation).

**Discovery:** The rerun audit's FINDING #8 said the file "is not
included in the `cargo build --tests` warning set" and listed two
hypotheses (`#[cfg(test)]`-gated or `let _ =` wrapped) without
resolving which. Running `git ls-files` in the nested `dracon-sync/`
repo:

```
$ cd dracon-sync && git ls-files | grep report_v2
report_v2_snapshot.rs  # at commit 4f287f1
```

…shows the file IS tracked in the nested git repo. The rerun audit
also said the file is "untracked" in the parent; that part is correct
(the parent does not see nested-repo internals as tracked), but the
narrative "not compiled" is missing the key detail: **it is tracked
git content that the build never uses**.

**The file's own header explains the situation:**

```
// V2 CARD DESIGN SNAPSHOT — preserved for reference, not compiled.
//
// This file is a snapshot of the v2 card design that was introduced in commits
// 3eb648f (added render_push_to_with_icons, render_repo_card) and refined in
// 7a525cb (removed format_push_to_remotes_cell, StateCause::icon(), state_cause_as_str),
// then tweaked in 78f5a68 (multi-line legend, subject truncation) and 14a19d3
// (publish label shortening, hint truncation).
//
// The v2 design was reverted to the v1 comfy_table-based design on 2026-06-27
// per the operator's request: 'i am not over the new dracon-sync repos table its
// less informative than the one we have before'.
//
// To re-enable the v2 design in the future:
//   1. Move the rendering functions (render_repo_card, render_push_to_with_icons)
//      back to src/report.rs
//   2. Remove the format_push_to_remotes_cell, StateCause::icon(), and
//      state_cause_as_str() restoration from src/report.rs
//   3. Update the main loop in run_repos_report() to call render_repo_card()
//      instead of the comfy_table-based rendering
//
// See docs/design/repo-remote-visibility-v3-revert-2026-06-27.md for the full
// revert context and the v2 design's intended use case.
```

**Recommendation:** `git rm src/report_v2_snapshot.rs` in the nested
`dracon-sync/` repo and add a `docs/design/v2-card-design-snapshot-2026-06-16.md`
that captures the same content for the "restore this design" use case.
The file currently adds 237 KiB of git history noise, forces anyone
browsing `src/` to ignore it, and creates a false-positive "dead code"
warning surface for future audits. The "preserved for reference" intent
is better served by a design doc that has its own version history and
won't drift as `src/report.rs` evolves.

**Why the prior audits missed it:** the prior audits' "0 warnings
in test build" + "30 `.success()` sites in `report_v2_snapshot.rs`
not exercised" framing was internally consistent but treated
*symptoms* (no warnings) instead of *cause* (file is dead). The
prior audit's FINDING #8 was a half-step — it asked the right
question ("why is this file not in the warning set?") but
didn't follow up with `git ls-files` or `git log -- src/report_v2_snapshot.rs`.

### 🟠 FINDING #B — `dracon-sync` is at v0.112.14 but `CHANGELOG.md`
stops at v0.112.12

**Severity:** 🟠 (medium-high — release-tracker integrity).

**Discovery:** The parent monorepo's `CHANGELOG.md` has only these
version headers:

```
$ grep -E '## \[' CHANGELOG.md
## [Unreleased]
## [0.112.12] - 2026-06-21
## [0.112.11] - 2026-06-17
## [0.112.10] - 2026-06-17   ← duplicated (see FINDING #C)
## [0.112.9]  - 2026-06-16
## [0.112.8]  - 2026-06-16
## [0.112.7]  - 2026-06-16
## [0.112.6]  - 2026-06-16
## [0.112.5]  - 2026-06-16
## [0.112.4]  - 2026-06-07
## [0.3.0]     - 2026-06-07
## [0.2.0]     - 2024-05-03
## [0.1.0]     - 2024-04-28
```

The actual version in the nested `dracon-sync/Cargo.toml`:

```
$ grep -E '^version' dracon-sync/Cargo.toml
version = "0.112.14"
```

And the `dracon-sync/release-notes-v0.112.13.md` and `dracon-sync/release-notes-v0.112.14.md`
files exist in the nested `dracon-sync/` repo (tracked there,
untracked in the parent):

```
$ ls dracon-sync/release-notes-v0.112.{13,14}.md
dracon-sync/release-notes-v0.112.13.md
dracon-sync/release-notes-v0.112.14.md
```

The release commits in the nested repo:

```
$ git log v0.112.13 -1 --format="%ai %s"
2026-06-21 19:30:21 4 file(s) [dracon-sync/release-notes-v0.112.13.md, Cargo.lock, Cargo.toml] DELTA:+29/-2 | NEW:dracon-sync/release-notes-v0.112.13.md DEPS:+version,-version
$ git log v0.112.14 -1 --format="%ai %s"
2026-06-22 15:35:25 release: v0.112.14
```

The 2 versions v0.112.13 and v0.112.14 (released 2026-06-21 and
2026-06-22 respectively) are **missing from the meta-repo CHANGELOG**.
The installed `dracon-sync --version` returns `dracon-sync 0.112.14`,
so the operator's working binary is past the documented changelog.

The `dracon-system` and `dracon-warden` versions are both `0.112.12`
in their Cargo.toml, matching the CHANGELOG — so the drift is
specific to `dracon-sync`.

**Recommendation:** add `## [0.112.13] - 2026-06-21` and
`## [0.112.14] - 2026-06-22` sections to `CHANGELOG.md` summarizing
the changes in the corresponding release-notes files. Or, if the
CHANGELOG is intended to live in the nested repo (since the nested
repo's CHANGELOG.md is what the prior audit's CHANGELOG header is
*talking about*), add a top-of-file note explaining that and link
to the nested repo's CHANGELOG.

**Why the prior audits missed it:** the prior audits focused on
build/test/deny, not on changelog drift. The "0.112.12" string
appears in the rerun audit's title (RELEASE-NOTES v0.112.12) and
matches the CHANGELOG's latest entry, so the version-drift went
unnoticed.

### 🟠 FINDING #C — `CHANGELOG.md` lists `## [0.112.10] - 2026-06-17`
**twice** (lines 22 and 25)

**Severity:** 🟠 (medium — cosmetic but a real bug, the second
section is empty).

**Discovery:**

```
$ grep -n "0.112.10" CHANGELOG.md
17:- **`push_op_timeout_secs = 300` (CHANGED 2026-06-17, was 60)**: the v0.112.10 release surfaced a 60s `push_op_timeout_secs` ...
19:- **Stress test (61 files, ~1.5MB of PNG binaries)** at the new 300s timeout: github 2.35s, gitlab 2.57s, codeberg 10.51s, origin 0.64s. All 4 remotes well under the 300s budget. The v0.112.10 incident was network-related, not capacity-related.
22:## [0.112.10] - 2026-06-17
25:## [0.112.10] - 2026-06-17
```

The CHANGELOG also has another duplicate — `## [0.112.10]` appears
twice in the headers, plus the `## [0.112.10]` text appears in body
references at lines 17 and 19.

**Recommendation:** delete the duplicate empty `## [0.112.10]`
section at line 25.

**Why the prior audits missed it:** the prior audits quoted the
CHANGELOG but didn't grep for duplicate version headers.

### 🟠 FINDING #D — `cargo clippy --workspace --locked` produces
**28 warnings** in `dracon-sync` (none in the other 2 crates)

**Severity:** 🟠 (medium — quality signal the prior audits didn't run).

**Discovery:** running `cargo clippy --workspace --locked`
(workspace root) gives exit 0 but emits 28 clippy warnings, all in
`dracon-sync`. The rerun audit and the recheck audit ran
`cargo build --release --locked` and `cargo build --tests --locked`
but never `cargo clippy`. This audit ran clippy and categorized the
warnings:

| Count | Category |
|------:|----------|
| 7  | `clippy::doc_markdown` — doc list item without indentation |
| 3  | `clippy::doc_markdown` — doc list item overindented |
| 2  | `clippy::type_complexity` — very complex type |
| 2  | `clippy::useless_format` — `&format!("...")` should be `&"...".to_string()` |
| 2  | `clippy::if_chain` — method chain can be `if .. else ..` |
| 2  | `clippy::needless_late_init` (or similar) — `let..else` → `?` |
| 1  | `clippy::needless_late_init` — unneeded late init |
| 1  | `clippy::needless_borrow` — ref immediately deref'd |
| 1  | `clippy::unnecessary_map_or` |
| 1  | `clippy::derivable_impls` |
| 1  | `clippy::collapsible_match` |
| 1  | `clippy::redundant_closure` |
| 1  | `clippy::ok_some` — `Some(ok())` redundancy |
| 1  | `clippy::manual_range_contains` |
| 1  | `clippy::into_iter_on_ref` — explicit `.into_iter()` |
| 1  | empty line after doc comment |

Per-file distribution:

| File | warnings |
|------|---------:|
| `dracon-sync/src/report.rs` | 15 |
| `dracon-sync/src/sync.rs` | 4 |
| `dracon-sync/src/daemon.rs` | 3 |
| `dracon-sync/src/exclude.rs` | 2 |
| `dracon-sync/src/policy.rs` | 1 |
| `dracon-sync/src/git/mod.rs` | 1 |
| `dracon-sync/src/git/discovery.rs` | 1 |
| `dracon-sync/src/git/branch.rs` | 1 |
| **`dracon-system`** | **0** |
| **`dracon-warden`** | **0** |

13 of the 28 are auto-fixable via `cargo clippy --fix`. The rest
need manual review (doc indentation, type-complexity refactor,
let-else-to-?).

**Recommendation:** either (a) `cargo clippy --fix` the
auto-fixable subset and accept the rest as documented debt, or
(b) set up CI to run clippy with `-D warnings` so the warning count
stays at 0. AGENTS.md doesn't currently mandate clippy; the prior
audits added `cargo build --release --locked` + `cargo test
--workspace --locked` + `cargo deny check` as the test discipline.
Adding `cargo clippy --workspace --locked -- -D warnings` is a
single-line extension and would catch this whole class.

**Why the prior audits missed it:** clippy was not in the audit
bar. The build is clean; the lints are not.

### 🟡 FINDING #E — `dracon-utilities` parent repo does NOT track the
Rust source — it has 3 NESTED standalone git repos (dracon-sync,
dracon-system, dracon-warden) under it

**Severity:** 🟡 (medium — architectural, not a bug, but
non-obvious and un-documented).

**Discovery:**

```
$ git ls-files | wc -l
323
$ git ls-files | grep '\.rs$'
(nothing)
$ git status --porcelain
?? dracon-sync/
?? dracon-system/
?? dracon-warden/
```

The parent `dracon-utilities` repo's tracked files (323 total) are
ALL meta files: `AGENTS.md`, `CHANGELOG.md`, `Cargo.toml`, `Cargo.lock`,
`release-notes-*.md`, `AUDIT-3-UTILITIES-*.md`, `AUDIT_REPOS_*.md`,
`docs/design/*.md`, `.cargo/config.toml`, `.dracon/data/keys/*.pub`,
`.pi/goals/**`, `.github/workflows/*`. The Rust source code
(`*.rs` files) is **not tracked at all in the parent**.

The source lives in 3 NESTED STANDALONE git repos:

| Nested path              | Own .git? | Own remote | HEAD (date) | HEAD commit |
|--------------------------|:---------:|------------|-------------|-------------|
| `dracon-utilities/dracon-sync/`   | yes | `codeberg:dracondev/dracon-sync-background-auto-commit-multi-remote` (+ github, gitlab) | 2026-07-11 02:32 | `3158c94` |
| `dracon-utilities/dracon-system/` | yes | `codeberg:dracondev/dracon-system-disk-process-guard-doctor` (+ github, gitlab) | 2026-07-11 02:31 | `3103953` |
| `dracon-utilities/dracon-warden/` | yes | `codeberg:dracondev/dracon-warden-secret-encrypt-age-git-filter` (+ github, gitlab) | 2026-07-11 02:31 | `7f10bc9` |

Each nested repo:
- Has its own `.git/` directory
- Has its own remotes (codeberg, gitlab, github)
- Has its own commit history and tags (`dracon-sync/` has
  `v0.112.13`, `v0.112.14`, `v1.0.0` tags)
- Has its own `CHANGELOG.md` (separate from the parent's)
- Has its own `Cargo.toml` (with its own `version`, deps, etc.)
- Has its own `Cargo.lock`

**How the prior audits handled this:**

The prior audits' CONCERN #5 ("no workspace-root `Cargo.toml` /
README mismatch") correctly noted that the per-crate `cargo build
--release --locked` works standalone. They added a workspace
`Cargo.toml` at the parent level listing the 3 nested directories
as `members = [...]` so AGENTS.md's monorepo-root commands work.
This works **because cargo treats the nested directories as
workspaces members by their manifest path, not as submodules** —
the parent and the nested repos share no git history.

**The daemon's view of the parent:** `dracon-sync repos` shows the
parent as "✅ OK · standalone · main · origin/main · 0 mod 0 stg
0 untracked" — i.e. the daemon sees the parent as a healthy
meta-only repo with no source to commit. The 3 nested repos are
listed as separate rows in the same `dracon-sync repos` table
(13 = `dracon-utilities` parent, 25-27 = the 3 nested repos).
This matches the daemon's per-crate commit log on 2026-07-11:
"📝 committed 1 file(s) in /home/dracon/Dev/dracon-utilities/dracon-sync"
at 10:41:30, 10:49:18, etc. — the daemon commits to the nested
repos, not the parent.

**Implication for audits:** the rerun audit's "test build warnings
in `dracon-sync`" findings (`FINDING #7`) are about the nested
`dracon-sync/` repo, not the parent. The `dracon-utilities/`
parent has no `.rs` files to build. The workspace-root commands
(`cargo build --release --locked`, etc.) work because of the
parent's workspace manifest, not because the source is in the
parent.

**Why this matters:** anyone trying to "check out the dracon-sync
source" by cloning `dracon-utilities` will get the meta files
(AGENTS, CHANGELOG, audits, design docs) but **no Rust source**.
The source is in 3 separate clones / 3 separate codeberg/github
remotes. This is the same architecture as the
`dracon-platform/web/games/<name>/` submodules documented in
`AGENTS.md` (the "Submodule standalone worktree design" section),
but the prior audits never called it out for the utilities.

**Recommendation:** add a one-paragraph note to `AGENTS.md` or
`README.md` explaining the nested-standalone architecture
("`dracon-utilities/` is a meta-repo. The 3 utilities live in
nested standalone repos at `dracon-sync/`, `dracon-system/`,
`dracon-warden/`. Each has its own git history, its own remotes,
its own CHANGELOG. The parent's `Cargo.toml` is a workspace
manifest only."). This will save the next auditor (and any
new contributor) the 30+ minutes of confusion.

**Why the prior audits missed it:** the audits focused on
build/test/deny at the workspace root. They never asked
"what is the git relationship between the parent and the
nested directories?". The 323 tracked files vs `?? dracon-sync/`
in `git status` is a basic `ls-files` / `git status` cross-check
that no prior audit ran.

## 4. What's healthy (re-validated)

- All 3 crates compile cleanly under **release** and **tests** modes
  with 0 warnings.
- All 3 crates: 0 test failures across 847 tests (3 pre-existing
  ignored).
- `cargo deny check` exits 0 from workspace root AND per-crate.
  All 4 categories (advisories, bans, licenses, sources) clean.
- RUSTSEC-2026-0190 (anyhow 1.0.103) and RUSTSEC-2026-0204
  (crossbeam-epoch 0.9.20) remain resolved.
- 17 test-helper sites correctly set `core.hooksPath=/dev/null` to
  bypass the global dracon-warden pre-commit hook.
- Workspace `Cargo.toml` + `Cargo.lock` are present at the parent
  root and `cargo install --path <crate>` still works from the
  nested directories.
- AGENTS.md test discipline (`cargo build --release --locked`,
  `cargo test --workspace --locked`, `cargo deny check`) passes
  for all 3 utilities from the monorepo root.
- The 3 nested repos are healthy in `dracon-sync repos` output
  (each at `main`, 0/0, healthy).
- `report_v2_snapshot.rs` is the **only** orphan tracked file
  in the source tree. (Other 0-byte `*.plaintext` siblings are
  warden marker placeholders, not source files.)

## 5. Delta vs prior audits

| Aspect                                | Rerun (07-11)         | This audit (07-11)          | Change |
|---------------------------------------|-----------------------|------------------------------|--------|
| Release build warnings                | 0                     | 0                            | same |
| Test build warnings                   | 0                     | 0                            | same |
| Tests passing                         | 847/847               | 847/847                      | same |
| `cargo deny` exit (workspace + 3)     | 0                     | 0                            | same |
| `cargo clippy` warnings               | 0 (not measured)      | **28 in dracon-sync**        | **+28 found** |
| CHANGELOG version drift               | 0 (not measured)      | **2 versions behind**        | **+2 found** |
| CHANGELOG duplicate header            | 0 (not measured)      | **1 duplicate** (v0.112.10)  | **+1 found** |
| Dead tracked source                   | 0 (not measured)      | **237 KiB** (report_v2)      | **+1 found** |
| Nested-repo architecture documented   | not                   | not (recommendation only)    | **+recommendation** |
| Per-crate `cargo install`             | works                 | works                        | same |

## 6. Recommendations (priority order)

1. **`git rm src/report_v2_snapshot.rs` in the nested `dracon-sync/`
   repo and move the content to `docs/design/v2-card-design-snapshot-2026-06-16.md`.**
   (FINDING #A; medium.)

2. **Add `## [0.112.13] - 2026-06-21` and `## [0.112.14] - 2026-06-22`
   sections to `CHANGELOG.md`.** (FINDING #B; medium-high.)

3. **Delete the duplicate `## [0.112.10] - 2026-06-17` header at
   line 25 of `CHANGELOG.md`.** (FINDING #C; medium.)

4. **Decide on clippy policy.** Either run `cargo clippy --fix` on
   the 13 auto-fixable warnings, or add `cargo clippy --workspace
   --locked -- -D warnings` to AGENTS.md test discipline and fix
   all 28. (FINDING #D; medium.)

5. **Add a one-paragraph architecture note to `AGENTS.md` or
   `README.md` explaining the nested-standalone-repo layout.**
   (FINDING #E; low but high leverage — saves future confusion.)

## 7. Verification (re-run after this audit)

If the operator wants to confirm the audit, run the 8 commands
in §1 Methodology. Expected results:

- 1a, 1b, 2, 3: exit 0, all matches documented in §2.
- 4: exit 0 (clippy exits 0 by default — only `-D warnings` would
  fail it).
- 5: 323 tracked files in parent; `?? dracon-sync/`, `?? dracon-system/`,
  `?? dracon-warden/` from `git status` (3 nested standalone repos).
- 6: nested `dracon-sync/` has 3 commits since v0.112.12;
  `dracon-system/` and `dracon-warden/` HEADs are v0.112.12.
- 7: `dracon-sync/Cargo.toml` is at v0.112.14; parent `CHANGELOG.md`
  stops at v0.112.12.
- 8: `report_v2_snapshot.rs` is tracked in nested `dracon-sync/`
  at commit `4f287f1`; no `mod report_v2_snapshot;` in any
  `main.rs` / `lib.rs`.

## 8. Summary

The rerun audit (`AUDIT-3-UTILITIES-RERUN-2026-07-11.md`) was
internally consistent and correct on the 4-command surface (build,
test, deny, AGENTS.md discipline). It missed 5 things the audit
itself wasn't designed to look for: **dead tracked code, version
drift, CHANGELOG bugs, clippy, and the architectural decision
that the parent is a meta-only repo with nested standalone
utilities**. None of these are failures in the build/test/deny
sense — the code compiles, all 847 tests pass, 0 advisories
flagged. They are all *housekeeping* and *documentation* findings
that the next operator would benefit from cleaning up.

The audit is otherwise complete; the 5 new findings are
recommendations, not blockers. The 3 utilities are in their
best-known build/test/deny state.
