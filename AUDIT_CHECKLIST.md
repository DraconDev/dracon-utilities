# Dracon Utilities Audit Checklist

Date: 2026-05-30

## Project Overview

This is a systematic audit of the dracon-utilities project, focusing on:
- Commit message format and determinism
- Code quality and tests
- Documentation accuracy
- Edge cases and bug fixes

---

## 1. Commit Message Format

### 1.1 Core Metrics (Must Have)

- [ ] **CLOSED:** Task completion detected from `[x]` markers
  - [ ] Works with `- [x]` (markdown)
  - [ ] Works with `* [x]` (markdown)
  - [ ] Works with `[x]` (plain text)
  - [ ] Strips `**bold**` markers
  - [ ] Strips backticks `` ` ``
  - [ ] Strips `[`, `]`, `|`
  - [ ] Multiple tasks joined with comma

- [ ] **WIP:** Task in-progress detected from `[~]` markers
  - [ ] Same sanitization as CLOSED

- [ ] **FILES:N** Total file count from `git diff --numstat`
  - [ ] Binary files counted (shown as `-`)

- [ ] **DIRS:X,Y** Top-level directories touched
  - [ ] BTreeSet for uniqueness
  - [ ] Limited to top 3

- [ ] **[file1, file2]** Top 3 changed files
  - [ ] Sorted by lines changed
  - [ ] Limited to 3 files

- [ ] **DELTA:+A/-B** Lines added/removed
  - [ ] Correct i64 parsing
  - [ ] Handles binary files (`-`)

### 1.2 Quality Metrics

- [ ] **TEST:T** Lines changed in test files
  - [ ] `is_test_file()` detects test patterns
  - [ ] Works for Rust, Python, JS/TS

- [ ] **BIN:B** Binary files changed
  - [ ] Detected from numstat `-` marker

- [ ] **TESTONLY:** All changes are test files
  - [ ] Triggers when 100% of files are tests

### 1.3 Ecosystem Metrics

- [ ] **NEW:file1,file2** Newly created files
  - [ ] From `git diff --name-status`
  - [ ] Excludes `.lock` files
  - [ ] Excludes `node_modules`
  - [ ] Limited to top 3 + `+Nmore`

- [ ] **DEL:file1,file2** Deleted files
  - [ ] Same exclusions as NEW
  - [ ] Limited to top 3 + `+Nmore`

- [ ] **DEPS:+dep1,-dep2** Dependency changes
  - [ ] Parses Cargo.toml (Rust)
  - [ ] Parses package.json (Node)
  - [ ] Parses requirements.txt (Python)
  - [ ] Parses go.mod (Go)
  - [ ] Skips version bumps (no actual deps)
  - [ ] Limited to top 5 + `+Nmore`

- [ ] **ENV:** Env files changed
  - [ ] Detects `.env`, `.env.*`, `.envrc`
  - [ ] Detects `.secrets`, `secrets.*`

### 1.4 Context Metrics

- [ ] **MERGE:** Merge commits
  - [ ] Detects `.git/MERGE_HEAD`

- [ ] **REVERT:** Revert commits
  - [ ] Detects `.git/REVERT_HEAD`

- [ ] **TAG:v1.0.0** Release tags
  - [ ] Uses `git describe --tags --exact-match`
  - [ ] Falls back to `git tag --points-at HEAD`

---

## 2. Code Quality

### 2.1 Tests

- [ ] All 398 tests pass
  - [ ] `--test-threads=1` (reliable)
  - [ ] `--test-threads=N` (parallel, may have flakiness)

### 2.2 Linting

- [x] `cargo clippy` passes with no warnings
  - [x] No `unwrap()` in production code
  - [x] No unnecessary clones
  - [x] Correct error handling

### 2.3 Build

- [ ] Debug build succeeds
- [ ] Release build succeeds
- [ ] Binary size reasonable (~10MB)

---

## 3. Documentation

### 3.1 AGENTS.md

- [ ] Core principle: "No AI at commit boundary"
- [ ] All metrics documented
- [ ] Examples are accurate
- [ ] `git log --grep=` commands work

### 3.2 README.md

- [ ] dracon-sync description updated
- [ ] Mentions deterministic (not AI-generated) messages
- [ ] Commit format examples included

### 3.3 Inline Comments

- [ ] `compute_blast_radius` has clear docs
- [ ] `sanitize_task_name` handles edge cases
- [ ] `detect_dependency_changes` lists supported formats

---

## 4. Edge Cases

### 4.1 Empty/Edge Cases

- [ ] Empty diff returns `0 file(s) DELTA:+0/-0`
- [ ] No staged files handled gracefully
- [ ] Binary file with no text changes works

### 4.2 Character Handling

- [ ] Unicode task names work
- [ ] Very long task names don't break format
- [ ] Special characters in paths handled

### 4.3 File Limits

- [ ] Many new files → top 3 + `+Nmore`
- [ ] Many deleted files → top 3 + `+Nmore`
- [ ] Many deps → top 5 + `+Nmore`
- [ ] Many tasks → all included (no limit)

---

## 5. Daemon Behavior

### 5.1 Timing

- [ ] `pulse_interval_secs = 1` (1 second scan)
- [ ] `inactivity_push_delay_secs = 5` (5 second commit delay)

### 5.2 Reliability

- [ ] Auto-restart on crash
- [ ] Logs incidents to `~/.local/state/dracon/dracon-sync-incidents.jsonl`
- [ ] Graceful handling of git errors

---

## 6. Deployment

### 6.1 Install Script

- [ ] Builds all binaries
- [ ] Installs to `~/.local/bin/`
- [ ] Sets up systemd services
- [ ] Restarts services

### 6.2 Service Files

- [ ] dracon-sync.service running
- [ ] dracon-system-guard.service running
- [ ] dracon-warden.service running

---

## 7. Cross-Repo Verification

### 7.1 New Format Commits

Check these repos have proper format:
- [ ] dracon-utilities
- [ ] dracon-platform
- [ ] browser-extensions-shared
- [ ] avid (with CLOSED: task names)

### 7.2 Old Format Commits

These are historical, not bugs:
- [ ] dracon-demons (old commits)
- [ ] dracon-code (some old commits)
- [ ] dracon-libs (some old commits)

---

## 8. Known Issues

### 8.1 Resolved

- [x] Backticks in task names → fixed (strip `` ` ``)
- [x] `**bold**` not stripped → fixed (extract identifier)
- [x] Version bumps showing DEPS: → fixed (skip if no actual deps)
- [ ] Long task names → not limiting (by design)
- [ ] avid old commits have backticks → will be clean going forward

### 8.2 Open Questions

- [ ] Should task list be limited? (current: no limit)
- [ ] Should we detect I18N changes?
- [ ] Should we detect SCHEMA (migration) changes?

---

## Sign-Off

- [ ] All checkboxes checked
- [ ] Tests pass
- [ ] Documentation updated
- [ ] Deployed and running

Date: _____________  Auditor: _____________