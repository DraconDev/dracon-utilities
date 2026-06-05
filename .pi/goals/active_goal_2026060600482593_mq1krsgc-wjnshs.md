{
  "version": 3,
  "id": "mq1krsgc-wjnshs",
  "objective": "=== Goal ===\nObjective: Make the dracon-utilities repo presentable: fix outdated/misleading content in READMEs and BLUEPRINTs, remove obvious clutter from the repo root, restructure scattered docs into a clean docs/ tree, and add a top-level ROADMAP pointing to canonical documentation.\n\nSuccess criteria:\n- All 4 READMEs (root, sync, system, warden) describe only the binaries that exist and use the same facts as AGENTS.md (no \"AI commit messages\" claim, no mention of removed `dracon-ai` binary).\n- All 3 BLUEPRINTs (sync, system, warden) reflect current behavior, not aspirational.\n- A `docs/` directory exists with: `ROADMAP.md`, `ARCHITECTURE.md` (merging `dracon-sync-architecture.md`), `OPERATIONS.md` (merging the operator-facing parts of dated plans), and `archive/` for kept historical docs.\n- Repo root contains only: README.md, CHANGELOG.md, CONTRIBUTING.md, AGENTS.md, LICENSE, Cargo.toml, Cargo.lock, install.sh, uninstall.sh, deny.toml, clippy.toml, rustfmt.toml, rust-toolchain.toml, tarpaulin.toml, flake.lock, flake.nix, scripts/, target/, the 3 binary dirs, and the existing .dracon/ + .pi/ + .gitignore/.gitattributes.\n- Clutter removed (no `pi-session-*.html`, no `rust_out`, no `autoresearch.jsonl`, no `debug.log`, no `SPEC.md`, no `dracon-sync-architecture.md` at root, no `todo.md`/`TODO.md`/`tasks.md` duplicates, no `audit.md`/`AUDIT.md`/`AUDIT_2026-05-29.md`/`AUDIT_CHECKLIST.md`).\n- Dated one-shot reports (`MASTER_ROADMAP_2026-06-01.md`, `STUCK_PUSH_TRIAGE_2026-06-02.md`, `REPOS_CLEANUP_PLAN_2026-06-01.md`, `REFACTORING_BLOCKER_ANALYSIS.md`) are either deleted or moved to `docs/archive/`.\n- A `docs/ROADMAP.md` exists that lists what each doc is for and what superseded.\n- A `docs/ARCHITECTURE.md` exists that replaces `dracon-sync-architecture.md` (or merges its content).\n- A single canonical \"where things are\" pointer in the root README.\n\nBoundaries:\n- In scope: markdown docs (READMEs, BLUEPRINTs, audit/plan/todo/archive), repo-root file removal, .gitignore updates to prevent re-clutter, restructuring into docs/.\n- In scope: fixing content drift between docs (root README says \"dracon-ai\", subdir doesn't exist; sync README says \"AI commit messages\", AGENTS.md says removed).\n- Out of scope: code changes, Cargo.toml restructuring, behavior changes, build system changes, AGENTS.md content (treat as source of truth for what binaries/features exist).\n- Out of scope: dependencies on external tools (no need to run/install anything beyond `ls`/`grep`/`git`).\n- Out of scope: sub-binary BLUEPRINTs beyond surface-level polish (no rewrites of their internals).\n\nConstraints:\n- No force-pushes. Changes go in a normal commit per logical step.\n- .gitignore must be updated to prevent re-clutter (autoresearch.jsonl, debug.log, pi-session-*.html, /rust_out must be ignored) BEFORE the deletion commit so daemon never re-commits them.\n- Use the existing dracon-warden IndexLock-aware patterns when staging the deletion (or commit during freeze if IndexLock can't be used). Actually: deletion of top-level files is a normal git operation, no working-tree writes involved, so IndexLock doesn't apply. But if files are being deleted by a hook, ensure no .git/index.lock contention.\n- Any doc that survives deletion must end up either in docs/ (canonical) or docs/archive/ (historical). No half-deleted files.\n- Sync daemon (dracon-sync) is running and will auto-commit. Either pause sync first (`dracon-sync pause`) or batch the deletion into a single commit so the auto-commit doesn't fragment the cleanup.\n- Commit messages should be deterministic facts (as per dracon-sync commit format), not prose.\n\nVerification contract:\n- `git log --oneline -10` shows a clean series of commits: (1) .gitignore updates, (2) deletions, (3) restructured docs, (4) README sync-up.\n- `ls /home/dracon/Dev/dracon-utilities/` matches the canonical file list above.\n- `find /home/dracon/Dev/dracon-utilities -maxdepth 2 -name 'README.md'` returns exactly 4 files (root + 3 subdirs).\n- `find /home/dracon/Dev/dracon-utilities -maxdepth 2 -name 'BLUEPRINT.md'` returns exactly 3 files (one per subdir).\n- `ls /home/dracon/Dev/dracon-utilities/docs/` returns ROADMAP.md, ARCHITECTURE.md, OPERATIONS.md, archive/.\n- Root README contains a \"Documentation\" section linking to docs/ROADMAP.md, docs/ARCHITECTURE.md, docs/OPERATIONS.md, and the 3 subdir READMEs.\n- `grep -r \"AI commit messages\\|AI-generated\" /home/dracon/Dev/dracon-utilities/dracon-sync/README.md` returns nothing.\n- `grep -r \"dracon-ai\" /home/dracon/Dev/dracon-utilities/README.md` returns nothing (or only historical mention if any).\n- `grep -r \"TODO\\|todo\" /home/dracon/Dev/dracon-utilities/{todo.md,TODO.md,tasks.md}` fails (files don't exist).\n- `git status` is clean at the end.\n\nIf blocked: stop and ask the user.",
  "status": "active",
  "autoContinue": true,
  "usage": {
    "tokensUsed": 650924,
    "activeSeconds": 75
  },
  "sisyphus": false,
  "createdAt": "2026-06-05T23:48:25.932Z",
  "updatedAt": "2026-06-05T23:49:45.441Z",
  "activePath": ".pi/goals/active_goal_2026060600482593_mq1krsgc-wjnshs.md",
  "taskList": {
    "tasks": [
      {
        "id": "audit-current-state",
        "title": "Audit current docs and root clutter",
        "status": "pending",
        "verificationContract": "Output a list of: (1) all md files with line counts, (2) all top-level clutter files, (3) every fact-claim that disagrees with AGENTS.md."
      },
      {
        "id": "update-gitignore",
        "title": "Update .gitignore to prevent re-clutter",
        "status": "pending",
        "verificationContract": ".gitignore contains patterns for: pi-session-*.html, /rust_out, autoresearch.jsonl, debug.log, *.html.tmp. dracon-sync pause issued, no auto-commits during cleanup."
      },
      {
        "id": "create-docs-dir",
        "title": "Create docs/ directory structure",
        "status": "pending",
        "verificationContract": "docs/ exists with ROADMAP.md, ARCHITECTURE.md, OPERATIONS.md, archive/ subdir."
      },
      {
        "id": "move-archive-docs",
        "title": "Move dated planning docs to docs/archive/",
        "status": "pending",
        "verificationContract": "MASTER_ROADMAP_2026-06-01.md, STUCK_PUSH_TRIAGE_2026-06-02.md, REPOS_CLEANUP_PLAN_2026-06-01.md, REFACTORING_BLOCKER_ANALYSIS.md, AUDIT_2026-05-29.md, audit.md, AUDIT.md, AUDIT_CHECKLIST.md, todo.md, TODO.md, tasks.md, SPEC.md all either deleted or moved to docs/archive/."
      },
      {
        "id": "remove-root-clutter",
        "title": "Remove non-md clutter from repo root",
        "status": "pending",
        "verificationContract": "pi-session-*.html, /rust_out, autoresearch.jsonl, debug.log removed. Repo root matches canonical list."
      },
      {
        "id": "rewrite-readmes",
        "title": "Rewrite 4 READMEs to be accurate and presentable",
        "status": "pending",
        "verificationContract": "All 4 READMEs are fact-aligned with AGENTS.md. Root README links to docs/ROADMAP.md, docs/ARCHITECTURE.md, docs/OPERATIONS.md. No mention of removed dracon-ai binary. No \"AI commit messages\" claim."
      },
      {
        "id": "polish-blueprints",
        "title": "Polish 3 BLUEPRINTs to reflect current state",
        "status": "pending",
        "verificationContract": "All 3 BLUEPRINTs match current behavior, no aspirational features claimed."
      },
      {
        "id": "final-verification",
        "title": "Final verification: clean tree, accurate docs, presentable repo",
        "status": "pending",
        "verificationContract": "git status clean. ls matches canonical list. All 7 verification checks from goal pass. dracon-sync resumed."
      }
    ],
    "blockCompletion": false,
    "proposedAt": "2026-06-05T23:48:25.942Z"
  }
}

# Goal Prompt

=== Goal ===
Objective: Make the dracon-utilities repo presentable: fix outdated/misleading content in READMEs and BLUEPRINTs, remove obvious clutter from the repo root, restructure scattered docs into a clean docs/ tree, and add a top-level ROADMAP pointing to canonical documentation.

Success criteria:
- All 4 READMEs (root, sync, system, warden) describe only the binaries that exist and use the same facts as AGENTS.md (no "AI commit messages" claim, no mention of removed `dracon-ai` binary).
- All 3 BLUEPRINTs (sync, system, warden) reflect current behavior, not aspirational.
- A `docs/` directory exists with: `ROADMAP.md`, `ARCHITECTURE.md` (merging `dracon-sync-architecture.md`), `OPERATIONS.md` (merging the operator-facing parts of dated plans), and `archive/` for kept historical docs.
- Repo root contains only: README.md, CHANGELOG.md, CONTRIBUTING.md, AGENTS.md, LICENSE, Cargo.toml, Cargo.lock, install.sh, uninstall.sh, deny.toml, clippy.toml, rustfmt.toml, rust-toolchain.toml, tarpaulin.toml, flake.lock, flake.nix, scripts/, target/, the 3 binary dirs, and the existing .dracon/ + .pi/ + .gitignore/.gitattributes.
- Clutter removed (no `pi-session-*.html`, no `rust_out`, no `autoresearch.jsonl`, no `debug.log`, no `SPEC.md`, no `dracon-sync-architecture.md` at root, no `todo.md`/`TODO.md`/`tasks.md` duplicates, no `audit.md`/`AUDIT.md`/`AUDIT_2026-05-29.md`/`AUDIT_CHECKLIST.md`).
- Dated one-shot reports (`MASTER_ROADMAP_2026-06-01.md`, `STUCK_PUSH_TRIAGE_2026-06-02.md`, `REPOS_CLEANUP_PLAN_2026-06-01.md`, `REFACTORING_BLOCKER_ANALYSIS.md`) are either deleted or moved to `docs/archive/`.
- A `docs/ROADMAP.md` exists that lists what each doc is for and what superseded.
- A `docs/ARCHITECTURE.md` exists that replaces `dracon-sync-architecture.md` (or merges its content).
- A single canonical "where things are" pointer in the root README.

Boundaries:
- In scope: markdown docs (READMEs, BLUEPRINTs, audit/plan/todo/archive), repo-root file removal, .gitignore updates to prevent re-clutter, restructuring into docs/.
- In scope: fixing content drift between docs (root README says "dracon-ai", subdir doesn't exist; sync README says "AI commit messages", AGENTS.md says removed).
- Out of scope: code changes, Cargo.toml restructuring, behavior changes, build system changes, AGENTS.md content (treat as source of truth for what binaries/features exist).
- Out of scope: dependencies on external tools (no need to run/install anything beyond `ls`/`grep`/`git`).
- Out of scope: sub-binary BLUEPRINTs beyond surface-level polish (no rewrites of their internals).

Constraints:
- No force-pushes. Changes go in a normal commit per logical step.
- .gitignore must be updated to prevent re-clutter (autoresearch.jsonl, debug.log, pi-session-*.html, /rust_out must be ignored) BEFORE the deletion commit so daemon never re-commits them.
- Use the existing dracon-warden IndexLock-aware patterns when staging the deletion (or commit during freeze if IndexLock can't be used). Actually: deletion of top-level files is a normal git operation, no working-tree writes involved, so IndexLock doesn't apply. But if files are being deleted by a hook, ensure no .git/index.lock contention.
- Any doc that survives deletion must end up either in docs/ (canonical) or docs/archive/ (historical). No half-deleted files.
- Sync daemon (dracon-sync) is running and will auto-commit. Either pause sync first (`dracon-sync pause`) or batch the deletion into a single commit so the auto-commit doesn't fragment the cleanup.
- Commit messages should be deterministic facts (as per dracon-sync commit format), not prose.

Verification contract:
- `git log --oneline -10` shows a clean series of commits: (1) .gitignore updates, (2) deletions, (3) restructured docs, (4) README sync-up.
- `ls /home/dracon/Dev/dracon-utilities/` matches the canonical file list above.
- `find /home/dracon/Dev/dracon-utilities -maxdepth 2 -name 'README.md'` returns exactly 4 files (root + 3 subdirs).
- `find /home/dracon/Dev/dracon-utilities -maxdepth 2 -name 'BLUEPRINT.md'` returns exactly 3 files (one per subdir).
- `ls /home/dracon/Dev/dracon-utilities/docs/` returns ROADMAP.md, ARCHITECTURE.md, OPERATIONS.md, archive/.
- Root README contains a "Documentation" section linking to docs/ROADMAP.md, docs/ARCHITECTURE.md, docs/OPERATIONS.md, and the 3 subdir READMEs.
- `grep -r "AI commit messages\|AI-generated" /home/dracon/Dev/dracon-utilities/dracon-sync/README.md` returns nothing.
- `grep -r "dracon-ai" /home/dracon/Dev/dracon-utilities/README.md` returns nothing (or only historical mention if any).
- `grep -r "TODO\|todo" /home/dracon/Dev/dracon-utilities/{todo.md,TODO.md,tasks.md}` fails (files don't exist).
- `git status` is clean at the end.

If blocked: stop and ask the user.

## Progress

- Status: running
- Auto-continue: on
- Sisyphus mode: no
- Time spent: 1m15s
- Tokens used: 651K (650,924) tokens
## Tasks

<!-- blockCompletion: false -->
- [ ] audit-current-state: Audit current docs and root clutter — contract: Output a list of: (1) all md files with line counts, (2) all top-level clutter files, (3) every fact-claim that disagrees with AGENTS.md.
- [ ] update-gitignore: Update .gitignore to prevent re-clutter — contract: .gitignore contains patterns for: pi-session-*.html, /rust_out, autoresearch.jsonl, debug.log, *.html.tmp. dracon-sync pause issued, no auto-commits during cleanup.
- [ ] create-docs-dir: Create docs/ directory structure — contract: docs/ exists with ROADMAP.md, ARCHITECTURE.md, OPERATIONS.md, archive/ subdir.
- [ ] move-archive-docs: Move dated planning docs to docs/archive/ — contract: MASTER_ROADMAP_2026-06-01.md, STUCK_PUSH_TRIAGE_2026-06-02.md, REPOS_CLEANUP_PLAN_2026-06-01.md, REFACTORING_BLOCKER_ANALYSIS.md, AUDIT_2026-05-29.md, audit.md, AUDIT.md, AUDIT_CHECKLIST.md, todo.md, TODO.md, tasks.md, SPEC.md all either deleted or moved to docs/archive/.
- [ ] remove-root-clutter: Remove non-md clutter from repo root — contract: pi-session-*.html, /rust_out, autoresearch.jsonl, debug.log removed. Repo root matches canonical list.
- [ ] rewrite-readmes: Rewrite 4 READMEs to be accurate and presentable — contract: All 4 READMEs are fact-aligned with AGENTS.md. Root README links to docs/ROADMAP.md, docs/ARCHITECTURE.md, docs/OPERATIONS.md. No mention of removed dracon-ai binary. No "AI commit messages" claim.
- [ ] polish-blueprints: Polish 3 BLUEPRINTs to reflect current state — contract: All 3 BLUEPRINTs match current behavior, no aspirational features claimed.
- [ ] final-verification: Final verification: clean tree, accurate docs, presentable repo — contract: git status clean. ls matches canonical list. All 7 verification checks from goal pass. dracon-sync resumed.

