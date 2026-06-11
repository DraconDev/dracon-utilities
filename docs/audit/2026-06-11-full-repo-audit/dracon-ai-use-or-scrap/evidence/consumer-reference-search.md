# Dracon AI reference search excluding generated audit evidence

## timestamp
2026-06-11T18:53:06+01:00

## references in source/docs/config, excluding docs/audit/evidence and target/
./CONTRIBUTING.md:14:4. **Describe your changes** — Include a clear PR description explaining *what* changed and *why*.
./CONTRIBUTING.md:32:└── dracon-ai/
./CHANGELOG.md:137:- **dracon-sync**: Scribe refactor — commit messages from diffs, not `project-state.md`
./CHANGELOG.md:140:  - Removed `scribe_update()` and `stage_project_state()` — replaced by direct commit message generation
./CHANGELOG.md:246:- **dracon-sync**: Version bumper prevents double-bump when both `scribe` and `ai-bumper` features enabled
./CHANGELOG.md:247:- **dracon-sync**: Scribe runs after version bumper (sees post-bump diff)
./CHANGELOG.md:253:- **install.sh**: Removed dracon-ai build (not in workspace); fixed nonexistent file references
./CHANGELOG.md:289:  - AI-powered commit messages (scribe)
./AUDIT.md:20:- **Doc drift:** AGENTS.md test counts are stale in every category; AGENTS.md also says "AI scribe was removed" but `dracon-sync/src/scribe.rs` and `simple_ai.rs` are still wired and callable behind a Cargo feature.
./AUDIT.md:49:### 1.2 P1 — `simple_ai.rs` is compiled into every default build of `dracon-sync` despite "AI scribe removed" claim
./AUDIT.md:52:- **CHANGELOG.md 0.112.0 "Scribe refactor" entry** says: "Removed `scribe_update()` and `stage_project_state()` — replaced by direct commit message generation … `project-state.md` is now manual-only: sync no longer auto-generates, stages, or commits it."
./AUDIT.md:54:  - `dracon-sync/src/scribe.rs:212` and `dracon-sync/src/scribe.rs:274` define `pub(crate) async fn generate_commit_message()`.
./AUDIT.md:55:  - The first impl (line 212) is gated on `#[cfg(feature = "scribe")]` and **calls `SimpleAiService::new().chat(messages).await`**, which posts to `{provider.endpoint}/chat/completions` (`dracon-sync/src/simple_ai.rs:285`).
./AUDIT.md:56:  - The second impl (line 274) under `#[cfg(not(feature = "scribe"))]` returns `None`.
./AUDIT.md:57:  - `dracon-sync/Cargo.toml` declares `[features] default = []` — the `scribe` feature is **not** in default features. So default builds do not link the LLM path.
./AUDIT.md:60:  1. **AGENTS.md drift** — the doc says "No LLM at the commit boundary" but the LLM-calling code is still in the tree and is reachable via `cargo build --features scribe`.
./AUDIT.md:62:  3. **Risk of regression** — if `scribe` is re-enabled in `default`, the AI path silently activates with no AGENTS.md warning.
./AUDIT.md:64:  - **Option A (remove):** delete `scribe.rs`, `simple_ai.rs`, and the `reqwest` features that only they use. Update AGENTS.md to say the LLM client is gone.
./AUDIT.md:65:  - **Option B (keep, document):** keep the code, gate `simple_ai.rs` behind `#[cfg(feature = "scribe")]`, and update AGENTS.md to acknowledge that an LLM-scribe path is available behind the feature flag.
./AUDIT.md:93:- **License metadata:** All 4 packages (`dracon-sync`, `dracon-system`, `dracon-warden`, `dracon-ai`) carry `license = "AGPL-3.0-only"`. `LICENSE` (33 KB) is the AGPL v3 text.
./AUDIT.md:174:- `dracon-sync/Cargo.toml:2`, `dracon-system/Cargo.toml:2`, `dracon-warden/Cargo.toml:2`, `dracon-ai/Cargo.toml:2` all set `license = "AGPL-3.0-only"`. Consistent.
./AUDIT.md:225:### 2.3 P3 — `dracon-system` has no `test_helpers` module while AGENTS.md prescribes one
./AUDIT.md:285:### 2.10 P3 — `dracon-ai/` lives in the repo but is not in the workspace
./AUDIT.md:287:- **Evidence:** `Cargo.toml` workspace `members = ["dracon-sync", "dracon-system", "dracon-warden"]`. `dracon-ai/` is a standalone Rust package (`dracon-ai/Cargo.toml`) with its own `Cargo.lock` (101 KB) and `src/main.rs` (77 KB).
./AUDIT.md:288:- **AGENTS.md / README / install.sh:** None mention `dracon-ai` as a project deliverable. The CHANGELOG 0.112.0 explicitly notes: *"`install.sh`: Removed dracon-ai build (not in workspace); fixed nonexistent file references"*.
./AUDIT.md:289:- **Impact:** Confusion for new contributors — the directory looks like a 4th binary, but `cargo build --workspace` does not build it. The 101 KB `dracon-ai/Cargo.lock` is a redundant lockfile.
./AUDIT.md:290:- **Fix:** Either (a) move `dracon-ai/` to its own repo, or (b) add it to the workspace members and fix any cross-deps, or (c) add a `dracon-ai/README.md` clarifying "not built by workspace".
./AUDIT.md:322:- **Fix:** Change line 35 to `cargo test --workspace -- --test-threads=1` (drop `--bins`). This is the same command AGENTS.md § "Testing" prescribes.
./AUDIT.md:349:### 4.3 P2 — AGENTS.md says "AI scribe was removed" but the code path still exists (see finding 1.2)
./AUDIT.md:351:The wording in AGENTS.md § "What sync doesn't need" and the § "Commit Messages" heading both imply the AI scribe is gone. The CHANGELOG 0.112.0 "Scribe refactor" entry reinforces this. The reality: `scribe.rs` + `simple_ai.rs` are still compiled and the LLM call site is reachable behind a feature flag.
./AUDIT.md:359:### 4.5 P3 — `AGENTS.md § "What This Is NOT"` correctly notes scribe is removed, contradicting the "What sync provides" / "Design Philosophy" sections
./AUDIT.md:362:- **AGENTS.md line 651:** "NOT AI-scribed messages (removed — they were useless for AI workflows)".
./AUDIT.md:363:- **But:** `scribe.rs` and `simple_ai.rs` exist and are wired. The doc is internally consistent *about the intent*, but the code contradicts the intent.
./AUDIT.md:374:- Utility table at the top is correct (3 binaries, no dracon-ai).
./AUDIT.md:405:### 5.3 P3 — `dracon-ai` is a separate package with its own `Cargo.lock` (101 KB) — see finding 2.10
./AUDIT.md:439:- **Fix:** Add a length cap and a clause-count cap to `generate_commit_message` (the local-fallback in `scribe.rs` and the `n file(s) in DIRS` regex path). Example: "truncate each clause to 50 chars; max 3 clauses; if more, replace with `+N more`".
./AUDIT.md:459:### 6.5 ✅ No "scribe_update" or "stage_project_state" calls in production code paths
./AUDIT.md:461:- The CHANGELOG 0.112.0 "Scribe refactor" entry says these were removed.
./AUDIT.md:462:- `rg 'scribe_update|stage_project_state' /home/dracon/Dev/dracon-utilities/dracon-sync/src/` returns 0 matches.
./AUDIT.md:498:| No AI at the commit boundary | ⚠️ PARTIAL — true for default build; `scribe` feature still wires `SimpleAiService::chat()` | finding 1.2 |
./AUDIT.md:527:10. **Resolve the AI scribe contradiction** (findings 1.2, 4.3, 4.5). Either delete `scribe.rs` + `simple_ai.rs` entirely (recommended — aligns with CHANGELOG and AGENTS.md "removed" claims), or feature-gate `simple_ai.rs` and update AGENTS.md to acknowledge the feature flag.
./AUDIT.md:529:12. **Decide `dracon-ai` policy** (finding 2.10). Either move to its own repo, add to workspace, or add a `dracon-ai/README.md` clarifying its standalone status.
./AUDIT.md:697:- Removed stale/dead artifacts: `dracon-warden/dracon-warden.service`, `dracon-sync/src/scribe.rs`, `dracon-sync/src/simple_ai.rs`, `dracon-sync/src/git.rs.test`, `dracon-warden/src/tests.rs.plaintext`, the tracked `pi-session-*.html`, and tracked `rust_out`.
./AUDIT.md:703:- Updated `AGENTS.md` test counts, test helper guidance, systemd hardening tables, local-state policy, `dracon-ai/` standalone validation policy, and commit-message guidance.
./AUDIT.md:705:- Fixed `dracon-ai/` standalone dependency paths and updated it to the current `dracon-libs` AI runtime contracts so it validates separately from the main workspace.
./AUDIT.md:713:- Per-crate counts: `dracon-sync` 431 passed, `dracon-system` 83 passed, `dracon-warden` 79 passed, `dracon-security` 99 passed + 6 ignored, `dracon-ai` standalone 7 passed.
./AUDIT.md:714:- `cargo test --manifest-path dracon-ai/Cargo.toml -- --test-threads=1` — passed: **7 passed**, 1 suite.
./UTILITY_BOUNDARIES.md:13:  - `dracon-ai`
./UTILITY_BOUNDARIES.md:16:  - Interactive utility: `dracon-ai`
./UTILITY_BOUNDARIES.md:39:- `dracon-ai`
./UTILITY_BOUNDARIES.md:51:- `dracon-ai`
./UTILITY_BOUNDARIES.md:56:  - May consume `dracon-ai`, but does not own sync/warden/system runtime roles.
./UTILITY_BOUNDARIES.md:71:- Active utility binaries are `dracon-sync`, `dracon-warden`, `dracon-system`, and `dracon-ai`.
./docs/public-readiness.md:72:- `cargo test --manifest-path dracon-ai/Cargo.toml -- --test-threads=1`
./Cargo.toml:8:    "dracon-ai",
./Cargo.toml:23:ai-routing-runtime = { path = "../dracon-libs/services/crates/ai/ai-routing-runtime" }
./Cargo.toml:24:ai-runtime-adapters = { path = "../dracon-libs/services/crates/ai/ai-runtime-adapters" }
./Cargo.toml:25:ai-runtime-config = { path = "../dracon-libs/services/crates/ai/ai-runtime-config" }
./Cargo.toml:31:dracon-ai-runtime-contracts = { path = "../dracon-libs/contracts/crates/ai/dracon-ai-runtime-contracts" }
./.pi/goals/archived/goal_2026060601095280_mq1krsgc-wjnshs.md:4:  "objective": "=== Goal ===\nObjective: Make the dracon-utilities repo presentable: fix outdated/misleading content in READMEs and BLUEPRINTs, remove obvious clutter from the repo root, restructure scattered docs into a clean docs/ tree, and add a top-level ROADMAP pointing to canonical documentation.\n\nSuccess criteria:\n- All 4 READMEs (root, sync, system, warden) describe only the binaries that exist and use the same facts as AGENTS.md (no \"AI commit messages\" claim, no mention of removed `dracon-ai` binary).\n- All 3 BLUEPRINTs (sync, system, warden) reflect current behavior, not aspirational.\n- A `docs/` directory exists with: `ROADMAP.md`, `ARCHITECTURE.md` (merging `dracon-sync-architecture.md`), `OPERATIONS.md` (merging the operator-facing parts of dated plans), and `archive/` for kept historical docs.\n- Repo root contains only: README.md, CHANGELOG.md, CONTRIBUTING.md, AGENTS.md, LICENSE, Cargo.toml, Cargo.lock, install.sh, uninstall.sh, deny.toml, clippy.toml, rustfmt.toml, rust-toolchain.toml, tarpaulin.toml, flake.lock, flake.nix, scripts/, target/, the 3 binary dirs, and the existing .dracon/ + .pi/ + .gitignore/.gitattributes.\n- Clutter removed (no `pi-session-*.html`, no `rust_out`, no `autoresearch.jsonl`, no `debug.log`, no `SPEC.md`, no `dracon-sync-architecture.md` at root, no `todo.md`/`TODO.md`/`tasks.md` duplicates, no `audit.md`/`AUDIT.md`/`AUDIT_2026-05-29.md`/`AUDIT_CHECKLIST.md`).\n- Dated one-shot reports (`MASTER_ROADMAP_2026-06-01.md`, `STUCK_PUSH_TRIAGE_2026-06-02.md`, `REPOS_CLEANUP_PLAN_2026-06-01.md`, `REFACTORING_BLOCKER_ANALYSIS.md`) are either deleted or moved to `docs/archive/`.\n- A `docs/ROADMAP.md` exists that lists what each doc is for and what superseded.\n- A `docs/ARCHITECTURE.md` exists that replaces `dracon-sync-architecture.md` (or merges its content).\n- A single canonical \"where things are\" pointer in the root README.\n\nBoundaries:\n- In scope: markdown docs (READMEs, BLUEPRINTs, audit/plan/todo/archive), repo-root file removal, .gitignore updates to prevent re-clutter, restructuring into docs/.\n- In scope: fixing content drift between docs (root README says \"dracon-ai\", subdir doesn't exist; sync README says \"AI commit messages\", AGENTS.md says removed).\n- Out of scope: code changes, Cargo.toml restructuring, behavior changes, build system changes, AGENTS.md content (treat as source of truth for what binaries/features exist).\n- Out of scope: dependencies on external tools (no need to run/install anything beyond `ls`/`grep`/`git`).\n- Out of scope: sub-binary BLUEPRINTs beyond surface-level polish (no rewrites of their internals).\n\nConstraints:\n- No force-pushes. Changes go in a normal commit per logical step.\n- .gitignore must be updated to prevent re-clutter (autoresearch.jsonl, debug.log, pi-session-*.html, /rust_out must be ignored) BEFORE the deletion commit so daemon never re-commits them.\n- Use the existing dracon-warden IndexLock-aware patterns when staging the deletion (or commit during freeze if IndexLock can't be used). Actually: deletion of top-level files is a normal git operation, no working-tree writes involved, so IndexLock doesn't apply. But if files are being deleted by a hook, ensure no .git/index.lock contention.\n- Any doc that survives deletion must end up either in docs/ (canonical) or docs/archive/ (historical). No half-deleted files.\n- Sync daemon (dracon-sync) is running and will auto-commit. Either pause sync first (`dracon-sync pause`) or batch the deletion into a single commit so the auto-commit doesn't fragment the cleanup.\n- Commit messages should be deterministic facts (as per dracon-sync commit format), not prose.\n\nVerification contract:\n- `git log --oneline -10` shows a clean series of commits: (1) .gitignore updates, (2) deletions, (3) restructured docs, (4) README sync-up.\n- `ls /home/dracon/Dev/dracon-utilities/` matches the canonical file list above.\n- `find /home/dracon/Dev/dracon-utilities -maxdepth 2 -name 'README.md'` returns exactly 4 files (root + 3 subdirs).\n- `find /home/dracon/Dev/dracon-utilities -maxdepth 2 -name 'BLUEPRINT.md'` returns exactly 3 files (one per subdir).\n- `ls /home/dracon/Dev/dracon-utilities/docs/` returns ROADMAP.md, ARCHITECTURE.md, OPERATIONS.md, archive/.\n- Root README contains a \"Documentation\" section linking to docs/ROADMAP.md, docs/ARCHITECTURE.md, docs/OPERATIONS.md, and the 3 subdir READMEs.\n- `grep -r \"AI commit messages\\|AI-generated\" /home/dracon/Dev/dracon-utilities/dracon-sync/README.md` returns nothing.\n- `grep -r \"dracon-ai\" /home/dracon/Dev/dracon-utilities/README.md` returns nothing (or only historical mention if any).\n- `grep -r \"TODO\\|todo\" /home/dracon/Dev/dracon-utilities/{todo.md,TODO.md,tasks.md}` fails (files don't exist).\n- `git status` is clean at the end.\n\nIf blocked: stop and ask the user.",
./.pi/goals/archived/goal_2026060601095280_mq1krsgc-wjnshs.md:57:        "verificationContract": "All 4 READMEs are fact-aligned with AGENTS.md. Root README links to docs/ROADMAP.md, docs/ARCHITECTURE.md, docs/OPERATIONS.md. No mention of removed dracon-ai binary. No \"AI commit messages\" claim."
./.pi/goals/archived/goal_2026060601095280_mq1krsgc-wjnshs.md:86:- All 4 READMEs (root, sync, system, warden) describe only the binaries that exist and use the same facts as AGENTS.md (no "AI commit messages" claim, no mention of removed `dracon-ai` binary).
./.pi/goals/archived/goal_2026060601095280_mq1krsgc-wjnshs.md:98:- In scope: fixing content drift between docs (root README says "dracon-ai", subdir doesn't exist; sync README says "AI commit messages", AGENTS.md says removed).
./.pi/goals/archived/goal_2026060601095280_mq1krsgc-wjnshs.md:119:- `grep -r "dracon-ai" /home/dracon/Dev/dracon-utilities/README.md` returns nothing (or only historical mention if any).
./AGENTS.md:35:**Workspace policy:** the root Cargo workspace intentionally includes `dracon-sync`, `dracon-system`, and `dracon-warden` only. `dracon-ai/` is a standalone subcrate and must be validated separately when touched; do not fold it into the main workspace without a separate compatibility review.
./AGENTS.md:37:Standalone validation for `dracon-ai/`:
./AGENTS.md:40:cargo test --manifest-path dracon-ai/Cargo.toml -- --test-threads=1
./AGENTS.md:804:- NOT AI-scribed messages (removed — they were useless)
./AGENTS.md:854:- `dracon-ai` standalone: 7 passed, 1 suite (`cargo test --manifest-path dracon-ai/Cargo.toml -- --test-threads=1`).
./.pi/goals/archived/goal_2026060823450664_mq5q0ws4-krdztf.md:22:        "evidence": "User updated .env files with real production values including DRACON_AI_API_KEY, AI_KEY_* provider keys, AI_KEY_ENCRYPTION_SECRET, and production lane allowlist.",
./.pi/goals/archived/goal_2026060823450664_mq5q0ws4-krdztf.md:30:        "evidence": "User's .env.example uses DRACON_AI_API_KEY (matches code at apis/ai-api/src/main.rs:34), has placeholder values, encrypted under age1z4atp...",
./.pi/goals/archived/goal_2026060823450664_mq5q0ws4-krdztf.md:46:        "evidence": "User's .env has real production values: DRACON_AI_API_KEY, AI_KEY_* provider keys, production lane allowlist. Committed (2c46c6b78) and pushed.",
./.pi/goals/archived/goal_2026060823450664_mq5q0ws4-krdztf.md:161:- [x] get-env-values: Get .env values from user — evidence: User updated .env files with real production values including DRACON_AI_API_KEY, AI_KEY_* provider keys, AI_KEY_ENCRYPTION_SECRET, and production lane allowlist.
./.pi/goals/archived/goal_2026060823450664_mq5q0ws4-krdztf.md:162:- [x] create-env-example: Create .env.example with placeholders — evidence: User's .env.example uses DRACON_AI_API_KEY (matches code at apis/ai-api/src/main.rs:34), has placeholder values, encrypted under age1z4atp...
./.pi/goals/archived/goal_2026060823450664_mq5q0ws4-krdztf.md:164:- [x] create-env-prod: Create .env with production values — evidence: User's .env has real production values: DRACON_AI_API_KEY, AI_KEY_* provider keys, production lane allowlist. Committed (2c46c6b78) and pushed.
./docs/public-release-plan.md:227:cargo test --manifest-path dracon-ai/Cargo.toml -- --test-threads=1
./.pi/goals/archived/goal_2026060116410767_mpvafvt3-jx5ana.md:4:  "objective": "Fix the sync binary to properly exclude untracked build artifacts from WARN/CONCERN, and investigate why 2 repos (dracon-platform, dracon-ai-lib) are in CONCERN state with unpushed commits.",
./.pi/goals/archived/goal_2026060116410767_mpvafvt3-jx5ana.md:19:        "title": "Investigate 2 CONCERN repos: dracon-platform and dracon-ai-lib",
./.pi/goals/archived/goal_2026060116410767_mpvafvt3-jx5ana.md:50:Fix the sync binary to properly exclude untracked build artifacts from WARN/CONCERN, and investigate why 2 repos (dracon-platform, dracon-ai-lib) are in CONCERN state with unpushed commits.
./.pi/goals/archived/goal_2026060116410767_mpvafvt3-jx5ana.md:62:- [x] investigate-concern-repos: Investigate 2 CONCERN repos: dracon-platform and dracon-ai-lib — evidence: Investigated 2 CONCERN repos:
./.pi/goals/archived/goal_2026060218063528_mpwfahka-hkwnhs.md:23:        "verificationContract": "For each of the 6 CONCERN repos (avid, ai-auto-writer, dracon-code, dracon-ai-lib, dracon-platform, dracon-voice-notifications) and 6 WARN repos with AHD>0 (cli-file-manager, browser-extensions-shared, Junk-Runner-bevy, dracoon-terminal-engine, dracoon-utilities, rust-ai-web-auto), run `git rev-list --objects --all | git cat-file --batch-check='%(objectsize) %(rest)' | sort -rn | head -20` to identify large objects. Document which repos need filter-repo and which just need normal commit/push."
./.pi/goals/archived/goal_2026060518254842_mq13a66t-11tg22.md:29:        "evidence": "See task-1 evidence. The only .env content found (dracon-ai-lib) is encrypted with dracon-warden, showing [DRACON_SECRET:...] in git history. No plaintext secrets found in any repo."
./.pi/goals/archived/goal_2026060518254842_mq13a66t-11tg22.md:60:- [x] task-2: Report findings with repo name, commit hash, file, and type of secret found — evidence: See task-1 evidence. The only .env content found (dracon-ai-lib) is encrypted with dracon-warden, showing [DRACON_SECRET:...] in git history. No plaintext secrets found in any repo.
./docs/archive/MASTER_ROADMAP_2026-06-01.md:83:- **Category D** (1 repo): Data dir only — dracon-ai-lib .dracon/
./docs/audit/audit-2026-06-07-delta.md:49:| P1-3 | P1 | `dracon-sync/BLUEPRINT.md` "AI Integration" contradictory section | ✅ **RESOLVED** | Section rewritten as "Deterministic Commit Protocol" (line 178-188); no `scribe`/`ai-bumper` features |
./docs/archive/STUCK_PUSH_TRIAGE_2026-06-02.md:17:| 5 | dracon-ai-lib | target/debug/examples/basic_chat-* | 78MB | `target/` |
./docs/archive/STUCK_PUSH_TRIAGE_2026-06-02.md:45:- **avid, ai-auto-writer, dracon-code, dracon-ai-lib, rust-ai-web-auto**: `--invert-paths --path target/`
./.pi/goals/archived/goal_2026060122023334_mpvoneju-0uop48.md:35:        "title": "Investigate 5 CONCERN repos (ai-auto-writer, dracon-code, avid, dracon-voice-notifications, dracon-ai-lib)",
./.pi/goals/archived/goal_2026060122023334_mpvoneju-0uop48.md:38:        "evidence": "Investigated 5 CONCERN repos:\n\n1. **ai-auto-writer** (ahead=6): 6 unpushed commits including:\n   - `chore: untrack target/ build artifacts` (target/ cleanup)\n   - `Bump to v0.1.1`, `Bump to dracon-ai-",
./.pi/goals/archived/goal_2026060122023334_mpvoneju-0uop48.md:82:- [x] investigate-concern-repos: Investigate 5 CONCERN repos (ai-auto-writer, dracon-code, avid, dracon-voice-notifications, dracon-ai-lib) — evidence: Investigated 5 CONCERN repos:
./.pi/goals/archived/goal_2026060122023334_mpvoneju-0uop48.md:86:   - `Bump to v0.1.1`, `Bump to dracon-ai-
./dracon-sync/BLUEPRINT.md:183:extracted from the diff (see AGENTS.md § Commit Messages). No scribe, no
./dracon-ai/BLUEPRINT.md:44:- Requires `--dangerous` flag or `DRACON_AI_DANGEROUS=1` to execute
./docs/archive/REPOS_CLEANUP_PLAN_2026-06-01.md:68:#### D1. dracon-ai-lib (branch: main)
./docs/archive/REPOS_CLEANUP_PLAN_2026-06-01.md:102:For Category D (dracon-ai-lib), additionally evaluate whether `.dracon/` should be tracked or added to .gitignore.
./docs/archive/REPOS_CLEANUP_PLAN_2026-06-01.md:142:### Step 4: Investigate dracon-ai-lib .dracon/ Data (D1)
./docs/archive/REPOS_CLEANUP_PLAN_2026-06-01.md:145:cd ~/Dev/dracon-ai-lib
./docs/archive/REPOS_CLEANUP_PLAN_2026-06-01.md:174:- **Low risk**: Investigating dracon-ai-lib .dracon/ — just inspection
./dracon-warden/src/main.rs:826:    // machine_nixos identity, so docs must describe the ambiguity instead of
./dracon-ai/README.md:1:# dracon-ai
./dracon-ai/README.md:3:`dracon-ai` is the **only** Dracon AI CLI. It is intentionally thin: it does **not** implement provider/model wiring itself.
./dracon-ai/README.md:9:- No direct provider hookup logic in this repo (no OpenRouter/OpenAI/Anthropic “native” client logic in `dracon-ai`).
./dracon-ai/README.md:15:### `dracon-ai` (default)
./dracon-ai/README.md:21:By default, interactive `do` mode is opened in a **new terminal tab** when possible. Use `dracon-ai do --same-terminal` to keep it in the current terminal.
./dracon-ai/README.md:23:### `dracon-ai status`
./dracon-ai/README.md:31:### `dracon-ai do [--plan] [--dangerous] [task...]`
./dracon-ai/README.md:36:- Plan-only: `dracon-ai do --plan ...` (or `DRACON_AI_APPLY=0`).
./dracon-ai/README.md:37:- Potentially dangerous commands are refused unless you pass `--dangerous` (or `DRACON_AI_DANGEROUS=1`). When refused, the command is printed so you can run it manually.
./dracon-ai/README.md:45:### `dracon-ai chat [options] [prompt...]`
./dracon-ai/README.md:50:- If no prompt is provided, `dracon-ai chat` starts interactive mode in a **new terminal tab** when possible.
./dracon-ai/README.md:63:dracon-ai
./dracon-ai/README.md:64:dracon-ai chat Say ok only.
./dracon-ai/README.md:65:printf "Say ok only.\n" | dracon-ai chat --stdin
./dracon-ai/README.md:66:dracon-ai chat --file prompt.txt
./dracon-ai/README.md:67:dracon-ai chat --intent engineer "Refactor this function."
./dracon-ai/README.md:70:### `dracon-ai cmd [options] <command...>`
./dracon-ai/README.md:77:dracon-ai cmd "journalctl --user -u dracon-sync.service -n 200"
./dracon-ai/README.md:78:dracon-ai cmd --timeout-secs 20 --max-bytes 200000 "rg -n \"DRACON_SECRET\" -S ."
./dracon-ai/README.md:83:`dracon-ai` does not select “provider + model” directly.
./dracon-ai/README.md:87:- `ai-runtime-config` resolves policy + secrets into provider specs and active/dev model sets.
./dracon-ai/README.md:88:- `ai-routing-runtime` routes lane/task to a concrete model id.
./dracon-ai/README.md:89:- `ai-runtime-adapters` provides the provider implementation (currently OpenAI-compatible HTTP adapter).
./dracon-ai/README.md:100:`dracon-ai` follows the `dracon-libs` resolution behavior.
./dracon-ai/README.md:105:- Routing policy: `platform/config/ai-routing-policy.json`
./dracon-ai/README.md:114:- `dracon-ai do ...` (plan+execute loop, default)
./dracon-ai/README.md:116:- `dracon-ai cmd ...` (one-shot capture+ask; requires `DRACON_AI_ALLOW_CMD=1`)
./docs/audit/audit-2026-06-06-full.md:519:Line 281:   - [x] AI scribe removed                            ✅ (and the section above contradicts this — see below)
./docs/audit/audit-2026-06-06-full.md:522:**F-7.2.1 [P1] Lines 180–189 of `dracon-sync/BLUEPRINT.md` describe an "AI Integration (Scribe + AI Bumper)" section that contradicts line 281.** Per v2 audit finding P1-3, the BLUEPRINT still has a "Features (compile-time)" block describing `scribe` and `ai-bumper` that the code doesn't have. **Remediation:** delete the contradictory section. **Effort:** 5 min.
./dracon-sync/README.md:270:LLM-scribed commit messages were removed — they hallucinated context and the AI reads the diff anyway. Mechanical facts are searchable (`git log --grep="JWT"`), honest, and compact.
./dracon-ai/dracon-ai.example.toml:2:# Path: ~/.dracon/utilities/ai/dracon-ai.toml
./dracon-ai/dracon-ai.example.toml:16:# DRACON_AI_APPLY=0         - Plan-only mode (don't execute commands)
./dracon-ai/dracon-ai.example.toml:17:# DRACON_AI_DANGEROUS=1     - Allow dangerous commands (use with caution)
./dracon-ai/dracon-ai.example.toml:18:# DRACON_AI_ALLOW_CMD=1     - Enable /cmd tool execution in REPL
./dracon-ai/dracon-ai.example.toml:19:# DRACON_AI_CONFIG=<path>   - Override config file location
./docs/public-release-branch/PUBLIC_RELEASE_PREP.md:157:cargo test --manifest-path dracon-ai/Cargo.toml -- --test-threads=1
./dracon-ai/src/main.rs:4:use dracon_ai_contracts::{RoutingTask, SelectionConstraints};
./dracon-ai/src/main.rs:5:use dracon_ai_runtime_contracts::models::{ChatMessage, ChatRequest};
./dracon-ai/src/main.rs:6:use dracon_ai_runtime_contracts::traits::AiProvider;
./dracon-ai/src/main.rs:18:struct DraconAiConfig {
./dracon-ai/src/main.rs:41:    if let Ok(p) = std::env::var("DRACON_AI_CONFIG") {
./dracon-ai/src/main.rs:49:            .join("dracon-ai.toml"),
./dracon-ai/src/main.rs:53:fn load_config() -> DraconAiConfig {
./dracon-ai/src/main.rs:55:        return DraconAiConfig::default();
./dracon-ai/src/main.rs:58:        return DraconAiConfig::default();
./dracon-ai/src/main.rs:62:        DraconAiConfig::default()
./dracon-ai/src/main.rs:68:    name = "dracon-ai",
./dracon-ai/src/main.rs:178:    /// 📝 Observe a repo and update its project-state.md (scribe)
./dracon-ai/src/main.rs:179:    Scribe {
./dracon-ai/src/main.rs:201:        // Use `--plan` (or DRACON_AI_APPLY=0) for plan-only.
./dracon-ai/src/main.rs:202:        plan: env_bool("DRACON_AI_APPLY").map(|v| !v).unwrap_or(false),
./dracon-ai/src/main.rs:203:        dangerous: env_bool("DRACON_AI_DANGEROUS").unwrap_or(false),
./dracon-ai/src/main.rs:361:            if std::env::var_os("DRACON_AI_ALLOW_CMD").is_none() {
./dracon-ai/src/main.rs:363:                    "raw cmd execution disabled. Set DRACON_AI_ALLOW_CMD=1 to enable."
./dracon-ai/src/main.rs:412:            println!("📜 AI_RUNTIME: dracon-libs policy + secrets (ai-runtime-config)");
./dracon-ai/src/main.rs:424:        Cmd::Scribe { repo } => run_scribe(&repo).await,
./dracon-ai/src/main.rs:538:    let tool = ansi("1;36", "dracon-ai"); // bold cyan
./dracon-ai/src/main.rs:673:            if status_ok("tmux", &["new-window", "-n", "dracon-ai", &cmd]) {
./dracon-ai/src/main.rs:706:                "dracon-ai",
./dracon-ai/src/main.rs:721:        c.args(["--tab", "--title=dracon-ai", "--", &exe_s])
./dracon-ai/src/main.rs:731:            .args(["--new-tab", "-p", "tabtitle=dracon-ai", "-e", &exe_s])
./dracon-ai/src/main.rs:810:        "You are dracon-ai, a computer-context assistant.",
./dracon-ai/src/main.rs:990:    println!("🔧 dracon-ai setup");
./dracon-ai/src/main.rs:1023:        println!("  dracon-ai setup --refresh");
./dracon-ai/src/main.rs:1092:    println!("Run 'dracon-ai status' to verify.");
./dracon-ai/src/main.rs:1269:async fn run_scribe(repo: &Path) -> Result<()> {
./dracon-ai/src/main.rs:1311:        r#"You are a scribe for a software project. Analyze the git history and project state, then write a concise project-state.md.
./dracon-ai/src/main.rs:1353:            "scribe",
./dracon-ai/src/main.rs:1362:        project_id: "scribe".to_string(),
./dracon-ai/src/main.rs:1388:    eprintln!("📝 scribe: updated {}", state_path.display());
./dracon-ai/src/main.rs:1612:                    "Plan only (--plan).\nRemove --plan (or set DRACON_AI_APPLY=1) to allow execution.\n{}",
./dracon-ai/src/main.rs:1624:                out.push_str("Refused to run potentially dangerous command(s) without --dangerous (or DRACON_AI_DANGEROUS=1).\n");
./dracon-ai/src/main.rs:1625:                out.push_str("You can run these manually, or re-run with --dangerous to let dracon-ai execute them.\n\n");
./dracon-ai/src/main.rs:1693:    set_title("dracon-ai do");
./dracon-ai/src/main.rs:1696:        ansi("1;36", "dracon-ai"),
./dracon-ai/src/main.rs:1746:                        dim("  /config             show resolved dracon-ai config")
./dracon-ai/src/main.rs:1999:    set_title("dracon-ai chat");
./dracon-ai/src/main.rs:2000:    let title = ansi("1;36", "dracon-ai");
./dracon-ai/src/main.rs:2025:        content: "You are dracon-ai (CLI). Be concise, practical, and command-oriented. If you need repo context, ask for it or request /cmd output.".to_string(),
./dracon-ai/src/main.rs:2248:            Cli::try_parse_from(["dracon-ai", "chat", "hello", "world"]).expect("chat parses");
./dracon-ai/src/main.rs:2260:        let cli = Cli::try_parse_from(["dracon-ai", "chat"]).expect("chat parses");
./dracon-ai/src/main.rs:2272:        let cli = Cli::try_parse_from(["dracon-ai", "cmd", "echo", "hi"]).expect("cmd parses");
./dracon-ai/src/main.rs:2281:        let cli = Cli::try_parse_from(["dracon-ai", "status"]).expect("status parses");
./dracon-ai/src/main.rs:2287:        let cli = Cli::try_parse_from(["dracon-ai", "do", "add", "nix", "package", "ripgrep"])
./dracon-ai/src/main.rs:2300:        let cli = Cli::try_parse_from(["dracon-ai", "do", "--plan", "echo", "hi"]).expect("do");
./.pi/goals/archived/goal_2026060912501271_mq6fkxxs-ig5gcc.md:4:  "objective": "=== Goal ===\nObjective: Triage and resolve all 6 dirty repos (1 CONCERN + 5 WARNs) in the latest `dracon-sync repos` report so each in-scope repo reaches a clean `git status` (0 mod, 0 stg, 0 sync-relevant untracked) with the original 0 CONCERN, 0 WARN target verified for the originally-dirty repos. Active-goal churn in any repo is treated as in-flight work the daemon auto-commits and is not counted against the verification.\n\nContext (from investigation):\n- **dracon-ai-lib (CONCERN)**: `origin` (https://github.com/DraconDev/dracon-ai-lib.git) is archived (intentional, per commit `archive: mark lib as archived, redirect to ai-api-sdk`). 13 commits are stranded locally, all in `.pi/goals/...`. The other 3 remotes (`github` SSH, `codeberg`, `gitlab`) all point to the old archive-commit `ce377a20`, not local HEAD. Incident ledger shows 10+ consecutive 403 failures.\n- **dracon-platform, DraconDev, ai-auto-repo-rot-scanner-todo-agent (WARN)**: 1 mod each, all in `.pi/goals/...` (operational data).\n- **browser-extensions-shared (WARN)**: 8 mod + 6 untracked, real source (`auto-form-filler`, `death-note-typing-practice`, `vidpro-extensi…`).\n- **dracon-utilities (WARN)**: 3 mod + 3 untracked, real source in `dracon-warden`.\n\nSuccess criteria:\n- Each of the 6 originally-dirty repos shows clean `git status --porcelain` — 0 mod, 0 stg, 0 sync-relevant untracked — excluding `.pi/goals/...` operational files AND active-goal churn in any repo (per Boundaries). Untracked files (real source or build artifacts) outside the active-goal churn scope are either committed, .gitignored, or deleted with user approval.\n- `dracon-sync repos` reports 0 CONCERN, with WARN=0 for the 6 originally-dirty repos.\n- The 13 stranded commits in `dracon-ai-lib` are resolved (pushed to a working remote, dropped with user approval, or the repo is excluded from sync with documented justification) — never silently dropped.\n- No new `STUCK_PUSH` entries appear in `~/.local/state/dracon/dracon-sync-incidents.jsonl` for these 6 repos.\n- All 3 mirror remotes for `dracon-ai-lib` (or its replacement) remain functional.\n\nBoundaries:\nIn scope: the 6 dirty repos in the original report; their remotes, refs, dirty state (mod, stg, AND untracked), and incident history.\nOut of scope:\n- The 13 OK repos (leave alone).\n- Daemon-managed files (`.gitignore`/`.gitattributes` blocks, `.dracon/data/keys/*.pub`, `.pi/goals/*.md` writes — including `.pi/goals/archived/`).\n- Un-archiving `dracon-ai-lib` on GitHub (user explicitly chose to archive).\n- **Active-goal churn in any repo** (where an \"active goal\" is identified by a `.pi/goals/active_goal_*.md` file modified within the last 10 minutes). For files modified by an active goal, the verification considers the repo clean as long as the daemon's auto-commit cycle is keeping up — evidenced by `git log -1 --name-only` showing a recent commit referencing the same `active_goal_*.md` filename (or its parent dir's operational files). This includes (but is not limited to): real-source files written by the active goal (audit reports in `apis/docs/audits/`, CHANGELOG/RELEASE_NOTES updates, test-results/.playwright-artifacts-0/, pnpm-lock.yaml, package-lock.json, etc.).\n\nConstraints:\n- No destructive git operations (`reset --hard`, `push --force`, dropping commits, removing remotes) without explicit user approval per operation.\n- The \"archive: mark lib as archived, redirect to ai-api-sdk\" decision in `dracon-ai-lib` is preserved.\n- Mirror remotes (codeberg, gitlab) must remain functional if modified.\n- If a fix strategy for `dracon-ai-lib` would discard the 13 commits, present the user with the 3 viable strategies and stop for approval.\n\nVerification contract:\n- For each of the 6 originally-dirty repos: `git -C <repo> status --porcelain` shows 0 entries outside the active-goal churn scope (per Boundaries). Concretely:\n  - **No `.pi/goals/...` mods or untracked files** (operational, out of scope).\n  - **No real-source mods or untracked files that pre-date the most recent active goal in that repo** (i.e., the daemon's auto-commit is keeping up — for any file `F` being modified, the most recent commit on the current branch must reference `F` or a file in the same directory written by the same active goal).\n  - **All pre-existing real-source dirty state** (from before the active goal started) must be committed and pushed.\n- `dracon-sync repos` STATUS line shows 0 CONCERN, with WARN=0 for the 6 originally-dirty repos. (Concurrent active-goal churn in repos outside the original 6 may show as WARN in the table but is documented and out of scope.)\n- For each touched repo, `git log --oneline -5` and `git remote -v` show the expected post-fix state.\n- `tail -20 ~/.local/state/dracon/dracon-sync-incidents.jsonl` contains no new `STUCK_PUSH` entries for the 6 repos since the fix was applied.\n- For `dracon-ai-lib`, `git ls-remote <chosen-remote>` (or `git status` if locally-only) confirms no stranded ahead commits.\n- Untracked files (in any in-scope repo) outside the active-goal churn scope are explicitly accounted for: each is either (a) committed and pushed, (b) added to `.gitignore` with user approval, (c) deleted with user approval, or (d) operational (`.pi/goals/...` or active-goal output) and out of scope.\n\nIf blocked: Stop and ask the user. In particular, the `dracon-ai-lib` fix strategy (drop 13 commits, re-point origin to codeberg/gitlab, unarchive on GitHub, or exclude from sync) is a real user decision and must be confirmed before any destructive op. Similarly, untracked-file disposition (.gitignore vs delete) requires user approval per file. If the verification cannot be stably met even with the active-goal churn exclusion, surface the residual dirty files and ask whether to pause the active goals or accept the unstable state.\n\nTasks:\n1. Diagnose all 6 dirty repos — gather `git status`, `git log --oneline -5`, `git remote -v`, and any incident-ledger entries for each. Output a per-repo summary including mod, stg, AND untracked files, AND identify any active goals in each repo.\n2. Resolve CONCERN: `dracon-ai-lib` — present the 3 viable strategies (drop 13 commits, re-point `origin` to a working mirror, unarchive on GitHub) with trade-offs, get user approval, then apply the chosen fix.\n3. Run `dracon-sync repair warns --apply` for the 3 `.pi/goals`-only WARNs (`dracon-platform`, `DraconDev`, `ai-auto-repo-rot-scanner-todo-agent`).\n4. Manually triage `browser-extensions-shared` (originally 8 mod + 6 untracked) — inspect each mod and untracked, commit/push real changes, .gitignore or delete untracked, get user approval for any destructive action. Identify and document any active goals in subdirectories.\n5. Manually triage `dracon-utilities` (originally 3 mod + 3 untracked in `dracon-warden`) — same workflow; account for untracked files explicitly.\n6. Verify — for each of the 6 repos: per-repo `git status` is clean outside the active-goal churn scope (operational files excluded, active-goal output tolerated when daemon auto-commit is keeping up); `dracon-sync repos` shows the 6 originally-dirty repos as OK with 0 CONCERN; tail the incident ledger to confirm no new stuck-push entries.",
./.pi/goals/archived/goal_2026060912501271_mq6fkxxs-ig5gcc.md:27:        "title": "Resolve CONCERN: dracon-ai-lib (archived origin, 13 stranded commits)",
./.pi/goals/archived/goal_2026060912501271_mq6fkxxs-ig5gcc.md:78:- **dracon-ai-lib (CONCERN)**: `origin` (https://github.com/DraconDev/dracon-ai-lib.git) is archived (intentional, per commit `archive: mark lib as archived, redirect to ai-api-sdk`). 13 commits are stranded locally, all in `.pi/goals/...`. The other 3 remotes (`github` SSH, `codeberg`, `gitlab`) all point to the old archive-commit `ce377a20`, not local HEAD. Incident ledger shows 10+ consecutive 403 failures.
./.pi/goals/archived/goal_2026060912501271_mq6fkxxs-ig5gcc.md:86:- The 13 stranded commits in `dracon-ai-lib` are resolved (pushed to a working remote, dropped with user approval, or the repo is excluded from sync with documented justification) — never silently dropped.
./.pi/goals/archived/goal_2026060912501271_mq6fkxxs-ig5gcc.md:88:- All 3 mirror remotes for `dracon-ai-lib` (or its replacement) remain functional.
./.pi/goals/archived/goal_2026060912501271_mq6fkxxs-ig5gcc.md:95:- Un-archiving `dracon-ai-lib` on GitHub (user explicitly chose to archive).
./.pi/goals/archived/goal_2026060912501271_mq6fkxxs-ig5gcc.md:100:- The "archive: mark lib as archived, redirect to ai-api-sdk" decision in `dracon-ai-lib` is preserved.
./.pi/goals/archived/goal_2026060912501271_mq6fkxxs-ig5gcc.md:102:- If a fix strategy for `dracon-ai-lib` would discard the 13 commits, present the user with the 3 viable strategies and stop for approval.
./.pi/goals/archived/goal_2026060912501271_mq6fkxxs-ig5gcc.md:112:- For `dracon-ai-lib`, `git ls-remote <chosen-remote>` (or `git status` if locally-only) confirms no stranded ahead commits.
./.pi/goals/archived/goal_2026060912501271_mq6fkxxs-ig5gcc.md:115:If blocked: Stop and ask the user. In particular, the `dracon-ai-lib` fix strategy (drop 13 commits, re-point origin to codeberg/gitlab, unarchive on GitHub, or exclude from sync) is a real user decision and must be confirmed before any destructive op. Similarly, untracked-file disposition (.gitignore vs delete) requires user approval per file. If the verification cannot be stably met even with the active-goal churn exclusion, surface the residual dirty files and ask whether to pause the active goals or accept the unstable state.
./.pi/goals/archived/goal_2026060912501271_mq6fkxxs-ig5gcc.md:119:2. Resolve CONCERN: `dracon-ai-lib` — present the 3 viable strategies (drop 13 commits, re-point `origin` to a working mirror, unarchive on GitHub) with trade-offs, get user approval, then apply the chosen fix.
./.pi/goals/archived/goal_2026060912501271_mq6fkxxs-ig5gcc.md:139:- [x] resolve-concern-ai-lib: Resolve CONCERN: dracon-ai-lib (archived origin, 13 stranded commits) — evidence: Applied option A+C as user approved:
./dracon-ai/Cargo.lock:15:name = "ai-routing-runtime"
./dracon-ai/Cargo.lock:20: "dracon-ai-contracts",
./dracon-ai/Cargo.lock:21: "dracon-ai-runtime-contracts",
./dracon-ai/Cargo.lock:26:name = "ai-runtime-adapters"
./dracon-ai/Cargo.lock:31: "dracon-ai-runtime-contracts",
./dracon-ai/Cargo.lock:38:name = "ai-runtime-config"
./dracon-ai/Cargo.lock:285:name = "dracon-ai"
./dracon-ai/Cargo.lock:288: "ai-routing-runtime",
./dracon-ai/Cargo.lock:289: "ai-runtime-adapters",
./dracon-ai/Cargo.lock:290: "ai-runtime-config",
./dracon-ai/Cargo.lock:294: "dracon-ai-contracts",
./dracon-ai/Cargo.lock:295: "dracon-ai-runtime-contracts",
./dracon-ai/Cargo.lock:308:name = "dracon-ai-contracts"
./dracon-ai/Cargo.lock:315:name = "dracon-ai-runtime-contracts"
./dracon-ai/Cargo.lock:320: "dracon-ai-contracts",
./dracon-ai/Cargo.toml:3:name = "dracon-ai"
./dracon-ai/Cargo.toml:21:ai-routing-runtime = { path = "../../dracon-libs/services/crates/ai/ai-routing-runtime" }
./dracon-ai/Cargo.toml:22:ai-runtime-adapters = { path = "../../dracon-libs/services/crates/ai/ai-runtime-adapters" }
./dracon-ai/Cargo.toml:23:ai-runtime-config = { path = "../../dracon-libs/services/crates/ai/ai-runtime-config" }
./dracon-ai/Cargo.toml:24:dracon-ai-contracts = { path = "../../dracon-libs/contracts/crates/ai/dracon-ai-contracts" }
./dracon-ai/Cargo.toml:25:dracon-ai-runtime-contracts = { path = "../../dracon-libs/contracts/crates/ai/dracon-ai-runtime-contracts" }
./docs/audit/audit-2026-06-06.md:41:This contradicts the v1 audit's positive finding "Deterministic commit messages — No LLM at the commit boundary" — if AI scribe was removed, why does `test-ai` still appear in docs?
./docs/audit/audit-2026-06-06.md:73:The BLUEPRINT contains "## AI Integration (Scribe + AI Bumper)" that claims:
./docs/audit/audit-2026-06-06.md:74:- "dracon-sync has integrated AI for generating commit messages (scribe)"
./docs/audit/audit-2026-06-06.md:75:- "Scribe: AI generates commit subjects from diffs"
./docs/audit/audit-2026-06-06.md:78:But line 281 says: `[x] AI scribe removed (was not useful for AI workflows)`
./docs/audit/audit-2026-06-06.md:80:And `AGENTS.md` states: "AI scribe was removed as AI-generated messages were not useful for AI workflows."
./docs/audit/audit-2026-06-06.md:82:**Fix:** Delete the "AI Integration (Scribe + AI Bumper)" section (lines 180-189) and the related "Features (compile-time)" entries for `scribe` and `ai-bumper`. Keep the Status item that documents the removal.
./.pi/goals/archived/goal_2026060612502634_mq2a752j-vuj9w5.md:19:        "title": "Fix the 2 WARN repos (pully-fully, dracon-ai-lib) by running `dracon-sync repair warns --apply`",
./.pi/goals/archived/goal_2026060612502634_mq2a752j-vuj9w5.md:59:- [x] fix-warn-repos: Fix the 2 WARN repos (pully-fully, dracon-ai-lib) by running `dracon-sync repair warns --apply`
./.dracon/demon-migration-audit.md:19:| dracon-ai-lib | 0 refs | 0 refs | ✅ Clean |
./.dracon/secret-audit-report.md:78:| dracon-ai-lib | ✅ Clean (.env encrypted with dracon-warden) |
./dracon-sync/src/sync.rs:1497:/// Uses `git describe --tags --always --exact-match` for exact tag match.
./dracon-sync/src/sync.rs:1502:        &["describe", "--tags", "--always", "--exact-match"],

## policy/config files under ~/.dracon mentioning AI
/home/dracon/.dracon/utilities/README.md:58:1. `dracon-ai`
/home/dracon/.dracon/utilities/README.md:66:- Rule: can consume `dracon-ai`, but core persistence/security remains owned by `dracon-sync` and `dracon-warden`.
/home/dracon/.dracon/utilities/sync/secrets/README.md:17:| AI provider keys | `ai/secrets/*.env` | simple_ai.rs | Mistral, NVIDIA, OpenRouter API keys for scribe/bumper |
/home/dracon/.dracon/utilities/ai/dracon-ai.toml:1:# dracon-ai policy (computer-context defaults)
/home/dracon/.dracon/utilities/ai/dracon-ai.toml:3:# dracon-ai is not a general chat client. It's a computer-context assistant that
/home/dracon/.dracon/utilities/ai/dracon-ai.toml:4:# plans shell commands and can execute them in a bounded loop (`dracon-ai do --apply`).
/home/dracon/.dracon/utilities/ai/dracon-ai.toml:6:# Override path with DRACON_AI_CONFIG.
/home/dracon/.dracon/utilities/sync/ai.toml:2:# Used by scribe (commit messages) and ai-bumper (version bump reasoning)
