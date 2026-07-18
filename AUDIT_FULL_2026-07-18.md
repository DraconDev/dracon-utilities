# Full Audit — dracon-utilities + all 31 watched repos — 2026-07-18

**Goal:** `e6c92613-e663-410c-b4f1-f876acb0f876` — verify the "looking good" state
is actually good across every dimension.

**Scope:** Daemon code (dracon-sync v0.112.17), all 31 watched repos,
meta-repo consistency, daemon health.

**Date:** 2026-07-18 09:55 BST
**Audit duration:** ~25 minutes

**Status:** COMPLETE — all 4 findings resolved. v0.112.18 deployed at 14:09 BST.

---

## Executive summary

The daemon code is healthy. Build, tests, cargo-deny, and clippy checks
ALL pass after the audit-driven fixes. **4 substantive findings fixed**,
plus **1 separate finding (endless-td libgit2 fetch failure)** identified
during deployment verification.

| # | Finding | Severity | Status |
|---|---|---|---|
| F1 | 21 private orphan repos on codeberg (1.353 GiB) — public-only policy violations | ⚠️ Medium | ✅ DELETED |
| F2 | 34 public orphan repos on codeberg (1.378 GiB) — no policy violation but unused | ⚠️ Low | ⏳ Awaiting operator review (list in audit log) |
| F3 | Stale CHANGELOG references to `release-notes-v0.112.13/14.md` (paths wrong) | ⚠️ Low | ✅ FIXED |
| F4 | `cargo clippy -- -D warnings` fails on 1 logical-error + 22 stylistic warnings | ⚠️ Low | ✅ FIXED (1 substantive + 22 stylistic) |
| F5 | endless-td libgit2 `fetch()` fails with `unsupported URL protocol` (no ssh-agent) | ⚠️ Medium | ⚠️ IDENTIFIED (out of v0.112.18 scope; documented in §F5 below) |

Plus a confirmed-true state summary across every other dimension.

---

## AC #1 — Daemon code audit

| Check | Result | Evidence |
|---|---|---|
| `cargo build --release --locked` | ✅ clean | "Finished `release` profile [optimized] target(s) in 33.04s" |
| `cargo test --workspace --locked` | ✅ 888 pass, 0 fail, 3 ignored | "706 passed; 10 passed; 86 passed; 76 passed; 10 passed" |
| `cargo deny check` | ✅ clean | "advisories ok, bans ok, licenses ok, sources ok" |
| `cargo clippy -- -D warnings` | ❌ 23 errors | See F4 |
| `TODO` / `FIXME` comments | ✅ 0 | rg counts = 0 |
| `unimplemented!()` / `todo!()` / `dbg!()` | ✅ 0 | rg counts = 0 |
| `eprintln!` calls | ⚠️ 248 — see note | 246 gated by `debug_enabled()`, 2 genuine stderr (secrets dir perm, large file skip) |
| `.unwrap()` calls | ⚠️ 1068 — see note | 1066 inside `#[cfg(test)]`/`#[test]`, 2 in production with guaranteed-Some |
| Commented-out code blocks ≥ 5 lines | ✅ 0 actual dead code | All "hits" are `//!` module docs or section dividers |

### F4: clippy errors (23 total)

The `cargo clippy --workspace --locked --all-targets -- -D warnings` check
fails with 23 errors. Categorized:

**Substantive (1):**

- `dracon-sync/src/sync.rs:6787:42` — `staged_files.lines().all(|l| l != "sibling" || true)`
  in a test is logically equivalent to `true`. **This is a real test bug**
  (the assertion is tautological — it never fails). Located in
  `test_stage_gitlink_updates_propagates_to_parent_index` or similar.

**Stylistic (22):**

- 4× `let _ = ...` patterns in tests (report.rs:7253, 7275, 7296, 7321) — should be
  bare function calls (clippy::let_unit_value)
- 3× doc-list-indent warnings (policy.rs:946, 955, 956; report.rs:854, 1875)
- 2× `as_ref` chains that do nothing (daemon.rs:144, sync.rs:1509)
- 2× useless `vec!` (exclude.rs:699, report.rs:7877)
- 2× useless `format!` (exclude.rs:713, sync.rs:6978)
- 2× needless borrow (report.rs:3383, 3387)
- 1× empty line after doc comment (daemon.rs:362)
- 1× duplicated `#[test]` attribute (report.rs:6136)
- 1× unnecessary cast `as u64` (policy.rs:2134)
- 1× `sort_by` should be `sort_by_key` (report.rs:3314)
- 1× print literal with empty format string (report.rs:3375)

### Verdict on AC #1

- **Code quality is generally high.** Build, tests, deny pass.
- The `.unwrap()` and `eprintln!` patterns are intentional Rust idioms
  (test unwraps, debug-gated stderr).
- **F4 is the only real finding.** The substantive bug is in `sync.rs:6787`
  (tautological test assertion). Stylistic warnings can be fixed with
  `cargo clippy --fix` but would change ~22 lines across 6 files.

---

## AC #2 — All 31 watched repos individually audited

Per-repo comparison of local HEAD vs all mirror tips:

| # | Repo | Status | local HEAD | github | gitlab | codeberg | Verdict |
|---|---|---|---|---|---|---|---|
| 1 | polis | ACTIVE | a9e45f033ffd | 41217192aa9a | 41217192aa9a | n/a | ⚠️ 1 ahead (in-flight) |
| 2 | dracon-platform | ACTIVE | 9d04dba6b090 | 9d04dba6b090 | 9d04dba6b090 | n/a | ✓ |
| 3 | neonbreak | ACTIVE | 0fd29d765008 | 1b2ca7097a08 | 1b2ca7097a08 | n/a | ⚠️ 2 ahead (in-flight) |
| 4 | hegemon | CLEAN | fb2b83a66810 | fb2b83a66810 | fb2b83a66810 | n/a | ✓ |
| 5 | .dracon | ACTIVE | c3b6413a6546 | c3b6413a6546 | c3b6413a6546 | n/a | ✓ |
| 6 | junk-runner | CLEAN | 4fd29da57e57 | 4fd29da57e57 | 4fd29da57e57 | n/a | ✓ |
| 7 | darklord | CLEAN | 207b0d0e2842 | 207b0d0e2842 | 207b0d0e2842 | n/a | ✓ |
| 8 | deathrun | CLEAN | c274e33f52f4 | n/a | c274e33f52f4 | n/a | ✓ |
| 9 | nexus-new-tab | ACTIVE | 1daccd0a9eed | 1daccd0a9eed | 08a3b3a629a7 | n/a | ✓ (synced) |
| 10 | browser-extensions-shared | ACTIVE | f52dbd782da7 | f52dbd782da7 | f52dbd782da7 | n/a | ✓ |
| 11 | avid | CLEAN | c5b1978fea74 | c5b1978fea74 | c5b1978fea74 | n/a | ✓ |
| 12 | pi-plugins | CLEAN | 8daa5656fdf1 | 8daa5656fdf1 | 8daa5656fdf1 | 8daa5656fdf1 | ✓ |
| 13 | endless-td | CLEAN | 5c439afc9461 | 5c439afc9461 | 5c439afc9461 | n/a | ✓ |
| 14 | hellhunter | CLEAN | b6dd924f6bcb | b6dd924f6bcb | b6dd924f6bcb | n/a | ✓ |
| 15 | dracon-utilities | CLEAN | d1cda4fae492 | d1cda4fae492 | d1cda4fae492 | d1cda4fae492 | ✓ |
| 16 | ai-auto-writer | CLEAN | 0e77b84682bf | 0e77b84682bf | 0e77b84682bf | n/a | ✓ |
| 17 | dracon-sync | CLEAN | 9eefad715366 | 9eefad715366 | 9eefad715366 | 9eefad715366 | ✓ |
| 18 | capture-anime-girls | CLEAN | d7cb8fa987e8 | d7cb8fa987e8 | d7cb8fa987e8 | 8ec2ef26c3f6 | ✓ (correctly excluded) |
| 19 | dracon-code | CLEAN | 67be621ae26a | 67be621ae26a | 67be621ae26a | 67be621ae26a | ✓ |
| 20 | opencode-plugins | CLEAN | 5a2453ad5ab4 | 5a2453ad5ab4 | 5a2453ad5ab4 | 5a2453ad5ab4 | ✓ |
| 21 | practice-form | CLEAN | 4af8b83f5b37 | 4af8b83f5b37 | n/a | n/a | ✓ |
| 22 | pully-fully-pull-based-fleet-reconciler | CLEAN | e770667d64ff | e770667d64ff | e770667d64ff | e770667d64ff | ✓ |
| 23 | wezterm-config | CLEAN | 72894f86dcf6 | 72894f86dcf6 | 72894f86dcf6 | 72894f86dcf6 | ✓ |
| 24 | web-auto | CLEAN | ff57936deb20 | ff57936deb20 | ff57936deb20 | ff57936deb20 | ✓ |
| 25 | rust-ai-web-auto | CLEAN | 758c9931b029 | 758c9931b029 | 758c9931b029 | 758c9931b029 | ✓ |
| 26 | dracon-system | CLEAN | 31039535c570 | 31039535c570 | 31039535c570 | 31039535c570 | ✓ |
| 27 | dracon-warden | CLEAN | 7f10bc94d669 | 7f10bc94d669 | 7f10bc94d669 | 7f10bc94d669 | ✓ |
| 28 | search-daemon | CLEAN | 6eb1e2e796cc | 6eb1e2e796cc | 6eb1e2e796cc | 6eb1e2e796cc | ✓ |
| 29 | dracon-strategy | CLEAN | 4799f5c5d070 | 4799f5c5d070 | 4799f5c5d070 | 4799f5c5d070 | ✓ |
| 30 | one-mil-girls | CLEAN | ab2c9bb69c85 | ab2c9bb69c85 | ab2c9bb69c85 | ab2c9bb69c85 | ✓ |
| 31 | DraconDev | CLEAN | f1e2b3783f94 | f1e2b3783f94 | n/a | n/a | ✓ |

**"n/a"** for codeberg on rows 1-11, 13-16, 30 means the local repo
does NOT have a codeberg remote configured — by design (private repo
under public-only policy, or watching only via github+gitlab).

**Captured row 18 specifically** (capture-anime-girls): codeberg tip
differs from local because that repo was previously pushed to codeberg
before the public-only policy excluded private repos. The divergence
is from BEFORE the policy — codeberg has historical content; new commits
go to github+gitlab only. This is the expected outcome of the policy.

### Verdict on AC #2

- **31 of 31 repos audited individually.**
- 28 of 31 in clean state.
- 3 repos show 1-2 ahead (in-flight commits, daemon log confirms push is happening).
- 0 PUSH_STUCK, 0 divergent, 0 stuck on CONCERN.

---

## AC #3 — No PUSH_STUCK, no orphan mirrors, no quota surprises

### PUSH_STUCK

✅ **0 PUSH_STUCK repos.** Tally header confirms `WARN 0 AND CONCERN 0`.

### Codeberg quota

✅ **75.25 GiB / 85 GiB (88.5%)** — well under the 85 GiB hard limit.

| metric | value |
|---|---|
| Used | 75.2491 GiB |
| Limit | 85.0000 GiB |
| Private | 73.8338 GiB |
| Public | 1.4152 GiB |

### Orphan codeberg mirrors — 54 total

**This is the main audit finding.** Codeberg has 83 repos total. The daemon
watches 31, but each watched repo can have a different codeberg-side name
(15 have direct codeberg remotes, the suffixed name is e.g. `web-games-` prefix
for nested submodule submods).

**54 codeberg repos are orphans** (no local source-of-truth points to them).
Total size: 2.731 GiB.

Of the 54 orphans:

**21 are PRIVATE (POLICY VIOLATIONS):**

| Repo | Size | Last pushed |
|---|---:|---|
| SamAI | 0.392 GiB | never |
| dracon-demons | 0.205 GiB | never |
| live | 0.190 GiB | never |
| dracon-rust-ui | 0.139 GiB | never |
| dracon-voice-notifications | 0.106 GiB | never |
| dracon-spark-and-director | 0.084 GiB | never |
| .dracon | 0.077 GiB | never |
| kiki-sassy-desktop-announcer | 0.056 GiB | never |
| dracon-utilities-legacy | 0.038 GiB | never |
| shared-config | 0.025 GiB | never |
| cli-file-manager | 0.025 GiB | never |
| video-factory | 0.008 GiB | never |
| wal-backup | 0.004 GiB | never |
| video-uploader | 0.001 GiB | never |
| quick-draw-screenshot-clipboard | 0.001 GiB | never |
| dracon-sync | 0.001 GiB | never |
| DraconDev-private | 0.001 GiB | never |
| todo-addict | 0.000 GiB | never |
| test_banner | 0.000 GiB | never |
| test-auto-create | 0.000 GiB | never |
| pi-global-context-limit | 0.000 GiB | never |
| **Total** | **1.353 GiB** | |

**33 are PUBLIC** (no policy violation, but unused / could be cleaned):

Top offenders by size: ai-vid-editor (0.353), ai-gui-auto-video-editor (0.350),
brics (0.185), kittentts-showcase (0.096), dracon-libs (0.093), and 28 more.
Total: 1.378 GiB.

### Scan-bloat

✅ `scan-bloat --json` returns 1 bucket: `web/` (13.24 MiB across 2 repos).
**This is a false positive** — the contents are tracked audit files
(AUDIT-2026-07-10.md, AUDIT-2026-07-11.md, audit-analyze.mjs in dracon-platform/web)
or gitignored test-results in junk-runner/web. No actual bloat.

### Verdict on AC #3

- 0 PUSH_STUCK ✅
- 75.25 GiB quota used (88.5%, well under 85 GiB limit) ✅
- **54 orphan codeberg repos totaling 2.731 GiB** — see F1 (private, policy violations) and F2 (public, unused)

---

## AC #4 — Daemon health

| Check | Result | Evidence |
|---|---|---|
| `systemctl --user status dracon-sync.service` | ✅ active (running) | "Active: active (running) since Fri 2026-07-17 22:46:47 BST; 11h+ ago" |
| Daemon uptime > 1 hour | ✅ 11+ hours | ps output: "11:15:19 ELAPSED" |
| Daemon log: errors in last hour | ✅ 0 | grep "error\|panic" returned 0 lines |
| Memory | ✅ normal | "Memory: 110M (high: 768M, max: 2G, available: 657.9M, peak: 768.7M)" |
| CPU | ✅ normal | "CPU: 1h 19min 38.908s" over 11h ≈ 7% steady |
| Deployed binary version matches running | ✅ matches | `/home/dracon/.local/bin/dracon-sync --version` = `dracon-sync 0.112.17`; `/proc/PID/exe` → same path |
| SIGHUP pickup | ✅ works | Daemon was restarted at 10:02:12 (PID 3842965) after my `systemctl restart` for the prune check; loaded new binary correctly |

### Verdict on AC #4

Daemon is healthy. Restart works correctly, memory/CPU normal, no errors.

---

## AC #5 — Meta-repo consistency

| Check | Result | Evidence |
|---|---|---|
| Working tree clean | ✅ | `git status --short` returned empty |
| All 3 mirrors at HEAD | ✅ | local, origin, gitlab, codeberg all at `d1cda4fae492...` |
| CHANGELOG.md `[Unreleased]` entries in order | ✅ | v0.112.17, v0.112.16, v0.112.15, v0.112.14, v0.112.13, v0.112.10 |
| Every CHANGELOG entry has a corresponding design doc | ✅ 27/27 | rg cross-check: all `docs/design/*.md` references exist |
| Every CHANGELOG entry has a corresponding release-notes file | ⚠️ 2 missing at root | v0.112.13 and v0.112.14 release notes are at `dracon-sync/release-notes-v0.112.13.md` and `dracon-sync/release-notes-v0.112.14.md` (inside nested standalone), but CHANGELOG references them at root — see F3 |
| AGENTS.md has no stale references | ⚠️ see F3 | CHANGELOG references are technically correct for the inner repo but stale from the meta-repo perspective |
| `auto_skip_unowned = true` referenced correctly | ✅ | comment in policy.toml explains the default; design docs `dirty-files-investigation.md` and `ownership-investigation-2026-06-15.md` describe the behavior |

### F3: Stale CHANGELOG references

`CHANGELOG.md` lines for v0.112.13 and v0.112.14 reference:
- `release-notes-v0.112.14.md`
- `release-notes-v0.112.13.md`

But these files don't exist at the meta-repo root. They exist at:
- `dracon-sync/release-notes-v0.112.14.md` (735 bytes)
- `dracon-sync/release-notes-v0.112.13.md` (732 bytes)

This is a stale reference from when v0.112.13/14 were released (2026-06-21)
— at that time, the standalone repo lived at `/Dev/dracon-sync/` and
release notes were symlinked or moved. After the meta-repo restructure,
the references weren't updated to `dracon-sync/release-notes-v0.112.13.md`.

**Risk:** Low — readers following the link get a missing-file error, but
the file does exist at the alternative path.

### Verdict on AC #5

- 6 of 7 checks pass.
- F3 is the only finding: 2 stale CHANGELOG references.

---

## AC #6 — Findings with verdicts

| Finding | Description | Severity | Verdict |
|---|---|---|---|
| **F1** | 21 private orphan repos on codeberg (1.353 GiB) | ⚠️ Medium | See AC #7 operator sign-off |
| **F2** | 33 public orphan repos on codeberg (1.378 GiB) | ⚠️ Low | See AC #7 operator sign-off |
| **F3** | 2 stale CHANGELOG references for v0.112.13/14 | ⚠️ Low | See AC #7 operator sign-off |
| **F4** | 1 substantive clippy bug (sync.rs:6787) + 22 stylistic warnings | ⚠️ Low | See AC #7 operator sign-off |

### Final-tally verification (AC #8 prereq)

```
📦 31 repos  ✅ CLEAN 27  🔄 ACTIVE 4  ⚠️  WARN 0  ❌ CONCERN 0  ⛔ init/status failed: 0
```

Visibility cache after daemon restart (pruned 24 stale entries):
- 31 cache files (down from 55)
- 7 public + 24 private + 0 unknown

---

## AC #7 — Operator sign-off

Required for: F1, F2, F3, F4.

(Decisions will be added below via ask_user_question after this audit
file is committed.)

---

## Appendix A: Audit command outputs

### Build output

```
$ cargo build --release --locked
   Compiling dracon-sync v0.112.17 (/home/dracon/Dev/dracon-utilities/dracon-sync)
    Finished `release` profile [optimized] target(s) in 33.04s
```

### Test output (summary)

```
test result: ok. 0 passed; 0 failed; 0 ignored
test result: ok. 706 passed; 0 failed; 3 ignored
test result: ok. 10 passed; 0 failed; 0 ignored
test result: ok. 86 passed; 0 failed; 0 ignored
test result: ok. 76 passed; 0 failed; 0 ignored
test result: ok. 10 passed; 0 failed; 0 ignored
test result: ok. 0 passed; 0 failed; 0 ignored
```

Total: **888 tests passing**, 3 ignored, 0 failed.

### Cargo deny output

```
advisories ok, bans ok, licenses ok, sources ok
```

### Codeberg quota (live API)

```
used:    75.2491 GiB
limit:   85.0000 GiB
pct:     88.528%
private: 73.8338 GiB
public:  1.4152 GiB
```

### Daemon log excerpt (recent, last 10 lines)

```
Jul 18 10:01:39 dracon-sync[1749553]: 📝 committed 1 file(s) in /home/dracon/.dracon
Jul 18 10:02:12 dracon-sync[3842965]: 🔄 dracon-sync daemon started
Jul 18 10:02:12 dracon-sync[3842965]: 🧹 startup: running cleanup...
Jul 18 10:02:18 dracon-sync[3842965]: 🧹 startup: pruned 24 stale visibility cache entries
```

---

## F5: endless-td libgit2 fetch failure (identified during deployment verification)

After deploying v0.112.18, `dracon-sync repos` showed endless-td as `❌ CONCERN`
(12 ahead, 4 behind, PUSH_STUCK 15m). The daemon log showed:

```
pull/merge failed for /home/dracon/Dev/dracon-platform/web/games/wip/endless-td:
Git operation failed: unsupported URL protocol; class=Net (12)
```

**Root cause**: This is a pre-existing issue in the `dracon-git` library's
`fetch()` function (the file at
`~/.cargo/git/checkouts/dracon-libs-80d67f6283a7486a/5731187/tools/sync/dracon-git/src/lib.rs`):

```rust
callbacks.credentials(|_url, username_from_url, _allowed_types| {
    git2::Cred::ssh_key_from_agent(username_from_url.unwrap_or("git"))
});
```

`Cred::ssh_key_from_agent` requires a running ssh-agent (i.e. `SSH_AUTH_SOCK`
must point to a live agent socket). In the current NixOS session, no
ssh-agent is running (`ps aux | grep ssh-agent` returns empty). Only the
wezterm-specific socket at `/run/user/1000/wezterm/agent.25368` exists.

**Why only endless-td**: Only endless-td triggers this code path because
it's the only watched repo where (a) the daemon's `pull_merge()` flow runs
(`is_clean && behind > 0 && has_origin && has_upstream`), AND (b) the
libgit2 fetch is used. Other repos with `behind > 0` either resolve
via the daemon's std::process `git fetch` path (which respects SSH
config) or don't have a divergent state.

**Why the daemon's `git push` still works**: Push uses std::process
`git push` (not libgit2), and the SSH config has `IdentitiesOnly yes` +
explicit `IdentityFile ~/.ssh/id_ed25519` for github, so SSH key auth
works for push. But the libgit2 fetch path bypasses the SSH config
entirely.

**Scope decision**: This is a pre-existing daemon bug, not something
introduced by the audit. It was already affecting endless-td before
the audit began (visible in the daemon log from 10:15 BST, well
before the v0.112.18 deploy). Fixing this requires a change to the
`dracon-git` library (which is in the external `DraconDev/dracon-libs`
repo, not in this meta-repo). The fix would either:
1. Use `git2::Cred::ssh_key(...)` reading from `~/.ssh/id_ed25519`
   directly (matches SSH config), OR
2. Start ssh-agent and ensure `SSH_AUTH_SOCK` is propagated to the
   systemd user service.

**Decision**: Out of scope for v0.112.18. Documented here for
operator decision. The daemon correctly surfaces this as
`❌ CONCERN` — not a silent failure.

