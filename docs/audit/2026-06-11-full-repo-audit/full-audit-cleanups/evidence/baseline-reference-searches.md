# Baseline reference searches

## timestamp
2026-06-11T20:07:37+01:00

## TODO/FIXME/HACK in tracked current files
./flake.nix:45:        # if the nixpkgs version is compatible.
./todo.md:1:# Dracon Utilities — TODO
./todo.md:6:- [x] Mass deletion guard: REMOVED entirely — IndexLock fixes root cause (clone race). Git revert is the safety net. Prometheus counter kept as always-0 for compat.
./.ralph/todo-sprint.md:1:# Dracon Utilities TODO Sprint
./.ralph/todo-sprint.md:53:- Backward compatibility maintained via `pub(crate) use submodule::*;` re-exports
./AUDIT.md:58:  - **However**, `dracon-sync/src/simple_ai.rs` (the OpenAI-compatible HTTP client, provider health tracking, prompt sanitization against injection) is **not** feature-gated and is compiled into every build. The file is 14 KB and pulls in `reqwest` (already a dep) plus the `simple_ai` module surface.
./AUDIT.md:87:- **Impact:** No runtime impact today; future edition bump will need this code to remain correct, so leaving it is also fine. Either remove the `unsafe` for now or leave it as a forward-compat signal.
./AUDIT.md:231:- **Fix:** Either (a) add a `test_helpers` module to `dracon-system` with a no-op `EnvRestorer` stub for forward-compat, or (b) qualify the AGENTS.md sentence to "in `dracon-sync` and `dracon-warden`" (since `dracon-system` has no env mutations).
./AUDIT.md:269:### 2.8 P3 — `git.rs.test` and `tests.rs.plaintext` are empty placeholder files
./AUDIT.md:355:### 4.4 P3 — `AGENTS.md` does not document the version-2024 forward-compat of the `unsafe { std::env::set_var }` pattern
./AUDIT.md:517:4. **Delete the empty placeholder files** `git.rs.test` and `tests.rs.platintext` (finding 2.8).
./AUDIT.md:685:- ✅ `dracon-warden` deprecated `watch_roots` alias is still accepted for backwards compat (CHANGELOG 0.3.0).
./scripts/verify-spec.sh:22:# Invariant 2: No blocking TODO comments
./scripts/verify-spec.sh:23:echo "--- Invariant 2: No blocking TODO comments ---"
./scripts/verify-spec.sh:24:if grep -r "FIXME:\|BLOCKING:" dracon-*/src/ --include="*.rs" 2>/dev/null; then
./scripts/verify-spec.sh:25:  echo "FAIL: Found FIXME: or BLOCKING: comments"
./scripts/verify-spec.sh:28:  echo "PASS: No blocking TODO comments"
./dracon-system/BLUEPRINT.md:112:- Main.rs re-exports everything via `mod policy; pub(crate) use policy::*;` for backward compatibility
./UTILITY_BOUNDARIES.md:36:  - Owns setup symlink reconciliation via explicit `[links]` policy in `/home/dracon/dracon/utilities/system/dracon-system.toml` (default: no legacy compatibility links and no `~/.config/dracon` linkage).
./AGENTS.md:834:- `cargo tree -d` may still show transitive duplicate versions after `cargo deny check` passes. Do not force-align transitive crates unless a compatible direct dependency upgrade removes the duplicate without changing behavior.
./CHANGELOG.md:54:- **`watch_roots` is still accepted** for backwards compatibility. When
./CHANGELOG.md:304:- **MINOR**: New features, backward compatible
./docs/public-release-plan.md:192:- Replace token-like examples with obviously synthetic placeholders.
./docs/ROADMAP.md:41:| `tasks.md` / `TODO.md` | Superseded by pi goals and current task workflow; no root TODO file is canonical. |
./dracon-sync/dracon-sync.example.toml:124:# The {repo} placeholder is replaced with the repo directory name.
./.dracon/project-state.md:4:TODO sprint — iteration 3: events + links modules extracted from system/main.rs
./.dracon/secret-audit-report.md:12:> - The credentials found (Paddle sandbox keys, OAuth client secrets, iDrive S3 keys, database URLs) are **placeholder/test values** used during development of the in-progress projects.
./.dracon/secret-audit-report.md:89:All values are placeholders for in-development private projects. No rotation, history cleanup, or pre-commit hooks needed.
./.dracon/audit-cli.md:103:**Note:** The `repair-concerns`, `repair-warns`, `stuck`, `dual-branch`, `repair-origins` top-level commands appear redundant with `repair` subcommand structure but are kept for backward compatibility.
./dracon-sync/src/policy.rs:845:                "remote[{}] '{}': push_url '{}' has no {{repo}} or {{account}} placeholder — repo names will not be substituted",
./dracon-sync/src/cooldown.rs:5://! that was never adopted. Kept as a placeholder for future consolidation.
./dracon-warden/BLUEPRINT.md:117:- **Backwards compat:** The old key works in 0.2.0 and emits a warning;
./docs/audit/audit-2026-06-06-full.md:654:`scripts/verify-spec.sh` checks 3 invariants: project compiles, no FIXMEs/BLOCKINGs, unit tests pass. **F-9.7.1 [P2] Uses `cargo test --lib`** which only works for library crates. This is a binary crate workspace, so the check would fail:
./docs/audit/audit-2026-06-06-full.md:665:The `verifyspec` script in `scripts/` checks for FIXMEs but uses pattern `FIXME:\|BLOCKING:` (with colons). The actual codebase uses different patterns (e.g., the audit found `TODO sprint — iteration 3: ...` in `.dracon/project-state.md`, which is a state-tracking note, not a code TODO). **Not a finding** — the check is intentionally narrow.
./dracon-warden/dracon-warden.example.toml:28:# The `watch_roots` key is still accepted for backwards compatibility but
./dracon-warden/src/main.rs:289:    /// compatibility; will be removed in a future release. When set
./dracon-warden/src/tests.rs:272:        fs::write(&secret, "AGE-SECRET-KEY-1XXXX").expect("write");
./dracon-warden/src/tests.rs:710:        // resolves correctly (backwards compat) AND emits a deprecation warning.
./dracon-warden/src/tests.rs:724:        // Effective roots still includes p1 (backwards compat)
./dracon-warden/src/security/src/lib.rs:825:    /// WARNING: This format uses a deterministic IV derived from the key (SHA-256 hash → first 16 bytes), which violates AES-CFB security requirements. Using the same IV for multiple encryptions leaks information about plaintext relationships. This format exists for backward compatibility with legacy git-seal ciphertexts. DO NOT use this for new encryptions. If you have ciphertexts created with this format, consider migrating to AES-256-GCM (encrypt_with_repo_key) with random nonces.

## stale dracon-ai CLI references
./UTILITY_BOUNDARIES.md:63:- `dracon-ai` was removed from this repo as an orphaned CLI wrapper; AI runtime crates remain in `dracon-libs`.
./Cargo.toml:29:dracon-ai-runtime-contracts = { path = "../dracon-libs/contracts/crates/ai/dracon-ai-runtime-contracts" }
./docs/public-readiness.md:72:- Former `dracon-ai/` CLI wrapper removed from this repo; validate `dracon-libs` AI runtime crates separately when touched.
./docs/public-release-plan.md:227:Former `dracon-ai/` CLI wrapper removed from this repo; validate `dracon-libs` AI runtime crates separately when touched.
./docs/public-release-branch/PUBLIC_RELEASE_PREP.md:157:Former `dracon-ai/` CLI wrapper removed from this repo; validate `dracon-libs` AI runtime crates separately when touched.
./AGENTS.md:35:**Workspace policy:** the root Cargo workspace intentionally includes `dracon-sync`, `dracon-system`, and `dracon-warden` only. The former `dracon-ai/` CLI wrapper was removed from this repo; AI runtime crates live in `dracon-libs` and are validated with that sibling workspace.
./AGENTS.md:848:- `dracon-ai` standalone: removed from this repo; validate `dracon-libs` AI runtime crates separately when touched.
./.dracon/demon-migration-audit.md:19:| dracon-ai-lib | 0 refs | 0 refs | ✅ Clean |
./.dracon/secret-audit-report.md:78:| dracon-ai-lib | ✅ Clean (.env encrypted with dracon-warden) |

## scribe/simple_ai references
./CONTRIBUTING.md:14:4. **Describe your changes** — Include a clear PR description explaining *what* changed and *why*.
./dracon-sync/BLUEPRINT.md:183:extracted from the diff (see AGENTS.md § Commit Messages). No scribe, no
./dracon-sync/README.md:270:LLM-scribed commit messages were removed — they hallucinated context and the AI reads the diff anyway. Mechanical facts are searchable (`git log --grep="JWT"`), honest, and compact.
./AUDIT.md:20:- **Doc drift:** AGENTS.md test counts are stale in every category; AGENTS.md also says "AI scribe was removed" but `dracon-sync/src/scribe.rs` and `simple_ai.rs` are still wired and callable behind a Cargo feature.
./AUDIT.md:49:### 1.2 P1 — `simple_ai.rs` is compiled into every default build of `dracon-sync` despite "AI scribe removed" claim
./AUDIT.md:52:- **CHANGELOG.md 0.112.0 "Scribe refactor" entry** says: "Removed `scribe_update()` and `stage_project_state()` — replaced by direct commit message generation … `project-state.md` is now manual-only: sync no longer auto-generates, stages, or commits it."
./AUDIT.md:54:  - `dracon-sync/src/scribe.rs:212` and `dracon-sync/src/scribe.rs:274` define `pub(crate) async fn generate_commit_message()`.
./AUDIT.md:55:  - The first impl (line 212) is gated on `#[cfg(feature = "scribe")]` and **calls `SimpleAiService::new().chat(messages).await`**, which posts to `{provider.endpoint}/chat/completions` (`dracon-sync/src/simple_ai.rs:285`).
./AUDIT.md:56:  - The second impl (line 274) under `#[cfg(not(feature = "scribe"))]` returns `None`.
./AUDIT.md:57:  - `dracon-sync/Cargo.toml` declares `[features] default = []` — the `scribe` feature is **not** in default features. So default builds do not link the LLM path.
./AUDIT.md:58:  - **However**, `dracon-sync/src/simple_ai.rs` (the OpenAI-compatible HTTP client, provider health tracking, prompt sanitization against injection) is **not** feature-gated and is compiled into every build. The file is 14 KB and pulls in `reqwest` (already a dep) plus the `simple_ai` module surface.
./AUDIT.md:60:  1. **AGENTS.md drift** — the doc says "No LLM at the commit boundary" but the LLM-calling code is still in the tree and is reachable via `cargo build --features scribe`.
./AUDIT.md:62:  3. **Risk of regression** — if `scribe` is re-enabled in `default`, the AI path silently activates with no AGENTS.md warning.
./AUDIT.md:64:  - **Option A (remove):** delete `scribe.rs`, `simple_ai.rs`, and the `reqwest` features that only they use. Update AGENTS.md to say the LLM client is gone.
./AUDIT.md:65:  - **Option B (keep, document):** keep the code, gate `simple_ai.rs` behind `#[cfg(feature = "scribe")]`, and update AGENTS.md to acknowledge that an LLM-scribe path is available behind the feature flag.
./AUDIT.md:225:### 2.3 P3 — `dracon-system` has no `test_helpers` module while AGENTS.md prescribes one
./AUDIT.md:322:- **Fix:** Change line 35 to `cargo test --workspace -- --test-threads=1` (drop `--bins`). This is the same command AGENTS.md § "Testing" prescribes.
./AUDIT.md:349:### 4.3 P2 — AGENTS.md says "AI scribe was removed" but the code path still exists (see finding 1.2)
./AUDIT.md:351:The wording in AGENTS.md § "What sync doesn't need" and the § "Commit Messages" heading both imply the AI scribe is gone. The CHANGELOG 0.112.0 "Scribe refactor" entry reinforces this. The reality: `scribe.rs` + `simple_ai.rs` are still compiled and the LLM call site is reachable behind a feature flag.
./AUDIT.md:359:### 4.5 P3 — `AGENTS.md § "What This Is NOT"` correctly notes scribe is removed, contradicting the "What sync provides" / "Design Philosophy" sections
./AUDIT.md:362:- **AGENTS.md line 651:** "NOT AI-scribed messages (removed — they were useless for AI workflows)".
./AUDIT.md:363:- **But:** `scribe.rs` and `simple_ai.rs` exist and are wired. The doc is internally consistent *about the intent*, but the code contradicts the intent.
./AUDIT.md:439:- **Fix:** Add a length cap and a clause-count cap to `generate_commit_message` (the local-fallback in `scribe.rs` and the `n file(s) in DIRS` regex path). Example: "truncate each clause to 50 chars; max 3 clauses; if more, replace with `+N more`".
./AUDIT.md:459:### 6.5 ✅ No "scribe_update" or "stage_project_state" calls in production code paths
./AUDIT.md:462:- `rg 'scribe_update|stage_project_state' /home/dracon/Dev/dracon-utilities/dracon-sync/src/` returns 0 matches.
./AUDIT.md:498:| No AI at the commit boundary | ⚠️ PARTIAL — true for default build; `scribe` feature still wires `SimpleAiService::chat()` | finding 1.2 |
./AUDIT.md:527:10. **Resolve the AI scribe contradiction** (findings 1.2, 4.3, 4.5). Either delete `scribe.rs` + `simple_ai.rs` entirely (recommended — aligns with CHANGELOG and AGENTS.md "removed" claims), or feature-gate `simple_ai.rs` and update AGENTS.md to acknowledge the feature flag.
./AUDIT.md:697:- Removed stale/dead artifacts: `dracon-warden/dracon-warden.service`, `dracon-sync/src/scribe.rs`, `dracon-sync/src/simple_ai.rs`, `dracon-sync/src/git.rs.test`, `dracon-warden/src/tests.rs.plaintext`, the tracked `pi-session-*.html`, and tracked `rust_out`.
./dracon-warden/src/main.rs:826:    // machine_nixos identity, so docs must describe the ambiguity instead of
./AGENTS.md:798:- NOT AI-scribed messages (removed — they were useless)
./dracon-sync/src/sync.rs:1497:/// Uses `git describe --tags --always --exact-match` for exact tag match.
./dracon-sync/src/sync.rs:1502:        &["describe", "--tags", "--always", "--exact-match"],
./docs/audit/audit-2026-06-07-delta.md:49:| P1-3 | P1 | `dracon-sync/BLUEPRINT.md` "AI Integration" contradictory section | ✅ **RESOLVED** | Section rewritten as "Deterministic Commit Protocol" (line 178-188); no `scribe`/`ai-bumper` features |
./docs/audit/audit-2026-06-06-full.md:519:Line 281:   - [x] AI scribe removed                            ✅ (and the section above contradicts this — see below)
./docs/audit/audit-2026-06-06-full.md:522:**F-7.2.1 [P1] Lines 180–189 of `dracon-sync/BLUEPRINT.md` describe an "AI Integration (Scribe + AI Bumper)" section that contradicts line 281.** Per v2 audit finding P1-3, the BLUEPRINT still has a "Features (compile-time)" block describing `scribe` and `ai-bumper` that the code doesn't have. **Remediation:** delete the contradictory section. **Effort:** 5 min.
./docs/audit/audit-2026-06-06.md:41:This contradicts the v1 audit's positive finding "Deterministic commit messages — No LLM at the commit boundary" — if AI scribe was removed, why does `test-ai` still appear in docs?
./docs/audit/audit-2026-06-06.md:74:- "dracon-sync has integrated AI for generating commit messages (scribe)"
./docs/audit/audit-2026-06-06.md:78:But line 281 says: `[x] AI scribe removed (was not useful for AI workflows)`
./docs/audit/audit-2026-06-06.md:80:And `AGENTS.md` states: "AI scribe was removed as AI-generated messages were not useful for AI workflows."
./docs/audit/audit-2026-06-06.md:82:**Fix:** Delete the "AI Integration (Scribe + AI Bumper)" section (lines 180-189) and the related "Features (compile-time)" entries for `scribe` and `ai-bumper`. Keep the Status item that documents the removal.
./CHANGELOG.md:140:  - Removed `scribe_update()` and `stage_project_state()` — replaced by direct commit message generation
./CHANGELOG.md:246:- **dracon-sync**: Version bumper prevents double-bump when both `scribe` and `ai-bumper` features enabled
./CHANGELOG.md:289:  - AI-powered commit messages (scribe)
./CHANGELOG.md:290:  - Version bumping (ai-bumper)
./.pi/goals/archived/goal_2026060601095280_mq1krsgc-wjnshs.md:4:  "objective": "=== Goal ===\nObjective: Make the dracon-utilities repo presentable: fix outdated/misleading content in READMEs and BLUEPRINTs, remove obvious clutter from the repo root, restructure scattered docs into a clean docs/ tree, and add a top-level ROADMAP pointing to canonical documentation.\n\nSuccess criteria:\n- All 4 READMEs (root, sync, system, warden) describe only the binaries that exist and use the same facts as AGENTS.md (no \"AI commit messages\" claim, no mention of removed `dracon-ai` binary).\n- All 3 BLUEPRINTs (sync, system, warden) reflect current behavior, not aspirational.\n- A `docs/` directory exists with: `ROADMAP.md`, `ARCHITECTURE.md` (merging `dracon-sync-architecture.md`), `OPERATIONS.md` (merging the operator-facing parts of dated plans), and `archive/` for kept historical docs.\n- Repo root contains only: README.md, CHANGELOG.md, CONTRIBUTING.md, AGENTS.md, LICENSE, Cargo.toml, Cargo.lock, install.sh, uninstall.sh, deny.toml, clippy.toml, rustfmt.toml, rust-toolchain.toml, tarpaulin.toml, flake.lock, flake.nix, scripts/, target/, the 3 binary dirs, and the existing .dracon/ + .pi/ + .gitignore/.gitattributes.\n- Clutter removed (no `pi-session-*.html`, no `rust_out`, no `autoresearch.jsonl`, no `debug.log`, no `SPEC.md`, no `dracon-sync-architecture.md` at root, no `todo.md`/`TODO.md`/`tasks.md` duplicates, no `audit.md`/`AUDIT.md`/`AUDIT_2026-05-29.md`/`AUDIT_CHECKLIST.md`).\n- Dated one-shot reports (`MASTER_ROADMAP_2026-06-01.md`, `STUCK_PUSH_TRIAGE_2026-06-02.md`, `REPOS_CLEANUP_PLAN_2026-06-01.md`, `REFACTORING_BLOCKER_ANALYSIS.md`) are either deleted or moved to `docs/archive/`.\n- A `docs/ROADMAP.md` exists that lists what each doc is for and what superseded.\n- A `docs/ARCHITECTURE.md` exists that replaces `dracon-sync-architecture.md` (or merges its content).\n- A single canonical \"where things are\" pointer in the root README.\n\nBoundaries:\n- In scope: markdown docs (READMEs, BLUEPRINTs, audit/plan/todo/archive), repo-root file removal, .gitignore updates to prevent re-clutter, restructuring into docs/.\n- In scope: fixing content drift between docs (root README says \"dracon-ai\", subdir doesn't exist; sync README says \"AI commit messages\", AGENTS.md says removed).\n- Out of scope: code changes, Cargo.toml restructuring, behavior changes, build system changes, AGENTS.md content (treat as source of truth for what binaries/features exist).\n- Out of scope: dependencies on external tools (no need to run/install anything beyond `ls`/`grep`/`git`).\n- Out of scope: sub-binary BLUEPRINTs beyond surface-level polish (no rewrites of their internals).\n\nConstraints:\n- No force-pushes. Changes go in a normal commit per logical step.\n- .gitignore must be updated to prevent re-clutter (autoresearch.jsonl, debug.log, pi-session-*.html, /rust_out must be ignored) BEFORE the deletion commit so daemon never re-commits them.\n- Use the existing dracon-warden IndexLock-aware patterns when staging the deletion (or commit during freeze if IndexLock can't be used). Actually: deletion of top-level files is a normal git operation, no working-tree writes involved, so IndexLock doesn't apply. But if files are being deleted by a hook, ensure no .git/index.lock contention.\n- Any doc that survives deletion must end up either in docs/ (canonical) or docs/archive/ (historical). No half-deleted files.\n- Sync daemon (dracon-sync) is running and will auto-commit. Either pause sync first (`dracon-sync pause`) or batch the deletion into a single commit so the auto-commit doesn't fragment the cleanup.\n- Commit messages should be deterministic facts (as per dracon-sync commit format), not prose.\n\nVerification contract:\n- `git log --oneline -10` shows a clean series of commits: (1) .gitignore updates, (2) deletions, (3) restructured docs, (4) README sync-up.\n- `ls /home/dracon/Dev/dracon-utilities/` matches the canonical file list above.\n- `find /home/dracon/Dev/dracon-utilities -maxdepth 2 -name 'README.md'` returns exactly 4 files (root + 3 subdirs).\n- `find /home/dracon/Dev/dracon-utilities -maxdepth 2 -name 'BLUEPRINT.md'` returns exactly 3 files (one per subdir).\n- `ls /home/dracon/Dev/dracon-utilities/docs/` returns ROADMAP.md, ARCHITECTURE.md, OPERATIONS.md, archive/.\n- Root README contains a \"Documentation\" section linking to docs/ROADMAP.md, docs/ARCHITECTURE.md, docs/OPERATIONS.md, and the 3 subdir READMEs.\n- `grep -r \"AI commit messages\\|AI-generated\" /home/dracon/Dev/dracon-utilities/dracon-sync/README.md` returns nothing.\n- `grep -r \"dracon-ai\" /home/dracon/Dev/dracon-utilities/README.md` returns nothing (or only historical mention if any).\n- `grep -r \"TODO\\|todo\" /home/dracon/Dev/dracon-utilities/{todo.md,TODO.md,tasks.md}` fails (files don't exist).\n- `git status` is clean at the end.\n\nIf blocked: stop and ask the user.",
./.pi/goals/archived/goal_2026060601095280_mq1krsgc-wjnshs.md:86:- All 4 READMEs (root, sync, system, warden) describe only the binaries that exist and use the same facts as AGENTS.md (no "AI commit messages" claim, no mention of removed `dracon-ai` binary).
