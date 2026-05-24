# Docs Cleanup — Dracon Utilities

## Goal
Clean up the markdown file mess in dracon-utilities repo. Consolidate, archive, or delete redundant documentation.

## Current State
13 markdown files, some duplicates/outdated:
- `TODO.md` (capital) — Main todo with audit tasks ✓ KEEP
- `todo.md` (lowercase) — Old todo, different content ❌ ARCHIVE
- `TODO-todo-context.md` — Another duplicate ❌ ARCHIVE
- `AUDIT.md` — Audit results ✓ KEEP
- `dracon-sync-architecture.md` — Architecture spec ✓ KEEP
- `SPEC.md` — Simple spec file ✓ KEEP
- `AGENTS.md` — Project instructions ✓ KEEP
- `README.md` — Main readme ✓ KEEP
- `CONTRIBUTING.md` — Contributing guide ✓ KEEP
- `CHANGELOG.md` — Changelog (keep if active)
- `OPEN_QUESTIONS.md` — Likely outdated ❌ ARCHIVE
- `UTILITY_BOUNDARIES.md` — Likely outdated ❌ ARCHIVE
- `autoresearch.ideas.md` — Ideas, not actionable ❌ ARCHIVE

## Tasks

### 1. Create ARCHIVE directory
- Create `ARCHIVE/` directory in repo root
- Move redundant docs there

### 2. Archive redundant files
Move to `ARCHIVE/`:
- `todo.md`
- `TODO-todo-context.md`
- `OPEN_QUESTIONS.md`
- `UTILITY_BOUNDARIES.md`
- `autoresearch.ideas.md`

### 3. Review CHANGELOG.md
- Keep if project has active releases
- Archive if stale/irrelevant

### 4. Verify README.md is up-to-date
- Check that README reflects current state
- Update if needed

### 5. Commit changes
- Commit ARCHIVE directory creation
- Update TODO.md to remove outdated entries

### 6. Push to verify
- Push commits to remote
- Verify no broken links in remaining docs

## Checklist
- [ ] Create ARCHIVE directory
- [ ] Move redundant files to ARCHIVE
- [ ] Review CHANGELOG.md
- [ ] Verify README.md
- [ ] Commit changes
- [ ] Push and verify