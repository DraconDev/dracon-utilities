# Systemic untracked-.md investigation + daemon guard (2026-06-16)

> **Operator said**: "but make sure we look over the
> sync and this could be a systemic issue we should
> make sure that least some mds dont go around being
> untrackerd"
>
> **Goal**: `e680cfa9-8d31-4a90-a923-a67732271194`
> **Status**: ACTIVE — investigation complete, guard
> in progress

## TL;DR

Investigation across all 14 watched repos:

1. **Only 1 untracked .md file** exists across all
   14 repos right now: the deferred
   `_template-visual-novel/README.md` in
   dracon-platform (the project goal `76ddaa7e`
   excluded from auto-staging).
2. **77 out of 293 AI sessions (26%) were launched
   from a subdirectory** of a watched repo, not
   from the repo root. This is the systemic CWD-
   drift pattern.
3. **3 doubled subdirs from past CWD-drift** exist
   in dracon-platform:
   - `apis/apis/docs/` (empty — the bug created
     the doubled dir, then someone re-ran from
     the correct path)
   - `web/web/test-results/ui-audit/` and
     `web/web/test-results/ui-audit-recon/`
     (PNGs there ARE tracked — the daemon caught
     the renamed path on later runs)
   - `web/games/games/` (this is **NOT** a
     doubled path; it was a deliberate rename
     from `web/games-hosted/games/` on
     2026-06-14)
4. **The 1 untracked .md we found originally
   (`platform-free-extension-shortlist.md`) was
   the ONLY case in 16 hours where CWD-drift
   produced an untracked content file that wasn't
   subsequently caught by the daemon**.

## Investigation method

### Step 1: Per-repo untracked scan

For each of the 14 watched repos, ran
`git ls-files --others --exclude-standard` and
filtered for `.md`/`.txt` files. Excluded dirs:
`.git/`, `node_modules/`, `target/`,
`build/`, `dist/`, `repo-runtime/`,
`.wxt/`, `.output/`.

**Result: 1 untracked .md across 14 repos.**

| Repo | Untracked .md/.txt | Class |
|------|-------------------:|-------|
| ai-auto-writer | 0 | clean |
| avid | 0 | clean |
| browser-extensions-shared | 0 | clean (was 1, fixed in `c19d21b8`) |
| dracon-ai-lib | 0 | clean |
| dracon-code | 0 | clean |
| DraconDev | 0 | clean |
| dracon-libs | 0 | clean |
| **dracon-platform** | **1** | deferred (per `76ddaa7e`) |
| dracon-utilities | 0 | clean |
| Junk-Runner-bevy | 0 | clean (11 untracked PNGs, not .md) |
| kiki-sassy-desktop-announcer | 0 | clean |
| pully-fully-pull-based-fleet-reconciler | 0 | clean |
| rust-ai-web-auto | 0 | clean |
| one-mil-girls (not in /Dev) | n/a | not in scope of this daemon |

The 1 untracked .md is
`dracon-platform/web/games/games/_lib/visual-
novel/README.md`, part of the deferred
`_template-visual-novel/` new-project tree.

### Step 2: AI session CWD analysis

Walked `/home/dracon/.pi/agent/sessions/**/*.jsonl`
to extract the `cwd` field from each session
header. For each CWD, classified as "repo root"
(exact match) or "subdirectory of repo root"
(CWD drift).

**Result: 26% of AI sessions (77/293) were
launched from a subdirectory of the watched repo,
not from the repo root.**

| Repo | Total sessions | CWD-drift sessions | Drift % | Top drift CWDs |
|------|---------------:|-------------------:|--------:|----------------|
| browser-extensions-shared | many | 63 | high | `/extensions/vidpro-extension` (16), `/death-note-typing-practice` (11), `/auto-form-filler` (7), `/web-automator` (5), `/extension-research` (4) |
| dracon-platform | many | 14 | high | `/web/` (4), `/apis/` (4), `/web/ai-hub/` (3), `/web/games/games/hellhunter` (1) |
| (other repos) | (low counts) | 0 | 0% | n/a |

### Step 3: Forensic trace of the 1 untracked .md

For the untracked
`platform-free-extension-shortlist.md` found in
`c19d21b8`, recovered the AI session log
(`/home/dracon/.pi/agent/sessions/--home-dracon-
Dev-browser-extensions-shared-docs-research-
extension-research--/2026-06-15T23-14-27-604Z_*.jsonl`)
and reconstructed the exact sequence:

1. **23:14:27** — agent launched with
   `cwd = /home/dracon/Dev/browser-extensions-shared/
   docs/research/extension-research/` (a
   subdirectory of the repo)
2. **23:18:06** — agent ran `test -e
   platform-free-extension-shortlist.md` → `missing`
3. **23:19:23** — agent ran `write` with relative
   path `docs/research/extension-research/platform-
   free-extension-shortlist.md`. Tool reported
   `Successfully wrote 11114 bytes`. The relative
   path got resolved against the deep CWD, so the
   file landed at the doubled path
4. **23:19:35-23:20:03** — agent ran completion
   verification: `test -f` passes, `rg` finds
   21 candidate matches, `git status --short` shows
   `?? docs/research/extension-research/platform-
   free-extension-shortlist.md` (the *intended*
   path, which the agent assumed it was at), python
   check passes 6/6 categories. Agent marks all
   checklist boxes `[x]`.
5. **23:20:38** — session ends. File is untracked
   at the doubled path. The agent's own verification
   said "complete" because all checks used the
   intended path (which was empty in the working
   tree).
6. **~9 hours later** — another agent created
   `platform-free-recent-monetizable.md` and
   **cross-linked to the file at the doubled path**
   because that's where the file actually lived.
7. **~16 hours later** — operator noticed the
   untracked file in the daemon's report
   (`browser-extensions-shared` had `1 UT` in the
   "untracked-only" state but the row was `OK`)

**The bug class is real but rare**: out of 77
CWD-drift sessions, only this one produced an
untracked .md file that wasn't subsequently caught
by the daemon's commit-all flow.

### Step 4: Verify other doubled paths

Walked the filesystem for subdirs where the parent
dir's name matches the subdir's name (the CWD-drift
fingerprint). Found:

| Doubled path | Repo | Class | Status |
|--------------|------|-------|--------|
| `apis/apis/docs/` | dracon-platform | CWD-drift artifact | **empty** — bug created the dir, the work landed elsewhere |
| `web/web/test-results/{ui-audit,ui-audit-recon}/` | dracon-platform | CWD-drift artifact (now) | PNGs ARE tracked (commit `c89caeb9` from 2026-06-06 captured them; subsequent runs are still in this doubled path because the agent keeps launching from `web/`) |
| `web/games/games/` | dracon-platform | **intentional rename** (NOT CWD-drift) | Tracked since 2026-06-14 (`b9d140f6`); the commit message says `web/{games-hosted => games}/games/.gitkeep` — deliberate path rename |

So there are 2 active CWD-drift artifacts in
dracon-platform (`apis/apis/`, `web/web/`) and 1
new CWD-drift surface (`web/web/test-results/`)
that's actively receiving tracked files but at the
wrong path. The `_template-visual-novel/` project
in `web/games/games/_lib/` is the deferred new
project.

## Why didn't the daemon catch it?

The current daemon's logic, per the comment in
`dracon-sync/src/report.rs`:

> "Untracked files remain visible in the UT column,
> but they are **not sync-relevant by themselves**.
> This keeps audit/research artifacts visible
> without turning build artifacts, screenshots, or
> local evidence into WARNs."

In other words: the daemon's commit-all policy
auto-stages modified and staged files, but it
**does NOT auto-stage untracked files by default**.
The comment from `0ab367b5` makes this explicit:

> "Junk-Runner-bevy 91 'MOD' was 3 untracked
> test-results/ PNGs."

The fix in `0ab367b5` was to ensure untracked
counts were tracked correctly (not lumped in with
modified), not to make untracked files
sync-relevant. So the daemon has always treated
"untracked-only" as benign, and the
`platform-free-extension-shortlist.md` case is
**expected behavior** by the current design.

## Recommendation: add a guard

Add a new daemon function that detects a
**specific sub-class** of untracked files that the
operator cares about:

1. **Untracked `.md` and `.txt` files that are
   NOT gitignored** — the type of file that is
   usually a deliverable or research artifact, not
   a build artifact
2. **Untracked files at a doubled-path location**
   (heuristic: the path contains
   `dirname(CWD)/dirname(CWD)/`) — strong signal
   of CWD-drift

The guard should:
- **NOT** change the existing WARN/concern
  classification (preserves commit-all policy)
- **NOT** auto-stage the files (operator decides)
- **DO** add a new field to the report: a
  "noteworthy untracked" list that shows the file
  path next to the repo name
- **DO** add a `dracon-sync doctor
  check-untracked-md` subcommand that walks all
  watched repos and lists every untracked .md/.txt

### Implementation plan

1. **Add `untracked_md_txt: Vec<String>` to the
   report's per-repo row** in `dracon-sync/src/
   report.rs`. Populate by running
   `git ls-files --others --exclude-standard | rg
   '\.(md|txt)$'` in each repo (cheap operation,
   ~50ms per repo).
2. **Add a new function `noteworthy_untracked()`
   in `dracon-sync/src/git/diff.rs` (or a new
   file)** that:
   - Takes a repo path
   - Returns `Vec<String>` of untracked .md/.txt
     files that are NOT gitignored
   - Uses `git ls-files --others --exclude-
     standard` filtered to `.md`/`.txt` extensions
3. **Add a unit test** that creates a temp
   repo, writes an untracked `.md` file, runs the
   function, and asserts the file is in the result.
4. **Add a CLI subcommand** `dracon-sync doctor
   check-untracked-md` that walks all watched
   repos and prints the untracked .md/.txt list
   grouped by repo.
5. **Add a periodic check** in the daemon's main
   loop (every 5 minutes, after each sync pass)
   that logs a warning if a new untracked .md
   appears since the last check.
6. **Update the live report** to show the
   noteworthy untracked count + first 3 paths in
   the HINT column (or a new column).

### CWD drift detection (optional, follow-up)

A separate follow-up could add a guard that
detects CWD-drift in AI sessions by walking
`/home/dracon/.pi/agent/sessions/**/*.jsonl` and
checking each `cwd` field against the watched
repos. If a session was launched from a
subdirectory of a repo, log a warning. This
requires no changes to the AI session tool
itself.

## Hard constraints honored

- **DO NOT auto-stage** untracked content
  (operator decides)
- **DO NOT change the WARN/concern classification**
  for untracked files (preserves `0ab367b5` and
  `76ddaa7e` policy)
- **DO NOT modify `_template-visual-novel`**
  (deferred per `76ddaa7e`)
- **DO NOT touch `web/games/games/`** (it's the
  intentional rename path, not a CWD-drift
  artifact)
- **Per-repo overrides still work** (the new
  function is a separate read-only check, doesn't
  affect auto_commit behavior)

## Next steps

1. Add `noteworthy_untracked()` to
   `dracon-sync/src/git/diff.rs`
2. Add unit tests
3. Add `dracon-sync doctor check-untracked-md`
   subcommand
4. Add periodic log line in the daemon loop
5. Add `untracked_md_txt` field to the report
6. `cargo test --workspace --locked`
7. `cargo build --release --locked`
8. Re-run the scan to confirm the guard fires
   for the (now-fixed) browser-extensions-shared
   case AND for the deferred dracon-platform case

## Implementation status (2026-06-16)

Steps 1, 2, 3, 6, 7, 8 complete. Steps 4, 5 deferred
(see "Deferred" section).

### Code changes

- **`dracon-sync/src/git/diff.rs`**: added
  `noteworthy_untracked(repo: &Path) -> Result<Vec<String>>`
  (lines after the existing `tracked_paths`). Runs
  `git ls-files --others --exclude-standard -z`
  (respects `.gitignore` re-include rules), filters
  to `.md` and `.txt` extensions, sorts, returns
  paths relative to the repo root. Includes a
  15-second timeout and `spawn_blocking` to avoid
  blocking the async runtime.
- **`dracon-sync/src/git/diff.rs`**: added 6 unit
  tests in a new `noteworthy_untracked_tests` mod:
  - `test_noteworthy_untracked_empty_repo`
  - `test_noteworthy_untracked_finds_md_files`
  - `test_noteworthy_untracked_finds_txt_files`
  - `test_noteworthy_untracked_ignores_gitignored`
  - `test_noteworthy_untracked_excludes_other_extensions`
  - `test_noteworthy_untracked_finds_doubled_path`
    (the actual bug class from goal `c19d21b8`)
- **`dracon-sync/src/main.rs`**: added
  `CheckUntrackedMd` variant to the `Command` enum
  with `--repo <path>` and `--json` flags.
- **`dracon-sync/src/main.rs`**: added
  `cmd_check_untracked_md()` handler that walks
  watched repos (or a single repo), collects
  untracked `.md`/`.txt`, and prints a table or
  JSON output. Sorts repos by untracked count
  (descending) then by path.

### Test results

```
running 6 tests
test git::diff::noteworthy_untracked_tests::test_noteworthy_untracked_empty_repo ... ok
test git::diff::noteworthy_untracked_tests::test_noteworthy_untracked_excludes_other_extensions ... ok
test git::diff::noteworthy_untracked_tests::test_noteworthy_untracked_finds_doubled_path ... ok
test git::diff::noteworthy_untracked_tests::test_noteworthy_untracked_finds_md_files ... ok
test git::diff::noteworthy_untracked_tests::test_noteworthy_untracked_finds_txt_files ... ok
test git::diff::noteworthy_untracked_tests::test_noteworthy_untracked_ignores_gitignored ... ok

test result: ok. 6 passed; 0 failed; 0 ignored
```

Full workspace: **857 tests passed, 0 failed, 9
ignored** (was 851 + 6 new = 857).

### Live verification

```
$ dracon-sync check-untracked-md
📝 Found 1 untracked .md/.txt file(s) across 14 repo(s):

  📦 /home/dracon/Dev/dracon-platform
    - web/games/games/_lib/visual-novel/README.md
```

The 1 untracked .md is the known-deferred
`_template-visual-novel` project per goal
`76ddaa7e`. The fix in goal `c19d21b8` (move
`platform-free-extension-shortlist.md` to the
correct path) is correctly detected as **clean**
by the new guard.

### JSON mode

```
$ dracon-sync check-untracked-md --json
{
  "total": 1,
  "repos": [
    {
      "repo": "/home/dracon/Dev/dracon-platform",
      "files": ["web/games/games/_lib/visual-novel/README.md"]
    },
    ...
  ]
}
```

### Single-repo mode

```
$ dracon-sync check-untracked-md --repo /home/dracon/Dev/browser-extensions-shared
✅ No untracked .md/.txt files found across 1 repo(s).
```

The previously-broken `browser-extensions-shared`
is now clean (the doubled-path file was moved and
committed in goal `c19d21b8` followup).

## Deferred (out of scope for this goal)

- **Step 4 (periodic log line in daemon loop)**:
  would require touching the daemon's main loop in
  `daemon.rs`. Could be a follow-up goal. The CLI
  subcommand already covers the "operator runs
  this manually" use case.
- **Step 5 (`untracked_md_txt` field in the
  report)**: would require touching `report.rs`
  and the live `dracon-sync repos` table. Could
  be a follow-up. The CLI subcommand covers the
  "I want to check now" use case.

These were deferred because the operator's
immediate concern was: "make sure some mds don't
go around being untracked". The CLI subcommand
achieves that goal (the operator can run it
periodically or whenever suspicious). The
report-integration and daemon-loop-integration
are nice-to-haves that can be added without
risk of breaking the existing commit-all
policy.
