# Dracon AI inventory

## timestamp
2026-06-11T18:50:02+01:00

## workspace manifest sections mentioning ai
README.md:29:**Invisible git sync for AI-powered development.** An auto-commit, multi-mirror daemon that watches your repos, commits every change with deterministic, facts-based messages, and pushes to GitHub, GitLab, and Codeberg simultaneously.
README.md:34:- Deterministic commit messages (routing keys for AI-to-AI communication)
README.md:36:- Self-healing and repair
README.md:45:Every metric is extracted deterministically from the diff — no AI, no guessing. Messages are optimized for `git log --grep=` queries.
README.md:80:**Git filter + repo hardening tool.** Encrypts secrets at rest in git while keeping plaintext in your working tree. Uses git hooks (not a daemon) as the primary enforcement layer.
README.md:93:dracon-warden keygen        # Generate new age keypair
README.md:108:├── services/ai/            <- AI adapters, router, lanes
README.md:112:**Key point:** `dracon-utilities` contains the CLI wrappers. `dracon-libs` contains shared library code. Only the CLI binaries get installed.
README.md:122:    ├── services/ai/
README.md:226:| [AGENTS.md](AGENTS.md) | AI agent guidelines |
README.md:232:AGPL v3 — See [LICENSE](LICENSE) for details.
Cargo.toml:8:    "dracon-ai",
Cargo.toml:23:ai-routing-runtime = { path = "../dracon-libs/services/crates/ai/ai-routing-runtime" }
Cargo.toml:24:ai-runtime-adapters = { path = "../dracon-libs/services/crates/ai/ai-runtime-adapters" }
Cargo.toml:25:ai-runtime-config = { path = "../dracon-libs/services/crates/ai/ai-runtime-config" }
Cargo.toml:31:dracon-ai-runtime-contracts = { path = "../dracon-libs/contracts/crates/ai/dracon-ai-runtime-contracts" }
AGENTS.md:33:**Key point:** `dracon-utilities` contains the CLI wrappers. `dracon-libs` contains shared library code. Only the CLI binaries get installed.
AGENTS.md:35:**Workspace policy:** the root Cargo workspace intentionally includes `dracon-sync`, `dracon-system`, and `dracon-warden` only. `dracon-ai/` is a standalone subcrate and must be validated separately when touched; do not fold it into the main workspace without a separate compatibility review.
AGENTS.md:37:Standalone validation for `dracon-ai/`:
AGENTS.md:40:cargo test --manifest-path dracon-ai/Cargo.toml -- --test-threads=1
AGENTS.md:53:    ├── services/ai/
AGENTS.md:72:dracon-sync is designed to be **invisible infrastructure** for an AI coder. The AI works on one repo at a time, makes changes, and sync handles the rest — the AI never needs to think about commits, pushes, or cross-repo coordination.
AGENTS.md:74:**The AI workflow:**
AGENTS.md:76:2. AI reads `dracon-utilities/.dracon/project-state.md` (if present, for manual context)
AGENTS.md:77:3. AI makes changes
AGENTS.md:82:- Auto-commit on every change (AI doesn't need to think about git)
AGENTS.md:83:- Deterministic commit messages (routing keys for AI-to-AI communication)
AGENTS.md:84:- Incident ledger for debugging (AI can read what went wrong)
AGENTS.md:85:- Freezing for pause (AI can pause sync during delicate operations)
AGENTS.md:89:- Global workspace state (AI works on one repo at a time)
AGENTS.md:90:- Session logging (AI doesn't "resume" — each session is fresh)
AGENTS.md:91:- Interactive features (AI runs non-interactively)
AGENTS.md:121:| `RestartSec` | `5` | Wait 5s before restart |
AGENTS.md:159:| `RestartSec` | `10` | Wait 10s before restart |
AGENTS.md:227:**FUNDING.yml is public and version-controlled.** Do not place API keys, tokens, account passwords, or any other secret material in it. The Warden key-management layer treats `FUNDING.yml` as plain text.
AGENTS.md:247:- **Broken tracking**: Repairs `origin/master: gone` refs → `origin/{branch}` (also runs every ~5 min in the loop)
AGENTS.md:249:- **Clone race guard (IndexLock)**: The true root cause was the **warden** — `publish_repo_pubkey()` writes `.pub` files to `.dracon/data/keys/` during `harden_repo()`. When triggered by filesystem events during `git clone`, these files appear before git's checkout phase, causing "Untracked working tree file would be overwritten by merge." The definitive fix uses git's own coordination protocol: **`IndexLock`** acquires `.git/index.lock` (same file git uses during checkout) before any working-tree writes. Uses `O_EXCL` (atomic create-new) — no TOCTOU race. If git holds the lock → warden/sync skip. If warden/sync hold it → git's checkout waits. This is exactly how git commands coordinate with each other. The old heuristics (grace period, HEAD check) are kept as defense-in-depth but the `IndexLock` is the primary coordination mechanism. Applied in both warden (`harden_repo` → `apply_overwrite_file` + `publish_repo_pubkey`) and sync (`ensure_standard_files`). The `once`/`repair` commands use `IndexLock::bypass()` since the user explicitly requested the operation.
AGENTS.md:307:Never delete, rename, untrack, or ignore `note.md`, `notes.md`, screenshot/image files, pasted-image files, or local task/session state unless the user explicitly approves that exact file or directory. Before any destructive cleanup, inspect both the filesystem and git history, preserve files on disk, and ask for confirmation if ownership is uncertain.
AGENTS.md:333:cat ~/.local/state/dracon/dracon-sync-incidents.jsonl | tail -20
AGENTS.md:336:cat ~/.local/state/dracon/dracon-sync-alerts.jsonl | tail -20
AGENTS.md:341:{"ts_unix":1714896000,"scope":"safety","repo":"/path/to/repo","reason":"description of what happened","action":"action_taken","backup_branch":null,"result":"result","details":"additional details"}
AGENTS.md:344:Common `scope` values: `safety` (safety guard triggers), `repair` (auto-repair), `sync` (sync operations), `mirror` (mirror push failures).
AGENTS.md:369:Safety: most `remove_dir_all` call sites in `dracon-system` check the path against both system and user-protected paths before executing. The guard-specific `check_safe_to_delete_guard` skips SYSTEM_PROTECTED (only checks user-protected) because the guard only deletes known artifact/cache directories (target/, node_modules/, ~/.cache/, Trash) which are legitimately under /home. The `--apply` flag is required for destructive operations.
AGENTS.md:373:The guard monitors processes using >`process_cpu_percent`% CPU for >`process_sustain_secs` seconds. All heavy processes are logged to a persistent JSONL file regardless of duration.
AGENTS.md:376:- Logs both `heavy-brief` (any spike) and `heavy-sustained` (after sustain threshold) events
AGENTS.md:378:- JSONL format: `{"ts":1234567890,"event":"heavy-brief","details":"pid=123 ppid=1 cmd=git args=git init cpu=61.7% ..."}`
AGENTS.md:430:Repo discovery searches up to **4 levels deep** from each watch root. Dot-prefixed directories (e.g. `.config/`, `.dracon/`) are descended into if they contain a `.git` directory — only skipped after the `.git` check fails. The hardcoded exclusions are `objects` and whatever is in `exclude_dir_names` from policy.
AGENTS.md:434:Push operations use `push_with_retries` with SSH hardening (`ConnectTimeout`, `ConnectionAttempts`) and automatic HTTPS fallback on persistent timeout. The `push_retries` policy setting is respected. All transient network failures should now trigger retries rather than failing immediately.
AGENTS.md:440:- **Less likely to conflict**: Merge handles parallel commits gracefully; rebase fails if the same lines were modified
AGENTS.md:463:If the GitHub repo already exists, reuse it. A previous suffix loop in `create_github_private_remote` created 15+ orphan repos (`dracon-demons-1` through `-9`). This happens when `gh repo create` fails with "Name already exists" and the code appends `-1`, `-2` instead of just reusing the existing repo. This pattern is explicitly banned in all repo creation functions.
AGENTS.md:467:Some platforms (GitLab, Forgejo) reject dots in project names. The `.dracon` repo (dot-prefixed) would fail on GitLab. Use `repo_name_map` to map local directory names to remote project names:
AGENTS.md:486:On push failures (origin or mirror remotes), `dracon-sync` can send a fire-and-forget HTTP POST to a configured webhook URL:
AGENTS.md:495:  "event": "push_failure",
AGENTS.md:503:The request runs in a background thread with a 5s timeout — webhook failures do not block sync operations.
AGENTS.md:546:**Global publish targets** are configured in the main `dracon-sync.toml`:
AGENTS.md:558:**Safety:** Dry-run publish (`cargo publish --dry-run`, `npm publish --dry-run`) runs before real publish. Registry pre-check skips already-published versions. Publish failures log incidents but don't break the sync cycle.
AGENTS.md:579:  repair    Repair and manage repositories (concerns, warns, origins, stuck repos, dual-branch)
AGENTS.md:588:**Subcommands of `repair` (all dry-run by default; pass `--apply` to execute):**
AGENTS.md:589:- `dracon-sync repair concerns` — repair concern repos
AGENTS.md:590:- `dracon-sync repair warns` — repair warn repos (dirty-only triage)
AGENTS.md:591:- `dracon-sync repair origins` — detect and repair origin URLs pointing to orphan `-N` suffixed repos
AGENTS.md:592:- `dracon-sync repair stuck-list` — list repos that are permanently stuck on push
AGENTS.md:593:- `dracon-sync repair stuck-unstuck <repo>` — unstuck a specific repo
AGENTS.md:594:- `dracon-sync repair dual-branch-list` — list repos with dual main/master
AGENTS.md:595:- `dracon-sync repair dual-branch-repair <repo>` — consolidate to main
AGENTS.md:615:**dracon-sync commit message generation:** Commit messages are simple mechanical facts (e.g., "update 3 file(s)") extracted from the diff. No AI, no LLM, no prose.
AGENTS.md:641:- `dracon-system guard clean` — clean all reclaimable space (targets, trash, nix, caches, node_modules)
AGENTS.md:654:  scrub-markers  Scan plaintext JSON files for DRACON_SECRET markers and optionally scrub them
AGENTS.md:655:  resmudge       Fix working-tree files that are still ciphertext (contain DRACON_SECRET markers)
AGENTS.md:656:  repair         System-wide repair pass for secret-related corruption
AGENTS.md:659:  keygen         Generate a new age keypair for this machine
AGENTS.md:665:- `pre-push`: Scans for plaintext secrets as defense-in-depth (catches `--no-verify` bypass)
AGENTS.md:686:Commit messages are **deterministic facts extracted from the diff**. No AI, no LLMs, no prose.
AGENTS.md:688:### Core Principle: No AI-Generated Messages
AGENTS.md:690:- **No LLM at the commit boundary** — zero AI calls when generating commit messages
AGENTS.md:692:- **No prose** — structured key-value pairs only
AGENTS.md:693:- **AI reads the diff, not the message** — the message is just an INDEX for searching
AGENTS.md:744:# Binary file added (context window warning for AI)
AGENTS.md:763:### How AI Searches This
AGENTS.md:804:- NOT AI-scribed messages (removed — they were useless)
AGENTS.md:806:- NOT natural language summaries — AI reads the diff
AGENTS.md:854:- `dracon-ai` standalone: 7 passed, 1 suite (`cargo test --manifest-path dracon-ai/Cargo.toml -- --test-threads=1`).
AGENTS.md:862:# Fast but may have flaky failures from shared global state:
AGENTS.md:866:**Known parallel-test issues:** some tests can fail unpredictably when running with default parallelism. Root causes:
Cargo.lock:287:name = "async-trait"
Cargo.lock:481: "num-traits",
Cargo.lock:1568: "iana-time-zone-haiku",
Cargo.lock:1576:name = "iana-time-zone-haiku"
Cargo.lock:1990:name = "num-traits"
Cargo.lock:2324: "num-traits",
Cargo.lock:2740: "wait-timeout",
Cargo.lock:2984:name = "stable_deref_trait"
Cargo.lock:3562:name = "wait-timeout"
Cargo.lock:4275: "stable_deref_trait",
Cargo.lock:4305: "async-trait",
docs/public-readiness.md:7:`dracon-utilities` is **not safe to publish as-is**. It is a plausible public candidate after a dedicated public-release cleanup branch, but the current tree and reachable history still contain local agent/task state, audit artifacts, operational logs, and secret-shaped fixture strings that should not be exposed without review.
docs/public-readiness.md:25:| Git state | `git status --porcelain=v2 --untracked-files=all` is clean on `main` | Good |
docs/public-readiness.md:47:Reachable history contains the same local-state families plus older audit/task artifacts. Because public publishing exposes history by default, this repo is **not public-ready until a public-release branch removes or rewrites those paths and verifies the rewritten history**.
docs/public-readiness.md:72:- `cargo test --manifest-path dracon-ai/Cargo.toml -- --test-threads=1`
docs/public-readiness.md:76:- `cargo fmt --workspace --check` is not a valid `cargo fmt` invocation on this toolchain.
docs/public-readiness.md:108:5. Review `AGENTS.md`; it is useful internally but should not be published unchanged unless you are comfortable exposing agent workflow details.
docs/ROADMAP.md:25:| **Architecture** | [ARCHITECTURE.md](ARCHITECTURE.md) | Sync architecture, AI-to-AI commit protocol |
docs/ROADMAP.md:27:| **AI Agent Guide** | [AGENTS.md](../AGENTS.md) | Guidelines for AI agents working in this repo |
docs/ROADMAP.md:31:The following documents are historical references, archived documents, or older drafts. Use the current docs above for implementation and operation details.
docs/ROADMAP.md:38:| `STUCK_PUSH_TRIAGE_2026-06-02.md` | Archived in [archive/](archive/); use `dracon-sync repair stuck-list` for current stuck-push triage. |
docs/OPERATIONS.md:72:cat ~/.local/state/dracon/dracon-sync-incidents.jsonl | tail -20
docs/OPERATIONS.md:77:{"ts_unix":1714896000,"scope":"safety","repo":"/path/to/repo","reason":"description","action":"action_taken","backup_branch":null,"result":"result","details":"additional details"}
docs/OPERATIONS.md:80:Common `scope` values: `safety` (safety guard triggers), `repair` (auto-repair), `sync` (sync operations), `mirror` (mirror push failures).
docs/OPERATIONS.md:113:dracon-sync repair stuck-list
docs/OPERATIONS.md:114:dracon-sync repair stuck-unstuck <repo>
docs/OPERATIONS.md:120:dracon-sync repair dual-branch-list
docs/OPERATIONS.md:121:dracon-sync repair dual-branch-repair <repo>
docs/OPERATIONS.md:124:### Origin Repair
docs/OPERATIONS.md:127:dracon-sync repair origins [--apply]
docs/public-release-plan.md:30:- `git status --porcelain=v2 --untracked-files=all`
docs/public-release-plan.md:41:git switch main
docs/public-release-plan.md:47:- Avoid changing `main` until the public branch is verified.
docs/public-release-plan.md:51:- `git status --porcelain=v2 --untracked-files=all` should show only intended docs.
docs/public-release-plan.md:63:git status --porcelain=v2 --untracked-files=all > ~/backups/dracon-utilities-public-release/status-before-public-cleanup.txt
docs/public-release-plan.md:81:- Main categories:
docs/public-release-plan.md:113:- If any path ownership is uncertain, stop and ask.
docs/public-release-plan.md:121:- Current history contains local-state paths even if the current tree is cleaned.
docs/public-release-plan.md:177:- Maintainer contact or issue template link
docs/public-release-plan.md:211:- No real-looking token matches, or every remaining match is clearly synthetic and documented.
docs/public-release-plan.md:227:cargo test --manifest-path dracon-ai/Cargo.toml -- --test-threads=1
docs/public-release-plan.md:235:- `cargo fmt --all --check` is not a suitable repo-equivalent because the sibling `dracon-libs` checkout can contain paths rustfmt tries to resolve; CI uses the package-specific command above.
docs/public-release-plan.md:244:git status --porcelain=v2 --untracked-files=all
docs/public-release-plan.md:281:- A path ownership is uncertain.
docs/public-release-plan.md:284:- Validation fails and the cause is not understood.
docs/ARCHITECTURE.md:18:**Core loop:** watch → detect change → wait for stability → commit → push to origin + mirrors.
docs/ARCHITECTURE.md:21:- Deterministic commit messages (no AI) — extractable facts from diffs for `git log --grep=` queries
docs/ARCHITECTURE.md:37:- Inode monitoring — catches the "many small files" failure mode
docs/ARCHITECTURE.md:41:Git filter + repo hardening. Encrypts secrets at rest with age encryption while keeping plaintext in the working tree.
docs/ARCHITECTURE.md:47:- Age x25519 keys — one keypair per machine, pubkeys published per-repo
docs/ARCHITECTURE.md:49:- Defense-in-depth — pre-push hook scans for plaintext secrets as a second layer
docs/ARCHITECTURE.md:51:## dracon-sync: AI-to-AI Commit Protocol
docs/ARCHITECTURE.md:53:Commit messages are deterministic facts extracted from diffs, not AI-generated prose. This makes the git log a queryable database for downstream AI agents.
docs/ARCHITECTURE.md:57:**The Worker (AI agent)** edits files and yields control. It never runs `git commit`.
docs/ARCHITECTURE.md:93:### Why Deterministic Over AI
docs/ARCHITECTURE.md:95:| Aspect | AI Commit | Deterministic Commit |
docs/ARCHITECTURE.md:108:├── services/ai/            ← AI adapters, router, lanes
docs/ARCHITECTURE.md:118:- `IndexLock::acquire()` — blocks until the lock is available
docs/ARCHITECTURE.md:119:- `IndexLock::bypass()` — for explicit user operations (`once`, `repair`)
.github/workflows/ci.yml:5:    branches: [main, master]
.github/workflows/ci.yml:7:    branches: [main, master]
.github/workflows/ci.yml:26:      - name: Install Rust toolchain
.github/workflows/ci.yml:27:        uses: dtolnay/rust-toolchain@stable
.github/workflows/ci.yml:48:        run: cargo clippy -p dracon-sync -p dracon-system -p dracon-warden -- -W clippy::pedantic -W clippy::nursery 2>&1 | tail -1
.github/workflows/ci.yml:64:      - name: Install Rust toolchain
.github/workflows/ci.yml:65:        uses: dtolnay/rust-toolchain@stable
.github/workflows/ci.yml:95:      - name: Install Rust toolchain
.github/workflows/ci.yml:96:        uses: dtolnay/rust-toolchain@stable
.github/workflows/ci.yml:146:    name: Minimum Toolchain
.github/workflows/ci.yml:155:        uses: dtolnay/rust-toolchain@stable
.github/workflows/ci.yml:188:      - name: Install Rust toolchain
.github/workflows/ci.yml:189:        uses: dtolnay/rust-toolchain@stable
.github/workflows/cla.yml:26:    if: github.event_name == 'issue_comment' && contains(github.event.comment.body, 'sign cla')
docs/public-release-branch/PUBLIC_RELEASE_PREP.md:5:Baseline commit: `43d7505d6e70debdd876295726387bb794c6bf15` (`main`)  
docs/public-release-branch/PUBLIC_RELEASE_PREP.md:10:This branch isolates the work required to make `dracon-utilities` public-release safe without changing `main`, repo visibility, mirrors, or published artifacts.
docs/public-release-branch/PUBLIC_RELEASE_PREP.md:24:2. Current/reachable history contains local agent/task state, audit artifacts, operational logs, and secret-shaped fixture strings.
docs/public-release-branch/PUBLIC_RELEASE_PREP.md:36:- Confirm `main` remains the normal internal/private development branch.
docs/public-release-branch/PUBLIC_RELEASE_PREP.md:42:git rev-parse main
docs/public-release-branch/PUBLIC_RELEASE_PREP.md:44:git diff --name-only main...public-release
docs/public-release-branch/PUBLIC_RELEASE_PREP.md:65:Important constraints:
docs/public-release-branch/PUBLIC_RELEASE_PREP.md:69:- Do not remove local state from `main`.
docs/public-release-branch/PUBLIC_RELEASE_PREP.md:74:git status --porcelain=v2 --untracked-files=all
docs/public-release-branch/PUBLIC_RELEASE_PREP.md:75:git diff --name-status main...public-release
docs/public-release-branch/PUBLIC_RELEASE_PREP.md:92:git status --porcelain=v2 --untracked-files=all > ~/backups/dracon-utilities-public-release/status-before-public-cleanup.txt
docs/public-release-branch/PUBLIC_RELEASE_PREP.md:118:Expected result: no real-looking token matches, or every remaining match is clearly synthetic and documented.
docs/public-release-branch/PUBLIC_RELEASE_PREP.md:157:cargo test --manifest-path dracon-ai/Cargo.toml -- --test-threads=1
docs/public-release-branch/PUBLIC_RELEASE_PREP.md:172:git status --porcelain=v2 --untracked-files=all
docs/public-release-branch/PUBLIC_RELEASE_PREP.md:187:Stop here and wait for explicit approval for Gate B and Gate C before any cleanup or history rewrite.
docs/archive/REFACTORING_BLOCKER_ANALYSIS.md:58:- **Option A (Trait-based)**: Define a `CooldownPolicy` trait that the daemon implements. The cooldown manager uses the trait to read/write state without direct struct references. Estimated: 3-4 hours.
docs/archive/REFACTORING_BLOCKER_ANALYSIS.md:74:A Unix domain socket (UDS) endpoint that provides JSON health status for the sync daemon:
docs/archive/REFACTORING_BLOCKER_ANALYSIS.md:131:### Pattern 1: Revert on Failure
docs/archive/REFACTORING_BLOCKER_ANALYSIS.md:133:- **The codebase is stable** — maintainers prefer working code over risky refactors
docs/archive/REFACTORING_BLOCKER_ANALYSIS.md:154:- `dracon-sync/src/daemon.rs` — contains cooldown logic for H-DAEMON
docs/archive/REFACTORING_BLOCKER_ANALYSIS.md:155:- `dracon-sync/src/sync.rs` — contains 30+ sync git calls for L-ASYNC-UNIFY
docs/archive/REPOS_CLEANUP_PLAN_2026-06-01.md:20:These repos have only untracked `target/`, `node_modules/`, or `.dracon/` directories. No code changes present. Action: update .gitignore to exclude these patterns, then run `dracon-sync repair-warns --apply`.
docs/archive/REPOS_CLEANUP_PLAN_2026-06-01.md:24:| A1 | dracon-terminal-engine | target/, crates/cargo-dracon/target/, crates/dracon-macros/target/ | 51G | main | DISCARD: add `target/` to .gitignore |
docs/archive/REPOS_CLEANUP_PLAN_2026-06-01.md:25:| A2 | dracon-platform | target/ | 8.7G | main | DISCARD: add `target/` to .gitignore |
docs/archive/REPOS_CLEANUP_PLAN_2026-06-01.md:26:| A3 | rust-ai-web-auto | target/ | 4.5G | main | DISCARD: add `target/` to .gitignore |
docs/archive/REPOS_CLEANUP_PLAN_2026-06-01.md:27:| A4 | avid | target/ | — | main | DISCARD: add `target/` to .gitignore |
docs/archive/REPOS_CLEANUP_PLAN_2026-06-01.md:28:| A5 | ai-auto-writer | target/ | — | main | DISCARD: add `target/` to .gitignore |
docs/archive/REPOS_CLEANUP_PLAN_2026-06-01.md:29:| A6 | browser-extensions-shared | node_modules/, cursor-style/node_modules/, wxt-shared/node_modules/ | 951M | main | DISCARD: add `node_modules/` to .gitignore |
docs/archive/REPOS_CLEANUP_PLAN_2026-06-01.md:30:| A7 | respec-spec-reconciler | node_modules/ | 231M | main | DISCARD: add `node_modules/` to .gitignore (stale branch B1 already deleted locally) |
docs/archive/REPOS_CLEANUP_PLAN_2026-06-01.md:31:| A8 | ai-auto-repo-rot-scanner-todo-agent | target/ | — | main | DISCARD: add `target/` to .gitignore |
docs/archive/REPOS_CLEANUP_PLAN_2026-06-01.md:32:| A9 | opencode-auto-review-completed-todos | node_modules/ | 61M | main | DISCARD: add `node_modules/` to .gitignore |
docs/archive/REPOS_CLEANUP_PLAN_2026-06-01.md:33:| A10 | pully-fully-pull-based-fleet-reconciler | pully-types/target/ | — | main | DISCARD: add `target/` to .gitignore |
docs/archive/REPOS_CLEANUP_PLAN_2026-06-01.md:34:| A11 | dracon-demons | target/ | — | main | DISCARD: add `target/` to .gitignore |
docs/archive/REPOS_CLEANUP_PLAN_2026-06-01.md:35:| A12 | wal-backup | target/ | — | main | DISCARD: add `target/` to .gitignore |
docs/archive/REPOS_CLEANUP_PLAN_2026-06-01.md:36:| A13 | pi-auto-review | node_modules/ | 264M | main | DISCARD: add `node_modules/` to .gitignore |
docs/archive/REPOS_CLEANUP_PLAN_2026-06-01.md:37:| A14 | video-uploader | target/ | — | main | DISCARD: add `target/` to .gitignore |
docs/archive/REPOS_CLEANUP_PLAN_2026-06-01.md:38:| A15 | dracon-code | target/, examples/phase2/example2/target/ | — | main | DISCARD: add `target/` to .gitignore |
docs/archive/REPOS_CLEANUP_PLAN_2026-06-01.md:46:| M1 | dracon-utilities | `.pi/goals/...md` (goal file update) | target/ (15G) | main | COMMIT goal file + add `target/` to .gitignore |
docs/archive/REPOS_CLEANUP_PLAN_2026-06-01.md:47:| M2 | rust-ai-web-auto | `.pi/goals/...md` (1 deleted, 1 added — goal archived) | target/ (4.5G) | main | COMMIT goal archive + add `target/` to .gitignore |
docs/archive/REPOS_CLEANUP_PLAN_2026-06-01.md:48:| M3 | cli-file-manager | `.pi/goals/...md`, `src/daemon.rs` (2 files) | cfm-lib/target/, target/ | main | COMMIT real changes + add `target/` to .gitignore |
docs/archive/REPOS_CLEANUP_PLAN_2026-06-01.md:56:- **Investigation**: Branch contained evolutionary spec reconciliation work — experimental AI-driven approach to spec parsing. The divergence was too large (49K lines) to merge safely, and the approach was deemed experimental.
docs/archive/REPOS_CLEANUP_PLAN_2026-06-01.md:68:#### D1. dracon-ai-lib (branch: main)
docs/archive/REPOS_CLEANUP_PLAN_2026-06-01.md:71:- **Recommendation**: INVESTIGATE — `.dracon/` may contain data that should be tracked or may be cache. Add `target/` to .gitignore, evaluate `.dracon/`.
docs/archive/REPOS_CLEANUP_PLAN_2026-06-01.md:75:The following 13 repos are confirmed clean: DraconDev, .dracon, youtube-video-uploader, volume-and-video-pro, tiles-tui-file-manager, SamAI, git-seal, obs-wayland-hotkey, kittentts-showcase, test-auto-create, opencode-auto-force-resume, opencode-auto-continue, dracon-libs.
docs/archive/REPOS_CLEANUP_PLAN_2026-06-01.md:100:This prevents future tracking of build artifacts. **Note: adding to .gitignore only stops tracking — it does NOT reclaim disk space.** To reclaim disk, run `git clean -fdx` or manually delete the untracked directories.
docs/archive/REPOS_CLEANUP_PLAN_2026-06-01.md:102:For Category D (dracon-ai-lib), additionally evaluate whether `.dracon/` should be tracked or added to .gitignore.
docs/archive/REPOS_CLEANUP_PLAN_2026-06-01.md:104:This can be automated using `dracon-sync repair-warns --apply` after the .gitignore patterns are in place.
docs/archive/REPOS_CLEANUP_PLAN_2026-06-01.md:114:# For rust-ai-web-auto (M2)
docs/archive/REPOS_CLEANUP_PLAN_2026-06-01.md:115:cd ~/Dev/rust-ai-web-auto
docs/archive/REPOS_CLEANUP_PLAN_2026-06-01.md:134:git log --oneline main..autoresearch/evolutionary-reconciler-2026-05-30
docs/archive/REPOS_CLEANUP_PLAN_2026-06-01.md:135:git diff main..autoresearch/evolutionary-reconciler-2026-05-30 --stat
docs/archive/REPOS_CLEANUP_PLAN_2026-06-01.md:142:### Step 4: Investigate dracon-ai-lib .dracon/ Data (D1)
docs/archive/REPOS_CLEANUP_PLAN_2026-06-01.md:145:cd ~/Dev/dracon-ai-lib
docs/archive/REPOS_CLEANUP_PLAN_2026-06-01.md:147:# Determine if .dracon/ contains data that should be tracked or if it's cache
docs/archive/REPOS_CLEANUP_PLAN_2026-06-01.md:153:### Step 5: Run Repair (Optional, After Steps 1-4)
docs/archive/REPOS_CLEANUP_PLAN_2026-06-01.md:156:dracon-sync repair-warns --apply
docs/archive/REPOS_CLEANUP_PLAN_2026-06-01.md:159:### Step 6: Reclaim Disk Space (Optional)
docs/archive/REPOS_CLEANUP_PLAN_2026-06-01.md:174:- **Low risk**: Investigating dracon-ai-lib .dracon/ — just inspection
docs/archive/REPOS_CLEANUP_PLAN_2026-06-01.md:197:- 89 commits ahead of main, 49,095 lines diverged across 19 files
docs/archive/STUCK_PUSH_TRIAGE_2026-06-02.md:14:| 2 | ai-auto-writer | target/debug/deps/ai_auto_writer-* | 283MB | `target/` |
docs/archive/STUCK_PUSH_TRIAGE_2026-06-02.md:16:| 4 | rust-ai-web-auto | target/release/deps/libchromiumoxide_cdp.rlib | 74MB | `target/` |
docs/archive/STUCK_PUSH_TRIAGE_2026-06-02.md:17:| 5 | dracon-ai-lib | target/debug/examples/basic_chat-* | 78MB | `target/` |
docs/archive/STUCK_PUSH_TRIAGE_2026-06-02.md:33:**Note:** For these 5 repos, sync should be able to handle them normally — the modified files just need to be committed and pushed. They may have been marked WARN/CONCERN due to other issues (uncommitted changes, stuck state from earlier failed pushes).
docs/archive/STUCK_PUSH_TRIAGE_2026-06-02.md:45:- **avid, ai-auto-writer, dracon-code, dracon-ai-lib, rust-ai-web-auto**: `--invert-paths --path target/`
docs/archive/STUCK_PUSH_TRIAGE_2026-06-02.md:53:- **High risk**: Force-push to all 4 remotes rewrites history — any existing clones will need to be re-cloned or `git reset --hard origin/main`'d
docs/archive/STUCK_PUSH_TRIAGE_2026-06-02.md:54:- **Medium risk**: If filter-repo misses any large files, the push will still fail
docs/archive/STUCK_PUSH_TRIAGE_2026-06-02.md:65:7. ⏳ Clear STUCK_PUSH state via `dracon-sync repair stuck-unstuck`
docs/design/warden-plaintext-sibling.md:1:# Warden plaintext-sibling escape hatch
docs/design/warden-plaintext-sibling.md:9:A file is treated as **intentionally plaintext** when a sibling file with the
docs/design/warden-plaintext-sibling.md:10:literal suffix `.plaintext` exists next to it in the working tree:
docs/design/warden-plaintext-sibling.md:14:config/example.env.plaintext ← exists ⇒ skip encryption
docs/design/warden-plaintext-sibling.md:18:CLI subcommand. The user runs `touch <path>.plaintext` to opt a file in, and
docs/design/warden-plaintext-sibling.md:19:`rm <path>.plaintext` to revoke.
docs/design/warden-plaintext-sibling.md:27:   — if `<path>.plaintext` exists, return the input unchanged (no encryption,
docs/design/warden-plaintext-sibling.md:28:   no version header, no marker scrub). Plaintext is stored as-is in the git blob.
docs/design/warden-plaintext-sibling.md:31:   originating file has a `.plaintext` sibling. If so, silently allow. If not,
docs/design/warden-plaintext-sibling.md:32:   fail as today. Net behavior: a push with only hatched plaintext is silent;
docs/design/warden-plaintext-sibling.md:33:   a push with un-hatched plaintext still fails.
docs/design/warden-plaintext-sibling.md:34:4. **`scrub-markers`** — skip files that have a `.plaintext` sibling (the
docs/design/warden-plaintext-sibling.md:35:   plaintext content is intentional, not a leaked marker).
docs/design/warden-plaintext-sibling.md:36:5. **`resmudge`** — skip files that have a `.plaintext` sibling (no decryption
docs/design/warden-plaintext-sibling.md:43:## What this does NOT protect against
docs/design/warden-plaintext-sibling.md:45:- The plaintext is stored verbatim in the git object database. Anyone with
docs/design/warden-plaintext-sibling.md:47:- The audit trail is implicit: the only record that a file was intentionally
docs/design/warden-plaintext-sibling.md:48:  plaintext is the existence of `<file>.plaintext` in the working tree /
docs/design/warden-plaintext-sibling.md:52:- `.plaintext` siblings are themselves plaintext tracked files; they do not
docs/design/warden-plaintext-sibling.md:53:  bypass the encryption filter, but they typically contain nothing of value.
docs/design/warden-plaintext-sibling.md:54:- `dracon-warden repair --strict` does NOT have a hatch mode: it still
docs/design/warden-plaintext-sibling.md:55:  reports plaintext committed without a `.plaintext` sibling, so
docs/design/warden-plaintext-sibling.md:56:  "I forgot to add the sibling" remains visible.
docs/design/warden-plaintext-sibling.md:60:`rm <path>.plaintext` → next `git add` triggers the clean filter → file is
docs/design/warden-plaintext-sibling.md:61:encrypted on the next commit. The `.plaintext` sibling itself can be removed
docs/design/warden-plaintext-sibling.md:62:from the repo with `git rm <path>.plaintext`.
docs/design/warden-plaintext-sibling.md:66:- **In scope:** solo or small-team repos where a handful of files contain
docs/design/warden-plaintext-sibling.md:75:- `dracon-warden/src/main.rs` — clean filter, pre-push hook, scrub-markers,
docs/design/warden-plaintext-sibling.md:81:- `dracon-warden/CREDENTIALS.md` — when plaintext is appropriate
docs/design/cli-print-style.md:47:| 🧯 | Repair | `dracon-sync status` (Repair cooldown) |
docs/design/cli-print-style.md:59:| ❌ | Fail / missing | `dracon-system doctor`, status indicators |
docs/design/cli-print-style.md:67:intentionally duplicated (no shared crate) to keep each binary self-contained.
docs/design/cli-print-style.md:125:7. Repair (cooldown, ledger)
docs/design/cli-print-style.md:133:⚠️  Some checks failed. Remediation:
docs/design/cli-print-style.md:137:Run with --json for machine-readable details.
docs/design/cli-print-style.md:174:as plain `WARNING: ...` text after the block.
docs/design/cli-print-style.md:188:Before: `scrub-markers`, `resmudge`, `repair`, `keygen`, `setup-hooks` all
docs/design/cli-print-style.md:200:- **`repair`**: header line `🛠️ repair (dry_run=X, strict=Y) · N repo(s) in
docs/design/cli-print-style.md:201:  scope`, then per-sub-step outputs, then a final summary line (`✅ repair
docs/design/cli-print-style.md:202:  complete · no remaining ciphertext` or `⚠️ repair complete · N ciphertext
docs/design/cli-print-style.md:203:  files remain`).
docs/design/cli-print-style.md:215:no styled footer) and a plain `5 events` footer.
docs/archive/MASTER_ROADMAP_2026-06-01.md:25:| `target/`/`node_modules/`/`node_modules/` build artifact dirs in 34 repos | ~249 GB untracked | ⚠️ Could reclaim with `git clean` |
docs/archive/MASTER_ROADMAP_2026-06-01.md:57:**Root Cause:** The dracon-warden policy had `target/`, `node_modules/`, `.cache/` in `plaintext_patterns`. The daemon generates `!{pattern}` for plaintext patterns, which un-ignored them. Any manual `.gitignore` edits were silently overwritten by the daemon on each pass.
docs/archive/MASTER_ROADMAP_2026-06-01.md:60:- Removed `target/`, `node_modules/`, `.cache/` from `plaintext_patterns` in `~/.dracon/utilities/warden/dracon-warden.toml`
docs/archive/MASTER_ROADMAP_2026-06-01.md:61:- Waited ~60s for daemon to reload and rewrite all 33 `.gitignore` files
docs/archive/MASTER_ROADMAP_2026-06-01.md:63:- Untracked 14,191+ target/ files in 3 repos (ai-auto-writer, avid, dracon-code)
docs/archive/MASTER_ROADMAP_2026-06-01.md:71:**Impact:** 0 `!target/` lines remaining, 0 tracked target/ files. The fix is structural — the daemon will never re-add `!target/`.
docs/archive/MASTER_ROADMAP_2026-06-01.md:83:- **Category D** (1 repo): Data dir only — dracon-ai-lib .dracon/
docs/archive/MASTER_ROADMAP_2026-06-01.md:94:**Status:** Not addressed. Branch `autoresearch/evolutionary-reconciler-2026-05-30` is 49,101 lines diverged from main with 16+ commits of 2-8K line spec changes.
docs/archive/MASTER_ROADMAP_2026-06-01.md:99:git log --oneline main..autoresearch/evolutionary-reconciler-2026-05-30
docs/archive/MASTER_ROADMAP_2026-06-01.md:110:### 2.2 Disk Space Reclamation (~249 GB available) 🟡
docs/archive/MASTER_ROADMAP_2026-06-01.md:112:**Status:** `target/` and `node_modules/` directories exist on disk but are no longer tracked. Could reclaim with `git clean -fdx`.
docs/archive/MASTER_ROADMAP_2026-06-01.md:114:**Caveat:** `.gitignore` only stops tracking — doesn't reclaim disk. `git clean -fdx` permanently deletes untracked files.
docs/archive/MASTER_ROADMAP_2026-06-01.md:138:2. **Better error messages**: The `run repair-warns --apply` hint could include estimated disk space saved.
docs/archive/MASTER_ROADMAP_2026-06-01.md:165:The system has two main daemons:
docs/archive/MASTER_ROADMAP_2026-06-01.md:182:- `plaintext_patterns`: Files matching these are tracked but NOT encrypted (e.g., `Cargo.lock`, `target/` was here)
docs/archive/MASTER_ROADMAP_2026-06-01.md:185:**Conflict to avoid:** A pattern in BOTH `plaintext_patterns` AND `hygiene_patterns` creates a `!pattern` un-ignore in the managed block, which re-includes the pattern for tracking. This was the root cause of the target/ tracking issue.
docs/archive/MASTER_ROADMAP_2026-06-01.md:199:5. **Evidence in goal files is truncated to ~200 chars.** For detailed investigations, write to a standalone markdown file (like REPOS_CLEANUP_PLAN_2026-06-01.md) for full context.
docs/archive/MASTER_ROADMAP_2026-06-01.md:236:- `/home/dracon/Dev/dracon-utilities/tasks.md` (22.6 KB) — Detailed task list
docs/archive/MASTER_ROADMAP_2026-06-01.md:240:- `~/.dracon/utilities/warden/dracon-warden.toml` — Warden policy (modified to remove target/ from plaintext_patterns)
docs/archive/MASTER_ROADMAP_2026-06-01.md:245:- `dracon-utilities/dracon-warden/src/main.rs` — Warden binary (source of !target/ generation logic)
docs/archive/MASTER_ROADMAP_2026-06-01.md:254:**Author:** Pi AI agent (consolidating findings from 4 audit threads)
docs/archive/SPEC.md:20:cargo test --lib --quiet 2>&1 | tail -5
docs/audit/audit-2026-06-07-delta.md:4:**Scope:** Status check + delta against the two 2026-06-06 audits
docs/audit/audit-2026-06-07-delta.md:9:**Baseline:** commit `40a8c381` (HEAD of `main` as of 23:00 UTC 2026-06-06)
docs/audit/audit-2026-06-07-delta.md:11:**Branch:** `main` (clean, only untracked `.pi/goals/active_goal_*.md` from the active pi session)
docs/audit/audit-2026-06-07-delta.md:21:| `cargo fmt --check` | **Failed — CI RED** | **Pass — CI GREEN** | **F-1.3 RESOLVED** |
docs/audit/audit-2026-06-07-delta.md:23:| `cargo test` (serial) | 575 passed, 0 failed | **590 passed, 0 failed** | +15 tests; all pass |
docs/audit/audit-2026-06-07-delta.md:25:| `test-ai` command | Documented in 6 places, missing | **0 references in source or docs** | **F-7.1.2 RESOLVED** |
docs/audit/audit-2026-06-07-delta.md:27:| `dracon-sync/BLUEPRINT.md` AI Integration | Contradictory section | **Rewritten as "Deterministic Commit Protocol"** | **F-7.2.1 RESOLVED** |
docs/audit/audit-2026-06-07-delta.md:33:| Sync.rs / system main.rs / warden main.rs line count | 4340 / 3412 / 2174 | 4469 / 3445 / 2347 | Slightly worse on all 3 monoliths |
docs/audit/audit-2026-06-07-delta.md:35:**Overall:** **Massive improvement** on the CI/clippy/fmt/doc axis — 3 of the 4 RED jobs from yesterday are now GREEN. The `test-ai` cleanup is complete, the freeze-marker incident from 2026-06-04 has a real fix, archived goal files are no longer bloating the repo, and the dead `deny.toml` entries are removed. The remaining P0/P1 surface is much smaller: 1 doc still has the old CLI paths, 2 small repo-hygiene items (note.md + tarpaulin), and a small clippy regression in system/warden.
docs/audit/audit-2026-06-07-delta.md:47:| P1-1 | P1 | `test-ai` command documented but does not exist | ✅ **RESOLVED** | `grep -r "test-ai\|TestAi\|test_ai" AGENTS.md docs/ dracon-sync/README.md dracon-sync/BLUEPRINT.md dracon-sync/src/main.rs` → 0 matches |
docs/audit/audit-2026-06-07-delta.md:49:| P1-3 | P1 | `dracon-sync/BLUEPRINT.md` "AI Integration" contradictory section | ✅ **RESOLVED** | Section rewritten as "Deterministic Commit Protocol" (line 178-188); no `scribe`/`ai-bumper` features |
docs/audit/audit-2026-06-07-delta.md:50:| P2-1 | P2 | 10 duplicate crates in Cargo.lock | 🟡 **PARTIALLY RESOLVED** | 10 duplicates remain (same 10 names: bech32, getrandom, hashbrown, rustc-hash, strsim, syn, toml, toml_datetime, toml_edit, winnow) — `cargo dedupe` not run; the v2 audit's "10" was accurate, the full audit's "20+" was a counting-method error |
docs/audit/audit-2026-06-07-delta.md:52:| P2-3 | P2 | 7 unused license entries in deny.toml | ✅ **RESOLVED** | 0BSD, AGPL-3.0, AGPL-3.0-or-later, CC0-1.0, Unicode-3.0, Unicode-DFS-2016, Zlib all removed; only per-crate exceptions remain (e.g., `Unicode-3.0` for `icu_*` crates) |
docs/audit/audit-2026-06-07-delta.md:62:| F-1.1 | P0 | CI is RED on lint job (clippy) | ✅ **RESOLVED** | `cargo clippy ... -D clippy::style` now exits 0; 4 warnings remain, 0 errors |
docs/audit/audit-2026-06-07-delta.md:65:| F-1.4 | P3 | Pedantic+nursery clippy is decorative | 🟡 **STILL OPEN** | CI still pipes to `tail -1` without pipefail; pedantic lints cannot fail the job |
docs/audit/audit-2026-06-07-delta.md:67:| F-1.7 | P3 | system has 1 production unwrap at main.rs:1262 | 🟡 **STILL OPEN** | (Not re-verified) |
docs/audit/audit-2026-06-07-delta.md:70:| F-1.10 | P3 | system main.rs is 3412 lines | 🟡 **STILL OPEN, unchanged** | Now **3445 lines** (+33) |
docs/audit/audit-2026-06-07-delta.md:71:| F-1.11 | P3 | warden main.rs is 2174 lines | 🟡 **STILL OPEN, worse** | Now **2347 lines** (+173) |
docs/audit/audit-2026-06-07-delta.md:76:| F-6.2 | P3 | EnvRestorer underused | 🟢 **IMPROVED** | Now in 7 files (up from 1): `dracon-sync/src/{git/mod.rs, daemon.rs, main.rs, report.rs, test_helpers.rs}` + `dracon-warden/src/security/{tests/common.rs, tests/security_critical_test.rs}`. 52 occurrences total. |
docs/audit/audit-2026-06-07-delta.md:77:| F-7.1.1 | P0 | AGENTS.md CLI surface wrong | 🟡 **PARTIALLY RESOLVED** | AGENTS.md is now correct; `docs/OPERATIONS.md:127` still has `dracon-sync repair-origins`; `dracon-sync/README.md:122-147` still has 7+ flat paths |
docs/audit/audit-2026-06-07-delta.md:78:| F-7.1.2 | P0 | `test-ai` documented but missing | ✅ **RESOLVED** | See P1-1 above |
docs/audit/audit-2026-06-07-delta.md:79:| F-7.2.1 | P1 | AI Integration section in sync BLUEPRINT | ✅ **RESOLVED** | See P1-3 above |
docs/audit/audit-2026-06-07-delta.md:82:| F-8.1 | P2 | Cargo.lock duplicates (20+ reported) | 🟡 **STILL OPEN** | 10 duplicates remain (v2 audit's count is correct, not the full audit's "20+") |
docs/audit/audit-2026-06-07-delta.md:86:| F-9.1 | P3 | install.sh lacks `set -e` | ✅ **RESOLVED** | `install.sh:2` is now `set -euo pipefail` |
docs/audit/audit-2026-06-07-delta.md:109:$ cargo check --workspace --all-targets 2>&1 | tail -3
docs/audit/audit-2026-06-07-delta.md:149:$ cargo deny check 2>&1 | tail -1
docs/audit/audit-2026-06-07-delta.md:158:## §3 — Documentation Drift — Remaining
docs/audit/audit-2026-06-07-delta.md:163:- dracon-sync repair-origins [--apply]
docs/audit/audit-2026-06-07-delta.md:164:+ dracon-sync repair origins [--apply]
docs/audit/audit-2026-06-07-delta.md:170:- dracon-sync repair-concerns
docs/audit/audit-2026-06-07-delta.md:171:- dracon-sync repair-concerns --apply
docs/audit/audit-2026-06-07-delta.md:172:- dracon-sync repair-warns
docs/audit/audit-2026-06-07-delta.md:173:- dracon-sync repair-warns --apply
docs/audit/audit-2026-06-07-delta.md:177:- dracon-sync dual-branch repair ~/Dev/repo
docs/audit/audit-2026-06-07-delta.md:178:- dracon-sync repair-origins
docs/audit/audit-2026-06-07-delta.md:179:- dracon-sync repair-origins --apply
docs/audit/audit-2026-06-07-delta.md:182:+ dracon-sync repair concerns
docs/audit/audit-2026-06-07-delta.md:183:+ dracon-sync repair concerns --apply
docs/audit/audit-2026-06-07-delta.md:184:+ dracon-sync repair warns
docs/audit/audit-2026-06-07-delta.md:185:+ dracon-sync repair warns --apply
docs/audit/audit-2026-06-07-delta.md:186:+ dracon-sync repair stuck-list
docs/audit/audit-2026-06-07-delta.md:187:+ dracon-sync repair stuck-unstuck ~/Dev/repo
docs/audit/audit-2026-06-07-delta.md:188:+ dracon-sync repair dual-branch-list
docs/audit/audit-2026-06-07-delta.md:189:+ dracon-sync repair dual-branch-repair ~/Dev/repo
docs/audit/audit-2026-06-07-delta.md:190:+ dracon-sync repair origins
docs/audit/audit-2026-06-07-delta.md:191:+ dracon-sync repair origins --apply
docs/audit/audit-2026-06-07-delta.md:200:| Doc | Claim | Actual | Status |
docs/audit/audit-2026-06-07-delta.md:214:The 1-line file is a leftover investigation note from an unrelated repo incident. It was identified yesterday as P2-5 and remains.
docs/audit/audit-2026-06-07-delta.md:267:- (plus 1 more in system tests, `discover` unused var at `dracon-warden/src/main.rs:1356`)
docs/audit/audit-2026-06-07-delta.md:275:- `dracon-system/src/main.rs`: 3412 → 3445 (+33)
docs/audit/audit-2026-06-07-delta.md:276:- `dracon-warden/src/main.rs`: 2174 → 2347 (+173)
docs/audit/audit-2026-06-07-delta.md:278:The CHANGELOG references a 0.3.0 release with a `repo_roots` rename for warden; the +173 lines for warden is consistent with the rename refactor. Not a regression, but worth noting that the architectural goal of < 1500 lines per main.rs is not yet met.
docs/audit/audit-2026-06-07-delta.md:286:While AGENTS.md is now correct, `dracon-sync/README.md` and `docs/OPERATIONS.md` still show the old flat paths. This is the same finding as 2026-06-06's P1-2, but partially resolved. Since AGENTS.md is the AI-facing reference, this is lower severity for AI workflows but still P3 for human readers.
docs/audit/audit-2026-06-07-delta.md:308:total: 590 passed, 0 failed
docs/audit/audit-2026-06-07-delta.md:315:total: 590 passed, 0 failed
docs/audit/audit-2026-06-07-delta.md:322:390 passed; 28 failed
docs/audit/audit-2026-06-07-delta.md:325:Same parallel-test failures as documented in AGENTS.md (PATH mutation, port collisions, env leakage). All 28 are in `git::tests`, `sync::tests`, `report::tests`, `release::tests`, `daemon::daemon_tests`, `sync::diff_tests`. CI uses `--test-threads=1` so these are noise, not regressions.
docs/audit/audit-2026-06-07-delta.md:335:| 1 | **Fix `dracon-sync/README.md` and `docs/OPERATIONS.md` flat CLI paths** | 7+ commands in README, 1 in OPERATIONS.md use old flat form (`repair-concerns`, `stuck list`, `dual-branch list`, `publish-status`) | Rewrite both using the same nested-subcommand syntax now in AGENTS.md (lines 516-535) | 10 min | zero |
docs/audit/audit-2026-06-07-delta.md:338:| 4 | **Silence dead-code warnings on `print.rs` helpers (8 new clippy warnings)** | system + warden both warn about unused `pub fn format_bytes/format_secs/should_color/onoff` | Add `#[allow(dead_code)]` on the `print` modules with a doc comment explaining they're public API awaiting callers | 5 min | zero |
docs/audit/audit-2026-06-07-delta.md:341:| 7 | **Fix the 4 remaining clippy warnings in sync** | `unused import: tokio_git_command` (report.rs:93), `field stop_reason is never read` (sync.rs:965), `field title is never read` (sync.rs:978), `function test_deletions_committed_when_intentional is never used` (sync.rs:3841) | Remove the unused import; `#[allow(dead_code)]` on the fields with comments, or wire them into the Goal metadata serialization; add `#[test]` to the dead test or remove it | 10 min | zero |
docs/audit/audit-2026-06-07-delta.md:343:| 9 | **Make pedantic+nursery clippy gate the build** | The CI step pipes output to `tail -1` so pedantic lints can never fail the job | Add `set -euo pipefail` and check `${PIPESTATUS[0]}` or grep for "warning:" in output | 15 min | low (might surface a wave of new warnings) |
docs/audit/audit-2026-06-07-delta.md:354:| Lines (largest .rs) | 4469 (sync.rs) | 3445 (main.rs) | 2347 (main.rs) | 10261 |
docs/audit/audit-2026-06-07-delta.md:365:| lint (clippy pedantic) | ⚠️ Decorative | tail -1 only |
docs/audit/audit-2026-06-07-delta.md:375:### `cargo deny` details
docs/audit/audit-2026-06-07-delta.md:401:- [x] `cargo test --test-threads=1` workspace: 590 passed, 0 failed (log: test-system-warden.log, test-sync.log, test-release.log)
docs/audit/audit-2026-06-07-delta.md:413:**The 2026-06-06 audit's most important findings have been addressed.** Three CI jobs (clippy, fmt, docs) flipped from RED to GREEN in 24 hours. The `test-ai` cleanup, freeze-marker TTL, archived-goals gitignore, and dead `deny.toml` entries are all done. AGENTS.md is now correct.
docs/audit/audit-2026-06-07-delta.md:415:**Remaining surface area is much smaller and lower-severity:**
docs/audit/audit-2026-06-07-delta.md:422:- 1 deferred (Cargo.lock dedupe, awaits `dracon-libs` pin)
docs/audit/audit-2026-06-07-delta.md:425:1. Fix `dracon-sync/README.md` and `docs/OPERATIONS.md` (10 min) → unblocks humans and AI agents who read those files
docs/audit/audit-2026-06-07-delta.md:430:6. Then tackle sync.rs modularization (8-12 h) — the biggest remaining architectural work
docs/audit/audit-2026-06-07-delta-summary.md:7:The 2026-06-06 audit's most important findings have been addressed. **3 CI jobs (clippy, fmt, docs) flipped from RED to GREEN** in 24 hours. The `test-ai` cleanup, freeze-marker TTL, archived-goals gitignore, and dead `deny.toml` entries are all done. AGENTS.md is now correct.
docs/audit/audit-2026-06-07-delta-summary.md:29:- F-7.1.2: `test-ai` references all removed (was 6 places)
docs/audit/audit-2026-06-07-delta-summary.md:31:- F-7.2.1: `dracon-sync/BLUEPRINT.md` "AI Integration" section rewritten
docs/audit/audit-2026-06-07-delta-summary.md:38:- F-9.1: `install.sh` has `set -euo pipefail`
docs/audit/audit-2026-06-07-delta-summary.md:44:## What remains
docs/audit/audit-2026-06-07-delta-summary.md:51:- Fix 4 remaining clippy warnings in sync (`tokio_git_command` import, dead `stop_reason`/`title` fields, dead `test_deletions_committed_when_intentional`)
docs/audit/audit-2026-06-07-delta-summary.md:52:- Update test counts in AGENTS.md (claims 686, real 590) and project-state.md (claims 575, real 590)
docs/audit/audit-2026-06-07-delta-summary.md:55:- `cargo dedupe` (1-2 h, awaits `dracon-libs` pin)
docs/audit/audit-2026-06-07-delta-summary.md:66:1. **Fix the 2 docs with flat CLI paths** (10 min) — unblocks humans and AI agents
docs/audit/audit-2026-06-07-delta-summary.md:70:Total: **~25 minutes** to clear all the remaining P3 surface.
docs/archive/dracon-sync-architecture.md:1:# Ratio & Fact Reporting — AI-to-AI Commit Architecture
docs/archive/dracon-sync-architecture.md:5:**The commit message is not written by an AI. It's written by a dumb deterministic script.**
docs/archive/dracon-sync-architecture.md:7:The Worker AI is an untrusted, chaotic coder that edits files. The Committer is a deterministic auditor that extracts raw data and stamps a routing key.
docs/archive/dracon-sync-architecture.md:15:The AI Coding Agent operates in an isolated sandbox. It has **no Git knowledge**. It just:
docs/archive/dracon-sync-architecture.md:30:# Extract what the Worker newly claimed to close
docs/archive/dracon-sync-architecture.md:54:  - verification: {tests_passed: 42, tests_failed: 0}
docs/archive/dracon-sync-architecture.md:61:When downstream AI queries `git show <hash>`:
docs/archive/dracon-sync-architecture.md:85:        "tests_failed": 0
docs/archive/dracon-sync-architecture.md:104:## How Downstream AI Consumes This
docs/archive/dracon-sync-architecture.md:106:### Debugging AI
docs/archive/dracon-sync-architecture.md:119:### Project Manager AI
docs/archive/dracon-sync-architecture.md:125:### Janitor AI (Ghost Code Finder)
docs/archive/dracon-sync-architecture.md:136:### Revert AI (Suspicious Batch Detector)
docs/archive/dracon-sync-architecture.md:155:Downstream AI sees `5 checked` vs `2 files`. **Flag as suspicious.**
docs/archive/dracon-sync-architecture.md:166:Downstream AI sees `0 checked` but `1 file modified`. **Flag as unanchored.**
docs/archive/dracon-sync-architecture.md:169:Worker runs out of tokens, leaves tests failing.
docs/archive/dracon-sync-architecture.md:174:       verification.tests_failed = 2
docs/archive/dracon-sync-architecture.md:177:CI/CD doesn't auto-revert because the title says `2 checked` — the Work completed. Tests failing is just bad luck, not incomplete work.
docs/archive/dracon-sync-architecture.md:181:## Why Deterministic Beats AI for Commits
docs/archive/dracon-sync-architecture.md:183:| Aspect | AI Commit (LLM) | Deterministic Commit (Script) |
docs/archive/dracon-sync-architecture.md:189:| **Downstream AI value** | Low (must parse prose) | High (structured data) |
docs/archive/dracon-sync-architecture.md:192:**The verdict:** Deterministic commits aren't "better" — they're the **only** way to build an AI-to-AI system that doesn't hallucinate.
docs/archive/dracon-sync-architecture.md:194:AI commits are fine for human-readable history in a repo that humans might occasionally browse. But for AI-to-AI, deterministic commits are superior because:
docs/archive/dracon-sync-architecture.md:198:4. Downstream AI can grep for exactly what it needs
docs/archive/dracon-sync-architecture.md:205:- The commit is generated by deterministic extraction, not generative AI
docs/archive/dracon-sync-architecture.md:206:- AI writes code. AI updates ledger. Environment audits.
docs/archive/dracon-sync-architecture.md:215:- It just reports raw counts and lets downstream AI judge
docs/archive/dracon-sync-architecture.md:219:- Downstream AI queries it programmatically
docs/archive/dracon-sync-architecture.md:235:│  AI Worker (Untrusted)                                      │
docs/archive/dracon-sync-architecture.md:246:│  1. Parse TODO.md diff → extract claims                    │
docs/archive/dracon-sync-architecture.md:260:│  Git Log (Database for Downstream AI)                         │
docs/archive/dracon-sync-architecture.md:269:│  Downstream AI Agents                                        │
docs/archive/dracon-sync-architecture.md:270:│  - Debugging AI: traces errors to commits                    │
docs/archive/dracon-sync-architecture.md:271:│  - Project Manager AI: queries ledger state                  │
docs/archive/dracon-sync-architecture.md:272:│  - Janitor AI: finds ghost code                               │
docs/archive/dracon-sync-architecture.md:273:│  - Revert AI: identifies suspicious commits                   │
docs/archive/dracon-sync-architecture.md:281:This architecture captures the paradigm shift required for AI-to-AI version control:
docs/archive/dracon-sync-architecture.md:285:3. **The Git log is a database** — downstream AI queries it programmatically
docs/audit/audit-2026-06-06.md:20:| Docs accuracy | ❌ P1 drift | `test-ai` documented but missing; broken CLI paths in 3 docs |
docs/audit/audit-2026-06-06.md:29:### P1-1: `test-ai` command documented but does not exist
docs/audit/audit-2026-06-06.md:32:- `dracon-sync/README.md:113` — `# Test AI provider connectivity / dracon-sync test-ai`
docs/audit/audit-2026-06-06.md:33:- `dracon-sync/BLUEPRINT.md:241` — `dracon-sync test-ai   # Test all AI providers`
docs/audit/audit-2026-06-06.md:34:- `dracon-sync/BLUEPRINT.md:280` — `[x] test-ai command for provider verification`
docs/audit/audit-2026-06-06.md:35:- `AGENTS.md:518` — `  test-ai          Test AI providers connectivity`
docs/audit/audit-2026-06-06.md:36:- `AGENTS.md:639` — `dracon-sync test-ai`
docs/audit/audit-2026-06-06.md:37:- `docs/OPERATIONS.md:179` — `dracon-sync test-ai`
docs/audit/audit-2026-06-06.md:39:**Reality:** The `Command` enum in `dracon-sync/src/main.rs` has no `TestAi` variant. `grep test.ai\|TestAi\|test_ai dracon-sync/src/main.rs` returns nothing.
docs/audit/audit-2026-06-06.md:41:This contradicts the v1 audit's positive finding "Deterministic commit messages — No LLM at the commit boundary" — if AI scribe was removed, why does `test-ai` still appear in docs?
docs/audit/audit-2026-06-06.md:43:**Fix:** Either implement the `test-ai` command (it was part of the AI Configuration flow documented in `docs/OPERATIONS.md` and `AGENTS.md` § AI Configuration) or remove all 6 references.
docs/audit/audit-2026-06-06.md:47:The actual CLI uses nested subcommands under `repair`, `config`, and `publish`, but docs show old flat-path forms:
docs/audit/audit-2026-06-06.md:51:| `dracon-sync repair-concerns` | `dracon-sync repair concerns` |
docs/audit/audit-2026-06-06.md:52:| `dracon-sync repair-warns` | `dracon-sync repair warns` |
docs/audit/audit-2026-06-06.md:53:| `dracon-sync repair-origins` | `dracon-sync repair origins` |
docs/audit/audit-2026-06-06.md:54:| `dracon-sync stuck list` | `dracon-sync repair stuck-list` |
docs/audit/audit-2026-06-06.md:55:| `dracon-sync dual-branch` | `dracon-sync repair dual-branch-list` |
docs/audit/audit-2026-06-06.md:62:- `AGENTS.md:510-523` — 5 broken commands (repair-concerns, repair-warns, edit-config, dual-branch, repair-origins)
docs/audit/audit-2026-06-06.md:63:- `docs/OPERATIONS.md:120-121` — 2 broken commands (dual-branch-list, dual-branch-repair)
docs/audit/audit-2026-06-06.md:67:**Fix:** Rewrite all three docs using the actual nested-subcommand syntax. The actual command list is in the `Command` enum in `dracon-sync/src/main.rs`.
docs/audit/audit-2026-06-06.md:69:### P1-3: sync BLUEPRINT has contradictory "AI Integration" section
docs/audit/audit-2026-06-06.md:73:The BLUEPRINT contains "## AI Integration (Scribe + AI Bumper)" that claims:
docs/audit/audit-2026-06-06.md:74:- "dracon-sync has integrated AI for generating commit messages (scribe)"
docs/audit/audit-2026-06-06.md:75:- "Scribe: AI generates commit subjects from diffs"
docs/audit/audit-2026-06-06.md:76:- "AI Bumper: AI decides semver bump level"
docs/audit/audit-2026-06-06.md:78:But line 281 says: `[x] AI scribe removed (was not useful for AI workflows)`
docs/audit/audit-2026-06-06.md:80:And `AGENTS.md` states: "AI scribe was removed as AI-generated messages were not useful for AI workflows."
docs/audit/audit-2026-06-06.md:82:**Fix:** Delete the "AI Integration (Scribe + AI Bumper)" section (lines 180-189) and the related "Features (compile-time)" entries for `scribe` and `ai-bumper`. Keep the Status item that documents the removal.
docs/audit/audit-2026-06-06.md:148:| `field title is never read` | `dracon-sync/src/sync.rs:974` (TaskDetail struct) | Remove field or use `#[allow(dead_code)]` |
docs/audit/audit-2026-06-06.md:168:The v1 audit contained 8 material errors identified by the auditor:
docs/audit/audit-2026-06-06.md:170:| # | v1 claim | v2 correction |
docs/audit/audit-2026-06-06.md:177:| 3 | `test-ai` command not mentioned | `test-ai` documented in 6 places but doesn't exist — new P1 |
docs/audit/audit-2026-06-06.md:186:**Root cause of v1 errors:** The grep filter `grep -vE '#\[(tokio::)?test\]'` only excludes lines that contain the test annotation marker, not lines inside test modules that don't have annotations on every line. The v1 hotspot file counts (sync.rs:281, etc.) were entirely from `#[cfg(test)] mod tests { ... }` regions.
docs/audit/audit-2026-06-06.md:247:| `dracon-sync/BLUEPRINT.md` | ❌ P1 drift | Contradictory AI section; `test-ai` command documented but missing |
docs/audit/audit-2026-06-06.md:252:| `docs/OPERATIONS.md` | ❌ P1 drift | 2 broken CLI commands; `test-ai` referenced |
docs/audit/audit-2026-06-06.md:253:| `AGENTS.md` | ❌ P1 drift | 5 broken CLI commands (lines 510-523); `test-ai` referenced (lines 518, 639) |
docs/audit/audit-2026-06-06.md:259:1. **Decide on `test-ai` command** (P1-1) — implement it or remove 6 doc references
docs/audit/audit-2026-06-06.md:261:3. **Delete contradictory AI Integration section in sync BLUEPRINT** (P1-3) — affects documentation accuracy
docs/audit/2026-06-11-full-repo-audit/inventory.tsv:2:/home/dracon/Dev/one-mil-girls	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/inventory.tsv:3:/home/dracon/Dev/pully-fully-pull-based-fleet-reconciler	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/inventory.tsv:4:/home/dracon/Dev/folder-auto-banner	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/inventory.tsv:5:/home/dracon/Dev/dracon-platform	main	2	0	3	0	0	DIRTY	OK	run repair-warns --apply
docs/audit/2026-06-11-full-repo-audit/inventory.tsv:6:/home/dracon/Dev/browser-extensions-shared	main	3	0	3	0	0	DIRTY	OK	run repair-warns --apply
docs/audit/2026-06-11-full-repo-audit/inventory.tsv:7:/home/dracon/Dev/ai-auto-writer	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/inventory.tsv:8:/home/dracon/Dev/video-factory	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/inventory.tsv:9:/home/dracon/Dev/rust-ai-web-auto	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/inventory.tsv:10:/home/dracon/Dev/video-uploader	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/inventory.tsv:11:/home/dracon/Dev/dracon-code	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/inventory.tsv:12:/home/dracon/Dev/dracon-utilities	main	0	0	1	0	0	DIRTY	OK	healthy
docs/audit/2026-06-11-full-repo-audit/inventory.tsv:13:/home/dracon/Dev/avid	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/inventory.tsv:14:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	main	0	0	1	0	0	DIRTY	OK	healthy
docs/audit/2026-06-11-full-repo-audit/inventory.tsv:15:/home/dracon/Dev/youtube-video-uploader	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/inventory.tsv:16:/home/dracon/.dracon	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/inventory.tsv:17:/home/dracon/Dev/dracon-ai-lib	main	0	0	0	13	0	AHEAD:13,STUCK_PUSH	STUCK	run repair-concerns --apply (push or rewrite)
docs/audit/2026-06-11-full-repo-audit/inventory.tsv:19:/home/dracon/Dev/dracon-libs	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/inventory.tsv:20:/home/dracon/Dev/kiki-sassy-desktop-announcer	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/inventory.tsv:21:/home/dracon/Dev/DraconDev	main	2	0	1	0	0	DIRTY	OK	run repair-warns --apply
docs/audit/2026-06-11-full-repo-audit/final-validation.tsv:4:ai-auto-repo-rot-scanner-todo-agent	1	0	0	
docs/audit/2026-06-11-full-repo-audit/final-validation.tsv:8:ai-auto-writer	0	0	0	
docs/audit/2026-06-11-full-repo-audit/final-validation.tsv:10:rust-ai-web-auto	0	0	0	
docs/audit/2026-06-11-full-repo-audit/final-validation.tsv:15:dracon-ai-lib	0	0	0	
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-utilities.risk.tsv:49:tracked	.ralph/cleanup-remaining.md
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-utilities.risk.tsv:50:tracked	.ralph/cleanup-remaining.state.json
docs/audit/2026-06-11-full-repo-audit/inventory.json:8:  "failures": 0,
docs/audit/2026-06-11-full-repo-audit/inventory.json:15:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/inventory.json:38:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/inventory.json:61:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/inventory.json:84:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/inventory.json:93:      "last_msg": "1 file(s) in apis [apis/services/ai-api/src/ai/client/mod.rs] DELTA:+5/…",
docs/audit/2026-06-11-full-repo-audit/inventory.json:100:      "hint": "run repair-warns --apply"
docs/audit/2026-06-11-full-repo-audit/inventory.json:107:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/inventory.json:123:      "hint": "run repair-warns --apply"
docs/audit/2026-06-11-full-repo-audit/inventory.json:126:      "repo": "/home/dracon/Dev/ai-auto-writer",
docs/audit/2026-06-11-full-repo-audit/inventory.json:130:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/inventory.json:153:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/inventory.json:172:      "repo": "/home/dracon/Dev/rust-ai-web-auto",
docs/audit/2026-06-11-full-repo-audit/inventory.json:176:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/inventory.json:185:      "last_msg": "docs(audit): document Dracon AI lib adoption + Section 7/8/9/10 renumbe…",
docs/audit/2026-06-11-full-repo-audit/inventory.json:199:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/inventory.json:222:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/inventory.json:231:      "last_msg": "4 file(s) in docs,plan [docs/AI-LIB-AUDIT.md, docs/README.md, docs/AI-S…",
docs/audit/2026-06-11-full-repo-audit/inventory.json:245:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/inventory.json:268:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/inventory.json:277:      "last_msg": "19 file(s) in examples,src,tests [src/cli.rs, src/ai_gen.rs, src/analyz…",
docs/audit/2026-06-11-full-repo-audit/inventory.json:287:      "repo": "/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent",
docs/audit/2026-06-11-full-repo-audit/inventory.json:291:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/inventory.json:300:      "last_msg": "refactor(ai): migrate from archived dracon-ai-lib to ai-api-sdk",
docs/audit/2026-06-11-full-repo-audit/inventory.json:314:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/inventory.json:337:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/inventory.json:356:      "repo": "/home/dracon/Dev/dracon-ai-lib",
docs/audit/2026-06-11-full-repo-audit/inventory.json:361:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/inventory.json:370:      "last_msg": "1 file(s) in crates [crates/ai-lib/src/providers/minimax.rs] DELTA:+48/…",
docs/audit/2026-06-11-full-repo-audit/inventory.json:374:      "push_error": "ahead=13, push failing",
docs/audit/2026-06-11-full-repo-audit/inventory.json:377:      "hint": "run repair-concerns --apply (push or rewrite)"
docs/audit/2026-06-11-full-repo-audit/inventory.json:407:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/inventory.json:430:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/inventory.json:453:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/inventory.json:469:      "hint": "run repair-warns --apply"
docs/audit/2026-06-11-full-repo-audit/release-readiness/inventory-current.json:8:  "failures": 0,
docs/audit/2026-06-11-full-repo-audit/release-readiness/inventory-current.json:15:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/release-readiness/inventory-current.json:31:      "hint": "run repair-warns --apply"
docs/audit/2026-06-11-full-repo-audit/release-readiness/inventory-current.json:38:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/release-readiness/inventory-current.json:61:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/release-readiness/inventory-current.json:77:      "hint": "run repair-warns --apply"
docs/audit/2026-06-11-full-repo-audit/release-readiness/inventory-current.json:100:      "hint": "run repair-warns --apply"
docs/audit/2026-06-11-full-repo-audit/release-readiness/inventory-current.json:107:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/release-readiness/inventory-current.json:123:      "hint": "run repair-warns --apply"
docs/audit/2026-06-11-full-repo-audit/release-readiness/inventory-current.json:130:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/release-readiness/inventory-current.json:153:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/release-readiness/inventory-current.json:172:      "repo": "/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent",
docs/audit/2026-06-11-full-repo-audit/release-readiness/inventory-current.json:176:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/release-readiness/inventory-current.json:199:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/release-readiness/inventory-current.json:215:      "hint": "run repair-warns --apply"
docs/audit/2026-06-11-full-repo-audit/release-readiness/inventory-current.json:218:      "repo": "/home/dracon/Dev/ai-auto-writer",
docs/audit/2026-06-11-full-repo-audit/release-readiness/inventory-current.json:222:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/release-readiness/inventory-current.json:245:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/release-readiness/inventory-current.json:254:      "last_msg": "1 file(s) in plugins [plugins/default-ai-providers/src/lib.rs] DELTA:+2…",
docs/audit/2026-06-11-full-repo-audit/release-readiness/inventory-current.json:261:      "hint": "run repair-warns --apply"
docs/audit/2026-06-11-full-repo-audit/release-readiness/inventory-current.json:264:      "repo": "/home/dracon/Dev/rust-ai-web-auto",
docs/audit/2026-06-11-full-repo-audit/release-readiness/inventory-current.json:268:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/release-readiness/inventory-current.json:291:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/release-readiness/inventory-current.json:314:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/release-readiness/inventory-current.json:333:      "repo": "/home/dracon/Dev/dracon-ai-lib",
docs/audit/2026-06-11-full-repo-audit/release-readiness/inventory-current.json:337:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/release-readiness/inventory-current.json:360:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/release-readiness/inventory-current.json:383:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/release-readiness/inventory-current.json:406:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/risk-paths/one-mil-girls.risk.tsv:13:tracked	.pi/goals/archived/goal_2026060318184883_mpyaiftl-mv1cfy.md
docs/audit/2026-06-11-full-repo-audit/risk-paths/one-mil-girls.risk.tsv:56:tracked	docs/audit/visual-qa/2026-06-10-post-inspiration-polish/01-main-menu.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/one-mil-girls.risk.tsv:79:tracked	docs/audit/visual-qa/after/main-menu.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/one-mil-girls.risk.tsv:83:tracked	docs/audit/visual-qa/before/main-menu.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/one-mil-girls.risk.tsv:98:tracked	docs/audit/visual-qa/convo-redesign-before/main-screen.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/one-mil-girls.risk.tsv:100:tracked	docs/audit/visual-qa/crops/main-menu-center.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/one-mil-girls.risk.tsv:102:tracked	docs/audit/visual-qa/crops/vn-dialogue-portrait.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/one-mil-girls.risk.tsv:113:tracked	docs/audit/visual-qa/effects-after/main-screen.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/one-mil-girls.risk.tsv:116:tracked	docs/audit/visual-qa/effects-before/main-screen.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/avid.risk.tsv:60:tracked	.ralph/videoai-pilot.md
docs/audit/2026-06-11-full-repo-audit/risk-paths/avid.risk.tsv:61:tracked	.ralph/videoai-pilot.state.json
docs/audit/2026-06-11-full-repo-audit/risk-paths/pully-fully-pull-based-fleet-reconciler.risk.tsv:22:tracked	.ralph/analysis/AI-OPS.md
docs/audit/2026-06-11-full-repo-audit/risk-paths/pully-fully-pull-based-fleet-reconciler.risk.tsv:41:tracked	dracon-fleet/env/platform/ai-api-service.env
docs/audit/audit-2026-06-06-full.md:17:| `cargo fmt --check` (CI flags) | ❌ **Fails** | `dracon-warden/tests/integration_test.rs:209` reformatting required — **CI is RED on lint job** |
docs/audit/audit-2026-06-06-full.md:25:| `test-ai` command (referenced in 6 doc locations) | ❌ Does not exist | P1 doc/code drift |
docs/audit/audit-2026-06-06-full.md:26:| Flat-vs-nested CLI command paths in AGENTS.md | ❌ Outdated | 12+ top-level commands are now nested (`repair concerns`, `config edit`, `publish run`, etc.) |
docs/audit/audit-2026-06-06-full.md:27:| `Cargo.lock` duplicate entries | ⚠️ **20+ duplicate crates** | vs. v2 audit's claim of 10; 5 crates have 3 versions |
docs/audit/audit-2026-06-06-full.md:31:**Overall:** The project is **functionally solid** and recently de-slopped (the CLI audit's recommendations have been largely applied). However, **the CI pipeline is red on the current `main` branch in at least two jobs (lint, docs)**. Several doc-vs-code drifts remain from the v2 audit. The biggest new findings are: (a) CI is broken on `main`, (b) 35 archived goal markdown files are tracked in git, (c) duplicate crate count is double the v2 estimate, (d) several AGENTS.md/CHANGELOG claims are stale (test count, modularization progress).
docs/audit/audit-2026-06-06-full.md:37:1. **Fix CI to be green on `main`.** The lint job fails on `cargo fmt --check` and `cargo clippy`; the docs job fails on `RUSTDOCFLAGS=-D warnings`. Without fixing, no one notices new lint regressions. **Effort:** 30 min. **Risk:** zero. (See F-1.1, F-1.2, F-1.3.)
docs/audit/audit-2026-06-06-full.md:38:2. **Update AGENTS.md, OPERATIONS.md, and `dracon-sync/README.md` to reflect nested subcommands.** The current AGENTS.md and OPERATIONS.md still show the old flat `repair-concerns`/`edit-config`/`stuck`/`dual-branch` paths, which no longer exist. This is the #1 user-facing defect. **Effort:** 1–2 h. **Risk:** zero. (See F-2.1.)
docs/audit/audit-2026-06-06-full.md:39:3. **Decide the fate of `test-ai`.** It is documented in 6 places and marked `[x] completed` in `dracon-sync/BLUEPRINT.md:280`, but the `TestAi` variant does not exist in the `Command` enum. Either implement it or remove all 6 references. **Effort:** 30 min (remove) or 2–4 h (implement). **Risk:** zero. (See F-2.2.)
docs/audit/audit-2026-06-06-full.md:40:4. **Stop tracking `.pi/goals/archived/*.md` in git.** 35 archived AI-session goal files are committed, bloating the repo and exposing ephemeral session state. Move them out of git (keep them on disk) or gitignore the `archived/` subdir. **Effort:** 15 min. **Risk:** zero. (See F-7.1.)
docs/audit/audit-2026-06-06-full.md:42:6. **Sync the test-count claim in AGENTS.md and `.dracon/project-state.md`.** AGENTS.md says "406 tests in `src/`"; `.dracon/project-state.md` says "All 706 tests passing". Actual `cargo test` count: **575 passed**. **Effort:** 5 min. **Risk:** zero. (See F-2.3.)
docs/audit/audit-2026-06-06-full.md:46:10. **Investigate the one-off `warden` test failure observed during audit.** Across 15 runs of `cargo test -p dracon-warden --tests`, 1 run showed `63 passed; 1 failed`. Subsequent 10 runs all passed. Probably a stale `.git/index.lock` from a crashed test in a sibling repo, but worth a one-time investigation. **Effort:** 30 min. **Risk:** zero. (See F-3.4.)
docs/audit/audit-2026-06-06-full.md:60:| 5 | Reliability & safety (--apply gates, repair, incident ledger, systemd) | Covered | §5 |
docs/audit/audit-2026-06-06-full.md:63:| 8 | Dependencies & supply chain (duplicates, deny, MSRV) | Covered | §8 |
docs/audit/audit-2026-06-06-full.md:73:**Evidence (run 2026-06-06 on `main`, with `../dracon-libs` cloned):**
docs/audit/audit-2026-06-06-full.md:102:- **F-1.2 [P0] CI is broken on `main`**: The CI workflow at `.github/workflows/ci.yml` runs this exact command in the **lint** job (no `--all-targets`, but the same flags). The exit code from `cargo clippy` on error is non-zero. This means **CI has been red on `main` for at least as long as `field_reassign_with_default` has been an error**. The previous v2 audit declared clippy "PASS with 4 warnings" — that was wrong; the v2 audit ran with different (looser) flags.
docs/audit/audit-2026-06-06-full.md:103:- **Why it matters:** Without a green CI, new lint regressions are invisible. The whole point of `-D` is to fail on the violation; the failure is being ignored.
docs/audit/audit-2026-06-06-full.md:122:-    assert!(help.contains("dracon-warden"), "help should mention binary name");
docs/audit/audit-2026-06-06-full.md:123:-    assert!(help.contains("setup-hooks"), "help should list setup-hooks command");
docs/audit/audit-2026-06-06-full.md:125:+        help.contains("dracon-warden"),
docs/audit/audit-2026-06-06-full.md:129:+        help.contains("setup-hooks"),
docs/audit/audit-2026-06-06-full.md:134:- **F-1.3 [P0] CI is broken on `main` (fmt)**: The lint job's first step runs `cargo fmt --check` and would have failed. The fix is to reformat the file with `cargo fmt`.
docs/audit/audit-2026-06-06-full.md:151:- **F-1.4 [P0] CI is broken on `main` (docs)**: The `docs` CI job sets `RUSTDOCFLAGS=-D warnings` and would fail on the 4 unresolved intra-doc links. The two visible are at `dracon-sync/src/sync.rs:1544` and `:1547`; the other 2 are likely the same file.
docs/audit/audit-2026-06-06-full.md:156:**Evidence:** The CI workflow has a separate `Clippy (pedantic — warnings only)` step that runs with `-W clippy::pedantic -W clippy::nursery` and pipes to `tail -1`. Output:
docs/audit/audit-2026-06-06-full.md:159:$ cargo clippy -p dracon-sync -p dracon-system -p dracon-warden -- -W clippy::pedantic -W clippy::nursery 2>&1 | tail -1
docs/audit/audit-2026-06-06-full.md:163:- 87+ pedantic/nursery rule violations. The CI only takes the last line of output, so it cannot fail on these (no exit-code check). This is intentional ("warnings only"), but means pedantic violations are not actually gate-kept.
docs/audit/audit-2026-06-06-full.md:164:- **F-1.5 [P3] Pedantic clippy is decorative**: If pedantic code quality is a goal, the step should `set -o pipefail` or check `exit ${PIPESTATUS[0]}`. Currently the only signal is "the line ends with `+N more rules`" — humans must visit the run logs to see what changed.
docs/audit/audit-2026-06-06-full.md:165:- **Remediation:** Add `set -euo pipefail` and `! grep -q "warning:" <<<"$output"` to the pedantic step, OR add `#![warn(clippy::pedantic)]` to each crate root. **Effort:** 15 min. **Risk:** low (might surface a wave of new warnings).
docs/audit/audit-2026-06-06-full.md:178:- **F-1.6 [P3] sync has 2 production unwraps** at `dracon-sync/src/sync.rs:1814` and `:1838`. Both are inside a `.filter(|t| t.evidence.is_some())` / `.filter(|t| t.skip_reason.is_some())` chain followed by `.as_ref().unwrap()`. The filter guarantees presence, so they cannot panic, but should be `.expect("filtered above")` or refactored to `if let Some(ev) = ...`. **Effort:** 5 min. **Risk:** zero.
docs/audit/audit-2026-06-06-full.md:179:- **F-1.7 [P3] system has 1 production unwrap** at `dracon-system/src/main.rs:1262`, on a `cache.lock().unwrap().insert(...)` Mutex. If the mutex is poisoned, the daemon panics. Not unusual for a static cache, but worth using `.lock().expect("cache mutex")` or replacing with `parking_lot::Mutex` (which can't poison). **Effort:** 5 min. **Risk:** zero.
docs/audit/audit-2026-06-06-full.md:180:- The v2 audit reported "17 production unwrap in sync, 3 in system, 6 in warden" using a naive grep. My Python brace-tracker is more accurate; **the v2 audit's production-unwrap numbers are wrong** (undercounted for sync, overcounted for warden).
docs/audit/audit-2026-06-06-full.md:186:- `dracon-sync/src/sync.rs:974` — `TaskDetail.title` field is never read. Either remove the field, prefix with `_`, or add `#[allow(dead_code)]` with a comment explaining intent. **Effort:** 2 min. **Risk:** zero.
docs/audit/audit-2026-06-06-full.md:193:3412 dracon-system/src/main.rs
docs/audit/audit-2026-06-06-full.md:196:2174 dracon-warden/src/main.rs
docs/audit/audit-2026-06-06-full.md:200:1307 dracon-sync/src/main.rs
docs/audit/audit-2026-06-06-full.md:203:- **F-1.9 [P3] `dracon-sync/src/sync.rs` is 4340 lines.** Per CHANGELOG ("Module extraction: `branch.rs`, `config.rs`, `diff.rs`, ... — 1,846 lines, 45% git/mod.rs reduction"), sync has been actively modularizing — the `git/mod.rs` shrank from ~4700 to 2611, but `sync.rs` is now the new monolith at 4340. Worth the same incremental extraction treatment, but the REFACTORING_BLOCKER_ANALYSIS.md (H-DAEMON) shows previous extraction attempts were reverted due to borrow-checker pain. **Suggested approach:** copy the pattern that worked for `git/mod.rs` (top-down extraction of sub-modules per the CHANGELOG). **Effort:** 8–12 h. **Risk:** medium (historical reverts).
docs/audit/audit-2026-06-06-full.md:204:- **F-1.10 [P3] `dracon-system/src/main.rs` is 3412 lines.** Per CHANGELOG and project-state.md, this is mid-refactor (events, links extracted; zram, doctor, safety pending). The project-state says "3,926 → 3,484 lines. Remaining: guard, storage, zram, doctor, safety". **Status: in progress, partially complete.** **Effort:** already underway. **Risk:** medium (the same coupling that blocked H-SEC-LIB).
docs/audit/audit-2026-06-06-full.md:205:- **F-1.11 [P3] `dracon-warden/src/main.rs` is 2174 lines.** REFACTORING_BLOCKER_ANALYSIS.md §H-SEC-LIB says a full split was attempted and reverted. The recommendation was "Option A (incremental)". **Status: not started, deferred.** **Effort:** 6–8 h. **Risk:** medium.
docs/audit/audit-2026-06-06-full.md:216:- `dracon-warden` is self-contained (vendored `dracon-security-kit` in `dracon-warden/src/security/`)
docs/audit/audit-2026-06-06-full.md:218:The `dracon-warden` self-vendoring is interesting — it's the only binary that doesn't depend on `dracon-libs`. This is a deliberate isolation choice (the security code base is small and the binary ships independently). It also means the v2 audit's claim that "duplicate crates come from dracon-libs" is partially false: the workspace pulls 2 versions of `toml` even with `dracon-warden`'s self-vendored security, because `dracon-warden` directly uses `toml = "0.8"` while `dracon-libs` transitively brings an older `toml_edit` → `toml`.
docs/audit/audit-2026-06-06-full.md:222:**Implementation:** RAII guard at `dracon-sync/src/git/status.rs:20` (sync) and `dracon-warden/src/main.rs:916` (warden). Both:
docs/audit/audit-2026-06-06-full.md:226:4. Have a `bypass()` constructor for one-shot commands (`once`, `repair`)
docs/audit/audit-2026-06-06-full.md:229:**Use sites (warden):** `main.rs:1034` (during `harden_repo` → `publish_repo_pubkey`), `main.rs:1027` (explicit `IndexLock::bypass()` for once/repair).
docs/audit/audit-2026-06-06-full.md:259:`grep -rnE 'canonicalize' dracon-sync/src/ dracon-system/src/ dracon-warden/src/` (excludes tests via Python strip): 0 production uses in sync, 0 in warden, 0 in system. **Wait** — the v2 audit claimed 6 production `canonicalize()` calls (4 in `dracon-system/src/safety.rs`). Let me re-verify.
docs/audit/audit-2026-06-06-full.md:266:**F-3.2 [P2] v2 audit's `canonicalize` claim is wrong (v2 audit drift).** The v2 audit (audit-2026-06-06.md §Positive Findings and §Statistics) said "10 production `canonicalize()` calls, 8 in `dracon-system/src/safety.rs`" and "6 `canonicalize()` calls in production, with 4 in `dracon-system/src/safety.rs`". My grep finds **0** in `safety.rs` (or anywhere in production code). The v2 audit's path-validation positive finding is therefore not corroborated.
docs/audit/audit-2026-06-06-full.md:274:(TBD — outside audit scope. But this is a useful negative finding: the v2 audit made a specific claim that the audit could not reproduce.)
docs/audit/audit-2026-06-06-full.md:286:✅ No plaintext secrets in tracked code. `.env.example` and similar templates are allowlisted.
docs/audit/audit-2026-06-06-full.md:292:- pre-push: Scans for plaintext secrets as defense-in-depth
docs/audit/audit-2026-06-06-full.md:296:(no such path; hooks are inline strings in main.rs)
docs/audit/audit-2026-06-06-full.md:301:### 3.5 One-off test failure observed
docs/audit/audit-2026-06-06-full.md:303:During the audit, `cargo test -p dracon-warden --tests` failed once with `1 failed` out of 64. The failure could not be reproduced across 10 subsequent runs. Suspected causes:
docs/audit/audit-2026-06-06-full.md:308:**F-3.4 [P2] Flaky test in dracon-warden.** Captured one failure across ~15 runs. Cannot pinpoint the test without more information. Recommended: add `--test-threads=1 --nocapture` rerun, or run each test in isolation. **Effort:** 30 min investigation. **Risk:** zero.
docs/audit/audit-2026-06-06-full.md:310:### 3.6 Pre-push plaintext-secret scanning
docs/audit/audit-2026-06-06-full.md:330:Verified against actual service files:
docs/audit/audit-2026-06-06-full.md:332:| Setting | AGENTS.md claim | dracon-sync.service | dracon-system-guard.service |
docs/audit/audit-2026-06-06-full.md:370:`grep -nE 'apply.*bool' dracon-sync/src/main.rs dracon-system/src/main.rs` shows:
docs/audit/audit-2026-06-06-full.md:373:dracon-sync/src/main.rs:147:   apply: bool,   # repair concerns
docs/audit/audit-2026-06-06-full.md:374:dracon-sync/src/main.rs:174:   apply: bool,   # repair warns
docs/audit/audit-2026-06-06-full.md:375:dracon-sync/src/main.rs:186:   apply: bool,   # repair origins
docs/audit/audit-2026-06-06-full.md:376:dracon-system/src/main.rs:170: apply: bool,   # storage --cleanup
docs/audit/audit-2026-06-06-full.md:377:dracon-system/src/main.rs:261: apply: bool,   # guard prune
docs/audit/audit-2026-06-06-full.md:378:dracon-system/src/main.rs:268: apply: bool,   # guard clean
docs/audit/audit-2026-06-06-full.md:379:dracon-system/src/main.rs:323: apply: bool,   # ... more
docs/audit/audit-2026-06-06-full.md:380:dracon-system/src/main.rs:739: apply: bool,
docs/audit/audit-2026-06-06-full.md:381:dracon-system/src/main.rs:854: apply: bool,
docs/audit/audit-2026-06-06-full.md:382:dracon-system/src/main.rs:1032: apply: bool,  # docker_prune
docs/audit/audit-2026-06-06-full.md:383:dracon-system/src/main.rs:1102: apply: bool,
docs/audit/audit-2026-06-06-full.md:384:dracon-system/src/main.rs:1130: apply: bool,
docs/audit/audit-2026-06-06-full.md:385:dracon-system/src/main.rs:1166: apply: bool,  # empty_trash
docs/audit/audit-2026-06-06-full.md:386:dracon-system/src/main.rs:1268: apply: bool,  # clean_nix_garbage
docs/audit/audit-2026-06-06-full.md:387:dracon-system/src/main.rs:1340: apply: bool,
docs/audit/audit-2026-06-06-full.md:400:### 5.4 Repair commands
docs/audit/audit-2026-06-06-full.md:402:All `repair` subcommands are `--apply`-gated and dry-run by default. The CLI surface (`dracon-sync repair --help`) shows:
docs/audit/audit-2026-06-06-full.md:409:- `dual-branch-repair` (no apply; explicit per-repo repair)
docs/audit/audit-2026-06-06-full.md:411:✅ Safety pattern is consistent: report-only commands have no `--apply`; repair commands default to dry-run and require `--apply` to mutate.
docs/audit/audit-2026-06-06-full.md:435:**F-6.1.1 [P2] AGENTS.md claim is stale.** AGENTS.md says: "**406 tests** in `src/`". Actual: 420 in sync alone. Total workspace: 575. **Remediation:** update the count or remove the specific number from AGENTS.md.
docs/audit/audit-2026-06-06-full.md:437:**F-6.1.2 [P2] `.dracon/project-state.md` claim is also stale.** Says: "All **706** tests passing after both extractions". Actual: 575. **Remediation:** re-run tests and update, or remove the specific number.
docs/audit/audit-2026-06-06-full.md:469:The proptest-regressions directory contains a single regression file from a security test. This is the standard proptest mechanism for recording failing test cases. ✅
docs/audit/audit-2026-06-06-full.md:473:AGENTS.md notes: "~10-20 tests fail unpredictably when running with default parallelism. Root causes: (1) `std::process::Command::new("git")` resolves from `PATH`, which concurrent tests modify for mock binaries; (2) `acquire_path_lock()` only serializes the subset of tests that explicitly acquire it; (3) some sync tests start TCP listeners on fixed ports for mock registries."
docs/audit/audit-2026-06-06-full.md:485:status, repos, health, metrics, once, daemon, sync-now, pause, resume, config, repair, publish, scaffold
docs/audit/audit-2026-06-06-full.md:488:**AGENTS.md §CLI Reference (`dracon-sync`) claims (15 top-level commands):**
docs/audit/audit-2026-06-06-full.md:490:status, validate-config, repos, repair-concerns, repair-warns, once, daemon, sync-now,
docs/audit/audit-2026-06-06-full.md:491:pause, resume, edit-config, test-ai, health, metrics, stuck, dual-branch, repair-origins,
docs/audit/audit-2026-06-06-full.md:499:| `repair-concerns` | `repair concerns` |
docs/audit/audit-2026-06-06-full.md:500:| `repair-warns` | `repair warns` |
docs/audit/audit-2026-06-06-full.md:502:| `test-ai` | **does not exist** |
docs/audit/audit-2026-06-06-full.md:503:| `stuck` (with subcommands) | `repair stuck-list` / `repair stuck-unstuck` |
docs/audit/audit-2026-06-06-full.md:504:| `dual-branch` (with subcommands) | `repair dual-branch-list` / `repair dual-branch-repair` |
docs/audit/audit-2026-06-06-full.md:505:| `repair-origins` | `repair origins` |
docs/audit/audit-2026-06-06-full.md:510:- **F-7.1.2 [P0] `test-ai` is documented in 6 places but does not exist.** Per `dracon-sync/BLUEPRINT.md:280`: `- [x] test-ai command for provider verification` (marked Completed). Per AGENTS.md:518 and AGENTS.md:639, per `dracon-sync/README.md:113`, per `dracon-sync/BLUEPRINT.md:241`, per `docs/OPERATIONS.md:179`. `grep TestAi dracon-sync/src/main.rs` returns 0. **Either implement it (2–4 h) or remove all 6 references (30 min).**
docs/audit/audit-2026-06-06-full.md:518:Line 280:   - [x] `test-ai` command for provider verification ❌ command does not exist
docs/audit/audit-2026-06-06-full.md:519:Line 281:   - [x] AI scribe removed                            ✅ (and the section above contradicts this — see below)
docs/audit/audit-2026-06-06-full.md:522:**F-7.2.1 [P1] Lines 180–189 of `dracon-sync/BLUEPRINT.md` describe an "AI Integration (Scribe + AI Bumper)" section that contradicts line 281.** Per v2 audit finding P1-3, the BLUEPRINT still has a "Features (compile-time)" block describing `scribe` and `ai-bumper` that the code doesn't have. **Remediation:** delete the contradictory section. **Effort:** 5 min.
docs/audit/audit-2026-06-06-full.md:534:### 7.4 `dracon-system` (the `--binaries-only` doc claim)
docs/audit/audit-2026-06-06-full.md:542:### 7.5 `OPERATIONS.md` claims about resource limits
docs/audit/audit-2026-06-06-full.md:548:## §8 — Dependencies & Supply Chain
docs/audit/audit-2026-06-06-full.md:561:- **F-8.1 [P2] v2 audit undercounted duplicates.** v2 said "10 duplicate crates" (bech32, getrandom, hashbrown, rustc-hash, strsim, syn, toml, toml_datetime, toml_edit, winnow). My count shows 20+, with `getrandom` and `hashbrown` actually having 3 versions. The v2 audit likely looked at a single Cargo.lock subfile (sync/, system/, or warden/) rather than the workspace lock.
docs/audit/audit-2026-06-06-full.md:587:`rust-toolchain.toml: channel = "stable"`. No explicit `rust-version` in any Cargo.toml. The CI's `msrv` job uses stable + clippy. ✅
docs/audit/audit-2026-06-06-full.md:621:- Sets git default branch to `main` (line 91–93).
docs/audit/audit-2026-06-06-full.md:624:- **F-9.1 [P3] `install.sh` is not `set -e`/`set -u`/`set -o pipefail` at the top.** Most of the script uses `|| true` and explicit error handling, but a stray failure could be silently swallowed. (Verified: `head -3 install.sh` does not show `set -e`.) **Effort:** 1 h to add `set -euo pipefail` after the args parser. **Risk:** medium (might surface latent bugs in error paths).
docs/audit/audit-2026-06-06-full.md:638:`dracon-sync/Cargo.toml: notify-rust = "4"`. The `gh` CLI is invoked as a subprocess for visibility/metadata sync. No `gh auth status` check at startup; failures propagate as HTTP errors. **Not a finding** — the design assumes `gh` is pre-authenticated.
docs/audit/audit-2026-06-06-full.md:646:All `repair`/`storage --cleanup`/`guard clean`/`guard prune` commands default to dry-run and require explicit `--apply`. Verified by the `--help` outputs of each. ✅
docs/audit/audit-2026-06-06-full.md:654:`scripts/verify-spec.sh` checks 3 invariants: project compiles, no FIXMEs/BLOCKINGs, unit tests pass. **F-9.7.1 [P2] Uses `cargo test --lib`** which only works for library crates. This is a binary crate workspace, so the check would fail:
docs/audit/audit-2026-06-06-full.md:684:- **F-10.1 [P1] Archived goal files are tracked in git.** 35 markdown files in `.pi/goals/archived/` are committed. AGENTS.md says `.pi/goals/*.md` is "managed by pi (auto-sync)" and "Sync daemon auto-commits" — so the active goal file is intentionally tracked. But the `archived/` subdir contains goals that are no longer active. **Why it matters:** repo bloat (each goal file is ~5–20 KB, so ~500 KB total) and ephemeral session state is exposed. **Remediation:** add `.pi/goals/archived/` to `.gitignore`, or `git rm -r --cached .pi/goals/archived/` and add the gitignore entry. **Effort:** 5 min. **Risk:** zero.
docs/audit/audit-2026-06-06-full.md:704:- **F-10.4 [P3] `debug.log` is untracked but not gitignored.** A 4 KB log file from a May 10 terminal-spawn debug session is sitting in the repo root. Not in `.gitignore`. **Remediation:** add `debug.log` to `.gitignore` (or `*.log` if not already covered — it is). Wait, let me check.
docs/audit/audit-2026-06-06-full.md:739:Lines 1–80 are `# --- BEGIN DRACON MANAGED BLOCK ---` to `# --- END DRACON MANAGED BLOCK ---`, containing:
docs/audit/audit-2026-06-06-full.md:771:$ head -3 dracon-sync/src/main.rs
docs/audit/audit-2026-06-06-full.md:805:| `cargo fmt --check` (CI) | **Fails → CI RED** |
docs/audit/audit-2026-06-06-full.md:807:| `cargo test --test-threads=1` workspace | **575 passed, 0 failed** |
docs/audit/audit-2026-06-06-full.md:826:| Path validation via `canonicalize()` in production | 0 (v2 audit's claim of 6 was wrong) |
docs/audit/audit-2026-06-06-full.md:829:| IndexLock coordination | Implemented in sync and warden (O_EXCL, RAII) |
docs/audit/audit-2026-06-06-full.md:838:| `dracon-sync/README.md` | ❌ P1 drift | 7 broken CLI paths; `test-ai` referenced |
docs/audit/audit-2026-06-06-full.md:841:| `dracon-sync/BLUEPRINT.md` | ❌ P1 drift | Contradictory AI section; `test-ai` marked done; `- [x]` in In Progress |
docs/audit/audit-2026-06-06-full.md:846:| `docs/OPERATIONS.md` | ❌ P1 drift | 2 broken CLI commands; `test-ai` referenced |
docs/audit/audit-2026-06-06-full.md:847:| `AGENTS.md` | ❌ P1 drift | 12+ broken CLI paths; `test-ai` referenced; test count wrong (406 vs 575) |
docs/audit/audit-2026-06-06-full.md:877:$ cargo check --workspace --all-targets 2>&1 | tail -3
docs/audit/audit-2026-06-06-full.md:886:    -D clippy::complexity -D clippy::perf -D clippy::style 2>&1 | tail -3
docs/audit/audit-2026-06-06-full.md:903:$ RUSTDOCFLAGS=-D warnings cargo doc -p dracon-sync -p dracon-system -p dracon-warden --no-deps 2>&1 | tail -5
docs/audit/audit-2026-06-06-full.md:911:$ cargo test -p dracon-sync -p dracon-system -p dracon-warden -- --test-threads=1 2>&1 | tail -3
docs/audit/audit-2026-06-06-full.md:914:$ cargo test -p dracon-sync -- --test-threads=1 2>&1 | tail -3
docs/audit/audit-2026-06-06-full.md:917:$ cargo test -p dracon-system -- --test-threads=1 2>&1 | tail -3
docs/audit/audit-2026-06-06-full.md:920:$ cargo test -p dracon-warden -- --test-threads=1 2>&1 | tail -3
docs/audit/audit-2026-06-06-full.md:927:$ cargo deny check 2>&1 | tail -3
docs/audit/audit-2026-06-06-full.md:930:$ cargo deny check licenses 2>&1 | tail -2
docs/audit/audit-2026-06-06-full.md:954:          resume, config, repair, publish, scaffold
docs/audit/audit-2026-06-06-full.md:956:$ ./target/release/dracon-sync repair --help
docs/audit/audit-2026-06-06-full.md:958:          dual-branch-list, dual-branch-repair
docs/audit/audit-2026-06-06-full.md:970:Commands: status, once, scrub-markers, resmudge, repair, filter-clean,
docs/audit/audit-2026-06-06-full.md:1040:3. **Updated findings the v2 audit claimed as PASS**: canonicalize() count in production is 0 (not 6 or 10), and clippy with CI flags is RED (not 4 warnings).
docs/audit/audit-2026-06-06-full.md:1073:1. **CI is broken on `main`** — should be fixed before merging more code. Apply fixes in F-1.2, F-1.3, F-1.4.
docs/audit/audit-2026-06-06-full.md:1074:2. **AGENTS.md CLI table is wrong** — any AI agent reading it will run broken commands. Apply F-7.1.1.
docs/audit/audit-2026-06-06-full.md:1075:3. **`test-ai` is documented but missing** — AI agents will try to run it and fail. Apply F-7.1.2.
docs/audit/audit-2026-06-06-full.md:1078:The remaining P2/P3 items can be batched into a follow-up audit cycle.
docs/audit/2026-06-11-full-repo-audit/final/inventory.tsv:2:/home/dracon/Dev/browser-extensions-shared	main	0	0	2	0	0	DIRTY	OK	healthy
docs/audit/2026-06-11-full-repo-audit/final/inventory.tsv:3:/home/dracon/Dev/dracon-utilities	main	1	0	1	0	0	DIRTY	OK	run repair-warns --apply
docs/audit/2026-06-11-full-repo-audit/final/inventory.tsv:4:/home/dracon/.dracon	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/final/inventory.tsv:5:/home/dracon/Dev/dracon-platform	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/final/inventory.tsv:6:/home/dracon/Dev/one-mil-girls	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/final/inventory.tsv:7:/home/dracon/Dev/folder-auto-banner	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/final/inventory.tsv:8:/home/dracon/Dev/dracon-code	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/final/inventory.tsv:10:/home/dracon/Dev/ai-auto-writer	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/final/inventory.tsv:11:/home/dracon/Dev/video-uploader	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/final/inventory.tsv:12:/home/dracon/Dev/rust-ai-web-auto	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/final/inventory.tsv:13:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/final/inventory.tsv:14:/home/dracon/Dev/DraconDev	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/final/inventory.tsv:15:/home/dracon/Dev/pully-fully-pull-based-fleet-reconciler	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/final/inventory.tsv:16:/home/dracon/Dev/video-factory	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/final/inventory.tsv:17:/home/dracon/Dev/avid	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/final/inventory.tsv:18:/home/dracon/Dev/youtube-video-uploader	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/final/inventory.tsv:19:/home/dracon/Dev/dracon-ai-lib	main	0	0	0	13	0	AHEAD:13,STUCK_PUSH	STUCK	run repair-concerns --apply (push or rewrite)
docs/audit/2026-06-11-full-repo-audit/final/inventory.tsv:20:/home/dracon/Dev/dracon-libs	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/final/inventory.tsv:21:/home/dracon/Dev/kiki-sassy-desktop-announcer	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/risk-paths/ai-auto-writer.risk.tsv:47:tracked	_archive/to-release/The-Silence-Between-Tokens-vera-kincaid/PUBLICATION-README.md
docs/audit/2026-06-11-full-repo-audit/risk-paths/ai-auto-writer.risk.tsv:48:tracked	_archive/to-release/The-Silence-Between-Tokens-vera-kincaid/The-Silence-Between-Tokens.epub
docs/audit/2026-06-11-full-repo-audit/risk-paths/ai-auto-writer.risk.tsv:49:tracked	_archive/to-release/The-Silence-Between-Tokens-vera-kincaid/The-Silence-Between-Tokens.mobi
docs/audit/2026-06-11-full-repo-audit/risk-paths/ai-auto-writer.risk.tsv:50:tracked	_archive/to-release/The-Silence-Between-Tokens-vera-kincaid/cover.jpg
docs/audit/2026-06-11-full-repo-audit/risk-paths/ai-auto-writer.risk.tsv:51:tracked	_archive/to-release/The-Silence-Between-Tokens-vera-kincaid/outline.md
docs/audit/2026-06-11-full-repo-audit/risk-paths/ai-auto-writer.risk.tsv:52:tracked	_archive/to-release/The-Silence-Between-Tokens-vera-kincaid/publication-metadata.json
docs/audit/2026-06-11-full-repo-audit/risk-paths/ai-auto-writer.risk.tsv:124:tracked	authors/fiction/grace-monroe/books/duke-scandal-secret/.outline-detailed.json
docs/audit/2026-06-11-full-repo-audit/risk-paths/Junk-Runner-bevy.risk.tsv:42:tracked	.pi/goals/notes_dev_failu[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSB0NWNVeUlzZWllRFRFUy85RkNzZ293blV1TWR0bFlielplYU9TNk1US1hZCnB5UGFJa2M2bzVMTGVVcWd4b1BrL1NaRW15bm02ZWhTYzE2M1V3amR4aWMKLT4gWDI1NTE5IDdKdEcvdHpGYXlWa2FvMXk2ZmR1bWduRHU5OGdPRHB1UklGdDU1K2VsVFUKd0lXWElUb0lYZEZ5KzFPTlFJYXRHWnpLNWYvTlNxcW1Db0VkVFZrS3c5dwotPiBYMjU1MTkgMitXeVFpNU92Z256RkVIZ2pOV1RrYVpoYnA3Y2FKdk1ZLzBrd1dQbTdDdwpKTkpER1lNei9MY2dwQ2M2dEwwU1BpZlJMR2lrR0xUWmY3cE0zK0cyM0xZCi0+IFgyNTUxOSAyczRSdzgwS3E0blJnNXlMRzNZazRxYUhqbzBKYnM4dk1XdjVNeWpPcWxvCm1PanMwZE15QllGWG9leXBzeXRublFub2NBdjB4MFV5eHc0VmFtTWxWa2cKLT4gWDI1NTE5IHdtQ0t2bjNFUFBoVnVxb1hZcnNjK042c2oyQUp0WVJ6VWRhNmJDRjNER3MKRzk2NVVpWWd2U2dCVUlKWFFTZXR5RWNZclNDSnkvY3Avc1VEdHFESWJEQQotPiBYMjU1MTkgM0ZUaDdVaTNEak5lNEFub3p2Q1VoOE5BWHZkV2tjN2lPSWhpK3VGdzFCbwpWTHdrMjBiY1p6cm4wQkthY2srdW02d0xhaG41Mjc4a2xzZGVCMVE4TlRnCi0+IFgyNTUxOSBPSGY5eEpXc3RmTkxHcXZJcUgyQ3ZvSStmRGVETDc4K2MwSlp6bWwxcENjCkJ5UzF4d0lTbWN3M29WaWdQNVR6MXlKTDVkdWljR1dBYzE0TzZIV2RBcHMKLT4gQFMtZ3JlYXNlClZhN2cza1A2UW00cUFncwotLS0gcjI3TFFueEs3anhqRUY0UWVpcGFJMk8wOXgrTlA1ZVN6YmpXSEk3VlRCVQofFMdFan10wT4qLUG05KISSgNNkyXERttYhnYXEkYyDQAcciFJmAXT2PhzTW7O+eR8eBG24kJdJfc/XbPU].md
docs/audit/2026-06-11-full-repo-audit/risk-paths/Junk-Runner-bevy.risk.tsv:117:tracked	assets/audio/sfx_repair.wav
docs/audit/2026-06-11-full-repo-audit/risk-paths/Junk-Runner-bevy.risk.tsv:121:tracked	assets/audio/sfx_system_failure.wav
docs/audit/2026-06-11-full-repo-audit/release-readiness/REPORT.md:10:Main blockers:
docs/audit/2026-06-11-full-repo-audit/release-readiness/REPORT.md:14:   - Current/reachable history contains local agent/task state, audit artifacts, operational logs, and secret-shaped fixture strings that need explicit cleanup/approval before public exposure.
docs/audit/2026-06-11-full-repo-audit/release-readiness/REPORT.md:17:   - GitHub push is OK, but GitLab mirror `main` is protected with push access set to `No one`.
docs/audit/2026-06-11-full-repo-audit/release-readiness/REPORT.md:21:   - No unexplained `CONCERN`/`STUCK_PUSH` remains after fetching `dracon-platform`.
docs/audit/2026-06-11-full-repo-audit/release-readiness/REPORT.md:22:   - Remaining rows are `DIRTY`/`WARN` with `push_status=OK`, caused by preserved user changes and branch state.
docs/audit/2026-06-11-full-repo-audit/release-readiness/REPORT.md:23:   - `Junk-Runner-bevy` is currently on `tauri2` with local changes; pushing `HEAD` to `origin main` is rejected because remote `main` has work not integrated locally.
docs/audit/2026-06-11-full-repo-audit/release-readiness/REPORT.md:43:dracon-platform                       2        0      0         0     OK          DIRTY     run repair-warns --apply
docs/audit/2026-06-11-full-repo-audit/release-readiness/REPORT.md:44:browser-extensions-shared             1        0      4         0     OK          DIRTY     run repair-warns --apply
docs/audit/2026-06-11-full-repo-audit/release-readiness/REPORT.md:45:Junk-Runner-bevy                      4        0      0         0     OK          DIRTY     run repair-warns --apply
docs/audit/2026-06-11-full-repo-audit/release-readiness/REPORT.md:46:dracon-utilities                      1        0      0         0     OK          DIRTY     run repair-warns --apply
docs/audit/2026-06-11-full-repo-audit/release-readiness/REPORT.md:47:pully-fully-pull-based-fleet-reconciler 2      0      1         0     OK          DIRTY     run repair-warns --apply
docs/audit/2026-06-11-full-repo-audit/release-readiness/REPORT.md:48:dracon-code                           2        0      0         0     OK          DIRTY     run repair-warns --apply
docs/audit/2026-06-11-full-repo-audit/release-readiness/REPORT.md:51:Interpretation: these are release-readiness warnings, not hidden sync failures. They need review/preservation decisions before release.
docs/audit/2026-06-11-full-repo-audit/release-readiness/REPORT.md:60:- `/home/dracon/Dev/dracon-utilities/docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/REPORT.md`
docs/audit/2026-06-11-full-repo-audit/release-readiness/REPORT.md:86:### `rust-ai-web-auto`
docs/audit/2026-06-11-full-repo-audit/release-readiness/REPORT.md:114:git rev-list --count main ^origin/main = 0
docs/audit/2026-06-11-full-repo-audit/release-readiness/REPORT.md:115:git rev-list --count origin/main ^main   = 0
docs/audit/2026-06-11-full-repo-audit/release-readiness/REPORT.md:116:git push --dry-run origin main           = Everything up-to-date
docs/audit/2026-06-11-full-repo-audit/release-readiness/REPORT.md:119:After `git fetch origin main`, the stale ahead count cleared. Current WARN is preserved user changes.
docs/audit/2026-06-11-full-repo-audit/release-readiness/REPORT.md:123:Current branch is `tauri2`, not `main`:
docs/audit/2026-06-11-full-repo-audit/release-readiness/REPORT.md:129:  main
docs/audit/2026-06-11-full-repo-audit/release-readiness/REPORT.md:133:Pushing current `HEAD` to `origin main` is rejected:
docs/audit/2026-06-11-full-repo-audit/release-readiness/REPORT.md:136:! [rejected] main -> main (fetch first)
docs/audit/2026-06-11-full-repo-audit/release-readiness/REPORT.md:137:error: failed to push some refs to 'https://github.com/DraconDev/Junk-Runner-bevy.git'
docs/audit/2026-06-11-full-repo-audit/release-readiness/REPORT.md:138:hint: Updates were rejected because the remote contains work that you do not have locally.
docs/audit/2026-06-11-full-repo-audit/release-readiness/REPORT.md:149:! [remote rejected] main -> main (pre-receive hook declined)
docs/audit/2026-06-11-full-repo-audit/release-readiness/REPORT.md:152:GitLab API confirms `main` push access is `No one`.
docs/audit/2026-06-11-full-repo-audit/release-readiness/REPORT.md:159:Required decision: unprotect/adjust GitLab `main` push access, push to an unprotected mirror branch, or remove the GitLab mirror remote for this repo.
docs/audit/2026-06-11-full-repo-audit/release-readiness/REPORT.md:185:   - For `one-mil-girls`, decide whether to unprotect/adjust GitLab `main`, use an unprotected branch, or remove the GitLab mirror from release scope.
docs/audit/2026-06-11-full-repo-audit/release-readiness/REPORT.md:188:   - For `Junk-Runner-bevy`, decide whether `tauri2` is release content or whether release should target `main`.
docs/audit/2026-06-11-full-repo-audit/release-readiness/REPORT.md:196:The project is **not release-ready yet**. The technical validation picture is mostly good for the affected Rust repos, and the sync notification/concern work is fixed/surfaced. The release blockers are governance/public-readiness and mirror-policy decisions, not unresolved compiler/test failures.
docs/audit/2026-06-11-full-repo-audit/risk-paths/folder-auto-banner.risk.tsv:27:tracked	.pi/goals/archived/goal_2026060520100089_mq1aifn7-6yezq4.md
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-before.json:8:  "failures": 0,
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-before.json:17:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-before.json:30:      "push_error": "ahead=2, push failing",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-before.json:33:      "hint": "run repair-concerns --apply (push or rewrite)"
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-before.json:40:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-before.json:56:      "hint": "run repair-warns --apply"
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-before.json:63:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-before.json:79:      "hint": "run repair-warns --apply"
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-before.json:105:      "repo": "/home/dracon/Dev/rust-ai-web-auto",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-before.json:109:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-before.json:125:      "hint": "run repair-warns --apply"
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-before.json:132:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-before.json:141:      "last_msg": "2 file(s) in docs [docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-…",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-before.json:148:      "hint": "run repair-warns --apply"
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-before.json:155:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-before.json:174:      "repo": "/home/dracon/Dev/dracon-ai-lib",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-before.json:178:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-before.json:201:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-before.json:224:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-before.json:240:      "hint": "run repair-warns --apply"
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-before.json:247:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-before.json:263:      "hint": "run repair-warns --apply"
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-before.json:266:      "repo": "/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-before.json:270:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-before.json:289:      "repo": "/home/dracon/Dev/ai-auto-writer",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-before.json:293:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-before.json:316:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-before.json:339:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-before.json:362:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-before.json:371:      "last_msg": "19 file(s) in examples,src,tests [src/cli.rs, src/ai_gen.rs, src/analyz…",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-before.json:385:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-before.json:408:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-before.json:8:  "failures": 0,
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-before.json:15:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-before.json:24:      "last_msg": "5 file(s) in web [web/ai-hub/{.cache.preserve/.cache.preserve => .cache…",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-before.json:38:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-before.json:61:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-before.json:77:      "hint": "run repair-warns --apply"
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-before.json:84:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-before.json:107:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-before.json:126:      "repo": "/home/dracon/Dev/rust-ai-web-auto",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-before.json:130:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-before.json:153:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-before.json:176:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-before.json:195:      "repo": "/home/dracon/Dev/dracon-ai-lib",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-before.json:200:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-before.json:213:      "push_error": "ahead=28, push failing",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-before.json:216:      "hint": "run repair-concerns --apply (push or rewrite)"
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-before.json:246:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-before.json:265:      "repo": "/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-before.json:269:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-before.json:292:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-before.json:311:      "repo": "/home/dracon/Dev/ai-auto-writer",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-before.json:315:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-before.json:338:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-before.json:361:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-before.json:384:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-before.json:393:      "last_msg": "19 file(s) in examples,src,tests [src/cli.rs, src/ai_gen.rs, src/analyz…",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-before.json:407:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-before.json:430:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-before.json:453:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:59:tracked	apis/services/ai-api/.env.dev
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:60:tracked	apis/services/ai-api/.env.example
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:61:tracked	apis/services/ai-api/.env.prod
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:62:tracked	apis/services/ai-api/ai-api-sdk/.env.example
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:63:tracked	apis/services/ai-api/src/handlers/image.rs
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:71:tracked	apis/services/email-api/.env.dev
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:72:tracked	apis/services/email-api/.env.example
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:73:tracked	apis/services/email-api/.env.prod
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:82:tracked	web/ai-hub/src/lib/chrome.config.ts
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:83:tracked	web/ai-hub/src/lib/types/chrome.d.ts
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:164:tracked	web/games-hosted/games/junk-runner/assets/sfx_repair-CnxSomk4.wav
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:168:tracked	web/games-hosted/games/junk-runner/assets/sfx_system_failure-DlAFpe7h.wav
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:208:tracked	web/screenshots/ai-hub-current/affiliates-fold.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:209:tracked	web/screenshots/ai-hub-current/affiliates.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:210:tracked	web/screenshots/ai-hub-current/compare-fold.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:211:tracked	web/screenshots/ai-hub-current/compare.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:212:tracked	web/screenshots/ai-hub-current/directory-fold.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:213:tracked	web/screenshots/ai-hub-current/directory.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:214:tracked	web/screenshots/ai-hub-current/free-fold.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:215:tracked	web/screenshots/ai-hub-current/free.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:216:tracked	web/screenshots/ai-hub-current/plans-fold-800.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:217:tracked	web/screenshots/ai-hub-current/plans-fold-final.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:218:tracked	web/screenshots/ai-hub-current/plans-fold.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:219:tracked	web/screenshots/ai-hub-current/plans-fullpage.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:220:tracked	web/screenshots/ai-hub-current/plans-intro.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:221:tracked	web/screenshots/ai-hub-current/plans-narrow.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:222:tracked	web/screenshots/ai-hub-current/plans-redbox-absolute.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:223:tracked	web/screenshots/ai-hub-current/plans-scrolled.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:224:tracked	web/screenshots/ai-hub-current/plans-top-400.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:225:tracked	web/screenshots/ai-hub-current/plans-top-debug.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:226:tracked	web/screenshots/ai-hub-current/plans-with-redbox.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:227:tracked	web/screenshots/ai-hub-current/plans.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:228:tracked	web/screenshots/ai-hub-current/promos-fold.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:229:tracked	web/screenshots/ai-hub-current/promos.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:230:tracked	web/screenshots/ai-hub-current/providers-fold.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:231:tracked	web/screenshots/ai-hub-current/providers.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:232:tracked	web/screenshots/ai-hub-current/rankings-fold.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:233:tracked	web/screenshots/ai-hub-current/rankings.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:234:tracked	web/screenshots/chrome-consistency/ai-hub-affiliates-signed-in.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:235:tracked	web/screenshots/chrome-consistency/ai-hub-affiliates-signed-out.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:236:tracked	web/screenshots/chrome-consistency/ai-hub-compare-signed-in.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:237:tracked	web/screenshots/chrome-consistency/ai-hub-compare-signed-out.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:238:tracked	web/screenshots/chrome-consistency/ai-hub-free-signed-in.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:239:tracked	web/screenshots/chrome-consistency/ai-hub-free-signed-out.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:240:tracked	web/screenshots/chrome-consistency/ai-hub-index-v2.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:241:tracked	web/screenshots/chrome-consistency/ai-hub-plans-signed-in.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:242:tracked	web/screenshots/chrome-consistency/ai-hub-plans-signed-out.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:243:tracked	web/screenshots/chrome-consistency/ai-hub-promos-signed-in.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:244:tracked	web/screenshots/chrome-consistency/ai-hub-promos-signed-out.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:245:tracked	web/screenshots/chrome-consistency/ai-hub-providers-signed-in.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:246:tracked	web/screenshots/chrome-consistency/ai-hub-providers-signed-out.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:247:tracked	web/screenshots/chrome-consistency/ai-hub-signed-in.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:248:tracked	web/screenshots/chrome-consistency/ai-hub-signed-out.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:249:tracked	web/screenshots/chrome-consistency/auth-login-check-email-signed-in.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:250:tracked	web/screenshots/chrome-consistency/auth-login-check-email-signed-out.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:296:tracked	web/screenshots/chrome-fixes/desktop-ai-hub-plans.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:302:tracked	web/screenshots/chrome-fixes/mobile-ai-hub-plans.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:306:tracked	web/screenshots/layout-fix-final/desktop-signed-in-ai-hub.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:307:tracked	web/screenshots/layout-fix-final/desktop-signed-out-ai-hub.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:308:tracked	web/screenshots/layout-fix-final/mobile-ai-hub-closed.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:309:tracked	web/screenshots/layout-fix-final/mobile-ai-hub-drawer-open.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:327:tracked	web/tests/shared/visual-snapshots.spec.ts-snapshots/ai-hub-compare-desktop-shared-linux.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:328:tracked	web/tests/shared/visual-snapshots.spec.ts-snapshots/ai-hub-directory-mobile-shared-linux.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:329:tracked	web/tests/shared/visual-snapshots.spec.ts-snapshots/ai-hub-directory-models-desktop-shared-linux.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:330:tracked	web/tests/shared/visual-snapshots.spec.ts-snapshots/ai-hub-directory-providers-desktop-shared-linux.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:331:tracked	web/tests/shared/visual-snapshots.spec.ts-snapshots/ai-hub-directory-vouchers-desktop-shared-linux.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:332:tracked	web/tests/shared/visual-snapshots.spec.ts-snapshots/ai-hub-plans-desktop-shared-linux.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:333:tracked	web/tests/shared/visual-snapshots.spec.ts-snapshots/ai-hub-provider-groq-desktop-shared-linux.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:334:tracked	web/tests/shared/visual-snapshots.spec.ts-snapshots/ai-hub-rankings-desktop-shared-linux.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:335:tracked	web/tests/shared/visual-snapshots.spec.ts-snapshots/ai-hub-rankings-mobile-shared-linux.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:336:tracked	web/tests/shared/visual-snapshots.spec.ts-snapshots/dashboard-check-email-desktop-shared-linux.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:340:tracked	web/web/test-results/ui-audit-recon/symptom-2-ai-hub.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:343:tracked	web/web/test-results/ui-audit/ai-hub-compare-desktop-1440.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:344:tracked	web/web/test-results/ui-audit/ai-hub-compare-mobile-375.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:345:tracked	web/web/test-results/ui-audit/ai-hub-desktop-1440.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:346:tracked	web/web/test-results/ui-audit/ai-hub-free-desktop-1440.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:347:tracked	web/web/test-results/ui-audit/ai-hub-free-mobile-375.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:348:tracked	web/web/test-results/ui-audit/ai-hub-mobile-375.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:349:tracked	web/web/test-results/ui-audit/ai-hub-plans-desktop-1440.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:350:tracked	web/web/test-results/ui-audit/ai-hub-plans-mobile-375.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:351:tracked	web/web/test-results/ui-audit/ai-hub-promos-desktop-1440.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:352:tracked	web/web/test-results/ui-audit/ai-hub-promos-mobile-375.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:353:tracked	web/web/test-results/ui-audit/ai-hub-providers-desktop-1440.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:354:tracked	web/web/test-results/ui-audit/ai-hub-providers-mobile-375.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:371:tracked	web/web/test-results/ui-audit/verify-ai-hub-landing-desktop-1440.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:372:tracked	web/web/test-results/ui-audit/verify-ai-hub-landing-mobile-375.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:379:untracked	web/ai-hub-browser-probe.mjs
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-platform.risk.tsv:380:untracked	web/ai-hub-signed-browser-probe.mjs
docs/audit/2026-06-11-full-repo-audit/risk-paths/ai-auto-repo-rot-scanner-todo-agent.risk.tsv:60:tracked	.ralph/old-loops/lop-remaining.md
docs/audit/2026-06-11-full-repo-audit/risk-paths/ai-auto-repo-rot-scanner-todo-agent.risk.tsv:61:tracked	.ralph/old-loops/lop-remaining.state.json
docs/audit/2026-06-11-full-repo-audit/risk-paths/ai-auto-repo-rot-scanner-todo-agent.risk.tsv:64:tracked	.ralph/old-loops/remaining-todos.md
docs/audit/2026-06-11-full-repo-audit/risk-paths/ai-auto-repo-rot-scanner-todo-agent.risk.tsv:65:tracked	.ralph/old-loops/remaining-todos.state.json
docs/audit/2026-06-11-full-repo-audit/risk-paths/ai-auto-repo-rot-scanner-todo-agent.risk.tsv:75:tracked	.ralph/supply-chain-security.md
docs/audit/2026-06-11-full-repo-audit/risk-paths/ai-auto-repo-rot-scanner-todo-agent.risk.tsv:76:tracked	.ralph/supply-chain-security.state.json
docs/audit/2026-06-11-full-repo-audit/release-readiness/inventory-current.tsv:2:/home/dracon/Dev/browser-extensions-shared	main	1	0	2	0	0	DIRTY	OK	run repair-warns --apply
docs/audit/2026-06-11-full-repo-audit/release-readiness/inventory-current.tsv:3:/home/dracon/Dev/dracon-utilities	main	0	0	1	0	0	DIRTY	OK	healthy
docs/audit/2026-06-11-full-repo-audit/risk-paths/rust-ai-web-auto.risk.tsv:34:tracked	click_chain_1780538519.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/rust-ai-web-auto.risk.tsv:35:tracked	click_chain_1780538550.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/rust-ai-web-auto.risk.tsv:36:tracked	click_chain_1780539957.png
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-before.tsv:2:/home/dracon/Dev/dracon-ai-lib	main	0	0	0	28	0	AHEAD:28,STUCK_PUSH	STUCK	run repair-concerns --apply (push or rewrite)
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-final-audit.json:8:  "failures": 0,
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-final-audit.json:15:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-final-audit.json:24:      "last_msg": "3 file(s) in docs [docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-…",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-final-audit.json:54:      "hint": "run repair-warns --apply"
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-final-audit.json:57:      "repo": "/home/dracon/Dev/dracon-ai-lib",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-final-audit.json:62:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-final-audit.json:71:      "last_msg": "1 file(s) in crates [crates/ai-lib/src/providers/minimax.rs] DELTA:+4/-4",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-final-audit.json:75:      "push_error": "ahead=29, push failing",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-final-audit.json:78:      "hint": "run repair-concerns --apply (push or rewrite)"
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-final-audit.json:85:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-final-audit.json:108:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-final-audit.json:117:      "last_msg": "5 file(s) in web [web/ai-hub/{.cache.preserve/.cache.preserve => .cache…",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-final-audit.json:124:      "hint": "run repair-warns --apply"
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-final-audit.json:131:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-final-audit.json:147:      "hint": "run repair-warns --apply"
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-final-audit.json:154:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-final-audit.json:177:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-final-audit.json:196:      "repo": "/home/dracon/Dev/rust-ai-web-auto",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-final-audit.json:200:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-final-audit.json:223:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-final-audit.json:246:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-final-audit.json:265:      "repo": "/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-final-audit.json:269:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-final-audit.json:292:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-final-audit.json:311:      "repo": "/home/dracon/Dev/ai-auto-writer",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-final-audit.json:315:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-final-audit.json:338:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-final-audit.json:361:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-final-audit.json:384:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-final-audit.json:393:      "last_msg": "19 file(s) in examples,src,tests [src/cli.rs, src/ai_gen.rs, src/analyz…",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-final-audit.json:407:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-final-audit.json:430:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-final-audit.json:453:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/git-evidence.txt:4:/home/dracon/Dev/dracon-ai-lib	main	0	0	0	28	0	AHEAD:28,STUCK_PUSH	STUCK	run repair-concerns --apply (push or rewrite)
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/git-evidence.txt:5:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/git-evidence.txt:7:origin	https://github.com/DraconDev/dracon-ai-lib.git (fetch)
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/git-evidence.txt:8:origin	https://github.com/DraconDev/dracon-ai-lib.git (push)
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/git-evidence.txt:10:* main dd14038 [origin/main: ahead 28] docs: tidy current tag section
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/git-evidence.txt:12:origin/main
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/git-evidence.txt:16:fff5e43 3 file(s) in crates [crates/contracts/src/lib.rs, crates/extract-keys/src/main.rs, crates/providers/src/lib.rs] DELTA:+13/-5
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/git-evidence.txt:18:a87ab96 2 file(s) in crates [crates/ai-models-catalog/README.md, crates/ai-lib/README.md] DELTA:+12/-7
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/git-evidence.txt:19:4eccdcc 2 file(s) in crates [crates/ai-lib/src/providers/minimax.rs, crates/ai-lib/src/providers/openai.rs] DELTA:+15/-15
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/git-evidence.txt:20:9cb5103 docs: fix stale ai-lib release tag wording
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/git-evidence.txt:25:6882198 simplify: drop the dracon-ai/* cutover theater; use the real repo URL
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/git-evidence.txt:27:3acafd9 docs: stage consumer cutover plan and align README to dracon-ai org
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/git-evidence.txt:29:cd8bc7f 1 file(s) in crates [crates/ai-lib/src/providers/minimax.rs] DELTA:+48/-37
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/git-evidence.txt:30:209cff3 1 file(s) in crates [crates/ai-lib/src/providers/minimax.rs] DELTA:+1/-1
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/git-evidence.txt:31:d70cf8a 1 file(s) in crates [crates/ai-lib/src/providers/minimax.rs] DELTA:+16/-13
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/git-evidence.txt:32:5fec442 17 file(s) in crates [Cargo.lock, crates/client/src/lib.rs, crates/providers/src/openai.rs] DELTA:+1415/-589
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/git-evidence.txt:64:--- config branch/main ---
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/git-evidence.txt:65:branch.main.remote origin
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/git-evidence.txt:66:branch.main.merge refs/heads/main
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/git-evidence.txt:68:remote.origin.url https://github.com/DraconDev/dracon-ai-lib.git
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/git-evidence.txt:72:fatal: unable to access 'https://github.com/DraconDev/dracon-ai-lib.git/': The requested URL returned error: 403
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/git-evidence.txt:74:dd14038e194c34b3279784efeafa01fbb64ac4f3 refs/heads/main
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/git-evidence.txt:80:--- git ls-remote origin main refs/tags/v0.2.0 ---
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/git-evidence.txt:81:ce377a20fa8b911f3201777c120779ebd56ff903	refs/heads/main
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/git-evidence.txt:84:{"defaultBranchRef":{"name":"main"},"description":"","isArchived":true,"url":"https://github.com/DraconDev/dracon-ai-lib","visibility":"PRIVATE"}
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/git-evidence.txt:86:{"archived":true,"default_branch":"main","full_name":"DraconDev/dracon-ai-lib","permissions":{"admin":true,"maintain":true,"pull":true,"push":true,"triage":true},"visibility":"private"}
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/git-evidence.txt:87:--- merge-base main origin/main ---
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/git-evidence.txt:89:--- rev-list count main ^origin/main ---
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/git-evidence.txt:91:--- rev-list count origin/main ^main ---
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/git-evidence.txt:95:--- origin/main ---
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/git-evidence.txt:97:--- origin/main log -10 ---
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/git-evidence.txt:99:14397af archive: fix remaining dracon-ai-sdk references to ai-api-sdk
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/git-evidence.txt:100:4fc7206 archive: mark lib as archived, redirect to ai-api-sdk
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/git-evidence.txt:108:--- local main commits not on origin/main -28 ---
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/git-evidence.txt:111:fff5e43 3 file(s) in crates [crates/contracts/src/lib.rs, crates/extract-keys/src/main.rs, crates/providers/src/lib.rs] DELTA:+13/-5
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/git-evidence.txt:113:a87ab96 2 file(s) in crates [crates/ai-models-catalog/README.md, crates/ai-lib/README.md] DELTA:+12/-7
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/git-evidence.txt:114:4eccdcc 2 file(s) in crates [crates/ai-lib/src/providers/minimax.rs, crates/ai-lib/src/providers/openai.rs] DELTA:+15/-15
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/git-evidence.txt:115:9cb5103 docs: fix stale ai-lib release tag wording
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/git-evidence.txt:120:6882198 simplify: drop the dracon-ai/* cutover theater; use the real repo URL
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/git-evidence.txt:122:3acafd9 docs: stage consumer cutover plan and align README to dracon-ai org
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/git-evidence.txt:124:cd8bc7f 1 file(s) in crates [crates/ai-lib/src/providers/minimax.rs] DELTA:+48/-37
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/git-evidence.txt:125:209cff3 1 file(s) in crates [crates/ai-lib/src/providers/minimax.rs] DELTA:+1/-1
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/git-evidence.txt:126:d70cf8a 1 file(s) in crates [crates/ai-lib/src/providers/minimax.rs] DELTA:+16/-13
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/git-evidence.txt:127:5fec442 17 file(s) in crates [Cargo.lock, crates/client/src/lib.rs, crates/providers/src/openai.rs] DELTA:+1415/-589
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/git-evidence.txt:129:b04f5db 5 file(s) in crates,docs [docs/release-tracking.md, crates/ai-lib/src/providers/minimax.rs, crates/ai-lib/tests/anthropic.rs] DELTA:+147/-12 | TEST:10 NEW:docs/release-tracking.md
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/git-evidence.txt:130:524b5b3 2 file(s) in crates [crates/ai-lib/src/providers/minimax.rs, crates/ai-lib/tests/integration.rs] DELTA:+41/-35 | TEST:14
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/git-evidence.txt:131:168b8d6 11 file(s) in crates [crates/ai-lib/src/lib.rs, crates/ai-models-catalog/tests/catalog.rs, crates/ai-lib/tests/anthropic.rs] DELTA:+90/-52 | TEST:70
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/git-evidence.txt:134:af66a06 feat: lift ai-api provider engine into ai-lib
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/git-evidence.txt:135:92d4f2b chore: rename repo to dracon-ai-lib
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/git-evidence.txt:136:43cdd4d chore: initial import of ai-lib and ai-models-catalog
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-after.json:8:  "failures": 0,
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-after.json:15:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-after.json:38:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-after.json:47:      "last_msg": "3 file(s) in docs [docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-…",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-after.json:61:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-after.json:70:      "last_msg": "5 file(s) in web [web/ai-hub/{.cache.preserve/.cache.preserve => .cache…",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-after.json:84:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-after.json:107:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-after.json:130:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-after.json:149:      "repo": "/home/dracon/Dev/rust-ai-web-auto",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-after.json:153:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-after.json:176:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-after.json:195:      "repo": "/home/dracon/Dev/dracon-ai-lib",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-after.json:201:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-after.json:214:      "push_error": "ahead=28, push failing",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-after.json:217:      "hint": "run repair-concerns --apply (push or rewrite)"
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-after.json:240:      "hint": "run repair-warns --apply"
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-after.json:247:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-after.json:266:      "repo": "/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-after.json:270:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-after.json:293:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-after.json:312:      "repo": "/home/dracon/Dev/ai-auto-writer",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-after.json:316:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-after.json:339:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-after.json:362:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-after.json:385:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-after.json:394:      "last_msg": "19 file(s) in examples,src,tests [src/cli.rs, src/ai_gen.rs, src/analyz…",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-after.json:408:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-after.json:431:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-after.json:454:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-code.risk.tsv:19:tracked	.ralph/phase3-dracon-ai-extraction.md
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-code.risk.tsv:20:tracked	.ralph/phase3-dracon-ai-extraction.state.json
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-code.risk.tsv:29:tracked	.ralph/remaining-work.md
docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-code.risk.tsv:30:tracked	.ralph/remaining-work.state.json
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:6:tracked	SamAI/.dracon/data/keys/owner_age1f7y5.pub
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:7:tracked	SamAI/.dracon/data/keys/owner_nixos.pub
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:8:tracked	SamAI/.env
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:9:tracked	SamAI/.env.example
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:10:tracked	SamAI/.env.production
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:11:tracked	SamAI/ai-job-finder/.ralph/deep-bug-fix-loop.md
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:12:tracked	SamAI/ai-job-finder/.ralph/deep-bug-fix-loop.state.json
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:13:tracked	SamAI/ai-job-finder/.ralph/fix-review-bugs.md
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:14:tracked	SamAI/ai-job-finder/.ralph/fix-review-bugs.state.json
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:15:tracked	SamAI/ai-job-finder/.ralph/fix-review-round2.md
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:16:tracked	SamAI/ai-job-finder/.ralph/fix-review-round2.state.json
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:17:tracked	SamAI/ai-job-finder/.ralph/full-polish-loop.md
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:18:tracked	SamAI/ai-job-finder/.ralph/full-polish-loop.state.json
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:19:tracked	SamAI/ai-job-finder/.ralph/full-redesign.md
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:20:tracked	SamAI/ai-job-finder/.ralph/full-redesign.state.json
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:21:tracked	SamAI/ai-job-finder/.ralph/loop-todos.md
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:22:tracked	SamAI/ai-job-finder/.ralph/loop-todos.state.json
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:23:tracked	SamAI/ai-job-finder/.ralph/make-it-work.md
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:24:tracked	SamAI/ai-job-finder/.ralph/make-it-work.state.json
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:25:tracked	SamAI/ai-job-finder/.ralph/next-pass-audit-fix.md
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:26:tracked	SamAI/ai-job-finder/.ralph/next-pass-audit-fix.state.json
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:27:tracked	SamAI/ai-job-finder/.ralph/next-phase-features.md
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:28:tracked	SamAI/ai-job-finder/.ralph/next-phase-features.state.json
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:29:tracked	SamAI/ai-job-finder/.ralph/options-overhaul-models.md
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:30:tracked	SamAI/ai-job-finder/.ralph/options-overhaul-models.state.json
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:31:tracked	SamAI/ai-job-finder/.ralph/popup-dark-redesign.md
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:32:tracked	SamAI/ai-job-finder/.ralph/popup-dark-redesign.state.json
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:33:tracked	SamAI/ai-job-finder/.ralph/remove-server-byok-only.md
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:34:tracked	SamAI/ai-job-finder/.ralph/remove-server-byok-only.state.json
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:35:tracked	SamAI/ai-job-finder/.ralph/ui-ux-improvements.md
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:36:tracked	SamAI/ai-job-finder/.ralph/ui-ux-improvements.state.json
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:37:tracked	SamAI/ai-job-finder/public/icon/128.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:38:tracked	SamAI/ai-job-finder/public/icon/16.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:39:tracked	SamAI/ai-job-finder/public/icon/32.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:40:tracked	SamAI/ai-job-finder/public/icon/48.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:41:tracked	SamAI/ai-job-finder/public/icon/96.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:42:tracked	SamAI/ai-job-finder/public/wxt.svg
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:43:tracked	SamAI/ai-job-finder/server/.env.example
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:44:tracked	SamAI/ai-job-finder/src/lib/styles/tokens.css
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:45:tracked	SamAI/assets/unnamed (1).png
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:46:tracked	SamAI/assets/unnamed (2).png
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:47:tracked	SamAI/assets/unnamed (3).png
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:48:tracked	SamAI/assets/unnamed (4).png
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:49:tracked	SamAI/assets/unnamed.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:50:tracked	SamAI/coverage/block-navigation.js
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:51:tracked	SamAI/coverage/favicon.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:52:tracked	SamAI/coverage/services/background/handlers/navigation.ts.html
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:53:tracked	SamAI/coverage/sort-arrow-sprite.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:54:tracked	SamAI/coverage/utils/formFiller/profileMapper.ts.html
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:55:tracked	SamAI/coverage/utils/formProfileTemplates.json.html
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:56:tracked	SamAI/coverage/utils/simpleFormProfiles.ts.html
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:57:tracked	SamAI/docs/CHROME_STORE_LISTING.md
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:58:tracked	SamAI/docs/FIREFOX_AMO_GUIDE.md
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:59:tracked	SamAI/docs/FIREFOX_SUBMISSION_GUIDE.md
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:60:tracked	SamAI/docs/assets/screenshots/1.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:61:tracked	SamAI/docs/assets/screenshots/2.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:62:tracked	SamAI/docs/assets/screenshots/3.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:63:tracked	SamAI/docs/assets/screenshots/4.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:64:tracked	SamAI/docs/assets/screenshots/5.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:65:tracked	SamAI/docs/assets/screenshots/SCREENSHOTS.md
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:66:tracked	SamAI/docs/assets/screenshots/t1.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:67:tracked	SamAI/entrypoints/profile-editor-page/index.html
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:68:tracked	SamAI/entrypoints/profile-editor-page/main.tsx
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:69:tracked	SamAI/entrypoints/profiles-page/index.html
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:70:tracked	SamAI/entrypoints/profiles-page/main.tsx
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:71:tracked	SamAI/public/1.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:72:tracked	SamAI/public/2.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:73:tracked	SamAI/public/3.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:74:tracked	SamAI/public/4.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:75:tracked	SamAI/public/440x280.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:76:tracked	SamAI/public/5.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:77:tracked	SamAI/public/icon/128.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:78:tracked	SamAI/public/icon/16.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:79:tracked	SamAI/public/icon/32.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:80:tracked	SamAI/public/icon/48.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:81:tracked	SamAI/public/icon/96.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:82:tracked	SamAI/public/wxt.svg
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:83:tracked	SamAI/services/background/handlers/navigation.ts
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:84:tracked	SamAI/src/content/SearchPanel/components/ProfileEditorPage.css
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:85:tracked	SamAI/src/content/SearchPanel/components/ProfileEditorPage.tsx
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:86:tracked	SamAI/src/content/SearchPanel/components/ProfilesPage.css
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:87:tracked	SamAI/src/content/SearchPanel/components/ProfilesPage.tsx
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:88:tracked	SamAI/src/content/SearchPanel/components/TabNavigation.tsx
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:89:tracked	SamAI/test/profile-editor-page.test.tsx
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:90:tracked	SamAI/test/profileMapper.test.ts
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:91:tracked	SamAI/test/simpleFormProfiles.test.ts
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:92:tracked	SamAI/utils/formFiller/profileMapper.ts
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:93:tracked	SamAI/utils/formProfileTemplates.json
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:94:tracked	SamAI/utils/simpleFormProfiles.ts
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:95:tracked	ai-ats/.dracon/data/keys/owner_age1f7y5.pub
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:96:tracked	ai-ats/.dracon/data/keys/owner_nixos.pub
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:97:tracked	ai-ats/.env
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:98:tracked	ai-ats/.env.example
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:99:tracked	ai-ats/docs/FIREFOX_AMO_GUIDE.md
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:100:tracked	ai-ats/docs/FIREFOX_SUBMISSION_GUIDE.md
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:101:tracked	ai-ats/public/icon/128.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:102:tracked	ai-ats/public/icon/16.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:103:tracked	ai-ats/public/icon/32.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:104:tracked	ai-ats/public/icon/48.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:105:tracked	ai-ats/public/icon/96.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:106:tracked	ai-ats/public/pdf.worker.min.mjs
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:107:tracked	ai-ats/public/wxt.svg
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:108:tracked	ai-ats/test-cvs/cv4_david_kim.txt
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:109:tracked	ai-ats/test-cvs/final/cv4_david_kim.txt
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:110:tracked	ai-ats/test-cvs/mixed/cv4_david_kim.txt
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:111:tracked	ai-ats/test-files/cv-david-kim.txt
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:216:tracked	auto-form-filler/.audit-ui/profile/Default/Local Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/CURRENT
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:217:tracked	auto-form-filler/.audit-ui/profile/Default/Local Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOCK
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:218:tracked	auto-form-filler/.audit-ui/profile/Default/Local Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOG
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:219:tracked	auto-form-filler/.audit-ui/profile/Default/Local Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOG.old
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:220:tracked	auto-form-filler/.audit-ui/profile/Default/Local Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/MANIFEST-000001
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:285:tracked	auto-form-filler/.audit-ui/profile/Default/Sync Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/CURRENT
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:286:tracked	auto-form-filler/.audit-ui/profile/Default/Sync Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOCK
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:287:tracked	auto-form-filler/.audit-ui/profile/Default/Sync Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOG
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:288:tracked	auto-form-filler/.audit-ui/profile/Default/Sync Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOG.old
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:289:tracked	auto-form-filler/.audit-ui/profile/Default/Sync Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/MANIFEST-000001
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:428:tracked	cursor-style/public/assets/cursors/crosshair-pointer.svg
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:464:tracked	cursor-style/public/assets/cursors/rain-pointer.svg
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:614:tracked	death-note-typing-practice/tests/e2e/screenshots/main-menu-after-pause.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:640:tracked	full-page-screenshot/EXTENSION_CONSTRAINTS.md
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:649:tracked	full-page-screenshot/entrypoints/editor/main.tsx
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:653:tracked	full-page-screenshot/entrypoints/popup/main.tsx
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:667:tracked	full-page-screenshot/tailwind.config.cjs
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:713:tracked	live-reload-pro/references/jnihajbhpnppcggbcgedagnkighmdlei/IconUnavailable.png
docs/audit/2026-06-11-full-repo-audit/risk-paths/browser-extensions-shared.risk.tsv:714:tracked	live-reload-pro/references/jnihajbhpnppcggbcgedagnkighmdlei/IconUnavailable@2x.png
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-after.tsv:2:/home/dracon/Dev/dracon-ai-lib	main	1	0	0	28	0	DIRTY,AHEAD:28,STUCK_PUSH	STUCK	run repair-concerns --apply (push or rewrite)
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-after-remediation.json:8:  "failures": 0,
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-after-remediation.json:15:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-after-remediation.json:38:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-after-remediation.json:54:      "hint": "run repair-warns --apply"
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-after-remediation.json:61:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-after-remediation.json:70:      "last_msg": "5 file(s) in crates,plugins [crates/dracon-ai/src/ai_client.rs, crates/…",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-after-remediation.json:80:      "repo": "/home/dracon/Dev/rust-ai-web-auto",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-after-remediation.json:84:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-after-remediation.json:107:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-after-remediation.json:116:      "last_msg": "5 file(s) in apis,web [web/ai-hub/src/routes/ai-hub/directory/+page.sve…",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-after-remediation.json:123:      "hint": "run repair-warns --apply"
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-after-remediation.json:130:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-after-remediation.json:153:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-after-remediation.json:176:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-after-remediation.json:192:      "hint": "run repair-warns --apply"
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-after-remediation.json:199:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-after-remediation.json:222:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-after-remediation.json:231:      "last_msg": "8 file(s) in dracon-ai-sdk [dracon-ai-sdk/src/lib.rs, dracon-ai-sdk/tes…",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-after-remediation.json:268:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-after-remediation.json:287:      "repo": "/home/dracon/Dev/dracon-ai-lib",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-after-remediation.json:291:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-after-remediation.json:310:      "repo": "/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-after-remediation.json:314:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-after-remediation.json:333:      "repo": "/home/dracon/Dev/ai-auto-writer",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-after-remediation.json:337:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-after-remediation.json:360:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-after-remediation.json:383:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-after-remediation.json:406:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/CLEANUP_MANIFEST.md:11:| `browser-extensions-shared` | cleaned tracked generated coverage | `SamAI/coverage/` | Generated coverage output, not source or user-owned project asset. |
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/CLEANUP_MANIFEST.md:12:| `ai-auto-repo-rot-scanner-todo-agent` | cleaned stale local runner event file | `.ralph/audit-remediation/.ralph-runner/events.jsonl` | Stale `.ralph-runner` generated event log outside `.pi`. |
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/CLEANUP_MANIFEST.md:21:| User-owned notes, screenshots, pasted-image files, project assets | Constraint requires preservation unless explicit approval is given. |
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-after-final.tsv:2:/home/dracon/Dev/dracon-ai-lib	main	0	0	0	29	0	AHEAD:29,STUCK_PUSH	STUCK	run repair-concerns --apply (push or rewrite)
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/after-fix.txt:3:/home/dracon/Dev/dracon-ai-lib	main	1	0	0	28	0	DIRTY,AHEAD:28,STUCK_PUSH	STUCK	run repair-concerns --apply (push or rewrite)
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/after-fix.txt:5:1 .M N... 100644 100644 100644 dceea034197eeb98a73cc82c62b9cdfa480570f4 dceea034197eeb98a73cc82c62b9cdfa480570f4 crates/ai-lib/src/providers/minimax.rs
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-after-final.json:8:  "failures": 0,
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-after-final.json:11:      "repo": "/home/dracon/Dev/dracon-ai-lib",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-after-final.json:16:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-after-final.json:25:      "last_msg": "1 file(s) in crates [crates/ai-lib/src/providers/minimax.rs] DELTA:+4/-4",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-after-final.json:29:      "push_error": "ahead=29, push failing",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-after-final.json:32:      "hint": "run repair-concerns --apply (push or rewrite)"
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-after-final.json:39:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-after-final.json:48:      "last_msg": "4 file(s) in docs [docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-…",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-after-final.json:78:      "hint": "run repair-warns --apply"
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-after-final.json:85:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-after-final.json:108:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-after-final.json:117:      "last_msg": "5 file(s) in web [web/ai-hub/{.cache.preserve/.cache.preserve => .cache…",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-after-final.json:131:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-after-final.json:154:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-after-final.json:177:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-after-final.json:196:      "repo": "/home/dracon/Dev/rust-ai-web-auto",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-after-final.json:200:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-after-final.json:223:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-after-final.json:246:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-after-final.json:265:      "repo": "/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-after-final.json:269:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-after-final.json:292:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-after-final.json:311:      "repo": "/home/dracon/Dev/ai-auto-writer",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-after-final.json:315:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-after-final.json:338:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-after-final.json:361:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-after-final.json:384:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-after-final.json:393:      "last_msg": "19 file(s) in examples,src,tests [src/cli.rs, src/ai_gen.rs, src/analyz…",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-after-final.json:407:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-after-final.json:430:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-after-final.json:453:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/final-git-evidence.txt:1:--- final git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/final-git-evidence.txt:3:origin	https://github.com/DraconDev/dracon-ai-lib.git (fetch)
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/final-git-evidence.txt:4:origin	https://github.com/DraconDev/dracon-ai-lib.git (push)
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/final-git-evidence.txt:6:* main b87f979 [origin/main: ahead 29] 1 file(s) in crates [crates/ai-lib/src/providers/minimax.rs] DELTA:+4/-4
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/final-git-evidence.txt:8:origin/main
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/final-git-evidence.txt:11:--- final rev-list count main ^origin/main ---
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/final-git-evidence.txt:15:fatal: unable to access 'https://github.com/DraconDev/dracon-ai-lib.git/': The requested URL returned error: 403
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/sync-evidence.txt:1:--- dracon-sync repair stuck-list ---
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/sync-evidence.txt:14:--- incident ledger recent dracon-ai-lib ---
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/sync-evidence.txt:15:{"ts_unix":1781178421,"scope":"concern","repo":"/home/dracon/Dev/dracon-ai-lib","reason":"AHEAD:28,STUCK_PUSH","action":"push_origin_head","backup_branch":null,"result":"fail","details":"git push failed in /home/dracon/Dev/dracon-ai-lib with status exit status: 128: remote: This repository was archived so it is read-only.\nfatal: unable to access 'https://github.com/DraconDev/dracon-ai-lib.git/': The requested URL returned error: 403"}
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/sync-evidence.txt:16:{"ts_unix":1781178558,"scope":"concern","repo":"/home/dracon/Dev/dracon-ai-lib","reason":"AHEAD:28,STUCK_PUSH","action":"push_origin_head","backup_branch":null,"result":"fail","details":"git push failed in /home/dracon/Dev/dracon-ai-lib with status exit status: 128: remote: This repository was archived so it is read-only.\nfatal: unable to access 'https://github.com/DraconDev/dracon-ai-lib.git/': The requested URL returned error: 403"}
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/sync-evidence.txt:17:{"ts_unix":1781178675,"scope":"concern","repo":"/home/dracon/Dev/dracon-ai-lib","reason":"AHEAD:28,STUCK_PUSH","action":"push_origin_head","backup_branch":null,"result":"fail","details":"git push failed in /home/dracon/Dev/dracon-ai-lib with status exit status: 128: remote: This repository was archived so it is read-only.\nfatal: unable to access 'https://github.com/DraconDev/dracon-ai-lib.git/': The requested URL returned error: 403"}
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/sync-evidence.txt:18:{"ts_unix":1781178791,"scope":"concern","repo":"/home/dracon/Dev/dracon-ai-lib","reason":"AHEAD:28,STUCK_PUSH","action":"push_origin_head","backup_branch":null,"result":"fail","details":"git push failed in /home/dracon/Dev/dracon-ai-lib with status exit status: 128: remote: This repository was archived so it is read-only.\nfatal: unable to access 'https://github.com/DraconDev/dracon-ai-lib.git/': The requested URL returned error: 403"}
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/sync-evidence.txt:19:{"ts_unix":1781178888,"scope":"concern","repo":"/home/dracon/Dev/dracon-ai-lib","reason":"AHEAD:28,STUCK_PUSH","action":"push_origin_head","backup_branch":null,"result":"fail","details":"git push failed in /home/dracon/Dev/dracon-ai-lib with status exit status: 128: remote: This repository was archived so it is read-only.\nfatal: unable to access 'https://github.com/DraconDev/dracon-ai-lib.git/': The requested URL returned error: 403"}
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/sync-evidence.txt:20:{"ts_unix":1781178999,"scope":"concern","repo":"/home/dracon/Dev/dracon-ai-lib","reason":"AHEAD:28,STUCK_PUSH","action":"push_origin_head","backup_branch":null,"result":"fail","details":"git push failed in /home/dracon/Dev/dracon-ai-lib with status exit status: 128: remote: This repository was archived so it is read-only.\nfatal: unable to access 'https://github.com/DraconDev/dracon-ai-lib.git/': The requested URL returned error: 403"}
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/sync-evidence.txt:21:{"ts_unix":1781179120,"scope":"concern","repo":"/home/dracon/Dev/dracon-ai-lib","reason":"AHEAD:28,STUCK_PUSH","action":"push_origin_head","backup_branch":null,"result":"fail","details":"git push failed in /home/dracon/Dev/dracon-ai-lib with status exit status: 128: remote: This repository was archived so it is read-only.\nfatal: unable to access 'https://github.com/DraconDev/dracon-ai-lib.git/': The requested URL returned error: 403"}
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/sync-evidence.txt:22:{"ts_unix":1781179248,"scope":"concern","repo":"/home/dracon/Dev/dracon-ai-lib","reason":"AHEAD:28,STUCK_PUSH","action":"push_origin_head","backup_branch":null,"result":"fail","details":"git push failed in /home/dracon/Dev/dracon-ai-lib with status exit status: 128: remote: This repository was archived so it is read-only.\nfatal: unable to access 'https://github.com/DraconDev/dracon-ai-lib.git/': The requested URL returned error: 403"}
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/sync-evidence.txt:23:{"ts_unix":1781179377,"scope":"concern","repo":"/home/dracon/Dev/dracon-ai-lib","reason":"AHEAD:28,STUCK_PUSH","action":"push_origin_head","backup_branch":null,"result":"fail","details":"git push failed in /home/dracon/Dev/dracon-ai-lib with status exit status: 128: remote: This repository was archived so it is read-only.\nfatal: unable to access 'https://github.com/DraconDev/dracon-ai-lib.git/': The requested URL returned error: 403"}
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/sync-evidence.txt:24:{"ts_unix":1781179511,"scope":"concern","repo":"/home/dracon/Dev/dracon-ai-lib","reason":"AHEAD:28,STUCK_PUSH","action":"push_origin_head","backup_branch":null,"result":"fail","details":"git push failed in /home/dracon/Dev/dracon-ai-lib with status exit status: 128: remote: This repository was archived so it is read-only.\nfatal: unable to access 'https://github.com/DraconDev/dracon-ai-lib.git/': The requested URL returned error: 403"}
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/sync-evidence.txt:25:{"ts_unix":1781179616,"scope":"concern","repo":"/home/dracon/Dev/dracon-ai-lib","reason":"AHEAD:28,STUCK_PUSH","action":"push_origin_head","backup_branch":null,"result":"fail","details":"git push failed in /home/dracon/Dev/dracon-ai-lib with status exit status: 128: remote: This repository was archived so it is read-only.\nfatal: unable to access 'https://github.com/DraconDev/dracon-ai-lib.git/': The requested URL returned error: 403"}
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/sync-evidence.txt:26:{"ts_unix":1781179717,"scope":"concern","repo":"/home/dracon/Dev/dracon-ai-lib","reason":"AHEAD:28,STUCK_PUSH","action":"push_origin_head","backup_branch":null,"result":"fail","details":"git push failed in /home/dracon/Dev/dracon-ai-lib with status exit status: 128: remote: This repository was archived so it is read-only.\nfatal: unable to access 'https://github.com/DraconDev/dracon-ai-lib.git/': The requested URL returned error: 403"}
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/sync-evidence.txt:27:{"ts_unix":1781179811,"scope":"concern","repo":"/home/dracon/Dev/dracon-ai-lib","reason":"AHEAD:28,STUCK_PUSH","action":"push_origin_head","backup_branch":null,"result":"fail","details":"git push failed in /home/dracon/Dev/dracon-ai-lib with status exit status: 128: remote: This repository was archived so it is read-only.\nfatal: unable to access 'https://github.com/DraconDev/dracon-ai-lib.git/': The requested URL returned error: 403"}
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/sync-evidence.txt:28:{"ts_unix":1781180066,"scope":"concern","repo":"/home/dracon/Dev/dracon-ai-lib","reason":"AHEAD:28,STUCK_PUSH","action":"push_origin_head","backup_branch":null,"result":"fail","details":"git push failed in /home/dracon/Dev/dracon-ai-lib with status exit status: 128: remote: This repository was archived so it is read-only.\nfatal: unable to access 'https://github.com/DraconDev/dracon-ai-lib.git/': The requested URL returned error: 403"}
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/sync-evidence.txt:29:{"ts_unix":1781180455,"scope":"concern","repo":"/home/dracon/Dev/dracon-ai-lib","reason":"AHEAD:28,STUCK_PUSH","action":"push_origin_head","backup_branch":null,"result":"fail","details":"git push failed in /home/dracon/Dev/dracon-ai-lib with status exit status: 128: remote: This repository was archived so it is read-only.\nfatal: unable to access 'https://github.com/DraconDev/dracon-ai-lib.git/': The requested URL returned error: 403"}
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/sync-evidence.txt:30:{"ts_unix":1781180627,"scope":"concern","repo":"/home/dracon/Dev/dracon-ai-lib","reason":"AHEAD:28,STUCK_PUSH","action":"push_origin_head","backup_branch":null,"result":"fail","details":"git push failed in /home/dracon/Dev/dracon-ai-lib with status exit status: 128: remote: This repository was archived so it is read-only.\nfatal: unable to access 'https://github.com/DraconDev/dracon-ai-lib.git/': The requested URL returned error: 403"}
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/sync-evidence.txt:31:{"ts_unix":1781180729,"scope":"concern","repo":"/home/dracon/Dev/dracon-ai-lib","reason":"AHEAD:28,STUCK_PUSH","action":"push_origin_head","backup_branch":null,"result":"fail","details":"git push failed in /home/dracon/Dev/dracon-ai-lib with status exit status: 128: remote: This repository was archived so it is read-only.\nfatal: unable to access 'https://github.com/DraconDev/dracon-ai-lib.git/': The requested URL returned error: 403"}
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/sync-evidence.txt:32:{"ts_unix":1781181111,"scope":"concern","repo":"/home/dracon/Dev/dracon-ai-lib","reason":"AHEAD:28,STUCK_PUSH","action":"push_origin_head","backup_branch":null,"result":"fail","details":"git push failed in /home/dracon/Dev/dracon-ai-lib with status exit status: 128: remote: This repository was archived so it is read-only.\nfatal: unable to access 'https://github.com/DraconDev/dracon-ai-lib.git/': The requested URL returned error: 403"}
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/sync-evidence.txt:33:{"ts_unix":1781181219,"scope":"concern","repo":"/home/dracon/Dev/dracon-ai-lib","reason":"AHEAD:28,STUCK_PUSH","action":"push_origin_head","backup_branch":null,"result":"fail","details":"git push failed in /home/dracon/Dev/dracon-ai-lib with status exit status: 128: remote: This repository was archived so it is read-only.\nfatal: unable to access 'https://github.com/DraconDev/dracon-ai-lib.git/': The requested URL returned error: 403"}
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/sync-evidence.txt:34:{"ts_unix":1781181320,"scope":"concern","repo":"/home/dracon/Dev/dracon-ai-lib","reason":"AHEAD:28,STUCK_PUSH","action":"push_origin_head","backup_branch":null,"result":"fail","details":"git push failed in /home/dracon/Dev/dracon-ai-lib with status exit status: 128: remote: This repository was archived so it is read-only.\nfatal: unable to access 'https://github.com/DraconDev/dracon-ai-lib.git/': The requested URL returned error: 403"}
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-final-audit.tsv:2:/home/dracon/Dev/dracon-ai-lib	main	0	0	0	29	0	AHEAD:29,STUCK_PUSH	STUCK	run repair-concerns --apply (push or rewrite)
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-before.json:8:  "failures": 0,
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-before.json:15:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-before.json:61:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-before.json:80:      "repo": "/home/dracon/Dev/dracon-ai-lib",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-before.json:85:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-before.json:98:      "push_error": "ahead=30, push failing",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-before.json:101:      "hint": "run repair-concerns --apply (push or rewrite)"
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-before.json:104:      "repo": "/home/dracon/Dev/rust-ai-web-auto",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-before.json:108:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-before.json:131:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-before.json:154:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-before.json:177:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-before.json:186:      "last_msg": "3 file(s) in docs [docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-…",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-before.json:200:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-before.json:223:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-before.json:246:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-before.json:265:      "repo": "/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-before.json:269:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-before.json:288:      "repo": "/home/dracon/Dev/ai-auto-writer",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-before.json:292:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-before.json:315:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-before.json:338:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-before.json:361:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-before.json:370:      "last_msg": "19 file(s) in examples,src,tests [src/cli.rs, src/ai_gen.rs, src/analyz…",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-before.json:384:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-before.json:407:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-before.json:430:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/one-mil-girls-gitlab-protected-branch.json:1:{"id":250617858,"name":"main","push_access_levels":[{"id":307968492,"access_level":0,"access_level_description":"No one","deploy_key_id":null,"user_id":null,"group_id":null}],"merge_access_levels":[{"id":271519612,"access_level":30,"access_level_description":"Developers + Maintainers","user_id":null,"group_id":null}],"allow_force_push":false,"unprotect_access_levels":[{"id":179794729,"access_level":40,"access_level_description":"Maintainers","user_id":null,"group_id":null}],"code_owner_approval_required":false,"inherited":false}
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:1:# `dracon-ai-lib` stuck-push investigation
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:7:`dracon-ai-lib` is marked `CONCERN` because it is clean locally but cannot push its local `main` branch to GitHub. The remote repository is archived/read-only, so `git push` fails with HTTP 403.
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:11:- Repo: `/home/dracon/Dev/dracon-ai-lib`
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:12:- Branch: `main`
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:13:- Upstream: `origin/main`
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:18:- Root cause: `DraconDev/dracon-ai-lib` is archived and read-only on GitHub.
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:26:`/home/dracon/Dev/dracon-utilities/docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/`
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:44:/home/dracon/Dev/dracon-ai-lib	main	0	0	0	28	0	AHEAD:28,STUCK_PUSH	STUCK	run repair-concerns --apply (push or rewrite)
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:50:* main dd14038 [origin/main: ahead 28] docs: tidy current tag section
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:51:origin	https://github.com/DraconDev/dracon-ai-lib.git (fetch)
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:52:origin	https://github.com/DraconDev/dracon-ai-lib.git (push)
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:53:branch.main.remote origin
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:54:branch.main.merge refs/heads/main
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:60:merge-base main origin/main = ce377a20fa8b911f3201777c120779ebd56ff903
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:61:rev-list --count main ^origin/main = 28
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:62:rev-list --count origin/main ^main = 0
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:65:So the local branch was strictly ahead of `origin/main`, not diverged.
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:67:## Push failure
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:69:`git push --dry-run origin main` failed with:
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:73:fatal: unable to access 'https://github.com/DraconDev/dracon-ai-lib.git/': The requested URL returned error: 403
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:76:`gh repo view DraconDev/dracon-ai-lib --json isArchived,visibility,defaultBranchRef,url,description` reported:
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:80:  "defaultBranchRef": {"name": "main"},
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:83:  "url": "https://github.com/DraconDev/dracon-ai-lib",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:88:`gh api repos/DraconDev/dracon-ai-lib --jq '{full_name,archived,visibility,default_branch,permissions}'` reported:
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:93:  "default_branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:94:  "full_name": "DraconDev/dracon-ai-lib",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:97:    "maintain": true,
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:110:`dracon-sync repair stuck-list` reported:
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:123:The incident ledger contains repeated `concern` entries for `/home/dracon/Dev/dracon-ai-lib` with:
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:128:result=fail
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:129:details=remote: This repository was archived so it is read-only.
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:130:details=fatal: unable to access 'https://github.com/DraconDev/dracon-ai-lib.git/': The requested URL returned error: 403
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:135:Initial validation found one clippy failure:
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:138:crates/ai-lib/src/providers/minimax.rs:220
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:147:- `cargo test --manifest-path dracon-ai-lib/Cargo.toml -- --test-threads=1` → **181 passed, 0 failed**
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:148:- `cargo clippy --manifest-path dracon-ai-lib/Cargo.toml --workspace -- -D warnings` → pass
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:153:b87f979 1 file(s) in crates [crates/ai-lib/src/providers/minimax.rs] DELTA:+4/-4
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:162:/home/dracon/Dev/dracon-ai-lib	main	0	0	0	29	0	AHEAD:29,STUCK_PUSH	STUCK	run repair-concerns --apply (push or rewrite)
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:168:* main b87f979 [origin/main: ahead 29] 1 file(s) in crates [crates/ai-lib/src/providers/minimax.rs] DELTA:+4/-4
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:169:origin	https://github.com/DraconDev/dracon-ai-lib.git (fetch)
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:170:origin	https://github.com/DraconDev/dracon-ai-lib.git (push)
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:171:merge-base main origin/main = ce377a20fa8b911f3201777c120779ebd56ff903
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:172:rev-list --count main ^origin/main = 29
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:173:rev-list --count origin/main ^main = 0
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:176:Final `git push --dry-run origin main` still fails with:
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:180:fatal: unable to access 'https://github.com/DraconDev/dracon-ai-lib.git/': The requested URL returned error: 403
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:185:`dracon-sync` is correct to mark `dracon-ai-lib` as a concern.
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:187:The repo is not unhealthy locally: it is clean, tests pass, and clippy passes. The concern is external to the working tree: GitHub has archived `DraconDev/dracon-ai-lib`, making the origin read-only. Because local `main` is 29 commits ahead of `origin/main`, every push attempt fails and the repo remains `AHEAD:29,STUCK_PUSH`.
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:189:## Remaining blockers
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:192:2. Pushing to `origin/main` is blocked until the repo is unarchived or the remote is changed to an active repository.
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:193:3. `dracon-sync repair-concerns --apply` is not a safe next step without approval because it may attempt push/rewrite behavior on a stuck repo.
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:200:1. **If `dracon-ai-lib` should continue to be the canonical repo**
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:201:   - Unarchive `DraconDev/dracon-ai-lib` on GitHub.
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:202:   - Re-run `git push origin main`.
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:205:2. **If `dracon-ai-lib` should move to a new active repo**
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:208:   - Push local `main` to the new remote.
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:221:- Push blocker evidence: `git push --dry-run origin main` in `git-evidence.txt` and `final-git-evidence.txt`
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:224:- Low-risk fix applied: commit `b87f979`, diff scope limited to `crates/ai-lib/src/providers/minimax.rs`
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:2:# dracon-ai-lib
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:4:Standalone Rust workspace for an importable BYOK AI client library.
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:7:provider API keys, base URLs, and models. `ai-lib` does not read `.env`, does
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:9:fail over, does not enforce quotas, and is not the `ai-api` gateway.
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:11:## [`crates/ai-lib`](crates/ai-lib/)
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:13:Super-simple Rust client for direct AI provider access.
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:17:A thin async client that calls OpenAI, OpenRouter, NVIDIA, Mistral, DeepSeek,
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:18:Apertis, and other OpenAI-compatible providers **directly** from your Rust
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:24:### When to use `ai-lib` directly vs. `ai-api` gateway
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:28:| Bring your own OpenAI/OpenRouter/NVIDIA/etc. API key | `ai-lib` simple consumer API |
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:29:| Bring your own Anthropic API key | `ai-lib` Anthropic API |
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:30:| Dracon user auth, per-tier quota, rate limits, dashboard billing | `ai-api` HTTP gateway |
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:31:| Pooled provider keys owned by dracon | `ai-api` key vault |
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:32:| Lane routing / failover / per-tier model selection | `ai-api` lane router |
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:36:`ai-lib` does **not** own lanes, routing, failover, quotas, or key pools.
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:37:`Lane` is only an opaque request tag on engine-level `AiRequest` values so a
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:38:gateway such as `ai-api` can carry the caller's selected lane into the provider
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:40:system prompts, failover, and key lookup.
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:42:For direct consumers, the simple API is still one thing: create `AiClient`,
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:48:ai-lib = { git = "https://github.com/DraconDev/dracon-ai-lib", tag = "v0.2.0" }
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:53:use ai_lib::{AiClient, ChatRequest, Message, Role};
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:55:let client = AiClient::new();
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:58:    "https://api.openai.com/v1".into(),
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:65:let response = client.chat(request).await?;
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:68:## [`crates/ai-models-catalog`](crates/ai-models-catalog/)
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:80:In short: `models.dev` is the external release metadata aggregator, `ai-models-catalog`
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:81:is the local typed mirror, and `ai-api` owns lane/routing decisions.
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:97:ai-lib = { git = "https://github.com/DraconDev/dracon-ai-lib", tag = "v0.2.0" }
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:108:# dracon-ai-lib — Consumer Guide
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:110:> **⚠️ ARCHIVED — Use [`ai-api-sdk`](https://github.com/DraconDev/dracon-ai-platform/tree/main/crates/ai-api-sdk) in the `dracon-ai-platform` repo instead. This lib is frozen at v0.2.0.**
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:112:The lib is a pure AI engine — Rust code that talks directly to OpenAI, Anthropic, Gemini, etc. It does **not** run a server. It does **not** ship a key vault. It does **not** auto-load secrets.
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:118:| **One consumer, one set of keys, no sharing** (solo dev) | The lib directly (`dracon-ai-client`) |
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:119:| **Two or more consumers sharing the same provider keys** | `ai-api` HTTP gateway via `ai-api-sdk` |
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:120:| **Cross-language or cross-network consumers** | `ai-api` HTTP gateway via `ai-api-sdk` |
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:124:1. **Quota collisions** — if `avid` and `ai-auto-writer` both hit `AI_KEY_OPENROUTER`, you burn through rate limits twice as fast.
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:126:3. **Disk-wipe fragility** — five consumers, five `.env` files, five backups. ai-api = one place.
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:127:4. **No per-consumer auth** — the lib is in-process, no consumer identity. ai-api can issue per-consumer API keys.
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:130:`ai-api` solves all of these. The lib is fine for solo dev. The moment you go multi-consumer, go gateway.
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:138:| **Direct — `dracon-ai-client` (this lib)** | `AiClient::from_env()` reads `AI_KEY_*` from your env | Solo dev. One consumer, one key set, no sharing. |
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:139:| **HTTP gateway — `ai-api-sdk` (in the platform repo)** | HTTP client to an `ai-api` server | Two or more consumers sharing keys. Cross-language. Cross-network. BYOK. |
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:141:### What ai-api gives you (that the lib doesn't)
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:143:- **Per-consumer API keys** — each consumer gets a unique `AI_API_KEY`. Revoke one, others keep working.
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:144:- **BYOK (Bring Your Own Key)** — consumers upload their own provider keys to ai-api via `POST /v1/keys`. Their key is stored encrypted, used only for their requests.
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:145:- **Central quota tracking** — one provider key, many consumers, but ai-api throttles per consumer so one consumer can't starve the others.
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:147:- **Cross-language** — anything that can speak HTTP (Python, JS, Go, shell) can use ai-api.
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:149:### What you give up going through ai-api
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:152:- ai-api becomes a hard dependency — it must be running for the consumer to work.
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:153:- You trust whoever runs ai-api with the keys (BYOK mitigates this if users upload their own).
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:163:→ Run `ai-api` somewhere. Have each consumer point at it via `ai-api-sdk`. BYOK is optional — consumers can use the platform's central key pool.
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:166:→ Run `ai-api`. External users upload their own keys via the BYOK endpoint. They get a consumer API key, never see provider keys directly. The consumer (your binary) uses `ai-api-sdk`.
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:169:→ `ai-api-sdk` (or any HTTP client) over `ai-api`. The lib is in-process and Rust-only.
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:176:## Three patterns (implementation details)
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:182:| **A. Consumer provides keys (default for solo dev)** | You want to manage your own keys | Set `AI_KEY_*` env vars yourself |
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:186:> **Going multi-consumer? Skip these patterns.** Use `ai-api-sdk` against an `ai-api` server with BYOK instead. See the [ai-api-sdk README](https://github.com/DraconDev/dracon-ai-platform) for the BYOK flow.
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:198:dracon-ai-client = { git = "https://github.com/DraconDev/dracon-ai-lib.git", tag = "v0.2.0" }
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:204:dracon-ai-contracts = { git = "https://github.com/DraconDev/dracon-ai-lib.git", tag = "v0.2.0" }
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:207:Set your own `AI_KEY_*` env vars (or use a `.env` file in your project):
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:211:AI_KEY_OPENAI=sk-your-key-here
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:212:AI_KEY_ANTHROPIC=sk-ant-your-key
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:218:use dracon_ai_client::AiClient;
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:219:use dracon_ai_contracts::ChatMessage;
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:221:#[tokio::main]
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:222:async fn main() -> Result<(), Box<dyn std::error::Error>> {
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:223:    // Reads AI_KEY_* from YOUR environment, not the lib's
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:224:    let client = AiClient::from_env()?;
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:228:    ]).await?;
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:235:If you don't set any keys, `from_env()` will fail with: `no AI_KEY_* environment variables found`.
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:241:If you want to use the lib's default keys (the ones in `dracon-ai-lib/.env`), you must explicitly opt in by calling `load_lib_env()`:
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:244:use dracon_ai_client::{AiClient, load_lib_env};
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:245:use dracon_ai_contracts::ChatMessage;
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:247:#[tokio::main]
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:248:async fn main() -> Result<(), Box<dyn std::error::Error>> {
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:253:    let client = AiClient::from_env()?;
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:257:    ]).await?;
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:264:- Finds the lib's `.env` (via `DRACON_AI_LIB_ENV` env var, manifest dir, or `./dracon-ai-lib/.env`)
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:269:**Consumer keys always win** — if you set `AI_KEY_OPENAI` before calling `load_lib_env()`, your key is kept.
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:284:This produces a plain `KEY=value` file with all the lib's keys. Copy it into your project:
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:288:AI_KEY_OPENAI=sk-...
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:289:AI_KEY_ANTHROPIC=sk-ant-...
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:313:1. **By default, consumers use their own keys** — they have to set `AI_KEY_*` themselves. The lib doesn't touch their env.
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:314:2. **The lib's keys are available** — they're in `.env` at the repo root. Consumers can opt in (`load_lib_env()`) or copy them (`extract-keys`).
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:316:4. **Warden encrypts the lib's `.env` in git** — the keys are safe in version control, but available in the working tree.
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:318:This is the simplest possible model. No magic, no embedded plaintext, no key distribution problems.
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:330:Author:     DraconDev <dracsharp@gmail.com>
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:332:Commit:     DraconDev <dracsharp@gmail.com>
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:335:    archive: mark lib as archived, redirect to ai-api-sdk
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:343:Author: DraconDev <dracsharp@gmail.com>
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:345:    archive: mark lib as archived, redirect to ai-api-sdk
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:348:ai-lib
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:349:ai-models-catalog
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:350:dracon-ai-client
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:351:dracon-ai-contracts
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:352:dracon-ai-core
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:353:dracon-ai-providers
docs/audit/2026-06-11-full-repo-audit/final/hygiene.tsv:10:/home/dracon/Dev/ai-auto-writer	217	3	4	80	0
docs/audit/2026-06-11-full-repo-audit/final/hygiene.tsv:12:/home/dracon/Dev/rust-ai-web-auto	46	2	2	1	0
docs/audit/2026-06-11-full-repo-audit/final/hygiene.tsv:13:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	77	1	6	1	0
docs/audit/2026-06-11-full-repo-audit/final/hygiene.tsv:19:/home/dracon/Dev/dracon-ai-lib	35	3	14	1	0
docs/audit/2026-06-11-full-repo-audit/final/final-validation.tsv:4:ai-auto-repo-rot-scanner-todo-agent	0	0	0	
docs/audit/2026-06-11-full-repo-audit/final/final-validation.tsv:8:ai-auto-writer	0	0	0	
docs/audit/2026-06-11-full-repo-audit/final/final-validation.tsv:10:rust-ai-web-auto	0	0	0	
docs/audit/2026-06-11-full-repo-audit/final/final-validation.tsv:15:dracon-ai-lib	0	0	0	
docs/audit/2026-06-11-full-repo-audit/final/inventory.json:8:  "failures": 0,
docs/audit/2026-06-11-full-repo-audit/final/inventory.json:15:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/final/inventory.json:38:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/final/inventory.json:54:      "hint": "run repair-warns --apply"
docs/audit/2026-06-11-full-repo-audit/final/inventory.json:61:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/final/inventory.json:84:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/final/inventory.json:93:      "last_msg": "9 file(s) in web [web/tests/ai-hub/ai-hub.spec.ts, web/PAGE-AUDIT.md, C…",
docs/audit/2026-06-11-full-repo-audit/final/inventory.json:107:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/final/inventory.json:130:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/final/inventory.json:153:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/final/inventory.json:195:      "repo": "/home/dracon/Dev/ai-auto-writer",
docs/audit/2026-06-11-full-repo-audit/final/inventory.json:199:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/final/inventory.json:222:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/final/inventory.json:241:      "repo": "/home/dracon/Dev/rust-ai-web-auto",
docs/audit/2026-06-11-full-repo-audit/final/inventory.json:245:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/final/inventory.json:264:      "repo": "/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent",
docs/audit/2026-06-11-full-repo-audit/final/inventory.json:268:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/final/inventory.json:291:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/final/inventory.json:314:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/final/inventory.json:337:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/final/inventory.json:360:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/final/inventory.json:369:      "last_msg": "19 file(s) in examples,src,tests [src/cli.rs, src/ai_gen.rs, src/analyz…",
docs/audit/2026-06-11-full-repo-audit/final/inventory.json:383:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/final/inventory.json:402:      "repo": "/home/dracon/Dev/dracon-ai-lib",
docs/audit/2026-06-11-full-repo-audit/final/inventory.json:407:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/final/inventory.json:416:      "last_msg": "1 file(s) in crates [crates/ai-lib/src/providers/minimax.rs] DELTA:+48/…",
docs/audit/2026-06-11-full-repo-audit/final/inventory.json:420:      "push_error": "ahead=13, push failing",
docs/audit/2026-06-11-full-repo-audit/final/inventory.json:423:      "hint": "run repair-concerns --apply (push or rewrite)"
docs/audit/2026-06-11-full-repo-audit/final/inventory.json:430:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/final/inventory.json:453:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-before.tsv:2:/home/dracon/Dev/dracon-ai-lib	main	0	0	0	30	0	AHEAD:30,STUCK_PUSH	STUCK	run repair-concerns --apply (push or rewrite)
docs/audit/2026-06-11-full-repo-audit/final/REPORT.md:4:Scope: every repo reported by `dracon-sync repos --json --full-path`, explicitly including `dracon-ai-lib`.
docs/audit/2026-06-11-full-repo-audit/final/REPORT.md:8:- Final inventory contains **20 Dracon-managed repos**.
docs/audit/2026-06-11-full-repo-audit/final/REPORT.md:12:- Remaining public-readiness blockers are documented and require explicit approval or access decisions before any publication step.
docs/audit/2026-06-11-full-repo-audit/final/REPORT.md:17:   - Removed `dracon-ai-lib` from `exclude_repos` in the sync policy so it is included in `dracon-sync repos`.
docs/audit/2026-06-11-full-repo-audit/final/REPORT.md:20:2. **`dracon-ai-lib`**
docs/audit/2026-06-11-full-repo-audit/final/REPORT.md:21:   - Fixed invalid origin URL and pointed it at the valid `https://github.com/DraconDev/dracon-ai-lib.git` remote.
docs/audit/2026-06-11-full-repo-audit/final/REPORT.md:23:   - Push remains blocked because the remote is archived/stuck; this requires an explicit recreate/unarchive/rewrite decision.
docs/audit/2026-06-11-full-repo-audit/final/REPORT.md:27:   - Fixed `cargo clippy --workspace -- -D warnings` failures across AI/billing/email/auth APIs:
docs/audit/2026-06-11-full-repo-audit/final/REPORT.md:39:5. **`ai-auto-writer`**
docs/audit/2026-06-11-full-repo-audit/final/REPORT.md:40:   - Fixed stale Dracon AI integration references to unavailable `dracon_ai_contracts` / `dracon_ai_client` APIs.
docs/audit/2026-06-11-full-repo-audit/final/REPORT.md:41:   - Reworked the service wrapper to use the repo's `ai-api-sdk` dependency.
docs/audit/2026-06-11-full-repo-audit/final/REPORT.md:46:   - Updated the CLI contract test to accept the existing plaintext-store/workspace error path when no passphrase is provided.
docs/audit/2026-06-11-full-repo-audit/final/REPORT.md:57:| `ai-auto-repo-rot-scanner-todo-agent` | 0 | 0 | 0 | pass |
docs/audit/2026-06-11-full-repo-audit/final/REPORT.md:61:| `ai-auto-writer` | 0 | 0 | 0 | pass |
docs/audit/2026-06-11-full-repo-audit/final/REPORT.md:63:| `rust-ai-web-auto` | 0 | 0 | 0 | pass |
docs/audit/2026-06-11-full-repo-audit/final/REPORT.md:68:| `dracon-ai-lib` | 0 | 0 | 0 | pass; push still stuck |
docs/audit/2026-06-11-full-repo-audit/final/REPORT.md:77:| `browser-extensions-shared` | No root package scripts; hygiene/public-readiness blocker remains |
docs/audit/2026-06-11-full-repo-audit/final/REPORT.md:78:| `DraconDev` | No documented local build/test command; docs/profile triage remains |
docs/audit/2026-06-11-full-repo-audit/final/REPORT.md:98:| `dracon-ai-lib` | **Blocked** | Local validation passes, but repo is ahead 13 and push is stuck. Needs explicit remote/recreate/unarchive/rewrite decision. |
docs/audit/2026-06-11-full-repo-audit/final/REPORT.md:99:| `DraconDev` | **Not public-ready yet** | Profile/research/draft repo; no local build/test command; contains draft/scratch/profile research artifacts and `.pi` goals. Needs explicit docs/public triage. |
docs/audit/2026-06-11-full-repo-audit/final/REPORT.md:105:## Remaining blockers / decisions
docs/audit/2026-06-11-full-repo-audit/final/REPORT.md:111:2. **`dracon-ai-lib`**
docs/audit/2026-06-11-full-repo-audit/final/REPORT.md:117:   - Constraint: do not delete, rename, untrack, ignore, or rewrite these without explicit approval.
docs/audit/2026-06-11-full-repo-audit/final/per-repo/one-mil-girls.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/final/per-repo/one-mil-girls.git.txt:13:* main c016882 [origin/main] 1 file(s) in .svelte-kit [.svelte-kit/ambient.d.ts] DELTA:+190/-192
docs/audit/2026-06-11-full-repo-audit/final/per-repo/one-mil-girls.git.txt:18:0fcf39c 6 file(s) in docs [docs/audit/2026-06-11-full-audit/menu/state.json] DELTA:+901/-0 | BIN:5 NEW:menu/00-title.png,menu/01-main-menu.png,menu/02-save-screen.png,menu/03-pause-menu.png,menu/04-settings.png,menu/state.json
docs/audit/2026-06-11-full-repo-audit/final/per-repo/one-mil-girls.git.txt:19:2250517 3 file(s) in .svelte-kit,src [.svelte-kit/ambient.d.ts, src/lib/components/SettingsScreen.svelte, src/lib/components/MainMenu.svelte] DELTA:+292/-200
docs/audit/2026-06-11-full-repo-audit/hygiene.tsv:7:/home/dracon/Dev/ai-auto-writer	217	3	4	80	0
docs/audit/2026-06-11-full-repo-audit/hygiene.tsv:9:/home/dracon/Dev/rust-ai-web-auto	46	2	2	1	0
docs/audit/2026-06-11-full-repo-audit/hygiene.tsv:14:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	77	1	6	1	0
docs/audit/2026-06-11-full-repo-audit/hygiene.tsv:17:/home/dracon/Dev/dracon-ai-lib	35	3	14	1	0
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-after-rust-clean.tsv:2:/home/dracon/Dev/browser-extensions-shared	main	1	0	2	0	0	DIRTY	OK	run repair-warns --apply
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-after-rust-clean.tsv:3:/home/dracon/Dev/dracon-code	main	1	0	0	0	0	DIRTY	OK	run repair-warns --apply
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-after-rust-clean.tsv:4:/home/dracon/Dev/dracon-utilities	main	0	0	1	0	0	DIRTY	OK	healthy
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-after-rust-clean.tsv:5:/home/dracon/Dev/dracon-platform	main	0	0	0	3	0	AHEAD:3,STUCK_PUSH	STUCK	run repair-concerns --apply (push or rewrite)
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-after.json:8:  "failures": 0,
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-after.json:15:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-after.json:34:      "repo": "/home/dracon/Dev/dracon-ai-lib",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-after.json:38:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-after.json:61:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-after.json:80:      "repo": "/home/dracon/Dev/rust-ai-web-auto",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-after.json:84:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-after.json:107:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-after.json:130:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-after.json:139:      "last_msg": "2 file(s) in plan [Cargo.lock, plan/remaining-work-inventory.md] DELTA:…",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-after.json:153:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-after.json:162:      "last_msg": "4 file(s) in docs [docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-…",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-after.json:199:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-after.json:215:      "hint": "run repair-warns --apply"
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-after.json:222:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-after.json:245:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-after.json:264:      "repo": "/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-after.json:268:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-after.json:287:      "repo": "/home/dracon/Dev/ai-auto-writer",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-after.json:291:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-after.json:314:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-after.json:337:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-after.json:360:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-after.json:369:      "last_msg": "19 file(s) in examples,src,tests [src/cli.rs, src/ai_gen.rs, src/analyz…",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-after.json:383:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-after.json:406:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-after.json:429:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:3:/home/dracon/Dev/dracon-ai-lib	main	0	0	0	30	0	AHEAD:30,STUCK_PUSH	STUCK	run repair-concerns --apply (push or rewrite)
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:5:{"archivedAt":"2026-06-08T20:06:42Z","createdAt":"2026-05-31T20:31:51Z","defaultBranchRef":{"name":"main"},"description":"","isArchived":true,"updatedAt":"2026-06-08T20:06:42Z","url":"https://github.com/DraconDev/dracon-ai-lib","visibility":"PRIVATE"}
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:7:{"archived":true,"archived_at":null,"default_branch":"main","description":null,"full_name":"DraconDev/dracon-ai-lib","html_url":"https://github.com/DraconDev/dracon-ai-lib","permissions":{"admin":true,"maintain":true,"pull":true,"push":true,"triage":true},"visibility":"private"}
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:10:origin	https://github.com/DraconDev/dracon-ai-lib.git (fetch)
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:11:origin	https://github.com/DraconDev/dracon-ai-lib.git (push)
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:13:* main 5d0ae6c [origin/main: ahead 30] 2 file(s) [pi-session-2026-06-04T15-35-11-680Z_019e9346-3240-7a0b-9c26-bb1652ca26ac.html, pi-session-2026-05-31T19-29-49-863Z_019e7f83-9327-77f1-ba62-393dabbf8696.html] DELTA:+0/-8499 | DEL:pi-session-2026-05-31T19-29-49-863Z_019e7f83-9327-77f1-ba62-393dabbf8696.html,pi-session-2026-06-04T15-35-11-680Z_019e9346-3240-7a0b-9c26-bb1652ca26ac.html
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:18:9cb5103 docs: fix stale ai-lib release tag wording
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:21:6882198 simplify: drop the dracon-ai/* cutover theater; use the real repo URL
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:22:ce377a2 (origin/main, origin/HEAD) 5 file(s) in .pi [.pi/goals/archived/goal_2026060813080905_mq53j6wk-tc1udv.md, .pi/goals/archived/goal_2026060816375771_mq5ckllx-58ktk5.md, .pi/goals/archived/goal_2026060817103628_mq5ebocf-8c0p11.md] DELTA:+405/-9 | NEW:archived/goal_2026060813080905_mq53j6wk-tc1udv.md,archived/goal_2026060816375771_mq5ckllx-58ktk5.md,archived/goal_2026060817103628_mq5ebocf-8c0p11.md TAG:v0.2.0-archived GOAL:complete TOKENS:55388K TIME:588m
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:23:14397af archive: fix remaining dracon-ai-sdk references to ai-api-sdk
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:24:4fc7206 archive: mark lib as archived, redirect to ai-api-sdk
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:47:--- archive commit details ---
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:49:Author:     DraconDev <dracsharp@gmail.com>
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:51:Commit:     DraconDev <dracsharp@gmail.com>
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:54:    archive: mark lib as archived, redirect to ai-api-sdk
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:62:Author:     DraconDev <dracsharp@gmail.com>
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:64:Commit:     DraconDev <dracsharp@gmail.com>
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:67:    archive: fix remaining dracon-ai-sdk references to ai-api-sdk
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:74:Author:     DraconDev <dracsharp@gmail.com>
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:76:Commit:     DraconDev <dracsharp@gmail.com>
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:92:docs/audit/2026-06-11-full-repo-audit/per-repo/ai-auto-repo-rot-scanner-todo-agent.git.txt:14:* main 90c4433 [origin/main] refactor(ai): migrate from archived dracon-ai-lib to ai-api-sdk
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:93:docs/audit/2026-06-11-full-repo-audit/per-repo/ai-auto-repo-rot-scanner-todo-agent.git.txt:16:90c4433 refactor(ai): migrate from archived dracon-ai-lib to ai-api-sdk
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:96:docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/ai-auto-repo-rot-scanner-todo-agent.git.txt:16:90c4433 refactor(ai): migrate from archived dracon-ai-lib to ai-api-sdk
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:98:docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/browser-extensions-shared.git.txt:20:* main 337c73a3d [origin/main] 2 file(s) in auto-form-filler,wxt-shared [wxt-shared/src/byok/BYOKSettings.tsx, auto-form-filler/entrypoints/options/App.tsx] DELTA:+17/-6
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:102:docs/audit/2026-06-11-full-repo-audit/post-funding/validation-logs/ai-auto-repo-rot-scanner-todo-agent.test.log:298:test output::sarif::tests::test_sarif_format_deprecated ... ok
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:103:docs/audit/2026-06-11-full-repo-audit/post-funding/validation-logs/ai-auto-repo-rot-scanner-todo-agent.test.log:577:test test_check_deprecated_configurable ... ok
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:104:docs/audit/2026-06-11-full-repo-audit/post-funding/validation-logs/ai-auto-repo-rot-scanner-todo-agent.test.log:578:test test_check_deprecated_default ... ok
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:109:docs/audit/2026-06-11-full-repo-audit/post-funding/validation-logs/dracon-platform.test.log:4:  --> apis/services/ai-api/ai-api-sdk/tests/sdk.rs:66:4
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:110:docs/audit/2026-06-11-full-repo-audit/post-funding/validation-logs/dracon-platform.test.log:11:warning: `ai-api-sdk` (test "sdk") generated 1 warning
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:112:docs/audit/2026-06-11-full-repo-audit/post-funding/validation-logs/dracon-platform.test.log:443:test apis/services/ai-api/ai-api-sdk/src/lib.rs - (line 8) ... ignored
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:113:docs/audit/2026-06-11-full-repo-audit/post-funding/validation-logs/dracon-platform.test.log:444:test apis/services/ai-api/ai-api-sdk/src/lib.rs - DraconAi::chat_stream (line 139) ... ignored
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:116:docs/audit/2026-06-11-full-repo-audit/post-funding/deps/dracon-code.tree.log:23:        ├── ai-api-sdk v0.2.0 (/home/dracon/Dev/dracon-platform/apis/services/ai-api/ai-api-sdk)
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:117:docs/audit/2026-06-11-full-repo-audit/post-funding/deps/dracon-code.tree.log:60:    ├── ai-api-sdk v0.2.0 (/home/dracon/Dev/dracon-platform/apis/services/ai-api/ai-api-sdk) (*)
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:118:docs/audit/2026-06-11-full-repo-audit/post-funding/deps/dracon-code.deny.log:16:11 │ ai-api-sdk = { workspace = true }
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:119:docs/audit/2026-06-11-full-repo-audit/post-funding/deps/dracon-code.deny.log:29:11 │ ai-api-sdk = { workspace = true }
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:120:docs/audit/2026-06-11-full-repo-audit/post-funding/deps/dracon-platform.tree.log:43:│   │       ├── ai-api-sdk v0.2.0 (/home/dracon/Dev/dracon-platform/apis/services/ai-api/ai-api-sdk)
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:121:docs/audit/2026-06-11-full-repo-audit/post-funding/deps/dracon-platform.tree.log:229:│           ├── ai-api-sdk v0.2.0 (/home/dracon/Dev/dracon-platform/apis/services/ai-api/ai-api-sdk)
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:122:docs/audit/2026-06-11-full-repo-audit/post-funding/deps/dracon-platform.tree.log:316:│   ├── ai-api-sdk v0.2.0 (/home/dracon/Dev/dracon-platform/apis/services/ai-api/ai-api-sdk)
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:123:docs/audit/2026-06-11-full-repo-audit/post-funding/deps/dracon-platform.tree.log:365:│   ├── ai-api-sdk v0.2.0 (/home/dracon/Dev/dracon-platform/apis/services/ai-api/ai-api-sdk)
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:124:docs/audit/2026-06-11-full-repo-audit/post-funding/deps/dracon-platform.tree.log:392:    ├── ai-api-sdk v0.2.0 (/home/dracon/Dev/dracon-platform/apis/services/ai-api/ai-api-sdk)
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:125:docs/audit/2026-06-11-full-repo-audit/post-funding/deps/dracon-platform.tree.log:470:│   ├── ai-api-sdk v0.2.0 (/home/dracon/Dev/dracon-platform/apis/services/ai-api/ai-api-sdk)
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:128:docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/dracon-ai-lib.git.txt:7:* main d8846da [origin/main: ahead 21] docs: make crate docs explicit BYOK-library contract
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:129:docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/dracon-ai-lib.git.txt:9:d8846da docs: make crate docs explicit BYOK-library contract
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:130:docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/dracon-ai-lib.git.txt:10:4b70129 4 file(s) in docs [docs/archive/legacy-key-management-design.md, docs/consumer-getting-started.md, README.md] DELTA:+26/-8
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:131:docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/dracon-ai-lib.git.txt:11:210c250 1 file(s) in docs [docs/{key-management-design.md => archive/legacy-key-management-design.md}] DELTA:+0/-0
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:133:docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/validation-logs/ai-auto-repo-rot-scanner-todo-agent.test.log:297:test output::sarif::tests::test_sarif_format_deprecated ... ok
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:134:docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/validation-logs/ai-auto-repo-rot-scanner-todo-agent.test.log:576:test test_check_deprecated_configurable ... ok
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:135:docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/validation-logs/ai-auto-repo-rot-scanner-todo-agent.test.log:577:test test_check_deprecated_default ... ok
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:140:docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/validation-logs/dracon-platform.test.log:5:  --> apis/services/ai-api/ai-api-sdk/tests/sdk.rs:66:4
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:141:docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/validation-logs/dracon-platform.test.log:12:warning: `ai-api-sdk` (test "sdk") generated 1 warning
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:143:docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/validation-logs/dracon-platform.test.log:445:test apis/services/ai-api/ai-api-sdk/src/lib.rs - (line 8) ... ignored
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:144:docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/validation-logs/dracon-platform.test.log:446:test apis/services/ai-api/ai-api-sdk/src/lib.rs - DraconAi::chat_stream (line 139) ... ignored
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:148:docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/PUBLIC_READINESS.md:36:3. **`dracon-ai-lib`**: decide remote strategy. It is still AHEAD:21 and push is blocked by the archived remote.
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:149:docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/PUBLIC_READINESS.md:60:- `dracon-ai-lib`: local validation passes, but push is blocked (AHEAD:21, archived remote).
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:150:docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/deps/dracon-code.tree.log:23:        ├── ai-api-sdk v0.2.0 (/home/dracon/Dev/dracon-platform/apis/services/ai-api/ai-api-sdk)
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:151:docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/deps/dracon-code.tree.log:60:    ├── ai-api-sdk v0.2.0 (/home/dracon/Dev/dracon-platform/apis/services/ai-api/ai-api-sdk) (*)
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:152:docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/deps/dracon-code.deny.log:18:11 │ ai-api-sdk = { workspace = true }
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:153:docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/deps/dracon-code.deny.log:31:11 │ ai-api-sdk = { workspace = true }
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:154:docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/deps/dracon-platform.tree.log:43:│   │       ├── ai-api-sdk v0.2.0 (/home/dracon/Dev/dracon-platform/apis/services/ai-api/ai-api-sdk)
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:155:docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/deps/dracon-platform.tree.log:229:│           ├── ai-api-sdk v0.2.0 (/home/dracon/Dev/dracon-platform/apis/services/ai-api/ai-api-sdk)
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:156:docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/deps/dracon-platform.tree.log:316:│   ├── ai-api-sdk v0.2.0 (/home/dracon/Dev/dracon-platform/apis/services/ai-api/ai-api-sdk)
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:157:docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/deps/dracon-platform.tree.log:365:│   ├── ai-api-sdk v0.2.0 (/home/dracon/Dev/dracon-platform/apis/services/ai-api/ai-api-sdk)
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:158:docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/deps/dracon-platform.tree.log:392:    ├── ai-api-sdk v0.2.0 (/home/dracon/Dev/dracon-platform/apis/services/ai-api/ai-api-sdk)
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:159:docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/deps/dracon-platform.tree.log:470:│   ├── ai-api-sdk v0.2.0 (/home/dracon/Dev/dracon-platform/apis/services/ai-api/ai-api-sdk)
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:161:docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.dracon-platform.git.txt:19:* main                                     928216dbd [origin/main] 8 file(s) in apis,web [apis/services/ai-api/tests/happy_path.rs, apis/services/ai-api/tests/common/mod.rs, apis/services/ai-api/ai-api-sdk/tests/sdk.rs] DELTA:+160/-120 | TEST:276 BIN:2
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:162:docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.dracon-platform.git.txt:29:928216dbd 8 file(s) in apis,web [apis/services/ai-api/tests/happy_path.rs, apis/services/ai-api/tests/common/mod.rs, apis/services/ai-api/ai-api-sdk/tests/sdk.rs] DELTA:+160/-120 | TEST:276 BIN:2
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:163:docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.dracon-ai-lib.git.txt:9:* main d8846da [origin/main: ahead 21] docs: make crate docs explicit BYOK-library contract
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:164:docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.dracon-ai-lib.git.txt:11:d8846da docs: make crate docs explicit BYOK-library contract
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:165:docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.dracon-ai-lib.git.txt:12:4b70129 4 file(s) in docs [docs/archive/legacy-key-management-design.md, docs/consumer-getting-started.md, README.md] DELTA:+26/-8
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:166:docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.dracon-ai-lib.git.txt:13:210c250 1 file(s) in docs [docs/{key-management-design.md => archive/legacy-key-management-design.md}] DELTA:+0/-0
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:167:docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.dracon-ai-lib.git.txt:17:d8846da docs: make crate docs explicit BYOK-library contract
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:168:docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.dracon-ai-lib.git.txt:18:4b70129 4 file(s) in docs [docs/archive/legacy-key-management-design.md, docs/consumer-getting-started.md, README.md] DELTA:+26/-8
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:171:docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.dracon-platform.git.txt:30:928216dbd 8 file(s) in apis,web [apis/services/ai-api/tests/happy_path.rs, apis/services/ai-api/tests/common/mod.rs, apis/services/ai-api/ai-api-sdk/tests/sdk.rs] DELTA:+160/-120 | TEST:276 BIN:2
docs/audit/2026-06-11-full-repo-audit/post-funding/inventory.tsv:2:/home/dracon/Dev/browser-extensions-shared	main	0	0	2	0	0	DIRTY	OK	healthy
docs/audit/2026-06-11-full-repo-audit/post-funding/inventory.tsv:4:/home/dracon/Dev/folder-auto-banner	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/post-funding/inventory.tsv:5:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/post-funding/inventory.tsv:6:/home/dracon/Dev/pully-fully-pull-based-fleet-reconciler	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/post-funding/inventory.tsv:7:/home/dracon/Dev/dracon-utilities	main	1	0	0	0	0	DIRTY	OK	run repair-warns --apply
docs/audit/2026-06-11-full-repo-audit/post-funding/inventory.tsv:8:/home/dracon/.dracon	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/post-funding/inventory.tsv:9:/home/dracon/Dev/DraconDev	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/post-funding/inventory.tsv:10:/home/dracon/Dev/one-mil-girls	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/post-funding/inventory.tsv:11:/home/dracon/Dev/dracon-code	main	2	0	0	0	0	DIRTY	OK	run repair-warns --apply
docs/audit/2026-06-11-full-repo-audit/post-funding/inventory.tsv:12:/home/dracon/Dev/dracon-platform	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/post-funding/inventory.tsv:13:/home/dracon/Dev/rust-ai-web-auto	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/post-funding/inventory.tsv:14:/home/dracon/Dev/dracon-ai-lib	main	0	0	0	18	0	AHEAD:18,STUCK_PUSH	STUCK	run repair-concerns --apply (push or rewrite)
docs/audit/2026-06-11-full-repo-audit/post-funding/inventory.tsv:15:/home/dracon/Dev/ai-auto-writer	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/post-funding/inventory.tsv:16:/home/dracon/Dev/video-uploader	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/post-funding/inventory.tsv:17:/home/dracon/Dev/video-factory	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/post-funding/inventory.tsv:18:/home/dracon/Dev/avid	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/post-funding/inventory.tsv:19:/home/dracon/Dev/youtube-video-uploader	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/post-funding/inventory.tsv:20:/home/dracon/Dev/dracon-libs	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/post-funding/inventory.tsv:21:/home/dracon/Dev/kiki-sassy-desktop-announcer	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.Junk-Runner-bevy.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.Junk-Runner-bevy.git.txt:14:  main        e1894697f [origin/main] Added SOLID_VS_SVELTE.md
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/dracon-platform-hooks-warden.txt:7:# Defense-in-depth: scans push for plaintext secrets.
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/dracon-platform-hooks-warden.txt:27:    # Scan for common plaintext secret patterns
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/dracon-platform-hooks-warden.txt:29:        echo "⚠️  Possible plaintext secrets detected in push."
docs/audit/2026-06-11-full-repo-audit/git-auth-prompt/REPORT.md:12:2. The new layout `~/.dracon/secrets/{pat,registry,ai,...}` was a **parallel store** that the code did **not** read.
docs/audit/2026-06-11-full-repo-audit/git-auth-prompt/REPORT.md:26:├── ai/          # AI provider keys
docs/audit/2026-06-11-full-repo-audit/git-auth-prompt/REPORT.md:80:- Global helper: `store` (plaintext `~/.git-credentials`, currently holds only codeberg + gitlab entries, no github).
docs/audit/2026-06-11-full-repo-audit/git-auth-prompt/REPORT.md:84:Test: `git ls-remote https://github.com/DraconDev/dracon-utilities.git HEAD` with the real config → returns the SHA, no prompt. So in a normal shell, the helper chain works and the user should not be prompted for github HTTPS.
docs/audit/2026-06-11-full-repo-audit/git-auth-prompt/REPORT.md:132:## Remaining decision — applied (option 2: PAT-based git helper)
docs/audit/2026-06-11-full-repo-audit/git-auth-prompt/REPORT.md:178:  git -C /home/dracon/Dev/dracon-utilities push --dry-run origin main
docs/audit/2026-06-11-full-repo-audit/git-auth-prompt/REPORT.md:195:## Constraints respected
docs/audit/2026-06-11-full-repo-audit/final/per-repo/dracon-utilities.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/final/per-repo/dracon-utilities.git.txt:17:* main                   858f1abc [origin/main] 3 file(s) in docs [docs/audit/2026-06-11-full-repo-audit/final/REPORT.md, docs/audit/2026-06-11-full-repo-audit/final/inventory.json, docs/audit/2026-06-11-full-repo-audit/final/inventory.tsv] DELTA:+152/-20 | NEW:final/REPORT.md
docs/audit/2026-06-11-full-repo-audit/final/per-repo/dracon-utilities.git.txt:22:0a77d11f 47 file(s) in docs [docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv, docs/audit/2026-06-11-full-repo-audit/final/inventory.json, docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv] DELTA:+5629/-0 | NEW:deps/deps.tsv,final/final-validation.tsv,final/hygiene.tsv,final/inventory.json,final/inventory.tsv,non-rust/non-rust.tsv,per-repo/.dracon.git.txt,per-repo/DraconDev.git.txt,per-repo/Junk-Runner-bevy.git.txt,per-repo/ai-auto-repo-rot-scanner-todo-agent.git.txt+37more
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/ai-auto-writer.risk.tsv:47:tracked	_archive/to-release/The-Silence-Between-Tokens-vera-kincaid/PUBLICATION-README.md
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/ai-auto-writer.risk.tsv:48:tracked	_archive/to-release/The-Silence-Between-Tokens-vera-kincaid/The-Silence-Between-Tokens.epub
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/ai-auto-writer.risk.tsv:49:tracked	_archive/to-release/The-Silence-Between-Tokens-vera-kincaid/The-Silence-Between-Tokens.mobi
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/ai-auto-writer.risk.tsv:50:tracked	_archive/to-release/The-Silence-Between-Tokens-vera-kincaid/cover.jpg
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/ai-auto-writer.risk.tsv:51:tracked	_archive/to-release/The-Silence-Between-Tokens-vera-kincaid/outline.md
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/ai-auto-writer.risk.tsv:52:tracked	_archive/to-release/The-Silence-Between-Tokens-vera-kincaid/publication-metadata.json
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/ai-auto-writer.risk.tsv:124:tracked	authors/fiction/grace-monroe/books/duke-scandal-secret/.outline-detailed.json
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.pully-fully-pull-based-fleet-reconciler.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.pully-fully-pull-based-fleet-reconciler.git.txt:14:* main f9cc9ffc [origin/main] 8 file(s) in fully,pully,pully-types [fully/bins/fully/src/bootstrap.rs, fully/crates/fully-core/src/fleet_status.rs, fully/bins/fully/src/main.rs] DELTA:+213/-116
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.pully-fully-pull-based-fleet-reconciler.git.txt:16:f9cc9ffc 8 file(s) in fully,pully,pully-types [fully/bins/fully/src/bootstrap.rs, fully/crates/fully-core/src/fleet_status.rs, fully/bins/fully/src/main.rs] DELTA:+213/-116
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.pully-fully-pull-based-fleet-reconciler.git.txt:17:364735af 6 file(s) in fully,pully [AUDIT_REPORT.md, pully/bins/pully/src/main.rs, pully/README.md] DELTA:+95/-14
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.pully-fully-pull-based-fleet-reconciler.git.txt:18:90575b82 8 file(s) in fully,pully [fully/docs/CLI.md, pully/docs/CLI.md, pully/bins/pully/src/main.rs] DELTA:+436/-203
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.pully-fully-pull-based-fleet-reconciler.git.txt:19:23a92627 5 file(s) in fully,pully [fully/crates/fully-core/src/fleet_status.rs, fully/bins/fully/src/main.rs, pully/crates/pully-core/src/service_reconciler/mod.rs] DELTA:+148/-39
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-final-audit.tsv:2:/home/dracon/Dev/dracon-ai-lib	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/post-funding/inventory.json:8:  "failures": 0,
docs/audit/2026-06-11-full-repo-audit/post-funding/inventory.json:15:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/post-funding/inventory.json:61:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/post-funding/inventory.json:80:      "repo": "/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent",
docs/audit/2026-06-11-full-repo-audit/post-funding/inventory.json:84:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/post-funding/inventory.json:107:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/post-funding/inventory.json:116:      "last_msg": "6 file(s) in fully,pully [AUDIT_REPORT.md, pully/bins/pully/src/main.rs…",
docs/audit/2026-06-11-full-repo-audit/post-funding/inventory.json:130:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/post-funding/inventory.json:146:      "hint": "run repair-warns --apply"
docs/audit/2026-06-11-full-repo-audit/post-funding/inventory.json:153:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/post-funding/inventory.json:176:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/post-funding/inventory.json:199:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/post-funding/inventory.json:222:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/post-funding/inventory.json:238:      "hint": "run repair-warns --apply"
docs/audit/2026-06-11-full-repo-audit/post-funding/inventory.json:245:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/post-funding/inventory.json:264:      "repo": "/home/dracon/Dev/rust-ai-web-auto",
docs/audit/2026-06-11-full-repo-audit/post-funding/inventory.json:268:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/post-funding/inventory.json:287:      "repo": "/home/dracon/Dev/dracon-ai-lib",
docs/audit/2026-06-11-full-repo-audit/post-funding/inventory.json:292:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/post-funding/inventory.json:305:      "push_error": "ahead=18, push failing",
docs/audit/2026-06-11-full-repo-audit/post-funding/inventory.json:308:      "hint": "run repair-concerns --apply (push or rewrite)"
docs/audit/2026-06-11-full-repo-audit/post-funding/inventory.json:311:      "repo": "/home/dracon/Dev/ai-auto-writer",
docs/audit/2026-06-11-full-repo-audit/post-funding/inventory.json:315:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/post-funding/inventory.json:338:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/post-funding/inventory.json:361:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/post-funding/inventory.json:384:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/post-funding/inventory.json:393:      "last_msg": "19 file(s) in examples,src,tests [src/cli.rs, src/ai_gen.rs, src/analyz…",
docs/audit/2026-06-11-full-repo-audit/post-funding/inventory.json:407:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/post-funding/inventory.json:430:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/post-funding/inventory.json:453:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-final-audit.json:8:  "failures": 0,
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-final-audit.json:15:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-final-audit.json:24:      "last_msg": "4 file(s) in docs [docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-…",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-final-audit.json:39:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-final-audit.json:52:      "push_error": "ahead=1, push failing",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-final-audit.json:55:      "hint": "run repair-concerns --apply (push or rewrite)"
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-final-audit.json:62:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-final-audit.json:78:      "hint": "run repair-warns --apply"
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-final-audit.json:81:      "repo": "/home/dracon/Dev/dracon-ai-lib",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-final-audit.json:85:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-final-audit.json:108:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-final-audit.json:127:      "repo": "/home/dracon/Dev/rust-ai-web-auto",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-final-audit.json:131:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-final-audit.json:147:      "hint": "run repair-warns --apply"
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-final-audit.json:154:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-final-audit.json:177:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-final-audit.json:186:      "last_msg": "2 file(s) in plan [Cargo.lock, plan/remaining-work-inventory.md] DELTA:…",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-final-audit.json:216:      "hint": "run repair-warns --apply"
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-final-audit.json:223:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-final-audit.json:246:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-final-audit.json:262:      "hint": "run repair-warns --apply"
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-final-audit.json:265:      "repo": "/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-final-audit.json:269:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-final-audit.json:288:      "repo": "/home/dracon/Dev/ai-auto-writer",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-final-audit.json:292:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-final-audit.json:315:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-final-audit.json:338:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-final-audit.json:361:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-final-audit.json:370:      "last_msg": "19 file(s) in examples,src,tests [src/cli.rs, src/ai_gen.rs, src/analyz…",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-final-audit.json:384:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-final-audit.json:407:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-final-audit.json:430:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/before/inventory.tsv:2:/home/dracon/Dev/dracon-code	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/before/inventory.tsv:3:/home/dracon/Dev/pully-fully-pull-based-fleet-reconciler	main	1	0	0	0	0	DIRTY	OK	run repair-warns --apply
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/before/inventory.tsv:5:/home/dracon/Dev/one-mil-girls	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/before/inventory.tsv:6:/home/dracon/Dev/dracon-utilities	main	0	0	1	0	0	DIRTY	OK	healthy
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/before/inventory.tsv:7:/home/dracon/Dev/dracon-platform	main	1	0	1	0	0	DIRTY	OK	run repair-warns --apply
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/before/inventory.tsv:8:/home/dracon/Dev/browser-extensions-shared	main	10	0	3	0	0	DIRTY	OK	run repair-warns --apply
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/before/inventory.tsv:9:/home/dracon/Dev/rust-ai-web-auto	main	4	0	0	0	0	DIRTY	OK	run repair-warns --apply
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/before/inventory.tsv:10:/home/dracon/.dracon	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/before/inventory.tsv:11:/home/dracon/Dev/dracon-ai-lib	main	2	0	0	21	0	DIRTY,AHEAD:21,STUCK_PUSH	STUCK	run repair-concerns --apply (push or rewrite)
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/before/inventory.tsv:12:/home/dracon/Dev/folder-auto-banner	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/before/inventory.tsv:13:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/before/inventory.tsv:14:/home/dracon/Dev/DraconDev	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/before/inventory.tsv:15:/home/dracon/Dev/ai-auto-writer	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/before/inventory.tsv:16:/home/dracon/Dev/video-uploader	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/before/inventory.tsv:17:/home/dracon/Dev/video-factory	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/before/inventory.tsv:18:/home/dracon/Dev/avid	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/before/inventory.tsv:19:/home/dracon/Dev/youtube-video-uploader	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/before/inventory.tsv:20:/home/dracon/Dev/dracon-libs	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/before/inventory.tsv:21:/home/dracon/Dev/kiki-sassy-desktop-announcer	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/ai-auto-repo-rot-scanner-todo-agent.risk.tsv:60:tracked	.ralph/old-loops/lop-remaining.md
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/ai-auto-repo-rot-scanner-todo-agent.risk.tsv:61:tracked	.ralph/old-loops/lop-remaining.state.json
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/ai-auto-repo-rot-scanner-todo-agent.risk.tsv:64:tracked	.ralph/old-loops/remaining-todos.md
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/ai-auto-repo-rot-scanner-todo-agent.risk.tsv:65:tracked	.ralph/old-loops/remaining-todos.state.json
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/ai-auto-repo-rot-scanner-todo-agent.risk.tsv:75:tracked	.ralph/supply-chain-security.md
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/ai-auto-repo-rot-scanner-todo-agent.risk.tsv:76:tracked	.ralph/supply-chain-security.state.json
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.youtube-video-uploader.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.youtube-video-uploader.git.txt:13:* main 771d422 [origin/main] Merge https://github.com/DraconDev/youtube-video-uploader
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-post-tests.json:8:  "failures": 0,
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-post-tests.json:16:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-post-tests.json:29:      "push_error": "ahead=1, push failing",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-post-tests.json:32:      "hint": "run repair-concerns --apply (push or rewrite)"
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-post-tests.json:35:      "repo": "/home/dracon/Dev/rust-ai-web-auto",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-post-tests.json:39:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-post-tests.json:55:      "hint": "run repair-warns --apply"
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-post-tests.json:62:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-post-tests.json:78:      "hint": "run repair-warns --apply"
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-post-tests.json:85:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-post-tests.json:94:      "last_msg": "1 file(s) in web [web/ai-hub/src/routes/ai-hub/+page.svelte] DELTA:+1/-1",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-post-tests.json:101:      "hint": "run repair-warns --apply"
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-post-tests.json:108:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-post-tests.json:117:      "last_msg": "2 file(s) in docs [docs/audit/2026-06-11-full-repo-audit/remaining-conc…",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-post-tests.json:131:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-post-tests.json:154:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-post-tests.json:163:      "last_msg": "5 file(s) in crates,plugins [crates/dracon-ai/src/ai_client.rs, crates/…",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-post-tests.json:177:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-post-tests.json:200:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-post-tests.json:223:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-post-tests.json:232:      "last_msg": "8 file(s) in dracon-ai-sdk [dracon-ai-sdk/src/lib.rs, dracon-ai-sdk/tes…",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-post-tests.json:269:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-post-tests.json:288:      "repo": "/home/dracon/Dev/dracon-ai-lib",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-post-tests.json:292:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-post-tests.json:311:      "repo": "/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-post-tests.json:315:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-post-tests.json:334:      "repo": "/home/dracon/Dev/ai-auto-writer",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-post-tests.json:338:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-post-tests.json:361:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-post-tests.json:384:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-post-tests.json:407:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/REPORT.md:1:# `dracon-ai-lib` unarchive and push recovery
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/REPORT.md:7:`DraconDev/dracon-ai-lib` has been unarchived and `dracon-ai-lib` push health has been restored.
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/REPORT.md:12:repo: /home/dracon/Dev/dracon-ai-lib
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/REPORT.md:13:branch: main
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/REPORT.md:14:upstream: origin/main
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/REPORT.md:26:Prior archive evidence showed the repo was intentionally archived on 2026-06-08 as part of a redirect to `ai-api-sdk`:
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/REPORT.md:29:- Archive commit: `4fc7206 archive: mark lib as archived, redirect to ai-api-sdk`
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/REPORT.md:30:- Follow-up commit: `14397af archive: fix remaining dracon-ai-sdk references to ai-api-sdk`
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/REPORT.md:33:However, no hard blocker was found that requires the repo to remain archived:
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/REPORT.md:40:Decision: unarchive and keep `dracon-ai-lib` active for direct BYOK Rust consumers, while preserving the guidance that `ai-api-sdk` is the right path for shared gateway/multi-consumer deployments.
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/REPORT.md:47:gh api -X PATCH repos/DraconDev/dracon-ai-lib \
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/REPORT.md:49:  -f description='Standalone Rust workspace for an importable BYOK AI client library.'
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/REPORT.md:55:{"archived":true,"default_branch":"main","full_name":"DraconDev/dracon-ai-lib","visibility":"private"}
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/REPORT.md:61:{"archived":false,"default_branch":"main","description":"Standalone Rust workspace for an importable BYOK AI client library.","full_name":"DraconDev/dracon-ai-lib","visibility":"private"}
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/REPORT.md:70:  "defaultBranchRef": {"name": "main"},
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/REPORT.md:71:  "description": "Standalone Rust workspace for an importable BYOK AI client library.",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/REPORT.md:74:  "url": "https://github.com/DraconDev/dracon-ai-lib",
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/REPORT.md:89:- `dracon-ai-lib` is active for direct BYOK Rust consumers.
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/REPORT.md:90:- `ai-api-sdk` remains the recommended path for shared gateway, multi-consumer, quota, BYOK-upload, or cross-language deployments.
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/REPORT.md:99:The remaining match is a historical draft under `docs/archive/`, not a stale repo-level archived notice.
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/REPORT.md:106:📝 committed 3 file(s) in /home/dracon/Dev/dracon-ai-lib
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/REPORT.md:112:git push origin main
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/REPORT.md:119:git push --dry-run origin main
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/REPORT.md:131:cargo test --manifest-path dracon-ai-lib/Cargo.toml -- --test-threads=1
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/REPORT.md:132:parsed validation tests: passed=181 failed=0 ignored=0
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/REPORT.md:134:cargo clippy --manifest-path dracon-ai-lib/Cargo.toml --workspace -- -D warnings
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/REPORT.md:144:/home/dracon/Dev/dracon-ai-lib	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/REPORT.md:150:* main 32ccd9f [origin/main] 3 file(s) [README.md, CHANGELOG.md, CONSUMERS.md] DELTA:+3/-4
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/REPORT.md:151:rev-list --count main ^origin/main = 0
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/REPORT.md:152:rev-list --count origin/main ^main = 0
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/REPORT.md:155:## Remaining blockers
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/REPORT.md:159:The repo remains private, because the task did not request visibility change. No secrets were exposed, rotated, rewritten, or pushed. No `.pi/` paths were changed.
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/REPORT.md:165:`/home/dracon/Dev/dracon-utilities/docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/`
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/post-verification.txt:2:{"archivedAt":null,"createdAt":"2026-05-31T20:31:51Z","defaultBranchRef":{"name":"main"},"description":"Standalone Rust workspace for an importable BYOK AI client library.","isArchived":false,"updatedAt":"2026-06-11T13:11:47Z","url":"https://github.com/DraconDev/dracon-ai-lib","visibility":"PRIVATE"}
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/post-verification.txt:4:{"archived":false,"default_branch":"main","description":"Standalone Rust workspace for an importable BYOK AI client library.","full_name":"DraconDev/dracon-ai-lib","html_url":"https://github.com/DraconDev/dracon-ai-lib","visibility":"private"}
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/post-verification.txt:7:/home/dracon/Dev/dracon-ai-lib	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/post-verification.txt:10:* main 32ccd9f [origin/main] 3 file(s) [README.md, CHANGELOG.md, CONSUMERS.md] DELTA:+3/-4
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/post-verification.txt:16:b87f979 1 file(s) in crates [crates/ai-lib/src/providers/minimax.rs] DELTA:+4/-4
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/post-verification.txt:19:fff5e43 3 file(s) in crates [crates/contracts/src/lib.rs, crates/extract-keys/src/main.rs, crates/providers/src/lib.rs] DELTA:+13/-5
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/post-verification.txt:21:a87ab96 2 file(s) in crates [crates/ai-models-catalog/README.md, crates/ai-lib/README.md] DELTA:+12/-7
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/post-verification.txt:22:4eccdcc 2 file(s) in crates [crates/ai-lib/src/providers/minimax.rs, crates/ai-lib/src/providers/openai.rs] DELTA:+15/-15
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/post-verification.txt:23:9cb5103 docs: fix stale ai-lib release tag wording
docs/audit/2026-06-11-full-repo-audit/per-repo/one-mil-girls.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/per-repo/one-mil-girls.git.txt:13:* main b817615 [origin/main] 1 file(s) in docs [docs/audit/2026-06-11-full-audit/REPORT.md] DELTA:+158/-0 | NEW:2026-06-11-full-audit/REPORT.md
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/before/inventory.json:8:  "failures": 0,
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/before/inventory.json:15:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/before/inventory.json:38:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/before/inventory.json:54:      "hint": "run repair-warns --apply"
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/before/inventory.json:84:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/before/inventory.json:107:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/before/inventory.json:130:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/before/inventory.json:139:      "last_msg": "8 file(s) in apis,web [apis/services/ai-api/tests/happy_path.rs, apis/s…",
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/before/inventory.json:146:      "hint": "run repair-warns --apply"
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/before/inventory.json:153:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/before/inventory.json:169:      "hint": "run repair-warns --apply"
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/before/inventory.json:172:      "repo": "/home/dracon/Dev/rust-ai-web-auto",
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/before/inventory.json:176:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/before/inventory.json:192:      "hint": "run repair-warns --apply"
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/before/inventory.json:199:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/before/inventory.json:218:      "repo": "/home/dracon/Dev/dracon-ai-lib",
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/before/inventory.json:224:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/before/inventory.json:237:      "push_error": "ahead=21, push failing",
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/before/inventory.json:240:      "hint": "run repair-concerns --apply (push or rewrite)"
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/before/inventory.json:247:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/before/inventory.json:266:      "repo": "/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent",
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/before/inventory.json:270:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/before/inventory.json:293:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/before/inventory.json:312:      "repo": "/home/dracon/Dev/ai-auto-writer",
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/before/inventory.json:316:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/before/inventory.json:339:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/before/inventory.json:362:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/before/inventory.json:385:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/before/inventory.json:394:      "last_msg": "19 file(s) in examples,src,tests [src/cli.rs, src/ai_gen.rs, src/analyz…",
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/before/inventory.json:408:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/before/inventory.json:431:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/before/inventory.json:454:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/final/per-repo/pully-fully-pull-based-fleet-reconciler.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/final/per-repo/pully-fully-pull-based-fleet-reconciler.git.txt:13:* main 929b8d02 [origin/main] 1 file(s) [AUDIT_REPORT.md] DELTA:+42/-0
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.rust-ai-web-auto.git.txt:1:REPO=/home/dracon/Dev/rust-ai-web-auto
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.rust-ai-web-auto.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.rust-ai-web-auto.git.txt:4:codeberg	git@codeberg.org:dracondev/rust-ai-web-auto.git (fetch)
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.rust-ai-web-auto.git.txt:5:codeberg	git@codeberg.org:dracondev/rust-ai-web-auto.git (push)
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.rust-ai-web-auto.git.txt:6:github	git@github.com:DraconDev/rust-ai-web-auto.git (fetch)
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.rust-ai-web-auto.git.txt:7:github	git@github.com:DraconDev/rust-ai-web-auto.git (push)
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.rust-ai-web-auto.git.txt:8:gitlab	git@gitlab.com:dracondev/rust-ai-web-auto.git (fetch)
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.rust-ai-web-auto.git.txt:9:gitlab	git@gitlab.com:dracondev/rust-ai-web-auto.git (push)
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.rust-ai-web-auto.git.txt:10:origin	https://github.com/DraconDev/rust-ai-web-auto.git (fetch)
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.rust-ai-web-auto.git.txt:11:origin	https://github.com/DraconDev/rust-ai-web-auto.git (push)
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.rust-ai-web-auto.git.txt:13:* main d1a40f6 [origin/main] 4 file(s) in tests [tests/local-contact-form/run_test.py, tests/local-form-test/run_test.py, tests/local-inventory-test/run_test.py] DELTA:+16/-0 | TEST:16 TESTONLY:local-contact-form/run_test.py,local-form-test/run_test.py,local-inventory-test/run_test.py,local-scroll-test/run_test.py
docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-after.tsv:2:/home/dracon/Dev/dracon-ai-lib	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/after/inventory.tsv:2:/home/dracon/Dev/one-mil-girls	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/after/inventory.tsv:3:/home/dracon/Dev/dracon-platform	main	4	0	0	0	0	DIRTY	OK	run repair-warns --apply
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/after/inventory.tsv:4:/home/dracon/Dev/dracon-code	main	3	0	0	0	0	DIRTY	OK	run repair-warns --apply
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/after/inventory.tsv:5:/home/dracon/Dev/dracon-ai-lib	main	0	0	0	28	0	AHEAD:28,STUCK_PUSH	STUCK	run repair-concerns --apply (push or rewrite)
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/after/inventory.tsv:6:/home/dracon/Dev/pully-fully-pull-based-fleet-reconciler	main	5	0	0	0	0	DIRTY	OK	run repair-warns --apply
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/after/inventory.tsv:8:/home/dracon/Dev/DraconDev	main	0	0	1	0	0	DIRTY	OK	healthy
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/after/inventory.tsv:9:/home/dracon/Dev/dracon-utilities	main	1	0	1	0	0	DIRTY	OK	run repair-warns --apply
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/after/inventory.tsv:10:/home/dracon/Dev/browser-extensions-shared	main	0	0	2	0	0	DIRTY	OK	healthy
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/after/inventory.tsv:11:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/after/inventory.tsv:12:/home/dracon/Dev/rust-ai-web-auto	main	4	0	0	0	0	DIRTY	OK	run repair-warns --apply
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/after/inventory.tsv:13:/home/dracon/.dracon	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/after/inventory.tsv:14:/home/dracon/Dev/folder-auto-banner	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/after/inventory.tsv:15:/home/dracon/Dev/ai-auto-writer	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/after/inventory.tsv:16:/home/dracon/Dev/video-uploader	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/after/inventory.tsv:17:/home/dracon/Dev/video-factory	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/after/inventory.tsv:18:/home/dracon/Dev/avid	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/after/inventory.tsv:19:/home/dracon/Dev/youtube-video-uploader	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/after/inventory.tsv:20:/home/dracon/Dev/dracon-libs	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/after/inventory.tsv:21:/home/dracon/Dev/kiki-sassy-desktop-announcer	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/per-repo/dracon-utilities.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/per-repo/dracon-utilities.git.txt:5:? docs/audit/2026-06-11-full-repo-audit/per-repo/ai-auto-writer.git.txt
docs/audit/2026-06-11-full-repo-audit/per-repo/dracon-utilities.git.txt:14:? docs/audit/2026-06-11-full-repo-audit/per-repo/rust-ai-web-auto.git.txt
docs/audit/2026-06-11-full-repo-audit/per-repo/dracon-utilities.git.txt:28:* main                   313de3cb [origin/main] 4 file(s) in dracon-warden [dracon-warden/src/security/tests/security_critical_test.rs, dracon-warden/src/security/src/modules/crypto.rs, dracon-warden/src/security/src/modules/filter.rs] DELTA:+20/-27 | TEST:35
docs/audit/2026-06-11-full-repo-audit/post-funding/validation-logs/final-validation.tsv:4:ai-auto-repo-rot-scanner-todo-agent	0	ai-auto-repo-rot-scanner-todo-agent.fmt.log	0	ai-auto-repo-rot-scanner-todo-agent.test.log	0	ai-auto-repo-rot-scanner-todo-agent.clippy.log
docs/audit/2026-06-11-full-repo-audit/post-funding/validation-logs/final-validation.tsv:8:ai-auto-writer	0	ai-auto-writer.fmt.log	0	ai-auto-writer.test.log	101	ai-auto-writer.clippy.log
docs/audit/2026-06-11-full-repo-audit/post-funding/validation-logs/final-validation.tsv:10:rust-ai-web-auto	0	rust-ai-web-auto.fmt.log	0	rust-ai-web-auto.test.log	0	rust-ai-web-auto.clippy.log
docs/audit/2026-06-11-full-repo-audit/post-funding/validation-logs/final-validation.tsv:15:dracon-ai-lib	0	dracon-ai-lib.fmt.log	0	dracon-ai-lib.test.log	101	dracon-ai-lib.clippy.log
docs/audit/2026-06-11-full-repo-audit/final/per-repo/folder-auto-banner.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/final/per-repo/folder-auto-banner.git.txt:13:* main a963582 [origin/main] 5 file(s) in src [CHANGELOG.md, RELEASE_NOTES_0.6.16.md, Cargo.lock] DELTA:+49/-49 | NEW:RELEASE_NOTES_0.6.16.md
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/REPORT.md:1:# Remaining Dracon Sync Concerns and Notification Audit
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/REPORT.md:8:- `rust-ai-web-auto` WARN was not a sync blocker. Its initial dirty state was clean/smudge filter / line-ending-only churn; after recheck it became OK. Later WARNs are ordinary user changes with `push_status=OK`.
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/REPORT.md:9:- `dracon-platform` CONCERN/STUCK was caused by the installed global Warden pre-push hook, not by the repo history. The hook was outdated and blocked a vendored sample private key because it lacked the `.plaintext` sibling escape hatch. Updating hooks and adding the intentional `.plaintext` marker restored push health.
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/REPORT.md:10:- There is no remaining `CONCERN` or `STUCK_PUSH` in the latest inventory. Remaining rows are `WARN`/`DIRTY` with `push_status=OK`, caused by preserved user changes.
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/REPORT.md:28:one-mil-girls                1        0      0         0     OK          DIRTY     run repair-warns --apply
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/REPORT.md:29:dracon-platform              2        0      0         0     OK          DIRTY     run repair-warns --apply
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/REPORT.md:31:dracon-utilities             1        0      0         0     OK          DIRTY     run repair-warns --apply
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/REPORT.md:32:dracon-code                  1        0      0         0     OK          DIRTY     run repair-warns --apply
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/REPORT.md:35:No `CONCERN` or `STUCK_PUSH` remains.
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/REPORT.md:37:## Root Cause: `rust-ai-web-auto`
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/REPORT.md:41:- `per-repo-before.rust-ai-web-auto.txt`
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/REPORT.md:56:- Remaining WARN is normal user change tracking, not push failure.
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/REPORT.md:57:- `git -C /home/dracon/Dev/rust-ai-web-auto push --dry-run origin main` returned `Everything up-to-date`.
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/REPORT.md:79:- Working tree contained user changes/deletions under `web/games-hosted/games/junk-runner/...`.
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/REPORT.md:81:- `git push --dry-run origin main` failed in the pre-push hook with:
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/REPORT.md:84:Possible plaintext secrets detected in push.
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/REPORT.md:93:- The installed global hook was outdated and lacked the `.plaintext` sibling escape hatch.
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/REPORT.md:97:- `dracon-warden/src/main.rs:2203-2235` documents and implements `<path>.plaintext` skip behavior in the pre-push hook.
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/REPORT.md:105:Then added the intentional plaintext marker:
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/REPORT.md:108:vendor/hyper-rustls-0.25-patched/examples/sample.rsa.plaintext
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/REPORT.md:111:After `git fetch origin main`:
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/REPORT.md:113:- `main` and `origin/main` are aligned.
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/REPORT.md:114:- `git push --dry-run origin main` returned `Everything up-to-date`.
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/REPORT.md:123:git push --dry-run origin main   Everything up-to-date
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/REPORT.md:126:Current WARN is preserved user changes, including `web/ai-hub/src/routes/ai-hub/directory/+page.svelte`.
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/REPORT.md:142:3. Stuck repos can be removed from active sync processing and retried later. That retry path did not create a fresh alert, so a repo could remain stuck without a visible repeated notification.
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/REPORT.md:143:4. No `webhook_url` is configured in `~/.dracon/utilities/sync/dracon-sync.toml`, so webhook notification was not available for this environment.
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/REPORT.md:144:5. The latest daemon run produced alert-ledger entries for `one-mil-girls` timeouts. Direct GitLab SSH push is blocked by GitLab protected-branch policy, and the API confirms `main` has `push_access_levels.access_level = 0` (`No one`). The daemon's HTTPS fallback has also timed out, so this is a mirror-policy blocker rather than a local repo problem:
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/REPORT.md:148: ! [remote rejected] main -> main (pre-receive hook declined)
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/REPORT.md:164:- Added `~/.local/state/dracon/dracon-sync-alerts.jsonl` with JSONL entries containing:
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/REPORT.md:168:  - `details`
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/REPORT.md:194:## Remaining WARNs
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/REPORT.md:206:Action: preserve user/generated change unless explicitly approved. GitHub push is OK after fetch. GitLab mirror push is currently blocked by protected-branch policy and needs an operator decision: unprotect/adjust GitLab `main` push access, push to an unprotected mirror branch, or remove the GitLab mirror remote for this repo.
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/REPORT.md:210:Current status includes user changes under `web/ai-hub/`:
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/REPORT.md:213: M web/ai-hub/src/routes/ai-hub/+page.server.ts
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/REPORT.md:214: M web/ai-hub/src/routes/ai-hub/+page.svelte
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/REPORT.md:224: M docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-final.json
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/REPORT.md:241:Current status includes a user change under `crates/dracon-ai/src/ai_client.rs`:
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/REPORT.md:244: M crates/dracon-ai/src/ai_client.rs
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/REPORT.md:247:Action: preserve user refactor/change unless explicitly approved. `git push --dry-run origin main` is up-to-date because this is a local change.
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/REPORT.md:263:`rust-ai-web-auto`:
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/REPORT.md:269:git push --dry-run origin main                  Everything up-to-date
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/REPORT.md:279:git push --dry-run origin main                  Everything up-to-date
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/REPORT.md:285:git -C /home/dracon/Dev/one-mil-girls push --dry-run origin main       Everything up-to-date
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/REPORT.md:286:git -C /home/dracon/Dev/browser-extensions-shared push --dry-run origin main  Everything up-to-date
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/REPORT.md:287:git -C /home/dracon/Dev/rust-ai-web-auto push --dry-run origin main    Everything up-to-date
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/REPORT.md:288:git -C /home/dracon/Dev/dracon-utilities push --dry-run origin main    Everything up-to-date
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/REPORT.md:289:git -C /home/dracon/Dev/one-mil-girls push gitlab main                 blocked by GitLab protected branch policy
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/REPORT.md:294:The remaining sync concerns are explained and either fixed, surfaced by the new alert path, or intentionally preserved:
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/REPORT.md:296:- `rust-ai-web-auto`: no sync blocker; WARNs are user changes.
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/REPORT.md:297:- `dracon-platform`: push blocker fixed by updating Warden hooks and adding `.plaintext` marker.
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/REPORT.md:298:- `dracon-ai-lib`: archived-remote blocker was handled in the prior investigation; current push is OK.
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.kiki-sassy-desktop-announcer.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.kiki-sassy-desktop-announcer.git.txt:13:* main 0155632 [origin/main] 2 file(s) in src [src/journal.rs, src/daemon.rs] DELTA:+2/-4
docs/audit/2026-06-11-full-repo-audit/final/per-repo/video-factory.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/final/per-repo/video-factory.git.txt:13:* main 698a658 [origin/main] 14 file(s) in crates [crates/api/src/routes.rs, crates/core/src/config.rs, crates/worker/src/ffmpeg.rs] DELTA:+225/-162
docs/audit/2026-06-11-full-repo-audit/final/per-repo/video-factory.git.txt:17:4215e5f 1 file(s) in src [src/main.rs] DELTA:+3/-3
docs/audit/2026-06-11-full-repo-audit/final/per-repo/dracon-ai-lib.git.txt:1:REPO=/home/dracon/Dev/dracon-ai-lib
docs/audit/2026-06-11-full-repo-audit/final/per-repo/dracon-ai-lib.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/final/per-repo/dracon-ai-lib.git.txt:4:origin	https://github.com/DraconDev/dracon-ai-lib.git (fetch)
docs/audit/2026-06-11-full-repo-audit/final/per-repo/dracon-ai-lib.git.txt:5:origin	https://github.com/DraconDev/dracon-ai-lib.git (push)
docs/audit/2026-06-11-full-repo-audit/final/per-repo/dracon-ai-lib.git.txt:7:* main cd8bc7f [origin/main: ahead 13] 1 file(s) in crates [crates/ai-lib/src/providers/minimax.rs] DELTA:+48/-37
docs/audit/2026-06-11-full-repo-audit/final/per-repo/dracon-ai-lib.git.txt:9:cd8bc7f 1 file(s) in crates [crates/ai-lib/src/providers/minimax.rs] DELTA:+48/-37
docs/audit/2026-06-11-full-repo-audit/final/per-repo/dracon-ai-lib.git.txt:10:209cff3 1 file(s) in crates [crates/ai-lib/src/providers/minimax.rs] DELTA:+1/-1
docs/audit/2026-06-11-full-repo-audit/final/per-repo/dracon-ai-lib.git.txt:11:d70cf8a 1 file(s) in crates [crates/ai-lib/src/providers/minimax.rs] DELTA:+16/-13
docs/audit/2026-06-11-full-repo-audit/final/per-repo/dracon-ai-lib.git.txt:12:5fec442 17 file(s) in crates [Cargo.lock, crates/client/src/lib.rs, crates/providers/src/openai.rs] DELTA:+1415/-589
docs/audit/2026-06-11-full-repo-audit/per-repo/pully-fully-pull-based-fleet-reconciler.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/per-repo/pully-fully-pull-based-fleet-reconciler.git.txt:13:* main 929b8d02 [origin/main] 1 file(s) [AUDIT_REPORT.md] DELTA:+42/-0
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-code.risk.tsv:19:tracked	.ralph/phase3-dracon-ai-extraction.md
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-code.risk.tsv:20:tracked	.ralph/phase3-dracon-ai-extraction.state.json
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-code.risk.tsv:29:tracked	.ralph/remaining-work.md
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-code.risk.tsv:30:tracked	.ralph/remaining-work.state.json
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.avid.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.avid.git.txt:14:* main                                     8d1f698 [origin/main] 19 file(s) in examples,src,tests [src/cli.rs, src/ai_gen.rs, src/analyzer.rs] DELTA:+1034/-453 | TEST:81
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.avid.git.txt:16:8d1f698 19 file(s) in examples,src,tests [src/cli.rs, src/ai_gen.rs, src/analyzer.rs] DELTA:+1034/-453 | TEST:81
docs/audit/2026-06-11-full-repo-audit/final/per-repo/DraconDev.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/final/per-repo/DraconDev.git.txt:13:* main f9a2e70 [origin/main] Merge https://github.com/DraconDev/DraconDev
docs/audit/2026-06-11-full-repo-audit/final/per-repo/ai-auto-writer.git.txt:1:REPO=/home/dracon/Dev/ai-auto-writer
docs/audit/2026-06-11-full-repo-audit/final/per-repo/ai-auto-writer.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/final/per-repo/ai-auto-writer.git.txt:4:codeberg	git@codeberg.org:dracondev/ai-auto-writer.git (fetch)
docs/audit/2026-06-11-full-repo-audit/final/per-repo/ai-auto-writer.git.txt:5:codeberg	git@codeberg.org:dracondev/ai-auto-writer.git (push)
docs/audit/2026-06-11-full-repo-audit/final/per-repo/ai-auto-writer.git.txt:6:github	git@github.com:DraconDev/ai-auto-writer.git (fetch)
docs/audit/2026-06-11-full-repo-audit/final/per-repo/ai-auto-writer.git.txt:7:github	git@github.com:DraconDev/ai-auto-writer.git (push)
docs/audit/2026-06-11-full-repo-audit/final/per-repo/ai-auto-writer.git.txt:8:gitlab	git@gitlab.com:dracondev/ai-auto-writer.git (fetch)
docs/audit/2026-06-11-full-repo-audit/final/per-repo/ai-auto-writer.git.txt:9:gitlab	git@gitlab.com:dracondev/ai-auto-writer.git (push)
docs/audit/2026-06-11-full-repo-audit/final/per-repo/ai-auto-writer.git.txt:10:origin	https://github.com/DraconDev/ai-auto-writer.git (fetch)
docs/audit/2026-06-11-full-repo-audit/final/per-repo/ai-auto-writer.git.txt:11:origin	https://github.com/DraconDev/ai-auto-writer.git (push)
docs/audit/2026-06-11-full-repo-audit/final/per-repo/ai-auto-writer.git.txt:14:* main                               aa5d0ebb [origin/main] 1 file(s) in src [src/services/dracon.rs] DELTA:+4/-32
docs/audit/2026-06-11-full-repo-audit/final/per-repo/ai-auto-writer.git.txt:19:9c829b43 Merge https://github.com/DraconDev/ai-auto-writer
docs/audit/2026-06-11-full-repo-audit/final/per-repo/youtube-video-uploader.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/final/per-repo/youtube-video-uploader.git.txt:13:* main 771d422 [origin/main] Merge https://github.com/DraconDev/youtube-video-uploader
docs/audit/2026-06-11-full-repo-audit/post-funding/hygiene.tsv:6:/home/dracon/Dev/rust-ai-web-auto	46	2	2	1	1	0
docs/audit/2026-06-11-full-repo-audit/post-funding/hygiene.tsv:10:/home/dracon/Dev/dracon-ai-lib	35	3	14	1	1	0
docs/audit/2026-06-11-full-repo-audit/post-funding/hygiene.tsv:13:/home/dracon/Dev/ai-auto-writer	217	3	4	80	1	0
docs/audit/2026-06-11-full-repo-audit/post-funding/hygiene.tsv:15:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	77	1	6	1	1	0
docs/audit/2026-06-11-full-repo-audit/final/per-repo/rust-ai-web-auto.git.txt:1:REPO=/home/dracon/Dev/rust-ai-web-auto
docs/audit/2026-06-11-full-repo-audit/final/per-repo/rust-ai-web-auto.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/final/per-repo/rust-ai-web-auto.git.txt:4:codeberg	git@codeberg.org:dracondev/rust-ai-web-auto.git (fetch)
docs/audit/2026-06-11-full-repo-audit/final/per-repo/rust-ai-web-auto.git.txt:5:codeberg	git@codeberg.org:dracondev/rust-ai-web-auto.git (push)
docs/audit/2026-06-11-full-repo-audit/final/per-repo/rust-ai-web-auto.git.txt:6:github	git@github.com:DraconDev/rust-ai-web-auto.git (fetch)
docs/audit/2026-06-11-full-repo-audit/final/per-repo/rust-ai-web-auto.git.txt:7:github	git@github.com:DraconDev/rust-ai-web-auto.git (push)
docs/audit/2026-06-11-full-repo-audit/final/per-repo/rust-ai-web-auto.git.txt:8:gitlab	git@gitlab.com:dracondev/rust-ai-web-auto.git (fetch)
docs/audit/2026-06-11-full-repo-audit/final/per-repo/rust-ai-web-auto.git.txt:9:gitlab	git@gitlab.com:dracondev/rust-ai-web-auto.git (push)
docs/audit/2026-06-11-full-repo-audit/final/per-repo/rust-ai-web-auto.git.txt:10:origin	https://github.com/DraconDev/rust-ai-web-auto.git (fetch)
docs/audit/2026-06-11-full-repo-audit/final/per-repo/rust-ai-web-auto.git.txt:11:origin	https://github.com/DraconDev/rust-ai-web-auto.git (push)
docs/audit/2026-06-11-full-repo-audit/final/per-repo/rust-ai-web-auto.git.txt:13:* main 3e73084 [origin/main] docs: add capabilities and limits guide
docs/audit/2026-06-11-full-repo-audit/final/per-repo/rust-ai-web-auto.git.txt:16:996b4ac docs(audit): document Dracon AI lib adoption + Section 7/8/9/10 renumbering
docs/audit/2026-06-11-full-repo-audit/final/per-repo/rust-ai-web-auto.git.txt:17:bede3bb docs: add Dracon AI lib section to README
docs/audit/2026-06-11-full-repo-audit/final/per-repo/rust-ai-web-auto.git.txt:18:3a55f5a 2 file(s) in examples,src [examples/dracon_ai_smoke.rs, src/env_keys.rs] DELTA:+12/-7
docs/audit/2026-06-11-full-repo-audit/final/per-repo/rust-ai-web-auto.git.txt:19:c698705 4 file(s) in examples,src [examples/dracon_ai_smoke.rs, src/doctor.rs, Cargo.lock] DELTA:+169/-1 | NEW:examples/dracon_ai_smoke.rs
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/after/inventory.json:8:  "failures": 0,
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/after/inventory.json:15:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/after/inventory.json:38:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/after/inventory.json:54:      "hint": "run repair-warns --apply"
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/after/inventory.json:61:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/after/inventory.json:77:      "hint": "run repair-warns --apply"
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/after/inventory.json:80:      "repo": "/home/dracon/Dev/dracon-ai-lib",
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/after/inventory.json:85:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/after/inventory.json:98:      "push_error": "ahead=28, push failing",
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/after/inventory.json:101:      "hint": "run repair-concerns --apply (push or rewrite)"
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/after/inventory.json:108:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/after/inventory.json:117:      "last_msg": "1 file(s) in pully [pully/bins/pully/src/main.rs] DELTA:+1/-42",
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/after/inventory.json:124:      "hint": "run repair-warns --apply"
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/after/inventory.json:154:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/after/inventory.json:177:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/after/inventory.json:193:      "hint": "run repair-warns --apply"
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/after/inventory.json:200:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/after/inventory.json:219:      "repo": "/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent",
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/after/inventory.json:223:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/after/inventory.json:242:      "repo": "/home/dracon/Dev/rust-ai-web-auto",
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/after/inventory.json:246:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/after/inventory.json:262:      "hint": "run repair-warns --apply"
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/after/inventory.json:269:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/after/inventory.json:292:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/after/inventory.json:311:      "repo": "/home/dracon/Dev/ai-auto-writer",
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/after/inventory.json:315:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/after/inventory.json:338:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/after/inventory.json:361:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/after/inventory.json:384:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/after/inventory.json:393:      "last_msg": "19 file(s) in examples,src,tests [src/cli.rs, src/ai_gen.rs, src/analyz…",
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/after/inventory.json:407:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/after/inventory.json:430:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/after/inventory.json:453:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-final.json:8:  "failures": 0,
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-final.json:11:      "repo": "/home/dracon/Dev/ai-auto-writer",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-final.json:15:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-final.json:38:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-final.json:57:      "repo": "/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-final.json:61:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-final.json:70:      "last_msg": "3 file(s) in src [Cargo.lock, src/ai/mod.rs, src/scanner.rs] DELTA:+37/…",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-final.json:107:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-final.json:116:      "last_msg": "8 file(s) in web [web/ai-hub/src/lib/server/catalog.ts, web/ai-hub/src/…",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-final.json:123:      "hint": "run repair-warns --apply"
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-final.json:130:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-final.json:139:      "last_msg": "1 file(s) in plugins [plugins/default-ai-providers/src/lib.rs] DELTA:+2…",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-final.json:153:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-final.json:162:      "last_msg": "1 file(s) in docs [docs/audit/2026-06-11-full-repo-audit/remaining-conc…",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-final.json:169:      "hint": "run repair-warns --apply"
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-final.json:176:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-final.json:192:      "hint": "run repair-warns --apply"
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-final.json:199:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-final.json:218:      "repo": "/home/dracon/Dev/rust-ai-web-auto",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-final.json:222:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-final.json:245:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-final.json:268:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-final.json:291:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-final.json:300:      "last_msg": "8 file(s) in dracon-ai-sdk [dracon-ai-sdk/src/lib.rs, dracon-ai-sdk/tes…",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-final.json:314:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-final.json:333:      "repo": "/home/dracon/Dev/dracon-ai-lib",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-final.json:337:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-final.json:360:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-final.json:383:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-final.json:406:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/per-repo/video-factory.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/per-repo/video-factory.git.txt:13:* main 698a658 [origin/main] 14 file(s) in crates [crates/api/src/routes.rs, crates/core/src/config.rs, crates/worker/src/ffmpeg.rs] DELTA:+225/-162
docs/audit/2026-06-11-full-repo-audit/per-repo/video-factory.git.txt:17:4215e5f 1 file(s) in src [src/main.rs] DELTA:+3/-3
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.dracon-code.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.dracon-code.git.txt:13:  backup-main-20260513                             13262567 security(dependency configuration): Updated dependency configuration in `deny.toml` for security and comp...
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.dracon-code.git.txt:14:  bevy-version                                     ef86290b [gui+src|wip] screenshot viewer, task persistence, fetch denylist UI, gui_refresh_secs poll wiring, ai_actions in plan prompt, dead code cleanup
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.dracon-code.git.txt:18:  egui-version                                     0de221d8 {"schema":"dracon.commit.v2","schema_rev":2,"commit_kind":"sync_event","actor":"dracon-sync","generator":{"name":"dracon-git","version":"0.1.0"},"event_fingerprint":"bcc7462f0ab438a932e8482e31fc41ac25fb3d82d26d0a96f0d53304e52a706b","ts":"1771992484","repo":"dracon-code","branch":"master","files":{"added":0,"modified":3,"deleted":0,"renamed":0,"type_change":0,"unknown":0},"changed_paths_full":["Cargo.lock","Cargo.toml","gui/src/main.rs"],"changed_paths":["Cargo.lock","Cargo.toml","gui/src/main.rs"],"changed_paths_total":3,"changed_paths_truncated":false,"top_level_scopes":[{"key":"Cargo.lock","count":1},{"key":"Cargo.toml","count":1},{"key":"gui","count":1}],"extension_summary":[{"key":"lock","count":1},{"key":"rs","count":1},{"key":"toml","count":1}],"domain_summary":[{"key":"code","count":1},{"key":"config","count":1},{"key":"lockfile","count":1}],"intent_tags":["behavior_change_possible","compiled_or_runtime_code_touched","configuration_update","dependency_lock_changed"],"risk_flags":["build_graph_or_dependency_surface"],"semantic":{"files_analyzed":1,"files_skipped":2,"symbols_total":74,"symbols_truncated":false,"symbols":[{"path":"gui/src/main.rs","language":"rust","name":"main","kind":"function","start_line":11,"end_line":25},{"path":"gui/src/main.rs","language":"rust","name":"GuiRuntimeConfig","kind":"struct","start_line":28,"end_line":33},{"path":"gui/src/main.rs","language":"rust","name":"DraconConfigFile","kind":"struct","start_line":36,"end_line":44},{"path":"gui/src/main.rs","language":"rust","name":"Screen","kind":"enum","start_line":47,"end_line":51},{"path":"gui/src/main.rs","language":"rust","name":"Screen","kind":"impl","start_line":53,"end_line":61},{"path":"gui/src/main.rs","language":"rust","name":"label","kind":"function","start_line":54,"end_line":60},{"path":"gui/src/main.rs","language":"rust","name":"HubSort","kind":"enum","start_line":64,"end_line":70},{"path":"gui/src/main.rs","language":"rust","name":"HubSort","kind":"impl","start_line":72,"end_line":82},{"path":"gui/src/main.rs","language":"rust","name":"label","kind":"function","start_line":73,"end_line":81},{"path":"gui/src/main.rs","language":"rust","name":"HubFilter","kind":"enum","start_line":85,"end_line":89},{"path":"gui/src/main.rs","language":"rust","name":"HubFilter","kind":"impl","start_line":91,"end_line":99},{"path":"gui/src/main.rs","language":"rust","name":"label","kind":"function","start_line":92,"end_line":98},{"path":"gui/src/main.rs","language":"rust","name":"SettingsForm","kind":"struct","start_line":102,"end_line":110},{"path":"gui/src/main.rs","language":"rust","name":"SettingsForm","kind":"impl","start_line":112,"end_line":179},{"path":"gui/src/main.rs","language":"rust","name":"from_body","kind":"function","start_line":113,"end_line":132},{"path":"gui/src/main.rs","language":"rust","name":"apply_to_body","kind":"function","start_line":134,"end_line":178},{"path":"gui/src/main.rs","language":"rust","name":"OperatorApp","kind":"struct","start_line":181,"end_line":198},{"path":"gui/src/main.rs","language":"rust","name":"OperatorApp","kind":"impl","start_line":200,"end_line":708},{"path":"gui/src/main.rs","language":"rust","name":"new","kind":"function","start_line":201,"end_line":240},{"path":"gui/src/main.rs","language":"rust","name":"refresh","kind":"function","start_line":242,"end_line":271},{"path":"gui/src/main.rs","language":"rust","name":"run_action","kind":"function","start_line":273,"end_line":280},{"path":"gui/src/main.rs","language":"rust","name":"save_config","kind":"function","start_line":282,"end_line":304},{"path":"gui/src/main.rs","language":"rust","name":"sorted_hub_rows","kind":"function","start_line":306,"end_line":348},{"path":"gui/src/main.rs","language":"rust","name":"nav_row","kind":"function","start_line":350,"end_line":365},{"path":"gui/src/main.rs","language":"rust","name":"project_screen","kind":"function","start_line":367,"end_line":450},{"path":"gui/src/main.rs","language":"rust","name":"hub_screen","kind":"function","start_line":452,"end_line":533},{"path":"gui/src/main.rs","language":"rust","name":"settings_screen","kind":"function","start_line":535,"end_line":707},{"path":"gui/src/main.rs","language":"rust","name":"OperatorApp","kind":"impl","start_line":710,"end_line":769},{"path":"gui/src/main.rs","language":"rust","name":"update","kind":"function","start_line":711,"end_line":768},{"path":"gui/src/main.rs","language":"rust","name":"apply_theme","kind":"function","start_line":771,"end_line":818},{"path":"gui/src/main.rs","language":"rust","name":"panel","kind":"function","start_line":820,"end_line":835},{"path":"gui/src/main.rs","language":"rust","name":"screen_title","kind":"function","start_line":837,"end_line":851},{"path":"gui/src/main.rs","language":"rust","name":"paint_background","kind":"function","start_line":853,"end_line":888},{"path":"gui/src/main.rs","language":"rust","name":"kv","kind":"function","start_line":890,"end_line":895},{"path":"gui/src/main.rs","language":"rust","name":"status_chip","kind":"function","start_line":897,"end_line":909},{"path":"gui/src/main.rs","language":"rust","name":"action_button","kind":"function","start_line":911,"end_line":928},{"path":"gui/src/main.rs","language":"rust","name":"tab_button","kind":"function","start_line":930,"end_line":952},{"path":"gui/src/main.rs","language":"rust","name":"chip_button","kind":"function","start_line":954,"end_line":969},{"path":"gui/src/main.rs","language":"rust","name":"truncate_middle","kind":"function","start_line":971,"end_line":978},{"path":"gui/src/main.rs","language":"rust","name":"draw_projects_table","kind":"function","start_line":980,"end_line":1065},{"path":"gui/src/main.rs","language":"rust","name":"draw_hub_table","kind":"function","start_line":1067,"end_line":1180},{"path":"gui/src/main.rs","language":"rust","name":"table_header","kind":"function","start_line":1182,"end_line":1190},{"path":"gui/src/main.rs","language":"rust","name":"table_row_bg","kind":"function","start_line":1192,"end_line":1198},{"path":"gui/src/main.rs","language":"rust","name":"is_active_repo","kind":"function","start_line":1200,"end_line":1205},{"path":"gui/src/main.rs","language":"rust","name":"phase_color","kind":"function","start_line":1207,"end_line":1217},{"path":"gui/src/main.rs","language":"rust","name":"trigger_color","kind":"function","start_line":1219,"end_line":1225},{"path":"gui/src/main.rs","language":"rust","name":"git_state_color","kind":"function","start_line":1227,"end_line":1241},{"path":"gui/src/main.rs","language":"rust","name":"FleetView","kind":"struct","start_line":1244,"end_line":1247},{"path":"gui/src/main.rs","language":"rust","name":"FleetRow","kind":"struct","start_line":1250,"end_line":1259},{"path":"gui/src/main.rs","language":"rust","name":"FleetRow","kind":"impl","start_line":1261,"end_line":1273},{"path":"gui/src/main.rs","language":"rust","name":"active_slice_label","kind":"function","start_line":1262,"end_line":1266},{"path":"gui/src/main.rs","language":"rust","name":"updated_label","kind":"function","start_line":1268,"end_line":1272},{"path":"gui/src/main.rs","language":"rust","name":"merge_discovered_repos","kind":"function","start_line":1275,"end_line":1291},{"path":"gui/src/main.rs","language":"rust","name":"compute_git_states","kind":"function","start_line":1293,"end_line":1297},{"path":"gui/src/main.rs","language":"rust","name":"git_state_for_repo","kind":"function","start_line":1299,"end_line":1324},{"path":"gui/src/main.rs","language":"rust","name":"parse_branch_sync","kind":"function","start_line":1326,"end_line":1348},{"path":"gui/src/main.rs","language":"rust","name":"discover_git_repos","kind":"function","start_line":1350,"end_line":1363},{"path":"gui/src/main.rs","language":"rust","name":"walk_for_git_repos","kind":"function","start_line":1365,"end_line":1407},{"path":"gui/src/main.rs","language":"rust","name":"refresh_view","kind":"function","start_line":1409,"end_line":1432},{"path":"gui/src/main.rs","language":"rust","name":"choose_selected_repo","kind":"function","start_line":1434,"end_line":1457},{"path":"gui/src/main.rs","language":"rust","name":"load_fleet_rows","kind":"function","start_line":1459,"end_line":1501},{"path":"gui/src/main.rs","language":"rust","name":"load_fleet_rows_sqlite","kind":"function","start_line":1503,"end_line":1547},{"path":"gui/src/main.rs","language":"rust","name":"load_gui_runtime_config","kind":"function","start_line":1549,"end_line":1580},{"path":"gui/src/main.rs","language":"rust","name":"default_fleet_db_path","kind":"function","start_line":1582,"end_line":1584},{"path":"gui/src/main.rs","language":"rust","name":"expand_tilde","kind":"function","start_line":1586,"end_line":1596},{"path":"gui/src/main.rs","language":"rust","name":"read_text_file","kind":"function","start_line":1598,"end_line":1601},{"path":"gui/src/main.rs","language":"rust","name":"run_json","kind":"function","start_line":1603,"end_line":1611},{"path":"gui/src/main.rs","language":"rust","name":"run_cmd","kind":"function","start_line":1613,"end_line":1619},{"path":"gui/src/main.rs","language":"rust","name":"run_cmd_in","kind":"function","start_line":1621,"end_line":1627},{"path":"gui/src/main.rs","language":"rust","name":"run_cmd_capture","kind":"function","start_line":1629,"end_line":1652},{"path":"gui/src/main.rs","language":"rust","name":"resolve_default_project","kind":"function","start_line":1654,"end_line":1659},{"path":"gui/src/main.rs","language":"rust","name":"append_log","kind":"function","start_line":1661,"end_line":1666},{"path":"gui/src/main.rs","language":"rust","name":"now_secs","kind":"function","start_line":1668,"end_line":1673},{"path":"gui/src/main.rs","language":"rust","name":"format_ts","kind":"function","start_line":1675,"end_line":1677}],"kind_summary":[{"key":"function","count":58},{"key":"impl","count":7},{"key":"struct","count":6},{"key":"enum","count":3}],"language_summary":[{"key":"rust","count":74}]},"status":{"ahead":0,"behind":0,"modified_files":1,"staged_files":0},"policy":{"deterministic":true,"ai_commit_messages":false}}
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.dracon-code.git.txt:20:* main                                             7e188308 [origin/main] 6 file(s) in plugins,src [src/executive/tool_registry.rs, src/extensions.rs, plugins/default-builtin-tools/plugin.toml] DELTA:+99/-18
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.dracon-code.git.txt:22:  temp-main                                        e1eaa26d Reset to origin/master
docs/audit/2026-06-11-full-repo-audit/final/per-repo/Junk-Runner-bevy.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/final/per-repo/Junk-Runner-bevy.git.txt:14:  main        e1894697f [origin/main] Added SOLID_VS_SVELTE.md
docs/audit/2026-06-11-full-repo-audit/final/per-repo/repos.tsv:9:/home/dracon/Dev/ai-auto-writer
docs/audit/2026-06-11-full-repo-audit/final/per-repo/repos.tsv:11:/home/dracon/Dev/rust-ai-web-auto
docs/audit/2026-06-11-full-repo-audit/final/per-repo/repos.tsv:12:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent
docs/audit/2026-06-11-full-repo-audit/final/per-repo/repos.tsv:18:/home/dracon/Dev/dracon-ai-lib
docs/audit/2026-06-11-full-repo-audit/final/per-repo/browser-extensions-shared.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/final/per-repo/browser-extensions-shared.git.txt:13:* main 6fb5ae2a8 [origin/main] 174 file(s) in auto-form-filler [auto-form-filler/.audit-ui/ui-audit-report.json, auto-form-filler/.audit-ui/profile/Default/BookmarkMergedSurfaceOrdering, auto-form-filler/.audit-ui/profile/Default/Extension Rules/LOG] DELTA:+63/-38 | BIN:88 NEW:Default/Account Web Data,Default/Account Web Data-journal,Default/Affiliation Database,Default/Affiliation Database-journal,Default/BookmarkMergedSurfaceOrdering,Cache_Data/60275391413c5610_0,Cache_Data/7e0f8c4ddb5a9a1e_0,Cache_Data/8121ea39bde7a9ef_0,Cache_Data/eb1e553f9fc0ebeb_0,Cache_Data/index+157more
docs/audit/2026-06-11-full-repo-audit/per-repo/DraconDev.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/per-repo/DraconDev.git.txt:16:* main 1cd5819 [origin/main] 1 file(s) [GITHUB_SPONSORS_PROFILE_COPY.txt] DELTA:+17/-16
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.one-mil-girls.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.one-mil-girls.git.txt:13:* main 79f6bbd [origin/main] 1 file(s) in .svelte-kit [.svelte-kit/ambient.d.ts] DELTA:+6/-8
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-after-rust-clean.json:8:  "failures": 0,
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-after-rust-clean.json:15:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-after-rust-clean.json:38:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-after-rust-clean.json:47:      "last_msg": "2 file(s) in auto-form-filler,vidpro-extension [auto-form-filler/AI_FOR…",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-after-rust-clean.json:54:      "hint": "run repair-warns --apply"
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-after-rust-clean.json:61:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-after-rust-clean.json:77:      "hint": "run repair-warns --apply"
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-after-rust-clean.json:84:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-after-rust-clean.json:103:      "repo": "/home/dracon/Dev/rust-ai-web-auto",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-after-rust-clean.json:107:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-after-rust-clean.json:130:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-after-rust-clean.json:153:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-after-rust-clean.json:162:      "last_msg": "5 file(s) in docs,dracon-ai [docs/audit/2026-06-11-full-repo-audit/rema…",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-after-rust-clean.json:177:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-after-rust-clean.json:190:      "push_error": "ahead=3, push failing",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-after-rust-clean.json:193:      "hint": "run repair-concerns --apply (push or rewrite)"
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-after-rust-clean.json:219:      "repo": "/home/dracon/Dev/dracon-ai-lib",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-after-rust-clean.json:223:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-after-rust-clean.json:246:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-after-rust-clean.json:265:      "repo": "/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-after-rust-clean.json:269:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-after-rust-clean.json:288:      "repo": "/home/dracon/Dev/ai-auto-writer",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-after-rust-clean.json:292:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-after-rust-clean.json:315:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-after-rust-clean.json:338:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-after-rust-clean.json:361:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-after-rust-clean.json:370:      "last_msg": "19 file(s) in examples,src,tests [src/cli.rs, src/ai_gen.rs, src/analyz…",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-after-rust-clean.json:384:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-after-rust-clean.json:407:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/REPORT.md:8:The cleanup checklist was executed for all locally available Dracon-managed repos with one hard exclusion: **`.pi/` was not cleaned or modified**.
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/REPORT.md:15:- **Blocked-needs-approval:** 418 paths, mainly `.env*`, `.ralph/*.md`, `.ralph/*.state.json`, ambiguous TODO/checklist docs, and source files whose names include "secret" but are actual code/tests.
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/REPORT.md:22:| `browser-extensions-shared` | Removed tracked generated coverage | `SamAI/coverage/` | Generated coverage output, not source or user-owned project asset. |
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/REPORT.md:23:| `ai-auto-repo-rot-scanner-todo-agent` | Removed stale local runner event file | `.ralph/audit-remediation/.ralph-runner/events.jsonl` | Stale `.ralph-runner` generated event log outside `.pi`. |
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/REPORT.md:50:## Remaining public-readiness blockers
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/REPORT.md:54:Remaining blockers:
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/REPORT.md:62:2. **`dracon-ai-lib`**
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/REPORT.md:66:   - `.env*`, `.envrc`, example secret files, and secret-like fixtures remain classified as `blocked-needs-approval`.
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/REPORT.md:75:   - `dracon-code`, `browser-extensions-shared`, and `dracon-ai-lib` have user-owned changes that were preserved.
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/REPORT.md:96:- `ai-auto-repo-rot-scanner-todo-agent`: `cargo fmt --all --check`, `cargo clippy --workspace -- -D warnings`, and `cargo test --workspace -- --test-threads=1` passed.
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/REPORT.md:116:The highest-risk browser profile/cache data and generated coverage were removed. `.pi` was proven unchanged. Remaining public-release blockers are now limited to secret rotation/approval decisions, preserved user-owned content, user-owned changes, and the pre-existing `dracon-ai-lib` push blocker.
docs/audit/2026-06-11-full-repo-audit/post-funding/REPORT.md:14:- A template was added at `~/.dracon/utilities/sync/templates/FUNDING.yml` with comments explaining GitHub's `.github/` discovery rule, the supported keys, the no-secrets contract, and the `dracon-sync` opt-out.
docs/audit/2026-06-11-full-repo-audit/post-funding/REPORT.md:17:- Every Dracon-managed repo now has a `.github/FUNDING.yml`. 19 of 20 already had it (committed manually by the operator with `github: [DraconDev]`). 1 (`dracon-ai-lib`) was missing; the standard-files flow scaffolded the empty default template into it. No existing `FUNDING.yml` content was overwritten.
docs/audit/2026-06-11-full-repo-audit/post-funding/REPORT.md:24:   - Comment block explains GitHub's `.github/` discovery rule, supported keys, and the no-secrets contract.
docs/audit/2026-06-11-full-repo-audit/post-funding/REPORT.md:32:   - `AGENTS.md` — Section "Standard Files" explains `FUNDING.yml` placement, the no-secrets rule, and the per-repo opt-out.
docs/audit/2026-06-11-full-repo-audit/post-funding/REPORT.md:36:   - `dracon-sync scaffold --repo /home/dracon/Dev/dracon-ai-lib --files '.github/FUNDING.yml'` → 1 file copied. No other repos touched.
docs/audit/2026-06-11-full-repo-audit/post-funding/REPORT.md:49:| `ai-auto-repo-rot-scanner-todo-agent` | 0 | 0 | 0 | pass (fmt drift fixed during this audit; verified after fix) |
docs/audit/2026-06-11-full-repo-audit/post-funding/REPORT.md:53:| `ai-auto-writer` | 0 | 0 | 101 | fmt/test pass; clippy reports pre-existing `unused import: ChatRequest` and 3x `returning the result of a let binding` (unchanged from prior audit) |
docs/audit/2026-06-11-full-repo-audit/post-funding/REPORT.md:55:| `rust-ai-web-auto` | 0 | 0 | 0 | pass |
docs/audit/2026-06-11-full-repo-audit/post-funding/REPORT.md:58:| `youtube-video-uploader` | 0 | 0 | 101 | fmt/test pass; clippy reports pre-existing `PLAINTEXT_MAGIC` / `PLAINTEXT_VERSION` dead-code (unchanged from prior audit) |
docs/audit/2026-06-11-full-repo-audit/post-funding/REPORT.md:60:| `dracon-ai-lib` | 0 | 0 | 101 | fmt/test pass; clippy reports pre-existing `.filter_map(..)` → `.map(..)` (unchanged from prior audit) |
docs/audit/2026-06-11-full-repo-audit/post-funding/REPORT.md:75:| `browser-extensions-shared` | No root package scripts; hygiene blocker (tracked secrets / browser profile data) remains; not in scope for this goal |
docs/audit/2026-06-11-full-repo-audit/post-funding/REPORT.md:76:| `DraconDev` | No documented local build/test command; docs/profile triage remains; not in scope for this goal |
docs/audit/2026-06-11-full-repo-audit/post-funding/REPORT.md:106:- Is public, version-controlled, and contains no secrets (no API keys, tokens, passwords).
docs/audit/2026-06-11-full-repo-audit/post-funding/REPORT.md:108:- The Warden key-management layer treats `FUNDING.yml` as plain text (it is in the ignore list of any secret-handling filter, since it has no `DRACON_SECRET` markers).
docs/audit/2026-06-11-full-repo-audit/post-funding/REPORT.md:110:The pre-existing public-readiness blockers from the prior audit remain documented:
docs/audit/2026-06-11-full-repo-audit/post-funding/REPORT.md:113:- `dracon-ai-lib` — local validation passes; push remains blocked (now AHEAD:15 after the FUNDING.yml commit).
docs/audit/2026-06-11-full-repo-audit/post-funding/REPORT.md:117:## Remaining blockers (unchanged from prior audit)
docs/audit/2026-06-11-full-repo-audit/post-funding/REPORT.md:120:2. **`dracon-ai-lib`** — AHEAD:15; push blocked. Needs explicit remote/recreate/rewrite decision.
docs/audit/2026-06-11-full-repo-audit/post-funding/REPORT.md:121:3. **`ai-auto-writer`, `video-factory`, `youtube-video-uploader`, `video-uploader`, `dracon-ai-lib`, `dracon-libs`** — pre-existing clippy warnings (unchanged by this change). Not blockers for the FUNDING.yml goal; tracked separately.
docs/audit/2026-06-11-full-repo-audit/final/per-repo/avid.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/final/per-repo/avid.git.txt:14:* main                                     8d1f698 [origin/main] 19 file(s) in examples,src,tests [src/cli.rs, src/ai_gen.rs, src/analyzer.rs] DELTA:+1034/-453 | TEST:81
docs/audit/2026-06-11-full-repo-audit/final/per-repo/avid.git.txt:16:8d1f698 19 file(s) in examples,src,tests [src/cli.rs, src/ai_gen.rs, src/analyzer.rs] DELTA:+1034/-453 | TEST:81
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/rust-ai-web-auto.risk.tsv:34:tracked	click_chain_1780538519.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/rust-ai-web-auto.risk.tsv:35:tracked	click_chain_1780538550.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/rust-ai-web-auto.risk.tsv:36:tracked	click_chain_1780539957.png
docs/audit/2026-06-11-full-repo-audit/per-repo/rust-ai-web-auto.git.txt:1:REPO=/home/dracon/Dev/rust-ai-web-auto
docs/audit/2026-06-11-full-repo-audit/per-repo/rust-ai-web-auto.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/per-repo/rust-ai-web-auto.git.txt:4:codeberg	git@codeberg.org:dracondev/rust-ai-web-auto.git (fetch)
docs/audit/2026-06-11-full-repo-audit/per-repo/rust-ai-web-auto.git.txt:5:codeberg	git@codeberg.org:dracondev/rust-ai-web-auto.git (push)
docs/audit/2026-06-11-full-repo-audit/per-repo/rust-ai-web-auto.git.txt:6:github	git@github.com:DraconDev/rust-ai-web-auto.git (fetch)
docs/audit/2026-06-11-full-repo-audit/per-repo/rust-ai-web-auto.git.txt:7:github	git@github.com:DraconDev/rust-ai-web-auto.git (push)
docs/audit/2026-06-11-full-repo-audit/per-repo/rust-ai-web-auto.git.txt:8:gitlab	git@gitlab.com:dracondev/rust-ai-web-auto.git (fetch)
docs/audit/2026-06-11-full-repo-audit/per-repo/rust-ai-web-auto.git.txt:9:gitlab	git@gitlab.com:dracondev/rust-ai-web-auto.git (push)
docs/audit/2026-06-11-full-repo-audit/per-repo/rust-ai-web-auto.git.txt:10:origin	https://github.com/DraconDev/rust-ai-web-auto.git (fetch)
docs/audit/2026-06-11-full-repo-audit/per-repo/rust-ai-web-auto.git.txt:11:origin	https://github.com/DraconDev/rust-ai-web-auto.git (push)
docs/audit/2026-06-11-full-repo-audit/per-repo/rust-ai-web-auto.git.txt:13:* main 996b4ac [origin/main] docs(audit): document Dracon AI lib adoption + Section 7/8/9/10 renumbering
docs/audit/2026-06-11-full-repo-audit/per-repo/rust-ai-web-auto.git.txt:15:996b4ac docs(audit): document Dracon AI lib adoption + Section 7/8/9/10 renumbering
docs/audit/2026-06-11-full-repo-audit/per-repo/rust-ai-web-auto.git.txt:16:bede3bb docs: add Dracon AI lib section to README
docs/audit/2026-06-11-full-repo-audit/per-repo/rust-ai-web-auto.git.txt:17:3a55f5a 2 file(s) in examples,src [examples/dracon_ai_smoke.rs, src/env_keys.rs] DELTA:+12/-7
docs/audit/2026-06-11-full-repo-audit/per-repo/rust-ai-web-auto.git.txt:18:c698705 4 file(s) in examples,src [examples/dracon_ai_smoke.rs, src/doctor.rs, Cargo.lock] DELTA:+169/-1 | NEW:examples/dracon_ai_smoke.rs
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/one-mil-girls.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/one-mil-girls.git.txt:13:* main a70be75 [origin/main] 5 file(s) in .svelte-kit,docs [.svelte-kit/ambient.d.ts, docs/audit/2026-06-11-full-audit/dialogue-icon/state.json, docs/audit/2026-06-11-full-audit/validation-after-dialogue.txt] DELTA:+628/-189 | BIN:1 NEW:dialogue-icon/01-dialogue.png,dialogue-icon/state.json,2026-06-11-full-audit/validation-after-dialogue.txt
docs/audit/2026-06-11-full-repo-audit/final/per-repo/dracon-platform.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/final/per-repo/dracon-platform.git.txt:15:  azumi-ver                                11f588f8d chore(goal): ai-hub-audit goal complete (6/6 tasks)
docs/audit/2026-06-11-full-repo-audit/final/per-repo/dracon-platform.git.txt:17:* main                                     8003ccebc [origin/main] 9 file(s) in web [web/tests/ai-hub/ai-hub.spec.ts, web/PAGE-AUDIT.md, Caddyfile.dev] DELTA:+134/-24 | TEST:62 BIN:2
docs/audit/2026-06-11-full-repo-audit/final/per-repo/dracon-platform.git.txt:20:  phase-1/api-core-lift                    0f5e8e22b [origin/phase-1/api-core-lift] 1 file(s) in apis [apis/ai-api/.env] DELTA:+1/-1 | ENV:
docs/audit/2026-06-11-full-repo-audit/final/per-repo/dracon-platform.git.txt:21:  phase-2/high-cluster                     7ad8ecca9 [origin/phase-2/high-cluster] 3 file(s) in web [web/ai-hub/src/lib/chrome.config.ts, web/ai-hub/src/routes/+layout.svelte, web/packages/chrome/src/lib/SiteSubNav.svelte] DELTA:+9/-19
docs/audit/2026-06-11-full-repo-audit/final/per-repo/dracon-platform.git.txt:25:  phase-4/specta-metrics                   d8f6a56e2 [origin/phase-4/specta-metrics] 1 file(s) in web [web/ai-hub/src/lib/icons.ts] DELTA:+4/-2
docs/audit/2026-06-11-full-repo-audit/final/per-repo/dracon-platform.git.txt:27:8003ccebc 9 file(s) in web [web/tests/ai-hub/ai-hub.spec.ts, web/PAGE-AUDIT.md, Caddyfile.dev] DELTA:+134/-24 | TEST:62 BIN:2
docs/audit/2026-06-11-full-repo-audit/final/per-repo/dracon-platform.git.txt:31:883481853 14 file(s) in apis,web [apis/services/ai-api/ai-api-sdk/tests/sdk.rs, apis/services/ai-api/tests/streaming.rs, apis/services/ai-api/src/handlers/tests.rs] DELTA:+113/-45 | TEST:123 BIN:1
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-before.tsv:2:/home/dracon/Dev/dracon-platform	main	1	0	0	2	0	DIRTY,AHEAD:2,STUCK_PUSH	STUCK	run repair-concerns --apply (push or rewrite)
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-before.tsv:3:/home/dracon/Dev/browser-extensions-shared	main	1	0	2	0	0	DIRTY	OK	run repair-warns --apply
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-before.tsv:4:/home/dracon/Dev/dracon-code	main	8	0	0	0	0	DIRTY	OK	run repair-warns --apply
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-before.tsv:5:/home/dracon/Dev/rust-ai-web-auto	main	2	0	0	0	0	DIRTY	OK	run repair-warns --apply
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-before.tsv:6:/home/dracon/Dev/dracon-utilities	main	1	0	1	0	0	DIRTY	OK	run repair-warns --apply
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-before.tsv:7:/home/dracon/.dracon	main	1	0	0	0	0	DIRTY	OK	run repair-warns --apply
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-before.tsv:8:/home/dracon/Dev/pully-fully-pull-based-fleet-reconciler	main	1	0	0	0	0	DIRTY	OK	run repair-warns --apply
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/pully-fully-pull-based-fleet-reconciler.risk.tsv:22:tracked	.ralph/analysis/AI-OPS.md
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/pully-fully-pull-based-fleet-reconciler.risk.tsv:41:tracked	dracon-fleet/env/platform/ai-api-service.env
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.dracon-platform.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.dracon-platform.git.txt:15:  azumi-ver                                11f588f8d chore(goal): ai-hub-audit goal complete (6/6 tasks)
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.dracon-platform.git.txt:17:* main                                     a1b780fa7 [origin/main] 4 file(s) in web [web/games-hosted/games/junk-runner/assets/index-LYCAb34z.js, web/games-hosted/games/junk-runner/assets/index-mfwwmdEg.js, web/games-hosted/games/junk-runner/index.html] DELTA:+14/-14 | NEW:assets/index-mfwwmdEg.js DEL:assets/index-LYCAb34z.js
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.dracon-platform.git.txt:20:  phase-1/api-core-lift                    0f5e8e22b [origin/phase-1/api-core-lift] 1 file(s) in apis [apis/ai-api/.env] DELTA:+1/-1 | ENV:
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.dracon-platform.git.txt:21:  phase-2/high-cluster                     7ad8ecca9 [origin/phase-2/high-cluster] 3 file(s) in web [web/ai-hub/src/lib/chrome.config.ts, web/ai-hub/src/routes/+layout.svelte, web/packages/chrome/src/lib/SiteSubNav.svelte] DELTA:+9/-19
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.dracon-platform.git.txt:25:  phase-4/specta-metrics                   d8f6a56e2 [origin/phase-4/specta-metrics] 1 file(s) in web [web/ai-hub/src/lib/icons.ts] DELTA:+4/-2
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.dracon-platform.git.txt:29:0b5cde6b3 2 file(s) in apis [apis/libs/api-core/src/request_id.rs, apis/services/ai-api/src/ai/client/mod.rs] DELTA:+137/-0 | NEW:src/request_id.rs
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.dracon-platform.git.txt:30:928216dbd 8 file(s) in apis,web [apis/services/ai-api/tests/happy_path.rs, apis/services/ai-api/tests/common/mod.rs, apis/services/ai-api/ai-api-sdk/tests/sdk.rs] DELTA:+160/-120 | TEST:276 BIN:2
docs/audit/2026-06-11-full-repo-audit/per-repo/repos.tsv:6:/home/dracon/Dev/ai-auto-writer
docs/audit/2026-06-11-full-repo-audit/per-repo/repos.tsv:8:/home/dracon/Dev/rust-ai-web-auto
docs/audit/2026-06-11-full-repo-audit/per-repo/repos.tsv:13:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent
docs/audit/2026-06-11-full-repo-audit/per-repo/repos.tsv:16:/home/dracon/Dev/dracon-ai-lib
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-utilities.risk.tsv:49:tracked	.ralph/cleanup-remaining.md
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-utilities.risk.tsv:50:tracked	.ralph/cleanup-remaining.state.json
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/per-repo-before.dracon-platform.txt:16:  azumi-ver                                11f588f8d chore(goal): ai-hub-audit goal complete (6/6 tasks)
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/per-repo-before.dracon-platform.txt:18:* main                                     d46a3711b [origin/main: ahead 2] chore(vendor): trim registry metadata from hyper-rustls patch
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/per-repo-before.dracon-platform.txt:21:  phase-1/api-core-lift                    0f5e8e22b [origin/phase-1/api-core-lift] 1 file(s) in apis [apis/ai-api/.env] DELTA:+1/-1 | ENV:
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/per-repo-before.dracon-platform.txt:22:  phase-2/high-cluster                     7ad8ecca9 [origin/phase-2/high-cluster] 3 file(s) in web [web/ai-hub/src/lib/chrome.config.ts, web/ai-hub/src/routes/+layout.svelte, web/packages/chrome/src/lib/SiteSubNav.svelte] DELTA:+9/-19
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/per-repo-before.dracon-platform.txt:26:  phase-4/specta-metrics                   d8f6a56e2 [origin/phase-4/specta-metrics] 1 file(s) in web [web/ai-hub/src/lib/icons.ts] DELTA:+4/-2
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/per-repo-before.dracon-platform.txt:28:origin/main
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/per-repo-before.dracon-platform.txt:38:a8ee79bc9 1 file(s) in web [web/ai-hub/src/lib/server/json-cache.ts] DELTA:+4/-3
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/per-repo-before.dracon-platform.txt:39:69ac758a0 1 file(s) in web [web/ai-hub/src/lib/server/openrouter.ts] DELTA:+2/-2
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/per-repo-before.dracon-platform.txt:40:f5d9e0ba9 2 file(s) in web [web/ai-hub/src/lib/server/openrouter.ts, web/tests/ai-hub/ai-hub.spec.ts] DELTA:+45/-18 | TEST:22
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/per-repo-before.dracon-platform.txt:41:424e808f7 2 file(s) in web [web/ai-hub/src/lib/server/artificial-analysis.ts, web/ai-hub/src/lib/server/json-cache.ts] DELTA:+36/-37
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/per-repo-before.dracon-platform.txt:43:4f744c22a 1 file(s) in web [web/ai-hub/src/lib/server/json-cache.ts] DELTA:+143/-15
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/per-repo-before.dracon-platform.txt:44:229cd8646 5 file(s) in web [web/ai-hub/{.cache.preserve/.cache.preserve => .cache.preserve2}/openrouter-models.json, web/ai-hub/.cache.preserve/.cache.preserve/https___artificialanalysis.ai_api_v2_data_llms_models.json, web/ai-hub/.cache.preserve/https___artificialanalysis.ai_api_v2_data_llms_models.json] DELTA:+2/-4 | NEW:.cache.preserve2/https___artificialanalysis.ai_api_v2_data_llms_models.json DEL:.cache.preserve/https___artificialanalysis.ai_api_v2_data_llms_models.json,.cache.preserve/https___artificialanalysis.ai_api_v2_data_llms_models.json,.cache.preserve/openrouter-models.json
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/per-repo-before.dracon-platform.txt:45:e69497188 4 file(s) in web [web/ai-hub/.cache.preserve/.cache.preserve/https___artificialanalysis.ai_api_v2_data_llms_models.json, web/ai-hub/.cache.preserve/.cache.preserve/openrouter-models.json, web/ai-hub/.cache.preserve/https___artificialanalysis.ai_api_v2_data_llms_models.json] DELTA:+4/-0 | NEW:.cache.preserve/https___artificialanalysis.ai_api_v2_data_llms_models.json,.cache.preserve/openrouter-models.json,.cache.preserve/https___artificialanalysis.ai_api_v2_data_llms_models.json,.cache.preserve/openrouter-models.json
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/per-repo-before.dracon-platform.txt:48:7fdf63bf1 11 file(s) in apis [apis/services/ai-api/tests/happy_path.rs, apis/services/auth-api/src/handlers/router.rs, apis/libs/api-core/src/request_id.rs] DELTA:+96/-87 | TEST:68
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/per-repo-before.dracon-platform.txt:54:⚠️  Possible plaintext secrets detected in push.
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/per-repo-before.dracon-platform.txt:57:error: failed to push some refs to 'https://github.com/DraconDev/dracon-platform.git'
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/dracon-utilities.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/dracon-utilities.git.txt:20:* main                   b27b5210 [origin/main] 5 file(s) in dracon-sync [dracon-sync/src/standard_files.rs, dracon-sync/src/policy.rs, AGENTS.md] DELTA:+133/-9
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/one-mil-girls.risk.tsv:13:tracked	.pi/goals/archived/goal_2026060318184883_mpyaiftl-mv1cfy.md
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/one-mil-girls.risk.tsv:54:tracked	docs/audit/2026-06-11-full-audit/menu/01-main-menu.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/one-mil-girls.risk.tsv:61:tracked	docs/audit/visual-qa/2026-06-10-post-inspiration-polish/01-main-menu.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/one-mil-girls.risk.tsv:84:tracked	docs/audit/visual-qa/after/main-menu.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/one-mil-girls.risk.tsv:88:tracked	docs/audit/visual-qa/before/main-menu.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/one-mil-girls.risk.tsv:103:tracked	docs/audit/visual-qa/convo-redesign-before/main-screen.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/one-mil-girls.risk.tsv:105:tracked	docs/audit/visual-qa/crops/main-menu-center.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/one-mil-girls.risk.tsv:107:tracked	docs/audit/visual-qa/crops/vn-dialogue-portrait.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/one-mil-girls.risk.tsv:118:tracked	docs/audit/visual-qa/effects-after/main-screen.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/one-mil-girls.risk.tsv:121:tracked	docs/audit/visual-qa/effects-before/main-screen.png
docs/audit/2026-06-11-full-repo-audit/final/per-repo/dracon-code.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/final/per-repo/dracon-code.git.txt:13:  backup-main-20260513                             13262567 security(dependency configuration): Updated dependency configuration in `deny.toml` for security and comp...
docs/audit/2026-06-11-full-repo-audit/final/per-repo/dracon-code.git.txt:14:  bevy-version                                     ef86290b [gui+src|wip] screenshot viewer, task persistence, fetch denylist UI, gui_refresh_secs poll wiring, ai_actions in plan prompt, dead code cleanup
docs/audit/2026-06-11-full-repo-audit/final/per-repo/dracon-code.git.txt:18:  egui-version                                     0de221d8 {"schema":"dracon.commit.v2","schema_rev":2,"commit_kind":"sync_event","actor":"dracon-sync","generator":{"name":"dracon-git","version":"0.1.0"},"event_fingerprint":"bcc7462f0ab438a932e8482e31fc41ac25fb3d82d26d0a96f0d53304e52a706b","ts":"1771992484","repo":"dracon-code","branch":"master","files":{"added":0,"modified":3,"deleted":0,"renamed":0,"type_change":0,"unknown":0},"changed_paths_full":["Cargo.lock","Cargo.toml","gui/src/main.rs"],"changed_paths":["Cargo.lock","Cargo.toml","gui/src/main.rs"],"changed_paths_total":3,"changed_paths_truncated":false,"top_level_scopes":[{"key":"Cargo.lock","count":1},{"key":"Cargo.toml","count":1},{"key":"gui","count":1}],"extension_summary":[{"key":"lock","count":1},{"key":"rs","count":1},{"key":"toml","count":1}],"domain_summary":[{"key":"code","count":1},{"key":"config","count":1},{"key":"lockfile","count":1}],"intent_tags":["behavior_change_possible","compiled_or_runtime_code_touched","configuration_update","dependency_lock_changed"],"risk_flags":["build_graph_or_dependency_surface"],"semantic":{"files_analyzed":1,"files_skipped":2,"symbols_total":74,"symbols_truncated":false,"symbols":[{"path":"gui/src/main.rs","language":"rust","name":"main","kind":"function","start_line":11,"end_line":25},{"path":"gui/src/main.rs","language":"rust","name":"GuiRuntimeConfig","kind":"struct","start_line":28,"end_line":33},{"path":"gui/src/main.rs","language":"rust","name":"DraconConfigFile","kind":"struct","start_line":36,"end_line":44},{"path":"gui/src/main.rs","language":"rust","name":"Screen","kind":"enum","start_line":47,"end_line":51},{"path":"gui/src/main.rs","language":"rust","name":"Screen","kind":"impl","start_line":53,"end_line":61},{"path":"gui/src/main.rs","language":"rust","name":"label","kind":"function","start_line":54,"end_line":60},{"path":"gui/src/main.rs","language":"rust","name":"HubSort","kind":"enum","start_line":64,"end_line":70},{"path":"gui/src/main.rs","language":"rust","name":"HubSort","kind":"impl","start_line":72,"end_line":82},{"path":"gui/src/main.rs","language":"rust","name":"label","kind":"function","start_line":73,"end_line":81},{"path":"gui/src/main.rs","language":"rust","name":"HubFilter","kind":"enum","start_line":85,"end_line":89},{"path":"gui/src/main.rs","language":"rust","name":"HubFilter","kind":"impl","start_line":91,"end_line":99},{"path":"gui/src/main.rs","language":"rust","name":"label","kind":"function","start_line":92,"end_line":98},{"path":"gui/src/main.rs","language":"rust","name":"SettingsForm","kind":"struct","start_line":102,"end_line":110},{"path":"gui/src/main.rs","language":"rust","name":"SettingsForm","kind":"impl","start_line":112,"end_line":179},{"path":"gui/src/main.rs","language":"rust","name":"from_body","kind":"function","start_line":113,"end_line":132},{"path":"gui/src/main.rs","language":"rust","name":"apply_to_body","kind":"function","start_line":134,"end_line":178},{"path":"gui/src/main.rs","language":"rust","name":"OperatorApp","kind":"struct","start_line":181,"end_line":198},{"path":"gui/src/main.rs","language":"rust","name":"OperatorApp","kind":"impl","start_line":200,"end_line":708},{"path":"gui/src/main.rs","language":"rust","name":"new","kind":"function","start_line":201,"end_line":240},{"path":"gui/src/main.rs","language":"rust","name":"refresh","kind":"function","start_line":242,"end_line":271},{"path":"gui/src/main.rs","language":"rust","name":"run_action","kind":"function","start_line":273,"end_line":280},{"path":"gui/src/main.rs","language":"rust","name":"save_config","kind":"function","start_line":282,"end_line":304},{"path":"gui/src/main.rs","language":"rust","name":"sorted_hub_rows","kind":"function","start_line":306,"end_line":348},{"path":"gui/src/main.rs","language":"rust","name":"nav_row","kind":"function","start_line":350,"end_line":365},{"path":"gui/src/main.rs","language":"rust","name":"project_screen","kind":"function","start_line":367,"end_line":450},{"path":"gui/src/main.rs","language":"rust","name":"hub_screen","kind":"function","start_line":452,"end_line":533},{"path":"gui/src/main.rs","language":"rust","name":"settings_screen","kind":"function","start_line":535,"end_line":707},{"path":"gui/src/main.rs","language":"rust","name":"OperatorApp","kind":"impl","start_line":710,"end_line":769},{"path":"gui/src/main.rs","language":"rust","name":"update","kind":"function","start_line":711,"end_line":768},{"path":"gui/src/main.rs","language":"rust","name":"apply_theme","kind":"function","start_line":771,"end_line":818},{"path":"gui/src/main.rs","language":"rust","name":"panel","kind":"function","start_line":820,"end_line":835},{"path":"gui/src/main.rs","language":"rust","name":"screen_title","kind":"function","start_line":837,"end_line":851},{"path":"gui/src/main.rs","language":"rust","name":"paint_background","kind":"function","start_line":853,"end_line":888},{"path":"gui/src/main.rs","language":"rust","name":"kv","kind":"function","start_line":890,"end_line":895},{"path":"gui/src/main.rs","language":"rust","name":"status_chip","kind":"function","start_line":897,"end_line":909},{"path":"gui/src/main.rs","language":"rust","name":"action_button","kind":"function","start_line":911,"end_line":928},{"path":"gui/src/main.rs","language":"rust","name":"tab_button","kind":"function","start_line":930,"end_line":952},{"path":"gui/src/main.rs","language":"rust","name":"chip_button","kind":"function","start_line":954,"end_line":969},{"path":"gui/src/main.rs","language":"rust","name":"truncate_middle","kind":"function","start_line":971,"end_line":978},{"path":"gui/src/main.rs","language":"rust","name":"draw_projects_table","kind":"function","start_line":980,"end_line":1065},{"path":"gui/src/main.rs","language":"rust","name":"draw_hub_table","kind":"function","start_line":1067,"end_line":1180},{"path":"gui/src/main.rs","language":"rust","name":"table_header","kind":"function","start_line":1182,"end_line":1190},{"path":"gui/src/main.rs","language":"rust","name":"table_row_bg","kind":"function","start_line":1192,"end_line":1198},{"path":"gui/src/main.rs","language":"rust","name":"is_active_repo","kind":"function","start_line":1200,"end_line":1205},{"path":"gui/src/main.rs","language":"rust","name":"phase_color","kind":"function","start_line":1207,"end_line":1217},{"path":"gui/src/main.rs","language":"rust","name":"trigger_color","kind":"function","start_line":1219,"end_line":1225},{"path":"gui/src/main.rs","language":"rust","name":"git_state_color","kind":"function","start_line":1227,"end_line":1241},{"path":"gui/src/main.rs","language":"rust","name":"FleetView","kind":"struct","start_line":1244,"end_line":1247},{"path":"gui/src/main.rs","language":"rust","name":"FleetRow","kind":"struct","start_line":1250,"end_line":1259},{"path":"gui/src/main.rs","language":"rust","name":"FleetRow","kind":"impl","start_line":1261,"end_line":1273},{"path":"gui/src/main.rs","language":"rust","name":"active_slice_label","kind":"function","start_line":1262,"end_line":1266},{"path":"gui/src/main.rs","language":"rust","name":"updated_label","kind":"function","start_line":1268,"end_line":1272},{"path":"gui/src/main.rs","language":"rust","name":"merge_discovered_repos","kind":"function","start_line":1275,"end_line":1291},{"path":"gui/src/main.rs","language":"rust","name":"compute_git_states","kind":"function","start_line":1293,"end_line":1297},{"path":"gui/src/main.rs","language":"rust","name":"git_state_for_repo","kind":"function","start_line":1299,"end_line":1324},{"path":"gui/src/main.rs","language":"rust","name":"parse_branch_sync","kind":"function","start_line":1326,"end_line":1348},{"path":"gui/src/main.rs","language":"rust","name":"discover_git_repos","kind":"function","start_line":1350,"end_line":1363},{"path":"gui/src/main.rs","language":"rust","name":"walk_for_git_repos","kind":"function","start_line":1365,"end_line":1407},{"path":"gui/src/main.rs","language":"rust","name":"refresh_view","kind":"function","start_line":1409,"end_line":1432},{"path":"gui/src/main.rs","language":"rust","name":"choose_selected_repo","kind":"function","start_line":1434,"end_line":1457},{"path":"gui/src/main.rs","language":"rust","name":"load_fleet_rows","kind":"function","start_line":1459,"end_line":1501},{"path":"gui/src/main.rs","language":"rust","name":"load_fleet_rows_sqlite","kind":"function","start_line":1503,"end_line":1547},{"path":"gui/src/main.rs","language":"rust","name":"load_gui_runtime_config","kind":"function","start_line":1549,"end_line":1580},{"path":"gui/src/main.rs","language":"rust","name":"default_fleet_db_path","kind":"function","start_line":1582,"end_line":1584},{"path":"gui/src/main.rs","language":"rust","name":"expand_tilde","kind":"function","start_line":1586,"end_line":1596},{"path":"gui/src/main.rs","language":"rust","name":"read_text_file","kind":"function","start_line":1598,"end_line":1601},{"path":"gui/src/main.rs","language":"rust","name":"run_json","kind":"function","start_line":1603,"end_line":1611},{"path":"gui/src/main.rs","language":"rust","name":"run_cmd","kind":"function","start_line":1613,"end_line":1619},{"path":"gui/src/main.rs","language":"rust","name":"run_cmd_in","kind":"function","start_line":1621,"end_line":1627},{"path":"gui/src/main.rs","language":"rust","name":"run_cmd_capture","kind":"function","start_line":1629,"end_line":1652},{"path":"gui/src/main.rs","language":"rust","name":"resolve_default_project","kind":"function","start_line":1654,"end_line":1659},{"path":"gui/src/main.rs","language":"rust","name":"append_log","kind":"function","start_line":1661,"end_line":1666},{"path":"gui/src/main.rs","language":"rust","name":"now_secs","kind":"function","start_line":1668,"end_line":1673},{"path":"gui/src/main.rs","language":"rust","name":"format_ts","kind":"function","start_line":1675,"end_line":1677}],"kind_summary":[{"key":"function","count":58},{"key":"impl","count":7},{"key":"struct","count":6},{"key":"enum","count":3}],"language_summary":[{"key":"rust","count":74}]},"status":{"ahead":0,"behind":0,"modified_files":1,"staged_files":0},"policy":{"deterministic":true,"ai_commit_messages":false}}
docs/audit/2026-06-11-full-repo-audit/final/per-repo/dracon-code.git.txt:20:* main                                             664aba62 [origin/main] 1 file(s) [COMPARATIVE_AUDIT.md] DELTA:+1/-1
docs/audit/2026-06-11-full-repo-audit/final/per-repo/dracon-code.git.txt:22:  temp-main                                        e1eaa26d Reset to origin/master
docs/audit/2026-06-11-full-repo-audit/final/per-repo/dracon-code.git.txt:29:1fc9373f 4 file(s) in docs,plan [docs/AI-LIB-AUDIT.md, docs/README.md, docs/AI-STRATEGY.md] DELTA:+154/-1 | NEW:docs/AI-LIB-AUDIT.md
docs/audit/2026-06-11-full-repo-audit/per-repo/dracon-ai-lib.git.txt:1:REPO=/home/dracon/Dev/dracon-ai-lib
docs/audit/2026-06-11-full-repo-audit/per-repo/dracon-ai-lib.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/per-repo/dracon-ai-lib.git.txt:4:origin	https://github.com/DraconDev/dracon-ai-lib.git (fetch)
docs/audit/2026-06-11-full-repo-audit/per-repo/dracon-ai-lib.git.txt:5:origin	https://github.com/DraconDev/dracon-ai-lib.git (push)
docs/audit/2026-06-11-full-repo-audit/per-repo/dracon-ai-lib.git.txt:7:* main cd8bc7f [origin/main: ahead 13] 1 file(s) in crates [crates/ai-lib/src/providers/minimax.rs] DELTA:+48/-37
docs/audit/2026-06-11-full-repo-audit/per-repo/dracon-ai-lib.git.txt:9:cd8bc7f 1 file(s) in crates [crates/ai-lib/src/providers/minimax.rs] DELTA:+48/-37
docs/audit/2026-06-11-full-repo-audit/per-repo/dracon-ai-lib.git.txt:10:209cff3 1 file(s) in crates [crates/ai-lib/src/providers/minimax.rs] DELTA:+1/-1
docs/audit/2026-06-11-full-repo-audit/per-repo/dracon-ai-lib.git.txt:11:d70cf8a 1 file(s) in crates [crates/ai-lib/src/providers/minimax.rs] DELTA:+16/-13
docs/audit/2026-06-11-full-repo-audit/per-repo/dracon-ai-lib.git.txt:12:5fec442 17 file(s) in crates [Cargo.lock, crates/client/src/lib.rs, crates/providers/src/openai.rs] DELTA:+1415/-589
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.dracon-libs.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.dracon-libs.git.txt:13:* main 2ff017b [origin/main] 1 file(s) [deny.toml] DELTA:+2/-0
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/avid.risk.tsv:60:tracked	.ralph/videoai-pilot.md
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/avid.risk.tsv:61:tracked	.ralph/videoai-pilot.state.json
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.youtube-video-uploader.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.youtube-video-uploader.git.txt:13:* main 771d422 [origin/main] Merge https://github.com/DraconDev/youtube-video-uploader
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/pully-fully-pull-based-fleet-reconciler.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/pully-fully-pull-based-fleet-reconciler.git.txt:4:1 .M N... 100644 100644 100644 f88c0f7c005499377d01ed8eaade3d55b81b4fcc f88c0f7c005499377d01ed8eaade3d55b81b4fcc pully/bins/pully/src/main.rs
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/pully-fully-pull-based-fleet-reconciler.git.txt:15:* main 23a92627 [origin/main] 5 file(s) in fully,pully [fully/crates/fully-core/src/fleet_status.rs, fully/bins/fully/src/main.rs, pully/crates/pully-core/src/service_reconciler/mod.rs] DELTA:+148/-39
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/pully-fully-pull-based-fleet-reconciler.git.txt:17:23a92627 5 file(s) in fully,pully [fully/crates/fully-core/src/fleet_status.rs, fully/bins/fully/src/main.rs, pully/crates/pully-core/src/service_reconciler/mod.rs] DELTA:+148/-39
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-after-remediation.tsv:2:/home/dracon/Dev/browser-extensions-shared	main	3	0	2	0	0	DIRTY	OK	run repair-warns --apply
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-after-remediation.tsv:3:/home/dracon/Dev/dracon-platform	main	5	0	0	0	0	DIRTY	OK	run repair-warns --apply
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-after-remediation.tsv:4:/home/dracon/Dev/dracon-utilities	main	2	0	1	0	0	DIRTY	OK	run repair-warns --apply
docs/audit/2026-06-11-full-repo-audit/final/per-repo/ai-auto-repo-rot-scanner-todo-agent.git.txt:1:REPO=/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent
docs/audit/2026-06-11-full-repo-audit/final/per-repo/ai-auto-repo-rot-scanner-todo-agent.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/final/per-repo/ai-auto-repo-rot-scanner-todo-agent.git.txt:4:codeberg	git@codeberg.org:dracondev/ai-auto-repo-rot-scanner-todo-agent.git (fetch)
docs/audit/2026-06-11-full-repo-audit/final/per-repo/ai-auto-repo-rot-scanner-todo-agent.git.txt:5:codeberg	git@codeberg.org:dracondev/ai-auto-repo-rot-scanner-todo-agent.git (push)
docs/audit/2026-06-11-full-repo-audit/final/per-repo/ai-auto-repo-rot-scanner-todo-agent.git.txt:6:github	git@github.com:DraconDev/ai-auto-repo-rot-scanner-todo-agent.git (fetch)
docs/audit/2026-06-11-full-repo-audit/final/per-repo/ai-auto-repo-rot-scanner-todo-agent.git.txt:7:github	git@github.com:DraconDev/ai-auto-repo-rot-scanner-todo-agent.git (push)
docs/audit/2026-06-11-full-repo-audit/final/per-repo/ai-auto-repo-rot-scanner-todo-agent.git.txt:8:gitlab	git@gitlab.com:dracondev/ai-auto-repo-rot-scanner-todo-agent.git (fetch)
docs/audit/2026-06-11-full-repo-audit/final/per-repo/ai-auto-repo-rot-scanner-todo-agent.git.txt:9:gitlab	git@gitlab.com:dracondev/ai-auto-repo-rot-scanner-todo-agent.git (push)
docs/audit/2026-06-11-full-repo-audit/final/per-repo/ai-auto-repo-rot-scanner-todo-agent.git.txt:10:origin	https://github.com/DraconDev/ai-auto-repo-rot-scanner-todo-agent.git (fetch)
docs/audit/2026-06-11-full-repo-audit/final/per-repo/ai-auto-repo-rot-scanner-todo-agent.git.txt:11:origin	https://github.com/DraconDev/ai-auto-repo-rot-scanner-todo-agent.git (push)
docs/audit/2026-06-11-full-repo-audit/final/per-repo/ai-auto-repo-rot-scanner-todo-agent.git.txt:13:* main 9607985 [origin/main] 1 file(s) in docs [docs/AUDIT-2026-06-10.md] DELTA:+527/-0 | NEW:docs/AUDIT-2026-06-10.md
docs/audit/2026-06-11-full-repo-audit/final/per-repo/ai-auto-repo-rot-scanner-todo-agent.git.txt:16:90c4433 refactor(ai): migrate from archived dracon-ai-lib to ai-api-sdk
docs/audit/2026-06-11-full-repo-audit/final/per-repo/ai-auto-repo-rot-scanner-todo-agent.git.txt:18:c70e485 1 file(s) in src [src/ai/mod.rs] DELTA:+1/-1
docs/audit/2026-06-11-full-repo-audit/final/per-repo/ai-auto-repo-rot-scanner-todo-agent.git.txt:19:eb44a00 1 file(s) in src [src/ai/mod.rs] DELTA:+1/-4
docs/audit/2026-06-11-full-repo-audit/per-repo/ai-auto-writer.git.txt:1:REPO=/home/dracon/Dev/ai-auto-writer
docs/audit/2026-06-11-full-repo-audit/per-repo/ai-auto-writer.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/per-repo/ai-auto-writer.git.txt:4:codeberg	git@codeberg.org:dracondev/ai-auto-writer.git (fetch)
docs/audit/2026-06-11-full-repo-audit/per-repo/ai-auto-writer.git.txt:5:codeberg	git@codeberg.org:dracondev/ai-auto-writer.git (push)
docs/audit/2026-06-11-full-repo-audit/per-repo/ai-auto-writer.git.txt:6:github	git@github.com:DraconDev/ai-auto-writer.git (fetch)
docs/audit/2026-06-11-full-repo-audit/per-repo/ai-auto-writer.git.txt:7:github	git@github.com:DraconDev/ai-auto-writer.git (push)
docs/audit/2026-06-11-full-repo-audit/per-repo/ai-auto-writer.git.txt:8:gitlab	git@gitlab.com:dracondev/ai-auto-writer.git (fetch)
docs/audit/2026-06-11-full-repo-audit/per-repo/ai-auto-writer.git.txt:9:gitlab	git@gitlab.com:dracondev/ai-auto-writer.git (push)
docs/audit/2026-06-11-full-repo-audit/per-repo/ai-auto-writer.git.txt:10:origin	https://github.com/DraconDev/ai-auto-writer.git (fetch)
docs/audit/2026-06-11-full-repo-audit/per-repo/ai-auto-writer.git.txt:11:origin	https://github.com/DraconDev/ai-auto-writer.git (push)
docs/audit/2026-06-11-full-repo-audit/per-repo/ai-auto-writer.git.txt:14:* main                               afa5d2b4 [origin/main] 53 file(s) in src,tests [src/logic/chapter_writer.rs, src/logic/outline_builder.rs, src/quality/checks.rs] DELTA:+2157/-845 | TEST:52
docs/audit/2026-06-11-full-repo-audit/per-repo/ai-auto-writer.git.txt:17:9c829b43 Merge https://github.com/DraconDev/ai-auto-writer
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/folder-auto-banner.risk.tsv:27:tracked	.pi/goals/archived/goal_2026060520100089_mq1aifn7-6yezq4.md
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.browser-extensions-shared.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.browser-extensions-shared.git.txt:15:* main f817438ac [origin/main] 640 file(s) in SamAI,auto-form-filler [SamAI/coverage/utils/store.ts.html, SamAI/coverage/utils/api.ts.html, SamAI/coverage/utils/simpleFormProfiles.ts.html] DELTA:+0/-13894 | TEST:351 BIN:260 DEL:coverage/base.css,coverage/block-navigation.js,coverage/clover.xml,coverage/coverage-final.json,onboarding/App.tsx.html,onboarding/index.html,settings/App.tsx.html,settings/index.html,coverage/favicon.png,coverage/index.html+630more
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.browser-extensions-shared.git.txt:17:f817438ac 640 file(s) in SamAI,auto-form-filler [SamAI/coverage/utils/store.ts.html, SamAI/coverage/utils/api.ts.html, SamAI/coverage/utils/simpleFormProfiles.ts.html] DELTA:+0/-13894 | TEST:351 BIN:260 DEL:coverage/base.css,coverage/block-navigation.js,coverage/clover.xml,coverage/coverage-final.json,onboarding/App.tsx.html,onboarding/index.html,settings/App.tsx.html,settings/index.html,coverage/favicon.png,coverage/index.html+630more
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.browser-extensions-shared.git.txt:18:843eb40bb 4 file(s) in job-finder,vidpro-extension [vidpro-extension/docs/GET-READY-CHECKLIST.md, job-finder/docs/P0_HARDENING_AUDIT.md, vidpro-extension/docs/NON-AI-GAPS.md] DELTA:+271/-1 | NEW:docs/GET-READY-CHECKLIST.md
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/Junk-Runner-bevy.risk.tsv:42:tracked	.pi/goals/notes_dev_failu[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBKYjIrcVUvcEdNZjF0NStPSVlzb2VEaFpzcjk0VDZ6L1I5SVpBQ2dwcVNvCmo5bXNXZ05VWUxDQnZhSTFYY3BsWlJOTTJ4RHlLekhvZW1HOFJNZkxJYU0KLT4gWDI1NTE5IHpJWDN3d0ZUNDI0SFJnMkhsRkR5eURuM3lBYWhQcVl2VmJPRHZha1pjREUKSThtTTZCWnVmR2lvK0lGNTJhMVdnOTFuV0YyNTI3RG1qT1EzTW4zRVZBTQotPiBYMjU1MTkgL1NhNWpBTHVhdHdRUTExYy85VjhOeGlYN2dZZkJobXRJQTJNOVAyYS8yOAoydUluOXRFcGRVRDdNK2RraGFHMUtnQXJqeTRLWDFXRDgwV1B4ZFNnaVdjCi0+IFgyNTUxOSBReExJYVZYeVoyQTA5ZFFMei9adVkvTmVuMFZCaThSS2dxUi9FSGlZOURFCnkwQUFWaUpqRHBCcTFNaitaTjhscC9Hb0lUckdqc1pMV1NMYUFQa0xzb3cKLT4gWDI1NTE5IFd6Qkp6QjNSN2hOMzh1NUJqM0NiTHljcjZHQVNYaU9IYjFVcTJhZndvRGMKZUdXQm9LYTVoNndaUUJzU3lQNlUycHB4a1VKNEZpRFpldFVUb2hObm9GcwotPiBYMjU1MTkgc1lTRGVvZW9tNnRESzBvekZacTc1UXNjUUJvdlNzTHdMejJ2bVZhNHlrSQo2ZzFCK0hoMnFhMkdmWDhCVUVMTzRRSzJSdnE4WG5GNTBFRVpObmhkZ2ZNCi0+IFgyNTUxOSAvd1o3R2dsQ2QxV280Sml3SUIzTXdpU3RXR2hsR0g5cy9maStvTGZ0WkZJCnAxQk4yaDJmVmJKU2Z2eDZTTVFiUTFERVlQUWRPWUlNS0cwWDlxTlcySHcKLT4gU003PWAwL1UtZ3JlYXNlID8oXDNaICwoICZPYyBOaT9UPFptbQpZL2RzTEtKYXJZOGMyVHU1RjQ5QlVlN0k2VmREOTFqbUZhdTRjNDJsTWxOTHpVRXJSV0JkVjNnak03dUZVWUNHCkdjQ1AwV2Q2eFhTTE5hVndmd0orNTVBQUxJb3BVTG8xaEJzY3FOUkkvalF1R01LYTNBCi0tLSBWcytaRjJYMUw2TUJiQmZxaFo3MkJrS05LSHMyM3dtNEZQY3VQMTlJV0FVCn4jN0yt+ncQ/NWh1ilXHSVGZhvvAxcARDl7zXy3uZbqDgOXVgd2p7jzkawhdbpJRLjkQhB4vP/4+kwDcOc=].md
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/Junk-Runner-bevy.risk.tsv:117:tracked	assets/audio/sfx_repair.wav
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/Junk-Runner-bevy.risk.tsv:121:tracked	assets/audio/sfx_system_failure.wav
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/video-factory.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/video-factory.git.txt:13:* main 698a658 [origin/main] 14 file(s) in crates [crates/api/src/routes.rs, crates/core/src/config.rs, crates/worker/src/ffmpeg.rs] DELTA:+225/-162
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/video-factory.git.txt:17:4215e5f 1 file(s) in src [src/main.rs] DELTA:+3/-3
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.dracon-utilities.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.dracon-utilities.git.txt:30:* main                   e7bf4d0e [origin/main] 4 file(s) in docs [docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/inventory.json, docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/PUBLIC_READINESS.md, docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/inventory.tsv] DELTA:+185/-106 | NEW:public-readiness-funding/PUBLIC_READINESS.md
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.dracon-utilities.git.txt:34:adf34aea 27 file(s) in docs [docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/inventory.json, docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/browser-extensions-shared.git.txt, docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/dracon-platform.git.txt] DELTA:+1206/-0 | NEW:deps/deps.tsv,public-readiness-funding/hygiene.tsv,public-readiness-funding/inventory.json,public-readiness-funding/inventory.tsv,non-rust/non-rust.tsv,per-repo/.dracon.git.txt,per-repo/DraconDev.git.txt,per-repo/Junk-Runner-bevy.git.txt,per-repo/ai-auto-repo-rot-scanner-todo-agent.git.txt,per-repo/ai-auto-writer.git.txt+17more
docs/audit/2026-06-11-full-repo-audit/per-repo/Junk-Runner-bevy.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/per-repo/Junk-Runner-bevy.git.txt:14:  main        e1894697f [origin/main] Added SOLID_VS_SVELTE.md
docs/audit/2026-06-11-full-repo-audit/final/per-repo/dracon-libs.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/final/per-repo/dracon-libs.git.txt:13:* main 2ff017b [origin/main] 1 file(s) [deny.toml] DELTA:+2/-0
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/DraconDev.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/DraconDev.git.txt:16:* main f9a2e70 [origin/main] Merge https://github.com/DraconDev/DraconDev
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before..dracon.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before..dracon.git.txt:13:* main f3daf8503 [origin/main] 3 file(s) in memory,utilities [utilities/sync/dracon-sync.toml, utilities/sync/templates/FUNDING.yml, memory/rag/rag_index.json] DELTA:+8/-7
docs/audit/2026-06-11-full-repo-audit/final/per-repo/video-uploader.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/final/per-repo/video-uploader.git.txt:13:* main 9d5e9f1 [origin/main] 2 file(s) in youtube-uploader-cli [youtube-uploader-cli/tests/cli.rs, youtube-uploader-cli/src/main.rs] DELTA:+5/-3 | TEST:6
docs/audit/2026-06-11-full-repo-audit/final/per-repo/video-uploader.git.txt:15:9d5e9f1 2 file(s) in youtube-uploader-cli [youtube-uploader-cli/tests/cli.rs, youtube-uploader-cli/src/main.rs] DELTA:+5/-3 | TEST:6
docs/audit/2026-06-11-full-repo-audit/final/per-repo/video-uploader.git.txt:17:b630d5f 4 file(s) in youtube-uploader,youtube-uploader-cli [youtube-uploader/src/youtube.rs, youtube-uploader/src/config.rs, youtube-uploader-cli/src/main.rs] DELTA:+80/-40
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.DraconDev.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.DraconDev.git.txt:13:* main e280732 [origin/main] 3 file(s) [README_SUGGESTED_FORM.md, SUGGESTED_FORM_USAGE.md, SUGGESTED_FORM_BLOCKERS.md] DELTA:+191/-0 | NEW:README_SUGGESTED_FORM.md,SUGGESTED_FORM_BLOCKERS.md,SUGGESTED_FORM_USAGE.md
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-final.tsv:2:/home/dracon/Dev/browser-extensions-shared	main	1	0	2	0	0	DIRTY	OK	run repair-warns --apply
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-final.tsv:3:/home/dracon/Dev/dracon-platform	main	0	0	1	0	0	DIRTY	OK	healthy
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-final.tsv:4:/home/dracon/Dev/dracon-utilities	main	0	0	1	0	0	DIRTY	OK	healthy
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/pi-proof/after.Junk-Runner-bevy.pi-files.txt:37:.pi/goals/notes_dev_failu[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBJR0hhQjJTMVoyTi8zMlRrZHZmR1B1dUt0NnVXaGE0N21XNERWU2RPLzNjCjNkMldxdjBUR1BKUUQ0M2l0TzBaTXpHR0kxQlBMdVNPNlVSRGw5dXZNK0UKLT4gWDI1NTE5IGdJK09zVHFIWXRsZUpxMnZDWHhDUFF3RzVGT3hnOXkvMFlMWXZZZUMzMzQKL3JBT3BkR2NxSlJPS2E2cVJNSCs0dmJvam5sa1ZBdS9iL1hscDdnV1hSawotPiBYMjU1MTkgRVJTVzhKZTU1SVY1N2VWVkpCclI4NHhhRDZ4NjBza2dSZkFjYjEvdnJpWQo0R25oYzd5bEI2dnhOdmg1ZXQ2WVNPcFZnL3NMZmZqcHhVaFhNcWttdGNJCi0+IFgyNTUxOSBHaE5zNmsrYzlkU2c0a2NaUXZyTW5hcGRLUkZDN0VENHRqOEVNMUdpQWlNCitGbTJseFhNdHB5Wi9pWVV0bkpTUUpaRVl3V3RLU2dLSXpLZzM1Vjh5b2cKLT4gWDI1NTE5IGVPQUZlZmRXeVZWMjBZMXhiRVR0cHdZeWNuejdEUlFyZFllelAvVENzaTgKM01DaVRGWTQ2RnBBeGwwVUtqdVBDeUxKYWNjMVJjb1N2R3VPSUsrV28yVQotPiBYMjU1MTkgWVRWZ1lnL1Z0UkF6c1NrUWE0dTlpL3JEQ0V6ZmtXY1J6WWQ0bUJ5S2lDYwp6eDhYOElvU0g0VzZuL0l6Sys1MDc3MFV5ZFZ3V0Z2eVBOdU9JSnpkdncwCi0+IFgyNTUxOSBKbW5hS2t1cjdWMVJ6bnkxZ0ViU2FKS0I4clhIZ1dpVTFnRFhnWUdXRVRFCmQ0L3pkbTVsN2dVTmRILzZWM3JyL3dRdXpZNEZ4NXNpNSs4OGhHNVFpY2sKLT4gektZci1ncmVhc2UgRmUgRWlPJCgtSyB2VU92TSZbIFhROWA2dgp4bkM1RXFjMWJkMWx0L2JpZjJ1SVgxeXNDak5ZcHFta3BHZjlhSGdsb3dnUUdIc2t3aWc5aWZacXlLRQotLS0gdG80YkIxNi90cjhZanAwVkdkRjBza2N1bExzdnNmVk1KVmIrZFN1NGJmZwr8f/BkZPHkX1Oxd2BqZ+ZkLUmao0HQvAXepXBXCMf1LeF+/jyPlpX3T3uulPuoZ15k7jrft+Aso7G0KwfD].md
docs/audit/2026-06-11-full-repo-audit/per-repo/avid.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/per-repo/avid.git.txt:14:* main                                     8d1f698 [origin/main] 19 file(s) in examples,src,tests [src/cli.rs, src/ai_gen.rs, src/analyzer.rs] DELTA:+1034/-453 | TEST:81
docs/audit/2026-06-11-full-repo-audit/per-repo/avid.git.txt:16:8d1f698 19 file(s) in examples,src,tests [src/cli.rs, src/ai_gen.rs, src/analyzer.rs] DELTA:+1034/-453 | TEST:81
docs/audit/2026-06-11-full-repo-audit/final/per-repo/kiki-sassy-desktop-announcer.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/final/per-repo/kiki-sassy-desktop-announcer.git.txt:13:* main 0155632 [origin/main] 2 file(s) in src [src/journal.rs, src/daemon.rs] DELTA:+2/-4
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.one-mil-girls.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.one-mil-girls.git.txt:13:* main 1846710 [origin/main] 2 file(s) in docs [docs/audit/2026-06-11-full-audit-v2/script-audit.json, docs/audit/visual-qa/convo-redesign-after/inspect/inspect.json] DELTA:+0/-1525 | DEL:2026-06-11-full-audit-v2/script-audit.json,inspect/inspect.json
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/per-repo-before.rust-ai-web-auto.txt:1:REPO=/home/dracon/Dev/rust-ai-web-auto
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/per-repo-before.rust-ai-web-auto.txt:6:codeberg	git@codeberg.org:dracondev/rust-ai-web-auto.git (fetch)
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/per-repo-before.rust-ai-web-auto.txt:7:codeberg	git@codeberg.org:dracondev/rust-ai-web-auto.git (push)
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/per-repo-before.rust-ai-web-auto.txt:8:github	git@github.com:DraconDev/rust-ai-web-auto.git (fetch)
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/per-repo-before.rust-ai-web-auto.txt:9:github	git@github.com:DraconDev/rust-ai-web-auto.git (push)
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/per-repo-before.rust-ai-web-auto.txt:10:gitlab	git@gitlab.com:dracondev/rust-ai-web-auto.git (fetch)
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/per-repo-before.rust-ai-web-auto.txt:11:gitlab	git@gitlab.com:dracondev/rust-ai-web-auto.git (push)
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/per-repo-before.rust-ai-web-auto.txt:12:origin	https://github.com/DraconDev/rust-ai-web-auto.git (fetch)
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/per-repo-before.rust-ai-web-auto.txt:13:origin	https://github.com/DraconDev/rust-ai-web-auto.git (push)
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/per-repo-before.rust-ai-web-auto.txt:15:* main 88f45ad [origin/main] 1 file(s) in scripts [scripts/inventory_monitor.rs] DELTA:+0/-8
docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/per-repo-before.rust-ai-web-auto.txt:17:origin/main
docs/audit/2026-06-11-full-repo-audit/per-repo/ai-auto-repo-rot-scanner-todo-agent.git.txt:1:REPO=/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent
docs/audit/2026-06-11-full-repo-audit/per-repo/ai-auto-repo-rot-scanner-todo-agent.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/per-repo/ai-auto-repo-rot-scanner-todo-agent.git.txt:5:codeberg	git@codeberg.org:dracondev/ai-auto-repo-rot-scanner-todo-agent.git (fetch)
docs/audit/2026-06-11-full-repo-audit/per-repo/ai-auto-repo-rot-scanner-todo-agent.git.txt:6:codeberg	git@codeberg.org:dracondev/ai-auto-repo-rot-scanner-todo-agent.git (push)
docs/audit/2026-06-11-full-repo-audit/per-repo/ai-auto-repo-rot-scanner-todo-agent.git.txt:7:github	git@github.com:DraconDev/ai-auto-repo-rot-scanner-todo-agent.git (fetch)
docs/audit/2026-06-11-full-repo-audit/per-repo/ai-auto-repo-rot-scanner-todo-agent.git.txt:8:github	git@github.com:DraconDev/ai-auto-repo-rot-scanner-todo-agent.git (push)
docs/audit/2026-06-11-full-repo-audit/per-repo/ai-auto-repo-rot-scanner-todo-agent.git.txt:9:gitlab	git@gitlab.com:dracondev/ai-auto-repo-rot-scanner-todo-agent.git (fetch)
docs/audit/2026-06-11-full-repo-audit/per-repo/ai-auto-repo-rot-scanner-todo-agent.git.txt:10:gitlab	git@gitlab.com:dracondev/ai-auto-repo-rot-scanner-todo-agent.git (push)
docs/audit/2026-06-11-full-repo-audit/per-repo/ai-auto-repo-rot-scanner-todo-agent.git.txt:11:origin	https://github.com/DraconDev/ai-auto-repo-rot-scanner-todo-agent.git (fetch)
docs/audit/2026-06-11-full-repo-audit/per-repo/ai-auto-repo-rot-scanner-todo-agent.git.txt:12:origin	https://github.com/DraconDev/ai-auto-repo-rot-scanner-todo-agent.git (push)
docs/audit/2026-06-11-full-repo-audit/per-repo/ai-auto-repo-rot-scanner-todo-agent.git.txt:14:* main 90c4433 [origin/main] refactor(ai): migrate from archived dracon-ai-lib to ai-api-sdk
docs/audit/2026-06-11-full-repo-audit/per-repo/ai-auto-repo-rot-scanner-todo-agent.git.txt:16:90c4433 refactor(ai): migrate from archived dracon-ai-lib to ai-api-sdk
docs/audit/2026-06-11-full-repo-audit/per-repo/ai-auto-repo-rot-scanner-todo-agent.git.txt:18:c70e485 1 file(s) in src [src/ai/mod.rs] DELTA:+1/-1
docs/audit/2026-06-11-full-repo-audit/per-repo/ai-auto-repo-rot-scanner-todo-agent.git.txt:19:eb44a00 1 file(s) in src [src/ai/mod.rs] DELTA:+1/-4
docs/audit/2026-06-11-full-repo-audit/per-repo/ai-auto-repo-rot-scanner-todo-agent.git.txt:20:0e6873d 1 file(s) in src [src/ai/mod.rs] DELTA:+14/-4
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:59:tracked	apis/services/ai-api/.env.dev
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:60:tracked	apis/services/ai-api/.env.example
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:61:tracked	apis/services/ai-api/.env.prod
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:62:tracked	apis/services/ai-api/ai-api-sdk/.env.example
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:63:tracked	apis/services/ai-api/src/handlers/image.rs
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:71:tracked	apis/services/email-api/.env.dev
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:72:tracked	apis/services/email-api/.env.example
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:73:tracked	apis/services/email-api/.env.prod
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:82:tracked	web/ai-hub-browser-probe.mjs
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:83:tracked	web/ai-hub-repeat-browser-probe.mjs
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:84:tracked	web/ai-hub-signed-browser-probe.mjs
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:85:tracked	web/ai-hub/src/lib/chrome.config.ts
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:86:tracked	web/ai-hub/src/lib/types/chrome.d.ts
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:167:tracked	web/games-hosted/games/junk-runner/assets/sfx_repair-CnxSomk4.wav
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:171:tracked	web/games-hosted/games/junk-runner/assets/sfx_system_failure-DlAFpe7h.wav
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:211:tracked	web/screenshots/ai-hub-current/affiliates-fold.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:212:tracked	web/screenshots/ai-hub-current/affiliates.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:213:tracked	web/screenshots/ai-hub-current/compare-fold.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:214:tracked	web/screenshots/ai-hub-current/compare.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:215:tracked	web/screenshots/ai-hub-current/directory-fold.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:216:tracked	web/screenshots/ai-hub-current/directory.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:217:tracked	web/screenshots/ai-hub-current/free-fold.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:218:tracked	web/screenshots/ai-hub-current/free.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:219:tracked	web/screenshots/ai-hub-current/plans-fold-800.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:220:tracked	web/screenshots/ai-hub-current/plans-fold-final.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:221:tracked	web/screenshots/ai-hub-current/plans-fold.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:222:tracked	web/screenshots/ai-hub-current/plans-fullpage.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:223:tracked	web/screenshots/ai-hub-current/plans-intro.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:224:tracked	web/screenshots/ai-hub-current/plans-narrow.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:225:tracked	web/screenshots/ai-hub-current/plans-redbox-absolute.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:226:tracked	web/screenshots/ai-hub-current/plans-scrolled.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:227:tracked	web/screenshots/ai-hub-current/plans-top-400.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:228:tracked	web/screenshots/ai-hub-current/plans-top-debug.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:229:tracked	web/screenshots/ai-hub-current/plans-with-redbox.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:230:tracked	web/screenshots/ai-hub-current/plans.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:231:tracked	web/screenshots/ai-hub-current/promos-fold.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:232:tracked	web/screenshots/ai-hub-current/promos.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:233:tracked	web/screenshots/ai-hub-current/providers-fold.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:234:tracked	web/screenshots/ai-hub-current/providers.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:235:tracked	web/screenshots/ai-hub-current/rankings-fold.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:236:tracked	web/screenshots/ai-hub-current/rankings.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:237:tracked	web/screenshots/chrome-consistency/ai-hub-affiliates-signed-in.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:238:tracked	web/screenshots/chrome-consistency/ai-hub-affiliates-signed-out.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:239:tracked	web/screenshots/chrome-consistency/ai-hub-compare-signed-in.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:240:tracked	web/screenshots/chrome-consistency/ai-hub-compare-signed-out.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:241:tracked	web/screenshots/chrome-consistency/ai-hub-free-signed-in.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:242:tracked	web/screenshots/chrome-consistency/ai-hub-free-signed-out.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:243:tracked	web/screenshots/chrome-consistency/ai-hub-index-v2.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:244:tracked	web/screenshots/chrome-consistency/ai-hub-plans-signed-in.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:245:tracked	web/screenshots/chrome-consistency/ai-hub-plans-signed-out.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:246:tracked	web/screenshots/chrome-consistency/ai-hub-promos-signed-in.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:247:tracked	web/screenshots/chrome-consistency/ai-hub-promos-signed-out.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:248:tracked	web/screenshots/chrome-consistency/ai-hub-providers-signed-in.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:249:tracked	web/screenshots/chrome-consistency/ai-hub-providers-signed-out.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:250:tracked	web/screenshots/chrome-consistency/ai-hub-signed-in.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:251:tracked	web/screenshots/chrome-consistency/ai-hub-signed-out.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:252:tracked	web/screenshots/chrome-consistency/auth-login-check-email-signed-in.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:253:tracked	web/screenshots/chrome-consistency/auth-login-check-email-signed-out.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:299:tracked	web/screenshots/chrome-fixes/desktop-ai-hub-plans.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:305:tracked	web/screenshots/chrome-fixes/mobile-ai-hub-plans.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:309:tracked	web/screenshots/layout-fix-final/desktop-signed-in-ai-hub.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:310:tracked	web/screenshots/layout-fix-final/desktop-signed-out-ai-hub.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:311:tracked	web/screenshots/layout-fix-final/mobile-ai-hub-closed.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:312:tracked	web/screenshots/layout-fix-final/mobile-ai-hub-drawer-open.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:330:tracked	web/tests/shared/visual-snapshots.spec.ts-snapshots/ai-hub-compare-desktop-shared-linux.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:331:tracked	web/tests/shared/visual-snapshots.spec.ts-snapshots/ai-hub-directory-mobile-shared-linux.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:332:tracked	web/tests/shared/visual-snapshots.spec.ts-snapshots/ai-hub-directory-models-desktop-shared-linux.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:333:tracked	web/tests/shared/visual-snapshots.spec.ts-snapshots/ai-hub-directory-providers-desktop-shared-linux.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:334:tracked	web/tests/shared/visual-snapshots.spec.ts-snapshots/ai-hub-directory-vouchers-desktop-shared-linux.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:335:tracked	web/tests/shared/visual-snapshots.spec.ts-snapshots/ai-hub-plans-desktop-shared-linux.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:336:tracked	web/tests/shared/visual-snapshots.spec.ts-snapshots/ai-hub-provider-groq-desktop-shared-linux.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:337:tracked	web/tests/shared/visual-snapshots.spec.ts-snapshots/ai-hub-rankings-desktop-shared-linux.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:338:tracked	web/tests/shared/visual-snapshots.spec.ts-snapshots/ai-hub-rankings-mobile-shared-linux.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:339:tracked	web/tests/shared/visual-snapshots.spec.ts-snapshots/dashboard-check-email-desktop-shared-linux.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:343:tracked	web/web/test-results/ui-audit-recon/symptom-2-ai-hub.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:346:tracked	web/web/test-results/ui-audit/ai-hub-compare-desktop-1440.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:347:tracked	web/web/test-results/ui-audit/ai-hub-compare-mobile-375.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:348:tracked	web/web/test-results/ui-audit/ai-hub-desktop-1440.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:349:tracked	web/web/test-results/ui-audit/ai-hub-free-desktop-1440.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:350:tracked	web/web/test-results/ui-audit/ai-hub-free-mobile-375.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:351:tracked	web/web/test-results/ui-audit/ai-hub-mobile-375.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:352:tracked	web/web/test-results/ui-audit/ai-hub-plans-desktop-1440.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:353:tracked	web/web/test-results/ui-audit/ai-hub-plans-mobile-375.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:354:tracked	web/web/test-results/ui-audit/ai-hub-promos-desktop-1440.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:355:tracked	web/web/test-results/ui-audit/ai-hub-promos-mobile-375.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:356:tracked	web/web/test-results/ui-audit/ai-hub-providers-desktop-1440.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:357:tracked	web/web/test-results/ui-audit/ai-hub-providers-mobile-375.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:374:tracked	web/web/test-results/ui-audit/verify-ai-hub-landing-desktop-1440.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-platform.risk.tsv:375:tracked	web/web/test-results/ui-audit/verify-ai-hub-landing-mobile-375.png
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/rust-ai-web-auto.git.txt:1:REPO=/home/dracon/Dev/rust-ai-web-auto
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/rust-ai-web-auto.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/rust-ai-web-auto.git.txt:5:codeberg	git@codeberg.org:dracondev/rust-ai-web-auto.git (fetch)
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/rust-ai-web-auto.git.txt:6:codeberg	git@codeberg.org:dracondev/rust-ai-web-auto.git (push)
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/rust-ai-web-auto.git.txt:7:github	git@github.com:DraconDev/rust-ai-web-auto.git (fetch)
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/rust-ai-web-auto.git.txt:8:github	git@github.com:DraconDev/rust-ai-web-auto.git (push)
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/rust-ai-web-auto.git.txt:9:gitlab	git@gitlab.com:dracondev/rust-ai-web-auto.git (fetch)
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/rust-ai-web-auto.git.txt:10:gitlab	git@gitlab.com:dracondev/rust-ai-web-auto.git (push)
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/rust-ai-web-auto.git.txt:11:origin	https://github.com/DraconDev/rust-ai-web-auto.git (fetch)
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/rust-ai-web-auto.git.txt:12:origin	https://github.com/DraconDev/rust-ai-web-auto.git (push)
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/rust-ai-web-auto.git.txt:14:* main 3c8763a [origin/main] 2 file(s) in scripts [scripts/README.md, README.md] DELTA:+23/-1
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/rust-ai-web-auto.git.txt:18:996b4ac docs(audit): document Dracon AI lib adoption + Section 7/8/9/10 renumbering
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/rust-ai-web-auto.git.txt:19:bede3bb docs: add Dracon AI lib section to README
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/rust-ai-web-auto.git.txt:20:3a55f5a 2 file(s) in examples,src [examples/dracon_ai_smoke.rs, src/env_keys.rs] DELTA:+12/-7
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.avid.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.avid.git.txt:14:* main                                     8d1f698 [origin/main] 19 file(s) in examples,src,tests [src/cli.rs, src/ai_gen.rs, src/analyzer.rs] DELTA:+1034/-453 | TEST:81
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.avid.git.txt:16:8d1f698 19 file(s) in examples,src,tests [src/cli.rs, src/ai_gen.rs, src/analyzer.rs] DELTA:+1034/-453 | TEST:81
docs/audit/2026-06-11-full-repo-audit/per-repo/kiki-sassy-desktop-announcer.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/per-repo/kiki-sassy-desktop-announcer.git.txt:13:* main 0155632 [origin/main] 2 file(s) in src [src/journal.rs, src/daemon.rs] DELTA:+2/-4
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.video-uploader.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.video-uploader.git.txt:13:* main 9d5e9f1 [origin/main] 2 file(s) in youtube-uploader-cli [youtube-uploader-cli/tests/cli.rs, youtube-uploader-cli/src/main.rs] DELTA:+5/-3 | TEST:6
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.video-uploader.git.txt:15:9d5e9f1 2 file(s) in youtube-uploader-cli [youtube-uploader-cli/tests/cli.rs, youtube-uploader-cli/src/main.rs] DELTA:+5/-3 | TEST:6
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.video-uploader.git.txt:17:b630d5f 4 file(s) in youtube-uploader,youtube-uploader-cli [youtube-uploader/src/youtube.rs, youtube-uploader/src/config.rs, youtube-uploader-cli/src/main.rs] DELTA:+80/-40
docs/audit/2026-06-11-full-repo-audit/per-repo/folder-auto-banner.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/per-repo/folder-auto-banner.git.txt:13:* main 3c51eb9 [origin/main] 4 file(s) in src [src/port_usage/mod.rs, src/project_insights.rs, src/git/mod.rs] DELTA:+90/-5
docs/audit/2026-06-11-full-repo-audit/per-repo/dracon-libs.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/per-repo/dracon-libs.git.txt:13:* main 2ff017b [origin/main] 1 file(s) [deny.toml] DELTA:+2/-0
docs/audit/2026-06-11-full-repo-audit/per-repo/video-uploader.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/per-repo/video-uploader.git.txt:13:* main a084400 [origin/main] Merge https://github.com/DraconDev/video-uploader
docs/audit/2026-06-11-full-repo-audit/per-repo/video-uploader.git.txt:16:b630d5f 4 file(s) in youtube-uploader,youtube-uploader-cli [youtube-uploader/src/youtube.rs, youtube-uploader/src/config.rs, youtube-uploader-cli/src/main.rs] DELTA:+80/-40
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.ai-auto-repo-rot-scanner-todo-agent.git.txt:1:REPO=/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.ai-auto-repo-rot-scanner-todo-agent.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.ai-auto-repo-rot-scanner-todo-agent.git.txt:4:codeberg	git@codeberg.org:dracondev/ai-auto-repo-rot-scanner-todo-agent.git (fetch)
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.ai-auto-repo-rot-scanner-todo-agent.git.txt:5:codeberg	git@codeberg.org:dracondev/ai-auto-repo-rot-scanner-todo-agent.git (push)
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.ai-auto-repo-rot-scanner-todo-agent.git.txt:6:github	git@github.com:DraconDev/ai-auto-repo-rot-scanner-todo-agent.git (fetch)
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.ai-auto-repo-rot-scanner-todo-agent.git.txt:7:github	git@github.com:DraconDev/ai-auto-repo-rot-scanner-todo-agent.git (push)
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.ai-auto-repo-rot-scanner-todo-agent.git.txt:8:gitlab	git@gitlab.com:dracondev/ai-auto-repo-rot-scanner-todo-agent.git (fetch)
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.ai-auto-repo-rot-scanner-todo-agent.git.txt:9:gitlab	git@gitlab.com:dracondev/ai-auto-repo-rot-scanner-todo-agent.git (push)
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.ai-auto-repo-rot-scanner-todo-agent.git.txt:10:origin	https://github.com/DraconDev/ai-auto-repo-rot-scanner-todo-agent.git (fetch)
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.ai-auto-repo-rot-scanner-todo-agent.git.txt:11:origin	https://github.com/DraconDev/ai-auto-repo-rot-scanner-todo-agent.git (push)
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.ai-auto-repo-rot-scanner-todo-agent.git.txt:13:* main e50effe [origin/main] 1 file(s) in .ralph [.ralph/audit-remediation/.ralph-runner/events.jsonl] DELTA:+0/-90 | DEL:.ralph-runner/events.jsonl
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:16:/home/dracon/Dev/dracon-code	tracked	.ralph/phase3-dracon-ai-extraction.md	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:17:/home/dracon/Dev/dracon-code	tracked	.ralph/phase3-dracon-ai-extraction.state.json	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:26:/home/dracon/Dev/dracon-code	tracked	.ralph/remaining-work.md	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:27:/home/dracon/Dev/dracon-code	tracked	.ralph/remaining-work.state.json	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:51:/home/dracon/Dev/pully-fully-pull-based-fleet-reconciler	tracked	.ralph/analysis/AI-OPS.md	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:108:/home/dracon/Dev/Junk-Runner-bevy	tracked	.pi/goals/notes_dev_failu[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSA3V2x3S3huRXYyMzNBNmlqa1MzditVOUw5QWlpR3UvV0kzd3QzK1JVTENvCnY2MGVsZFlRc1J6Z1JhYkhPOERwdlM5M1NtaW1WZi9CdWFpMmZYaXBzZ3MKLT4gWDI1NTE5ICtFOFowV2ZSd0ZxTmtHcllRVGRTS3VSWjgvK0RMeDVKWWF3RmxCTVJER00KbWw0QnpBbzcxWG8wQ2dPNTdsdnZFM0JEUTgzOUhaZ3ZzWUVnYXFEYWVxMAotPiBYMjU1MTkgZGtJeE1FZjVMRXZjRGFacVlGZUQ2ZmgzbmR6eDA3MkRxdStKVmhZRHFrawo5Y1dkMnI1Z2VtYnVCYStUV3UvMy9JNHRCTEd2TVpORVdGZ3I1cmhQb2ZFCi0+IFgyNTUxOSB0emVBYVozekhOTjBKTjQ0T3FQRnBVYkk0K2szVG1vSzFlUzY1ZkoxeFVNCkltSXBZRGhna2VMZjZxV1gzRGtiVzlybUxPK210My9hNERBcmhWK2ZyLzAKLT4gWDI1NTE5IGpWeGVxaUMrSlVQRDJyRFVCMU11QUV6NXRvOVhROGlwTWJJOFYrNW1nVjgKcDlIcUEzTUNuZE1BUC9EUlA3T3pUdE01Ym5HY2kvbUxIRTBjYlV4SmNVTQotPiBYMjU1MTkgMHU3MnRwZlJiNHVDNWFkU1lqZkI4KzNwUWQya3hJdk91ZjB2eENsMzZHcwoxODg0K3I2aVo0N2J3TkRZUFdkenh0Ukt0MEFWVG9lamppd3dRejhUUGZFCi0+IFgyNTUxOSBneStLOFhQQ0FFdXdTVG9aOHFYMmlhVU9JcGVtRWdaZmxRQUJrMmxlQVFBCktYalZzdVVjK3hydzlUL0p0bEM5cjRqNEhDQTE0VTRmUXZhd0ppdGI3RzQKLT4geW5VcDEtZ3JlYXNlIG9mOWU8cHUgcT94RCBRfUlHIGx8ClFuOFlIYWIrSGw2U3N3SlUzNkRubzY4VjBvQ2ZRbEc5YVlZRFBudi9xMGhYbjJDTHdzVDIvWWMKLS0tIFJUNWwwY3Y1TWZFWDFSZTR0VWNVR1FPamdrZUc1clFLcGdEbHF2MkN2QlEK/PmXXF5tJd9fbxt2aqnFfMzclE0PEiocNXnXkI8gboUS6ubLIXnt4ww7xwmzoaqKNYZGABNX1sYWLLQnHQ==].md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:183:/home/dracon/Dev/Junk-Runner-bevy	tracked	assets/audio/sfx_repair.wav	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:187:/home/dracon/Dev/Junk-Runner-bevy	tracked	assets/audio/sfx_system_failure.wav	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:316:/home/dracon/Dev/one-mil-girls	tracked	.pi/goals/archived/goal_2026060318184883_mpyaiftl-mv1cfy.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:359:/home/dracon/Dev/one-mil-girls	tracked	docs/audit/2026-06-11-full-audit/menu/01-main-menu.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:390:/home/dracon/Dev/one-mil-girls	tracked	docs/audit/visual-qa/2026-06-10-post-inspiration-polish/01-main-menu.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:413:/home/dracon/Dev/one-mil-girls	tracked	docs/audit/visual-qa/after/main-menu.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:417:/home/dracon/Dev/one-mil-girls	tracked	docs/audit/visual-qa/before/main-menu.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:433:/home/dracon/Dev/one-mil-girls	tracked	docs/audit/visual-qa/convo-redesign-before/main-screen.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:435:/home/dracon/Dev/one-mil-girls	tracked	docs/audit/visual-qa/crops/main-menu-center.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:437:/home/dracon/Dev/one-mil-girls	tracked	docs/audit/visual-qa/crops/vn-dialogue-portrait.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:448:/home/dracon/Dev/one-mil-girls	tracked	docs/audit/visual-qa/effects-after/main-screen.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:451:/home/dracon/Dev/one-mil-girls	tracked	docs/audit/visual-qa/effects-before/main-screen.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:638:/home/dracon/Dev/dracon-utilities	tracked	.ralph/cleanup-remaining.md	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:639:/home/dracon/Dev/dracon-utilities	tracked	.ralph/cleanup-remaining.state.json	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:646:/home/dracon/Dev/dracon-utilities	tracked	docs/audit/2026-06-11-full-repo-audit/final/per-repo/ai-auto-repo-rot-scanner-todo-agent.git.txt	blocked-needs-approval	possibly obsolete doc/checklist; needs approval
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:647:/home/dracon/Dev/dracon-utilities	tracked	docs/audit/2026-06-11-full-repo-audit/final/risk-paths/ai-auto-repo-rot-scanner-todo-agent.risk.tsv	blocked-needs-approval	possibly obsolete doc/checklist; needs approval
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:648:/home/dracon/Dev/dracon-utilities	tracked	docs/audit/2026-06-11-full-repo-audit/per-repo/ai-auto-repo-rot-scanner-todo-agent.git.txt	blocked-needs-approval	possibly obsolete doc/checklist; needs approval
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:649:/home/dracon/Dev/dracon-utilities	tracked	docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/ai-auto-repo-rot-scanner-todo-agent.git.txt	blocked-needs-approval	possibly obsolete doc/checklist; needs approval
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:650:/home/dracon/Dev/dracon-utilities	tracked	docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/ai-auto-repo-rot-scanner-todo-agent.git.txt	blocked-needs-approval	possibly obsolete doc/checklist; needs approval
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:651:/home/dracon/Dev/dracon-utilities	tracked	docs/audit/2026-06-11-full-repo-audit/risk-paths/ai-auto-repo-rot-scanner-todo-agent.risk.tsv	blocked-needs-approval	possibly obsolete doc/checklist; needs approval
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:658:/home/dracon/Dev/dracon-utilities	untracked	docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.ai-auto-repo-rot-scanner-todo-agent.git.txt	blocked-needs-approval	possibly obsolete doc/checklist; needs approval
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:659:/home/dracon/Dev/dracon-utilities	untracked	docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/pi-proof/before.ai-auto-repo-rot-scanner-todo-agent.pi-files.txt	blocked-needs-approval	possibly obsolete doc/checklist; needs approval
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:660:/home/dracon/Dev/dracon-utilities	untracked	docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/pi-proof/before.ai-auto-repo-rot-scanner-todo-agent.pi-untracked.txt	blocked-needs-approval	possibly obsolete doc/checklist; needs approval
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:715:/home/dracon/Dev/dracon-platform	tracked	apis/services/ai-api/.env.dev	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:716:/home/dracon/Dev/dracon-platform	tracked	apis/services/ai-api/.env.example	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:717:/home/dracon/Dev/dracon-platform	tracked	apis/services/ai-api/.env.prod	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:718:/home/dracon/Dev/dracon-platform	tracked	apis/services/ai-api/ai-api-sdk/.env.example	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:725:/home/dracon/Dev/dracon-platform	tracked	apis/services/email-api/.env.dev	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:726:/home/dracon/Dev/dracon-platform	tracked	apis/services/email-api/.env.example	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:727:/home/dracon/Dev/dracon-platform	tracked	apis/services/email-api/.env.prod	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:810:/home/dracon/Dev/dracon-platform	tracked	web/games-hosted/games/junk-runner/assets/sfx_repair-CnxSomk4.wav	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:814:/home/dracon/Dev/dracon-platform	tracked	web/games-hosted/games/junk-runner/assets/sfx_system_failure-DlAFpe7h.wav	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:834:/home/dracon/Dev/dracon-platform	tracked	web/screenshots/ai-hub-current/affiliates-fold.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:835:/home/dracon/Dev/dracon-platform	tracked	web/screenshots/ai-hub-current/affiliates.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:836:/home/dracon/Dev/dracon-platform	tracked	web/screenshots/ai-hub-current/compare-fold.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:837:/home/dracon/Dev/dracon-platform	tracked	web/screenshots/ai-hub-current/compare.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:838:/home/dracon/Dev/dracon-platform	tracked	web/screenshots/ai-hub-current/directory-fold.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:839:/home/dracon/Dev/dracon-platform	tracked	web/screenshots/ai-hub-current/directory.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:840:/home/dracon/Dev/dracon-platform	tracked	web/screenshots/ai-hub-current/free-fold.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:841:/home/dracon/Dev/dracon-platform	tracked	web/screenshots/ai-hub-current/free.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:842:/home/dracon/Dev/dracon-platform	tracked	web/screenshots/ai-hub-current/plans-fold-800.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:843:/home/dracon/Dev/dracon-platform	tracked	web/screenshots/ai-hub-current/plans-fold-final.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:844:/home/dracon/Dev/dracon-platform	tracked	web/screenshots/ai-hub-current/plans-fold.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:845:/home/dracon/Dev/dracon-platform	tracked	web/screenshots/ai-hub-current/plans-fullpage.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:846:/home/dracon/Dev/dracon-platform	tracked	web/screenshots/ai-hub-current/plans-intro.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:847:/home/dracon/Dev/dracon-platform	tracked	web/screenshots/ai-hub-current/plans-narrow.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:848:/home/dracon/Dev/dracon-platform	tracked	web/screenshots/ai-hub-current/plans-redbox-absolute.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:849:/home/dracon/Dev/dracon-platform	tracked	web/screenshots/ai-hub-current/plans-scrolled.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:850:/home/dracon/Dev/dracon-platform	tracked	web/screenshots/ai-hub-current/plans-top-400.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:851:/home/dracon/Dev/dracon-platform	tracked	web/screenshots/ai-hub-current/plans-top-debug.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:852:/home/dracon/Dev/dracon-platform	tracked	web/screenshots/ai-hub-current/plans-with-redbox.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:853:/home/dracon/Dev/dracon-platform	tracked	web/screenshots/ai-hub-current/plans.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:854:/home/dracon/Dev/dracon-platform	tracked	web/screenshots/ai-hub-current/promos-fold.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:855:/home/dracon/Dev/dracon-platform	tracked	web/screenshots/ai-hub-current/promos.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:856:/home/dracon/Dev/dracon-platform	tracked	web/screenshots/ai-hub-current/providers-fold.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:857:/home/dracon/Dev/dracon-platform	tracked	web/screenshots/ai-hub-current/providers.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:858:/home/dracon/Dev/dracon-platform	tracked	web/screenshots/ai-hub-current/rankings-fold.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:859:/home/dracon/Dev/dracon-platform	tracked	web/screenshots/ai-hub-current/rankings.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:860:/home/dracon/Dev/dracon-platform	tracked	web/screenshots/chrome-consistency/ai-hub-affiliates-signed-in.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:861:/home/dracon/Dev/dracon-platform	tracked	web/screenshots/chrome-consistency/ai-hub-affiliates-signed-out.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:862:/home/dracon/Dev/dracon-platform	tracked	web/screenshots/chrome-consistency/ai-hub-compare-signed-in.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:863:/home/dracon/Dev/dracon-platform	tracked	web/screenshots/chrome-consistency/ai-hub-compare-signed-out.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:864:/home/dracon/Dev/dracon-platform	tracked	web/screenshots/chrome-consistency/ai-hub-free-signed-in.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:865:/home/dracon/Dev/dracon-platform	tracked	web/screenshots/chrome-consistency/ai-hub-free-signed-out.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:866:/home/dracon/Dev/dracon-platform	tracked	web/screenshots/chrome-consistency/ai-hub-index-v2.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:867:/home/dracon/Dev/dracon-platform	tracked	web/screenshots/chrome-consistency/ai-hub-plans-signed-in.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:868:/home/dracon/Dev/dracon-platform	tracked	web/screenshots/chrome-consistency/ai-hub-plans-signed-out.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:869:/home/dracon/Dev/dracon-platform	tracked	web/screenshots/chrome-consistency/ai-hub-promos-signed-in.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:870:/home/dracon/Dev/dracon-platform	tracked	web/screenshots/chrome-consistency/ai-hub-promos-signed-out.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:871:/home/dracon/Dev/dracon-platform	tracked	web/screenshots/chrome-consistency/ai-hub-providers-signed-in.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:872:/home/dracon/Dev/dracon-platform	tracked	web/screenshots/chrome-consistency/ai-hub-providers-signed-out.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:873:/home/dracon/Dev/dracon-platform	tracked	web/screenshots/chrome-consistency/ai-hub-signed-in.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:874:/home/dracon/Dev/dracon-platform	tracked	web/screenshots/chrome-consistency/ai-hub-signed-out.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:875:/home/dracon/Dev/dracon-platform	tracked	web/screenshots/chrome-consistency/auth-login-check-email-signed-in.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:876:/home/dracon/Dev/dracon-platform	tracked	web/screenshots/chrome-consistency/auth-login-check-email-signed-out.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:922:/home/dracon/Dev/dracon-platform	tracked	web/screenshots/chrome-fixes/desktop-ai-hub-plans.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:928:/home/dracon/Dev/dracon-platform	tracked	web/screenshots/chrome-fixes/mobile-ai-hub-plans.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:932:/home/dracon/Dev/dracon-platform	tracked	web/screenshots/layout-fix-final/desktop-signed-in-ai-hub.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:933:/home/dracon/Dev/dracon-platform	tracked	web/screenshots/layout-fix-final/desktop-signed-out-ai-hub.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:934:/home/dracon/Dev/dracon-platform	tracked	web/screenshots/layout-fix-final/mobile-ai-hub-closed.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:935:/home/dracon/Dev/dracon-platform	tracked	web/screenshots/layout-fix-final/mobile-ai-hub-drawer-open.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:947:/home/dracon/Dev/dracon-platform	tracked	web/tests/shared/visual-snapshots.spec.ts-snapshots/ai-hub-compare-desktop-shared-linux.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:948:/home/dracon/Dev/dracon-platform	tracked	web/tests/shared/visual-snapshots.spec.ts-snapshots/ai-hub-directory-mobile-shared-linux.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:949:/home/dracon/Dev/dracon-platform	tracked	web/tests/shared/visual-snapshots.spec.ts-snapshots/ai-hub-directory-models-desktop-shared-linux.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:950:/home/dracon/Dev/dracon-platform	tracked	web/tests/shared/visual-snapshots.spec.ts-snapshots/ai-hub-directory-providers-desktop-shared-linux.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:951:/home/dracon/Dev/dracon-platform	tracked	web/tests/shared/visual-snapshots.spec.ts-snapshots/ai-hub-directory-vouchers-desktop-shared-linux.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:952:/home/dracon/Dev/dracon-platform	tracked	web/tests/shared/visual-snapshots.spec.ts-snapshots/ai-hub-plans-desktop-shared-linux.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:953:/home/dracon/Dev/dracon-platform	tracked	web/tests/shared/visual-snapshots.spec.ts-snapshots/ai-hub-provider-groq-desktop-shared-linux.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:954:/home/dracon/Dev/dracon-platform	tracked	web/tests/shared/visual-snapshots.spec.ts-snapshots/ai-hub-rankings-desktop-shared-linux.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:955:/home/dracon/Dev/dracon-platform	tracked	web/tests/shared/visual-snapshots.spec.ts-snapshots/ai-hub-rankings-mobile-shared-linux.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:956:/home/dracon/Dev/dracon-platform	tracked	web/tests/shared/visual-snapshots.spec.ts-snapshots/dashboard-check-email-desktop-shared-linux.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:960:/home/dracon/Dev/dracon-platform	tracked	web/web/test-results/ui-audit-recon/symptom-2-ai-hub.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:963:/home/dracon/Dev/dracon-platform	tracked	web/web/test-results/ui-audit/ai-hub-compare-desktop-1440.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:964:/home/dracon/Dev/dracon-platform	tracked	web/web/test-results/ui-audit/ai-hub-compare-mobile-375.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:965:/home/dracon/Dev/dracon-platform	tracked	web/web/test-results/ui-audit/ai-hub-desktop-1440.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:966:/home/dracon/Dev/dracon-platform	tracked	web/web/test-results/ui-audit/ai-hub-free-desktop-1440.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:967:/home/dracon/Dev/dracon-platform	tracked	web/web/test-results/ui-audit/ai-hub-free-mobile-375.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:968:/home/dracon/Dev/dracon-platform	tracked	web/web/test-results/ui-audit/ai-hub-mobile-375.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:969:/home/dracon/Dev/dracon-platform	tracked	web/web/test-results/ui-audit/ai-hub-plans-desktop-1440.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:970:/home/dracon/Dev/dracon-platform	tracked	web/web/test-results/ui-audit/ai-hub-plans-mobile-375.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:971:/home/dracon/Dev/dracon-platform	tracked	web/web/test-results/ui-audit/ai-hub-promos-desktop-1440.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:972:/home/dracon/Dev/dracon-platform	tracked	web/web/test-results/ui-audit/ai-hub-promos-mobile-375.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:973:/home/dracon/Dev/dracon-platform	tracked	web/web/test-results/ui-audit/ai-hub-providers-desktop-1440.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:974:/home/dracon/Dev/dracon-platform	tracked	web/web/test-results/ui-audit/ai-hub-providers-mobile-375.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:991:/home/dracon/Dev/dracon-platform	tracked	web/web/test-results/ui-audit/verify-ai-hub-landing-desktop-1440.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:992:/home/dracon/Dev/dracon-platform	tracked	web/web/test-results/ui-audit/verify-ai-hub-landing-mobile-375.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1000:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/.env	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1001:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/.env.example	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1002:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/.env.production	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1003:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/ai-job-finder/.ralph/loop-todos.md	blocked-needs-approval	possibly obsolete doc/checklist; needs approval
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1004:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/ai-job-finder/.ralph/loop-todos.state.json	blocked-needs-approval	possibly obsolete doc/checklist; needs approval
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1005:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/ai-job-finder/TEST-CHECKLIST.md	blocked-needs-approval	possibly obsolete doc/checklist; needs approval
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1006:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/ai-job-finder/TODO.md	blocked-needs-approval	possibly obsolete doc/checklist; needs approval
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1007:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/ai-job-finder/public/icon/128.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1008:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/ai-job-finder/public/icon/16.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1009:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/ai-job-finder/public/icon/32.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1010:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/ai-job-finder/public/icon/48.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1011:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/ai-job-finder/public/icon/96.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1012:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/ai-job-finder/server/.env.example	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1013:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/ai-job-finder/src/lib/styles/tokens.css	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1014:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/assets/unnamed (1).png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1015:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/assets/unnamed (2).png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1016:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/assets/unnamed (3).png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1017:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/assets/unnamed (4).png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1018:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/assets/unnamed.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1019:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/coverage/base.css	cleaned	generated coverage artifact
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1020:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/coverage/block-navigation.js	cleaned	generated coverage artifact
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1021:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/coverage/clover.xml	cleaned	generated coverage artifact
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1022:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/coverage/coverage-final.json	cleaned	generated coverage artifact
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1023:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/coverage/entrypoints/onboarding/App.tsx.html	cleaned	generated coverage artifact
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1024:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/coverage/entrypoints/onboarding/index.html	cleaned	generated coverage artifact
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1025:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/coverage/entrypoints/settings/App.tsx.html	cleaned	generated coverage artifact
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1026:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/coverage/entrypoints/settings/index.html	cleaned	generated coverage artifact
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1027:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/coverage/favicon.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1028:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/coverage/index.html	cleaned	generated coverage artifact
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1029:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/coverage/prettify.css	cleaned	generated coverage artifact
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1030:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/coverage/prettify.js	cleaned	generated coverage artifact
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1031:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/coverage/services/background/handlers/ai.ts.html	cleaned	generated coverage artifact
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1032:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/coverage/services/background/handlers/index.html	cleaned	generated coverage artifact
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1033:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/coverage/services/background/handlers/navigation.ts.html	cleaned	generated coverage artifact
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1034:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/coverage/services/background/index.html	cleaned	generated coverage artifact
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1035:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/coverage/services/background/messageHandlers.ts.html	cleaned	generated coverage artifact
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1036:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/coverage/sort-arrow-sprite.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1037:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/coverage/sorter.js	cleaned	generated coverage artifact
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1038:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/coverage/src/content/SearchPanel.tsx.html	cleaned	generated coverage artifact
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1039:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/coverage/src/content/index.html	cleaned	generated coverage artifact
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1040:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/coverage/test/__mocks__/@dracon/wxt-shared/byok.ts.html	cleaned	generated coverage artifact
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1041:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/coverage/test/__mocks__/@dracon/wxt-shared/index.html	cleaned	generated coverage artifact
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1042:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/coverage/utils/api.ts.html	cleaned	generated coverage artifact
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1043:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/coverage/utils/autoFormFiller.ts.html	cleaned	generated coverage artifact
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1044:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/coverage/utils/byokStore.ts.html	cleaned	generated coverage artifact
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1045:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/coverage/utils/codeFetcher.ts.html	cleaned	generated coverage artifact
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1046:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/coverage/utils/debug.ts.html	cleaned	generated coverage artifact
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1047:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/coverage/utils/formFiller/aiGenerator.ts.html	cleaned	generated coverage artifact
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1048:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/coverage/utils/formFiller/analyzer.ts.html	cleaned	generated coverage artifact
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1049:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/coverage/utils/formFiller/filler.ts.html	cleaned	generated coverage artifact
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1050:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/coverage/utils/formFiller/index.html	cleaned	generated coverage artifact
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1051:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/coverage/utils/formFiller/profileMapper.ts.html	cleaned	generated coverage artifact
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1052:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/coverage/utils/formProfileTemplates.json.html	cleaned	generated coverage artifact
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1053:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/coverage/utils/gmail/index.html	cleaned	generated coverage artifact
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1054:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/coverage/utils/gmail/index.ts.html	cleaned	generated coverage artifact
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1055:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/coverage/utils/index.html	cleaned	generated coverage artifact
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1056:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/coverage/utils/optionGenerator.ts.html	cleaned	generated coverage artifact
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1057:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/coverage/utils/otp/index.html	cleaned	generated coverage artifact
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1058:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/coverage/utils/otp/index.ts.html	cleaned	generated coverage artifact
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1059:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/coverage/utils/simpleFormProfiles.ts.html	cleaned	generated coverage artifact
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1060:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/coverage/utils/store.ts.html	cleaned	generated coverage artifact
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1061:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/coverage/utils/text.ts.html	cleaned	generated coverage artifact
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1062:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/docs/assets/screenshots/1.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1063:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/docs/assets/screenshots/2.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1064:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/docs/assets/screenshots/3.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1065:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/docs/assets/screenshots/4.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1066:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/docs/assets/screenshots/5.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1067:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/docs/assets/screenshots/SCREENSHOTS.md	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1068:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/docs/assets/screenshots/t1.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1069:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/public/1.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1070:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/public/2.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1071:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/public/3.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1072:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/public/4.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1073:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/public/440x280.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1074:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/public/5.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1075:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/public/icon/128.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1076:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/public/icon/16.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1077:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/public/icon/32.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1078:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/public/icon/48.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1079:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/public/icon/96.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1080:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/src/content/SearchPanel/components/NotesTab.tsx	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1081:/home/dracon/Dev/browser-extensions-shared	tracked	SamAI/utils/notes.ts	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1082:/home/dracon/Dev/browser-extensions-shared	tracked	ai-ats/.env	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1083:/home/dracon/Dev/browser-extensions-shared	tracked	ai-ats/.env.example	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1084:/home/dracon/Dev/browser-extensions-shared	tracked	ai-ats/CHECKLIST.md	blocked-needs-approval	possibly obsolete doc/checklist; needs approval
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1085:/home/dracon/Dev/browser-extensions-shared	tracked	ai-ats/public/icon/128.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1086:/home/dracon/Dev/browser-extensions-shared	tracked	ai-ats/public/icon/16.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1087:/home/dracon/Dev/browser-extensions-shared	tracked	ai-ats/public/icon/32.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1088:/home/dracon/Dev/browser-extensions-shared	tracked	ai-ats/public/icon/48.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1089:/home/dracon/Dev/browser-extensions-shared	tracked	ai-ats/public/icon/96.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1142:/home/dracon/Dev/browser-extensions-shared	tracked	auto-form-filler/.audit-ui/aria-check2/Default/Local Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/CURRENT	cleaned	browser profile/cache/history/local-storage data outside .pi
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1143:/home/dracon/Dev/browser-extensions-shared	tracked	auto-form-filler/.audit-ui/aria-check2/Default/Local Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOCK	cleaned	browser profile/cache/history/local-storage data outside .pi
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1144:/home/dracon/Dev/browser-extensions-shared	tracked	auto-form-filler/.audit-ui/aria-check2/Default/Local Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOG	cleaned	browser profile/cache/history/local-storage data outside .pi
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1145:/home/dracon/Dev/browser-extensions-shared	tracked	auto-form-filler/.audit-ui/aria-check2/Default/Local Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/MANIFEST-000001	cleaned	browser profile/cache/history/local-storage data outside .pi
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1171:/home/dracon/Dev/browser-extensions-shared	tracked	auto-form-filler/.audit-ui/aria-check2/Default/Sync Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/CURRENT	cleaned	browser profile/cache/history/local-storage data outside .pi
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1172:/home/dracon/Dev/browser-extensions-shared	tracked	auto-form-filler/.audit-ui/aria-check2/Default/Sync Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOCK	cleaned	browser profile/cache/history/local-storage data outside .pi
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1173:/home/dracon/Dev/browser-extensions-shared	tracked	auto-form-filler/.audit-ui/aria-check2/Default/Sync Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOG	cleaned	browser profile/cache/history/local-storage data outside .pi
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1174:/home/dracon/Dev/browser-extensions-shared	tracked	auto-form-filler/.audit-ui/aria-check2/Default/Sync Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/MANIFEST-000001	cleaned	browser profile/cache/history/local-storage data outside .pi
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1229:/home/dracon/Dev/browser-extensions-shared	tracked	auto-form-filler/.audit-ui/aria-test2/Default/Local Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/CURRENT	cleaned	browser profile/cache/history/local-storage data outside .pi
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1230:/home/dracon/Dev/browser-extensions-shared	tracked	auto-form-filler/.audit-ui/aria-test2/Default/Local Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOCK	cleaned	browser profile/cache/history/local-storage data outside .pi
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1231:/home/dracon/Dev/browser-extensions-shared	tracked	auto-form-filler/.audit-ui/aria-test2/Default/Local Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOG	cleaned	browser profile/cache/history/local-storage data outside .pi
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1232:/home/dracon/Dev/browser-extensions-shared	tracked	auto-form-filler/.audit-ui/aria-test2/Default/Local Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/MANIFEST-000001	cleaned	browser profile/cache/history/local-storage data outside .pi
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1258:/home/dracon/Dev/browser-extensions-shared	tracked	auto-form-filler/.audit-ui/aria-test2/Default/Sync Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/CURRENT	cleaned	browser profile/cache/history/local-storage data outside .pi
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1259:/home/dracon/Dev/browser-extensions-shared	tracked	auto-form-filler/.audit-ui/aria-test2/Default/Sync Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOCK	cleaned	browser profile/cache/history/local-storage data outside .pi
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1260:/home/dracon/Dev/browser-extensions-shared	tracked	auto-form-filler/.audit-ui/aria-test2/Default/Sync Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOG	cleaned	browser profile/cache/history/local-storage data outside .pi
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1261:/home/dracon/Dev/browser-extensions-shared	tracked	auto-form-filler/.audit-ui/aria-test2/Default/Sync Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/MANIFEST-000001	cleaned	browser profile/cache/history/local-storage data outside .pi
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1277:/home/dracon/Dev/browser-extensions-shared	tracked	auto-form-filler/.audit-ui/dd-debug/Default/AutofillAiModelCache/LOCK	cleaned	browser profile/cache/history/local-storage data outside .pi
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1278:/home/dracon/Dev/browser-extensions-shared	tracked	auto-form-filler/.audit-ui/dd-debug/Default/AutofillAiModelCache/LOG	cleaned	browser profile/cache/history/local-storage data outside .pi
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1322:/home/dracon/Dev/browser-extensions-shared	tracked	auto-form-filler/.audit-ui/dd-debug/Default/Local Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/CURRENT	cleaned	browser profile/cache/history/local-storage data outside .pi
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1323:/home/dracon/Dev/browser-extensions-shared	tracked	auto-form-filler/.audit-ui/dd-debug/Default/Local Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOCK	cleaned	browser profile/cache/history/local-storage data outside .pi
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1324:/home/dracon/Dev/browser-extensions-shared	tracked	auto-form-filler/.audit-ui/dd-debug/Default/Local Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOG	cleaned	browser profile/cache/history/local-storage data outside .pi
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1325:/home/dracon/Dev/browser-extensions-shared	tracked	auto-form-filler/.audit-ui/dd-debug/Default/Local Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/MANIFEST-000001	cleaned	browser profile/cache/history/local-storage data outside .pi
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1350:/home/dracon/Dev/browser-extensions-shared	tracked	auto-form-filler/.audit-ui/dd-debug/Default/Sync Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/CURRENT	cleaned	browser profile/cache/history/local-storage data outside .pi
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1351:/home/dracon/Dev/browser-extensions-shared	tracked	auto-form-filler/.audit-ui/dd-debug/Default/Sync Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOCK	cleaned	browser profile/cache/history/local-storage data outside .pi
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1352:/home/dracon/Dev/browser-extensions-shared	tracked	auto-form-filler/.audit-ui/dd-debug/Default/Sync Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOG	cleaned	browser profile/cache/history/local-storage data outside .pi
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1353:/home/dracon/Dev/browser-extensions-shared	tracked	auto-form-filler/.audit-ui/dd-debug/Default/Sync Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/MANIFEST-000001	cleaned	browser profile/cache/history/local-storage data outside .pi
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1414:/home/dracon/Dev/browser-extensions-shared	tracked	auto-form-filler/.audit-ui/popup-check/Default/Local Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/CURRENT	cleaned	browser profile/cache/history/local-storage data outside .pi
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1415:/home/dracon/Dev/browser-extensions-shared	tracked	auto-form-filler/.audit-ui/popup-check/Default/Local Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOCK	cleaned	browser profile/cache/history/local-storage data outside .pi
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1416:/home/dracon/Dev/browser-extensions-shared	tracked	auto-form-filler/.audit-ui/popup-check/Default/Local Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOG	cleaned	browser profile/cache/history/local-storage data outside .pi
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1417:/home/dracon/Dev/browser-extensions-shared	tracked	auto-form-filler/.audit-ui/popup-check/Default/Local Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/MANIFEST-000001	cleaned	browser profile/cache/history/local-storage data outside .pi
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1443:/home/dracon/Dev/browser-extensions-shared	tracked	auto-form-filler/.audit-ui/popup-check/Default/Sync Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/CURRENT	cleaned	browser profile/cache/history/local-storage data outside .pi
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1444:/home/dracon/Dev/browser-extensions-shared	tracked	auto-form-filler/.audit-ui/popup-check/Default/Sync Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOCK	cleaned	browser profile/cache/history/local-storage data outside .pi
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1445:/home/dracon/Dev/browser-extensions-shared	tracked	auto-form-filler/.audit-ui/popup-check/Default/Sync Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOG	cleaned	browser profile/cache/history/local-storage data outside .pi
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1446:/home/dracon/Dev/browser-extensions-shared	tracked	auto-form-filler/.audit-ui/popup-check/Default/Sync Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/MANIFEST-000001	cleaned	browser profile/cache/history/local-storage data outside .pi
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1595:/home/dracon/Dev/browser-extensions-shared	tracked	death-note-typing-practice/tests/e2e/screenshots/main-menu-after-pause.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1620:/home/dracon/Dev/browser-extensions-shared	tracked	full-page-screenshot/EXTENSION_CONSTRAINTS.md	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1629:/home/dracon/Dev/browser-extensions-shared	tracked	full-page-screenshot/entrypoints/editor/main.tsx	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1633:/home/dracon/Dev/browser-extensions-shared	tracked	full-page-screenshot/entrypoints/popup/main.tsx	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1647:/home/dracon/Dev/browser-extensions-shared	tracked	full-page-screenshot/tailwind.config.cjs	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1686:/home/dracon/Dev/browser-extensions-shared	tracked	live-reload-pro/references/jnihajbhpnppcggbcgedagnkighmdlei/IconUnavailable.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1687:/home/dracon/Dev/browser-extensions-shared	tracked	live-reload-pro/references/jnihajbhpnppcggbcgedagnkighmdlei/IconUnavailable@2x.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1752:/home/dracon/Dev/rust-ai-web-auto	tracked	.env	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1753:/home/dracon/Dev/rust-ai-web-auto	tracked	.env.example	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1754:/home/dracon/Dev/rust-ai-web-auto	tracked	.pi/goals/active_goal_2026060902473467_mq5zckgi-e991vb.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1755:/home/dracon/Dev/rust-ai-web-auto	tracked	.pi/goals/archived/goal_2026053120540756_mpu74jlb-xnkw0s.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1756:/home/dracon/Dev/rust-ai-web-auto	tracked	.pi/goals/archived/goal_2026053121482425_mpu8fceo-y34bei.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1757:/home/dracon/Dev/rust-ai-web-auto	tracked	.pi/goals/archived/goal_2026060112273640_mpuhcnui-0vekoi.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1758:/home/dracon/Dev/rust-ai-web-auto	tracked	.pi/goals/archived/goal_2026060112440792_mpv4x744-pzio9k.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1759:/home/dracon/Dev/rust-ai-web-auto	tracked	.pi/goals/archived/goal_2026060113032381_mpv5ggun-xtdwwk.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1760:/home/dracon/Dev/rust-ai-web-auto	tracked	.pi/goals/archived/goal_2026060121391978_mpvcligm-fqxn4f.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1761:/home/dracon/Dev/rust-ai-web-auto	tracked	.pi/goals/archived/goal_2026060122160765_mpvpcsaz-9qgvsy.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1762:/home/dracon/Dev/rust-ai-web-auto	tracked	.pi/goals/archived/goal_2026060210521488_mpvrkyyj-3mj7k3.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1763:/home/dracon/Dev/rust-ai-web-auto	tracked	.pi/goals/archived/goal_2026060211154584_mpwh9yyu-8pa0cb.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1764:/home/dracon/Dev/rust-ai-web-auto	tracked	.pi/goals/archived/goal_2026060218522168_mpwwpvar-i3bl4e.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1765:/home/dracon/Dev/rust-ai-web-auto	tracked	.pi/goals/archived/goal_2026060315544992_mpxcywek-os6vu4.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1766:/home/dracon/Dev/rust-ai-web-auto	tracked	.pi/goals/archived/goal_2026060319022612_mpyda6ag-26ehg7.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1767:/home/dracon/Dev/rust-ai-web-auto	tracked	.pi/goals/archived/goal_2026060322303653_mpykrg1c-7asulp.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1768:/home/dracon/Dev/rust-ai-web-auto	tracked	.pi/goals/archived/goal_2026060323214677_mpymjx80-uxsb7s.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1769:/home/dracon/Dev/rust-ai-web-auto	tracked	.pi/goals/archived/goal_2026060416104877_mpzmpfqe-fwxtxy.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1770:/home/dracon/Dev/rust-ai-web-auto	tracked	.pi/goals/archived/goal_2026060419433826_mpztv8mj-ma48re.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1771:/home/dracon/Dev/rust-ai-web-auto	tracked	.pi/goals/archived/goal_2026060420543270_mpzwlu6y-gsbrzs.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1772:/home/dracon/Dev/rust-ai-web-auto	tracked	.pi/goals/archived/goal_2026060423182778_mpzzcame-078ng4.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1773:/home/dracon/Dev/rust-ai-web-auto	tracked	.pi/goals/archived/goal_2026060506412675_mq07s5o6-gf4n7c.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1774:/home/dracon/Dev/rust-ai-web-auto	tracked	.pi/goals/archived/goal_2026060510340949_mq0pj6pu-6j1nvd.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1775:/home/dracon/Dev/rust-ai-web-auto	tracked	.pi/goals/archived/goal_2026060519300590_mq188jp0-87r8fk.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1776:/home/dracon/Dev/rust-ai-web-auto	tracked	.pi/goals/archived/goal_2026060721311870_mq46u3vf-lcdc16.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1777:/home/dracon/Dev/rust-ai-web-auto	tracked	.pi/goals/archived/goal_2026060818284565_mq52w2qo-ezl07k.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1778:/home/dracon/Dev/rust-ai-web-auto	tracked	.pi/goals/archived/goal_2026060823472536_mq5qsqwb-oe2ko1.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1779:/home/dracon/Dev/rust-ai-web-auto	tracked	.pi/goals/archived/goal_2026060900211422_mq5tfift-qv13vs.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1780:/home/dracon/Dev/rust-ai-web-auto	tracked	.pi/goals/archived/goal_2026060901384746_mq5wmbrh-akrbt4.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1781:/home/dracon/Dev/rust-ai-web-auto	tracked	MEGA_CHECKLIST.md	blocked-needs-approval	possibly obsolete doc/checklist; needs approval
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1782:/home/dracon/Dev/rust-ai-web-auto	tracked	click_chain_1780538519.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1783:/home/dracon/Dev/rust-ai-web-auto	tracked	click_chain_1780538550.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1784:/home/dracon/Dev/rust-ai-web-auto	tracked	click_chain_1780539957.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1785:/home/dracon/Dev/rust-ai-web-auto	tracked	docs/API_KEYS.md	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1786:/home/dracon/Dev/rust-ai-web-auto	tracked	docs/automation-master-checklist.md	blocked-needs-approval	possibly obsolete doc/checklist; needs approval
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1787:/home/dracon/Dev/rust-ai-web-auto	tracked	extension/icons/icon128.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1788:/home/dracon/Dev/rust-ai-web-auto	tracked	extension/icons/icon48.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1789:/home/dracon/Dev/rust-ai-web-auto	tracked	scripts/screenshot_only.rs	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1790:/home/dracon/Dev/rust-ai-web-auto	tracked	scroll_test_1780538508.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1791:/home/dracon/Dev/rust-ai-web-auto	tracked	scroll_test_1780598187.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1792:/home/dracon/Dev/rust-ai-web-auto	tracked	scroll_test_1780598299.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1793:/home/dracon/Dev/rust-ai-web-auto	tracked	scroll_test_1780602379.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1794:/home/dracon/Dev/rust-ai-web-auto	tracked	scroll_test_1781082862.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1795:/home/dracon/.dracon	tracked	data/ai/secrets/azure.env	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1796:/home/dracon/.dracon	tracked	data/ai/secrets/modal.env	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1797:/home/dracon/.dracon	tracked	data/ai/secrets/nvidia.env	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1798:/home/dracon/.dracon	tracked	data/ai/secrets/openrouter.env	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1799:/home/dracon/.dracon	tracked	data/ai/secrets/zai.env	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1801:/home/dracon/.dracon	tracked	secrets/ai/minimax.env	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1822:/home/dracon/.dracon	tracked	secrets/ssh/legacy_archive/main1.key	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1823:/home/dracon/.dracon	tracked	secrets/ssh/legacy_archive/main1.key.bak	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1824:/home/dracon/.dracon	tracked	secrets/ssh/legacy_archive/main1.pub.bak	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1832:/home/dracon/.dracon	tracked	secrets/ssh/main1.key	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1833:/home/dracon/.dracon	tracked	secrets/ssh/main1.key.pub	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1842:/home/dracon/.dracon	tracked	utilities/sync/ai/secrets/mistral.env	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1843:/home/dracon/.dracon	tracked	utilities/sync/ai/secrets/nvidia.env	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1844:/home/dracon/.dracon	tracked	utilities/sync/ai/secrets/openrouter.env	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1851:/home/dracon/Dev/dracon-ai-lib	tracked	.env	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1852:/home/dracon/Dev/dracon-ai-lib	tracked	.env.example	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1853:/home/dracon/Dev/dracon-ai-lib	tracked	.pi/goals/archived/goal_2026053121495992_mpu8240y-aobl2r.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1854:/home/dracon/Dev/dracon-ai-lib	tracked	.pi/goals/archived/goal_2026060102321437_mpugtb4f-82ryaq.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1855:/home/dracon/Dev/dracon-ai-lib	tracked	.pi/goals/archived/goal_2026060121193146_mpv56bb9-1tm2an.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1856:/home/dracon/Dev/dracon-ai-lib	tracked	.pi/goals/archived/goal_2026060123165134_mpvrmjhm-pknz4m.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1857:/home/dracon/Dev/dracon-ai-lib	tracked	.pi/goals/archived/goal_2026060210011760_mpwehb17-9tq3v5.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1858:/home/dracon/Dev/dracon-ai-lib	tracked	.pi/goals/archived/goal_2026060210385994_mpwfm8m2-o16qqa.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1859:/home/dracon/Dev/dracon-ai-lib	tracked	.pi/goals/archived/goal_2026060211120567_mpwgv778-gfdgir.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1860:/home/dracon/Dev/dracon-ai-lib	tracked	.pi/goals/archived/goal_2026060212521240_mpwkld79-u6b4c0.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1861:/home/dracon/Dev/dracon-ai-lib	tracked	.pi/goals/archived/goal_2026060214212564_mpwm8hys-a2ky77.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1862:/home/dracon/Dev/dracon-ai-lib	tracked	.pi/goals/archived/goal_2026060215052605_mpwoi3iu-3qlf61.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1863:/home/dracon/Dev/dracon-ai-lib	tracked	.pi/goals/archived/goal_2026060217222514_mpwu7sxu-c5e4um.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1864:/home/dracon/Dev/dracon-ai-lib	tracked	.pi/goals/archived/goal_2026060218052396_mpwvp55w-mf378w.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1865:/home/dracon/Dev/dracon-ai-lib	tracked	.pi/goals/archived/goal_2026060317495547_mpy5dihe-174b31.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1866:/home/dracon/Dev/dracon-ai-lib	tracked	.pi/goals/archived/goal_2026060415231066_mpzi21xo-6k45hv.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1867:/home/dracon/Dev/dracon-ai-lib	tracked	.pi/goals/archived/goal_2026060416554220_mpzm5wwy-4v5mkd.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1868:/home/dracon/Dev/dracon-ai-lib	tracked	.pi/goals/archived/goal_2026060420041353_mpzusmll-tkuavh.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1869:/home/dracon/Dev/dracon-ai-lib	tracked	.pi/goals/archived/goal_2026060421392982_mpzy1kjm-7ooplq.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1870:/home/dracon/Dev/dracon-ai-lib	tracked	.pi/goals/archived/goal_2026060422553256_mq00aa8p-q78q3b.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1871:/home/dracon/Dev/dracon-ai-lib	tracked	.pi/goals/archived/goal_2026060500465297_mq052bj9-1hi21x.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1872:/home/dracon/Dev/dracon-ai-lib	tracked	.pi/goals/archived/goal_2026060511014963_mq0qqfj2-7mdgp7.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1873:/home/dracon/Dev/dracon-ai-lib	tracked	.pi/goals/archived/goal_2026060517130651_mq13cixj-o88468.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1874:/home/dracon/Dev/dracon-ai-lib	tracked	.pi/goals/archived/goal_2026060519131616_mq18k8nb-8z83jr.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1875:/home/dracon/Dev/dracon-ai-lib	tracked	.pi/goals/archived/goal_2026060600082972_mq1j5294-ibcxfd.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1876:/home/dracon/Dev/dracon-ai-lib	tracked	.pi/goals/archived/goal_2026060614351979_mq29t6kn-pno183.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1877:/home/dracon/Dev/dracon-ai-lib	tracked	.pi/goals/archived/goal_2026060621301132_mq2jyqty-gznsws.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1878:/home/dracon/Dev/dracon-ai-lib	tracked	.pi/goals/archived/goal_2026060723330028_mq4cpmy6-srv0bj.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1879:/home/dracon/Dev/dracon-ai-lib	tracked	.pi/goals/archived/goal_2026060811353888_mq4gszze-8rib4x.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1880:/home/dracon/Dev/dracon-ai-lib	tracked	.pi/goals/archived/goal_2026060813080905_mq53j6wk-tc1udv.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1881:/home/dracon/Dev/dracon-ai-lib	tracked	.pi/goals/archived/goal_2026060816375771_mq5ckllx-58ktk5.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1882:/home/dracon/Dev/dracon-ai-lib	tracked	.pi/goals/archived/goal_2026060817103628_mq5ebocf-8c0p11.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1883:/home/dracon/Dev/dracon-ai-lib	tracked	.pi/goals/goal_events.jsonl	blocked-by-pi-exclusion	.pi local task state must not be cleaned
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1884:/home/dracon/Dev/dracon-ai-lib	tracked	crates/client/.env	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1909:/home/dracon/Dev/folder-auto-banner	tracked	.pi/goals/archived/goal_2026060520100089_mq1aifn7-6yezq4.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1932:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	tracked	.env.example	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1933:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	tracked	.pi/goals/archived/goal_2026052215594313_mph0t2t2-fih9qt.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1934:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	tracked	.pi/goals/archived/goal_2026060720352138_mq46cla3-g1mrn4.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1935:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	tracked	.pi/goals/archived/goal_2026060910175032_mq5pof3w-ohek53.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1936:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	tracked	.ralph/audit-remediation/.ralph-runner/events.jsonl	cleaned	non-.pi local state directory/file
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1937:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	tracked	.ralph/audit-remediation/.ralph-runner/final-summary.md	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1938:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	tracked	.ralph/audit-remediation/.ralph-runner/status.json	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1939:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	tracked	.ralph/audit-remediation/.ralph-runner/transcripts/iteration-001-7ac8b2d6-2ee3-41f1-b2d1-e1dab486b722.md	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1940:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	tracked	.ralph/audit-remediation/.ralph-runner/transcripts/iteration-001-d634c61c-66f8-46c0-87c3-95f661a2f920.md	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1941:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	tracked	.ralph/audit-remediation/.ralph-runner/transcripts/iteration-002-7ac8b2d6-2ee3-41f1-b2d1-e1dab486b722.md	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1942:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	tracked	.ralph/audit-remediation/.ralph-runner/transcripts/iteration-002-d634c61c-66f8-46c0-87c3-95f661a2f920.md	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1943:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	tracked	.ralph/audit-remediation/.ralph-runner/transcripts/iteration-003-7ac8b2d6-2ee3-41f1-b2d1-e1dab486b722.md	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1944:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	tracked	.ralph/audit-remediation/.ralph-runner/transcripts/iteration-003-d634c61c-66f8-46c0-87c3-95f661a2f920.md	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1945:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	tracked	.ralph/audit-remediation/.ralph-runner/transcripts/iteration-004-7ac8b2d6-2ee3-41f1-b2d1-e1dab486b722.md	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1946:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	tracked	.ralph/audit-remediation/.ralph-runner/transcripts/iteration-004-d634c61c-66f8-46c0-87c3-95f661a2f920.md	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1947:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	tracked	.ralph/audit-remediation/.ralph-runner/transcripts/iteration-005-7ac8b2d6-2ee3-41f1-b2d1-e1dab486b722.md	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1948:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	tracked	.ralph/audit-remediation/.ralph-runner/transcripts/iteration-005-d634c61c-66f8-46c0-87c3-95f661a2f920.md	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1949:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	tracked	.ralph/audit-remediation/.ralph-runner/transcripts/iteration-006-7ac8b2d6-2ee3-41f1-b2d1-e1dab486b722.md	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1950:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	tracked	.ralph/audit-remediation/.ralph-runner/transcripts/iteration-006-d634c61c-66f8-46c0-87c3-95f661a2f920.md	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1951:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	tracked	.ralph/audit-remediation/.ralph-runner/transcripts/iteration-007-7ac8b2d6-2ee3-41f1-b2d1-e1dab486b722.md	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1952:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	tracked	.ralph/audit-remediation/.ralph-runner/transcripts/iteration-007-d634c61c-66f8-46c0-87c3-95f661a2f920.md	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1953:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	tracked	.ralph/audit-remediation/.ralph-runner/transcripts/iteration-008-7ac8b2d6-2ee3-41f1-b2d1-e1dab486b722.md	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1954:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	tracked	.ralph/audit-remediation/.ralph-runner/transcripts/iteration-008-d634c61c-66f8-46c0-87c3-95f661a2f920.md	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1955:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	tracked	.ralph/audit-remediation/.ralph-runner/transcripts/iteration-009-7ac8b2d6-2ee3-41f1-b2d1-e1dab486b722.md	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1956:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	tracked	.ralph/audit-remediation/.ralph-runner/transcripts/iteration-009-d634c61c-66f8-46c0-87c3-95f661a2f920.md	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1957:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	tracked	.ralph/audit-remediation/.ralph-runner/transcripts/iteration-010-7ac8b2d6-2ee3-41f1-b2d1-e1dab486b722.md	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1958:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	tracked	.ralph/audit-remediation/.ralph-runner/transcripts/iteration-011-7ac8b2d6-2ee3-41f1-b2d1-e1dab486b722.md	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1959:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	tracked	.ralph/audit-remediation/.ralph-runner/transcripts/iteration-012-7ac8b2d6-2ee3-41f1-b2d1-e1dab486b722.md	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1960:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	tracked	.ralph/audit-remediation/.ralph-runner/transcripts/iteration-013-7ac8b2d6-2ee3-41f1-b2d1-e1dab486b722.md	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1961:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	tracked	.ralph/audit-remediation/.ralph-runner/transcripts/iteration-014-7ac8b2d6-2ee3-41f1-b2d1-e1dab486b722.md	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1962:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	tracked	.ralph/audit-remediation/.ralph-runner/transcripts/iteration-015-7ac8b2d6-2ee3-41f1-b2d1-e1dab486b722.md	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1963:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	tracked	.ralph/audit-remediation/.ralph-runner/transcripts/iteration-016-7ac8b2d6-2ee3-41f1-b2d1-e1dab486b722.md	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1964:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	tracked	.ralph/audit-remediation/.ralph-runner/transcripts/iteration-017-7ac8b2d6-2ee3-41f1-b2d1-e1dab486b722.md	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1965:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	tracked	.ralph/audit-remediation/.ralph-runner/transcripts/iteration-018-7ac8b2d6-2ee3-41f1-b2d1-e1dab486b722.md	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1966:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	tracked	.ralph/audit-remediation/.ralph-runner/transcripts/iteration-019-7ac8b2d6-2ee3-41f1-b2d1-e1dab486b722.md	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1967:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	tracked	.ralph/audit-remediation/.ralph-runner/transcripts/iteration-020-7ac8b2d6-2ee3-41f1-b2d1-e1dab486b722.md	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1968:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	tracked	.ralph/audit-remediation/RALPH.md	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1969:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	tracked	.ralph/audit-remediation/RALPH_PROGRESS.md	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1970:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	tracked	.ralph/docs-cleanup.md	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1971:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	tracked	.ralph/docs-cleanup.state.json	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1972:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	tracked	.ralph/old-loops/abandoned-deprecated.md	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1973:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	tracked	.ralph/old-loops/abandoned-deprecated.state.json	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1974:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	tracked	.ralph/old-loops/audit-remediation.md	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1975:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	tracked	.ralph/old-loops/audit-remediation.state.json	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1976:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	tracked	.ralph/old-loops/audit-tasklist-loop.md	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1977:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	tracked	.ralph/old-loops/audit-tasklist-loop.state.json	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1978:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	tracked	.ralph/old-loops/audit-todo-implementation.md	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1979:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	tracked	.ralph/old-loops/audit-todo-implementation.state.json	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1980:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	tracked	.ralph/old-loops/dogfood-fixes.md	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1981:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	tracked	.ralph/old-loops/dogfood-fixes.state.json	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1982:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	tracked	.ralph/old-loops/enhancement-todo.md	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1983:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	tracked	.ralph/old-loops/full-audit-v0_8.md	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1984:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	tracked	.ralph/old-loops/full-audit-v0_8.state.json	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1985:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	tracked	.ralph/old-loops/full-codebase-audit.md	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1986:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	tracked	.ralph/old-loops/full-codebase-audit.state.json	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1987:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	tracked	.ralph/old-loops/implement-todo-items.md	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1988:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	tracked	.ralph/old-loops/implement-todo-items.state.json	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1989:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	tracked	.ralph/old-loops/lop-remaining.md	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1990:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	tracked	.ralph/old-loops/lop-remaining.state.json	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1991:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	tracked	.ralph/old-loops/p1-enhancements.md	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1992:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	tracked	.ralph/old-loops/p1-enhancements.state.json	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1993:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	tracked	.ralph/old-loops/remaining-todos.md	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1994:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	tracked	.ralph/old-loops/remaining-todos.state.json	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1995:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	tracked	.ralph/old-loops/research.md	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1996:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	tracked	.ralph/old-loops/research.state.json	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1997:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	tracked	.ralph/old-loops/scanner-improvements.md	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1998:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	tracked	.ralph/old-loops/scanner-improvements.state.json	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1999:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	tracked	.ralph/old-loops/todo-all.md	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2000:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	tracked	.ralph/old-loops/todo-all.state.json	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2001:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	tracked	.ralph/repo-analysis-loop.md	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2002:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	tracked	.ralph/repo-analysis.md	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2003:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	tracked	.ralph/repo-analysis.state.json	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2004:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	tracked	.ralph/supply-chain-security.md	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2005:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	tracked	.ralph/supply-chain-security.state.json	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2006:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	tracked	.repo-rot/DraconDev_ai-auto-repo-rot-scanner-todo-agent.yaml	blocked-needs-approval	possibly obsolete doc/checklist; needs approval
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2007:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	tracked	.repo-rot/sqsalvataggio_ai-auto-repo-rot-scanner-todo-agent.yaml	blocked-needs-approval	possibly obsolete doc/checklist; needs approval
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2008:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	tracked	TODO.md	blocked-needs-approval	possibly obsolete doc/checklist; needs approval
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2009:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	tracked	secrets.source	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2010:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	tracked	secrets.source.sh	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2024:/home/dracon/Dev/ai-auto-writer	tracked	.env	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2025:/home/dracon/Dev/ai-auto-writer	tracked	.env.example	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2026:/home/dracon/Dev/ai-auto-writer	tracked	.env.turso	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2027:/home/dracon/Dev/ai-auto-writer	tracked	.pi/goals/archived/goal_2026053123023425_mpu9986z-0gjd1y.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2028:/home/dracon/Dev/ai-auto-writer	tracked	.pi/goals/archived/goal_2026060102412538_mpugawjq-h83jg3.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2029:/home/dracon/Dev/ai-auto-writer	tracked	.pi/goals/archived/goal_2026060122574314_mpvqtryd-ss97mr.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2030:/home/dracon/Dev/ai-auto-writer	tracked	.pi/goals/archived/goal_2026060518174912_mq12fe7r-q4mpwa.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2031:/home/dracon/Dev/ai-auto-writer	tracked	.pi/goals/archived/goal_2026060612571315_mq17apdj-wbmnvk.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2032:/home/dracon/Dev/ai-auto-writer	tracked	.ralph/50-books-loop.md	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2033:/home/dracon/Dev/ai-auto-writer	tracked	.ralph/50-books-loop.state.json	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2034:/home/dracon/Dev/ai-auto-writer	tracked	.ralph/audit-loop.md	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2035:/home/dracon/Dev/ai-auto-writer	tracked	.ralph/audit-loop.state.json	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2036:/home/dracon/Dev/ai-auto-writer	tracked	.ralph/audit-md-loop.md	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2037:/home/dracon/Dev/ai-auto-writer	tracked	.ralph/audit-md-loop.state.json	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2038:/home/dracon/Dev/ai-auto-writer	tracked	.ralph/audit-remediation.md	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2039:/home/dracon/Dev/ai-auto-writer	tracked	.ralph/audit-remediation.state.json	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2040:/home/dracon/Dev/ai-auto-writer	tracked	.ralph/audit-tasks.md	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2041:/home/dracon/Dev/ai-auto-writer	tracked	.ralph/audit-tasks.state.json	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2042:/home/dracon/Dev/ai-auto-writer	tracked	.ralph/batch-book-generation.md	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2043:/home/dracon/Dev/ai-auto-writer	tracked	.ralph/batch-book-generation.state.json	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2044:/home/dracon/Dev/ai-auto-writer	tracked	.ralph/fix-batch-issues.md	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2045:/home/dracon/Dev/ai-auto-writer	tracked	.ralph/fix-batch-issues.state.json	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2046:/home/dracon/Dev/ai-auto-writer	tracked	_archive/to-release/RELEASE-CHECKLIST.md	blocked-needs-approval	possibly obsolete doc/checklist; needs approval
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2047:/home/dracon/Dev/ai-auto-writer	tracked	_archive/to-release/The-Gearing-Deep-dex-morrow/cover.jpg	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2048:/home/dracon/Dev/ai-auto-writer	tracked	_archive/to-release/The-Hourglass-Binding-silas-croft/cover.jpg	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2049:/home/dracon/Dev/ai-auto-writer	tracked	_archive/to-release/The-Refracted-Mile-eira-silas/cover.jpg	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2050:/home/dracon/Dev/ai-auto-writer	tracked	_archive/to-release/The-Salt-Bound-raven-steele/cover.jpg	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2051:/home/dracon/Dev/ai-auto-writer	tracked	_archive/to-release/The-Scarlet-Transmission-tolkien_grim/cover.jpg	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2052:/home/dracon/Dev/ai-auto-writer	tracked	_archive/to-release/The-Silence-Between-Tokens-vera-kincaid/PUBLICATION-README.md	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2053:/home/dracon/Dev/ai-auto-writer	tracked	_archive/to-release/The-Silence-Between-Tokens-vera-kincaid/The-Silence-Between-Tokens.epub	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2054:/home/dracon/Dev/ai-auto-writer	tracked	_archive/to-release/The-Silence-Between-Tokens-vera-kincaid/The-Silence-Between-Tokens.mobi	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2055:/home/dracon/Dev/ai-auto-writer	tracked	_archive/to-release/The-Silence-Between-Tokens-vera-kincaid/cover.jpg	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2056:/home/dracon/Dev/ai-auto-writer	tracked	_archive/to-release/The-Silence-Between-Tokens-vera-kincaid/outline.md	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2057:/home/dracon/Dev/ai-auto-writer	tracked	_archive/to-release/The-Silence-Between-Tokens-vera-kincaid/publication-metadata.json	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2058:/home/dracon/Dev/ai-auto-writer	tracked	_archive/to-release/The-Thorns-Remember-elena-voss/cover.jpg	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2059:/home/dracon/Dev/ai-auto-writer	tracked	_archive/to-release/The-Ultraviolet-Elegy-nova-chen/cover.jpg	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2060:/home/dracon/Dev/ai-auto-writer	tracked	_archive/to-release/Varnish-marple_hart/cover.jpg	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2061:/home/dracon/Dev/ai-auto-writer	tracked	_archive/to-release/test1.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2062:/home/dracon/Dev/ai-auto-writer	tracked	_archive/to-release/test2.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2063:/home/dracon/Dev/ai-auto-writer	tracked	_archive/to-release/test3.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2064:/home/dracon/Dev/ai-auto-writer	tracked	_archive/to-release/test4.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2065:/home/dracon/Dev/ai-auto-writer	tracked	_archive/to-release/test5.png	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2066:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/amber-raven/books/crimson-fortune/chapters/09-the-sunken-archives-secrets.md	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2067:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/charlotte-belle/books/under-the-flame/chapters/03-secret-ingredients.md	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2068:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/clover-honeydew/books/echoes-of-the-veil/chapters/05-the-keepers-secret.md	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2069:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/dante-cross/books/the-last-echo/chapters/02-the-scholars-secret.md	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2070:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/derek-stone/author-photo.jpg	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2071:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/derek-stone/books/dungeon-core-awakening/cover.jpg	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2072:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/dex-morrow/author-photo.jpg	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2073:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/dex-morrow/books/the-dungeon-below/cover.jpg	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2074:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/dexter-graves/author-photo.jpg	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2075:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/dexter-graves/books/the-black-legion/cover.jpg	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2076:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/elara_nightshade/author-photo.jpg	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2077:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/elena-frost/books/the-alabaster-alibi/chapters/03-secrets-in-the-archive.md	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2078:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/elena-voss/author-photo.jpg	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2079:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/elena-voss/books/thorns-of-the-heart/cover.jpg	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2080:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/ezra-blackthorn/books/the-silent-canvas/chapters/04-the-gallery-of-secrets.md	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2081:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/grace-monroe/books/duke-scandal-secret/.checkpoint.json	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2082:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/grace-monroe/books/duke-scandal-secret/.outline-detailed.json	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2083:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/grace-monroe/books/duke-scandal-secret/chapters/01-chapter-1.snapshot.json	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2084:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/grace-monroe/books/duke-scandal-secret/chapters/01-the-librarian-of-thornwood.md	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2085:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/grace-monroe/books/duke-scandal-secret/chapters/02-chapter-2.snapshot.json	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2086:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/grace-monroe/books/duke-scandal-secret/chapters/02-the-french-letters.md	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2087:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/grace-monroe/books/duke-scandal-secret/chapters/03-chapter-3.snapshot.json	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2088:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/grace-monroe/books/duke-scandal-secret/chapters/03-the-note-under-the-door.md	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2089:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/grace-monroe/books/duke-scandal-secret/chapters/04-chapter-4.snapshot.json	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2090:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/grace-monroe/books/duke-scandal-secret/chapters/04-the-flaw-in-the-glass.md	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2091:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/grace-monroe/books/duke-scandal-secret/chapters/05-chapter-5.snapshot.json	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2092:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/grace-monroe/books/duke-scandal-secret/chapters/05-the-false-engagement.md	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2093:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/grace-monroe/books/duke-scandal-secret/chapters/06-chapter-6.snapshot.json	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2094:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/grace-monroe/books/duke-scandal-secret/chapters/06-the-terms-of-the-lie.md	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2095:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/grace-monroe/books/duke-scandal-secret/chapters/07-chapter-7.snapshot.json	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2096:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/grace-monroe/books/duke-scandal-secret/chapters/07-riding-lessons.md	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2097:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/grace-monroe/books/duke-scandal-secret/chapters/08-chapter-8.snapshot.json	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2098:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/grace-monroe/books/duke-scandal-secret/chapters/08-the-coded-journal.md	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2099:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/grace-monroe/books/duke-scandal-secret/chapters/09-chapter-9.snapshot.json	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2100:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/grace-monroe/books/duke-scandal-secret/chapters/09-the-scandal-sheets.md	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2101:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/grace-monroe/books/duke-scandal-secret/chapters/10-chapter-10.snapshot.json	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2102:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/grace-monroe/books/duke-scandal-secret/chapters/10-the-elopement-that-wasnt.md	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2103:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/grace-monroe/books/duke-scandal-secret/chapters/11-chapter-11.snapshot.json	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2104:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/grace-monroe/books/duke-scandal-secret/chapters/11-the-dover-confrontation.md	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2105:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/grace-monroe/books/duke-scandal-secret/chapters/12-chapter-12.snapshot.json	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2106:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/grace-monroe/books/duke-scandal-secret/chapters/12-the-innkeepers-bandages.md	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2107:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/grace-monroe/books/duke-scandal-secret/chapters/13-chapter-13.snapshot.json	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2108:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/grace-monroe/books/duke-scandal-secret/chapters/13-the-scandal-breaks.md	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2109:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/grace-monroe/books/duke-scandal-secret/chapters/14-chapter-14.snapshot.json	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2110:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/grace-monroe/books/duke-scandal-secret/chapters/14-the-investigators-resolve.md	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2111:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/grace-monroe/books/duke-scandal-secret/chapters/15-chapter-15.snapshot.json	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2112:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/grace-monroe/books/duke-scandal-secret/chapters/15-the-darkest-hour.md	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2113:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/grace-monroe/books/duke-scandal-secret/chapters/16-chapter-16.snapshot.json	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2114:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/grace-monroe/books/duke-scandal-secret/chapters/16-the-committee-room.md	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2115:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/grace-monroe/books/duke-scandal-secret/chapters/17-chapter-17.snapshot.json	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2116:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/grace-monroe/books/duke-scandal-secret/chapters/17-the-confession-in-whitehall.md	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2117:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/grace-monroe/books/duke-scandal-secret/chapters/18-chapter-18.snapshot.json	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2118:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/grace-monroe/books/duke-scandal-secret/chapters/18-the-ring-and-the-revelation.md	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2119:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/grace-monroe/books/duke-scandal-secret/chapters/19-chapter-19.snapshot.json	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2120:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/grace-monroe/books/duke-scandal-secret/chapters/19-the-chapel-wedding.md	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2121:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/grace-monroe/books/duke-scandal-secret/chapters/20-chapter-20.snapshot.json	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2122:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/grace-monroe/books/duke-scandal-secret/chapters/20-the-master-key.md	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2123:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/grace-monroe/books/duke-scandal-secret/characters.json	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2124:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/grace-monroe/books/duke-scandal-secret/characters.md	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2125:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/grace-monroe/books/duke-scandal-secret/duke-scandal-secret.md	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2126:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/grace-monroe/books/duke-scandal-secret/global-ledger.json	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2127:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/grace-monroe/books/duke-scandal-secret/outline.md	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2128:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/grace-monroe/books/the-last-summer-bloom/chapters/08-the-secret-unveiled.md	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2129:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/hester-crawford/books/secret-party/characters.md	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2130:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/iris-thorn/books/whispers-of-the-forgotten/chapters/04-the-towns-secret.md	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2131:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/lily-ashford/books/the-weight-of-wisteria/chapters/04-every-secret-shore.md	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2132:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/maple-thornwood/books/the-whispering-heartstone/chapters/02-the-scholars-secret.md	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2133:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/marcus-thorne/author-photo.jpg	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2134:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/mira-thornwood/author-photo.jpg	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2135:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/mira-thornwood/books/fae-prince-forced-marriage/cover.jpg	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2136:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/nova-chen/author-photo.jpg	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2137:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/nova-chen/books/neon-requiem/chapters/06-the-silence-between-notes.md	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2138:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/nova-chen/books/neon-requiem/chapters/08-the-lockets-secret-symphony.md	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2139:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/nova-chen/books/neon-requiem/cover.jpg	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2140:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/nova-vale/author-photo.jpg	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2141:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/nova-vale/books/rebellion-in-sector-nine/chapters/04-the-apothecarys-secret.md	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2142:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/nova-vale/books/rebellion-in-sector-nine/cover.jpg	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2143:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/raven-steele/author-photo.jpg	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2144:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/raven-steele/books/the-broken-legion/cover.jpg	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2145:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/sage-willowmere/books/the-whispering-throne/chapters/08-the-secret-in-the-silent-cells.md	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2146:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/seraphina-ashford/books/echoes-of-the-arcanum/chapters/07-the-lockets-secret.md	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2147:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/seraphina-ashford/books/enemy-prince-marriage-alliance/chapters/09-salt-and-secrets.md	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2148:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/silas-croft/author-photo.jpg	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2149:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/silas-croft/books/murder-at-the-magical-bookshop/cover.jpg	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2150:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/test-review/books/a-robot-who-discovers-music/chapters/01-the-first-note-in-the-static.md	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2151:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/test-review/books/a-robot-who-discovers-music/chapters/01-the-first-note-in-the-static.snapshot.json	preserved	potential user-owned note/screenshot/project asset
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2152:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/vivian-cross/books/the-echo-chamber/chapters/05-the-old-mills-secret.md	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2153:/home/dracon/Dev/ai-auto-writer	tracked	authors/fiction/vivienne-laurent/books/blueprints-of-the-heart/chapters/07-unlocked-secrets.md	blocked-needs-approval	secret-like file requires rotation/approval before removal
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2154:/home/dracon/Dev/ai-auto-writer	tracked	todo.md	blocked-needs-approval	possibly obsolete doc/checklist; needs approval
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2222:/home/dracon/Dev/avid	tracked	.ralph/videoai-pilot.md	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:2223:/home/dracon/Dev/avid	tracked	.ralph/videoai-pilot.state.json	blocked-needs-approval	local task/session state; ambiguous user-owned content
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/repos.tsv:5:/home/dracon/Dev/rust-ai-web-auto
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/repos.tsv:9:/home/dracon/Dev/dracon-ai-lib
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/repos.tsv:12:/home/dracon/Dev/ai-auto-writer
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/repos.tsv:14:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.dracon-utilities.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.dracon-utilities.git.txt:21:* main                   6f1ac538 [origin/main] 82 file(s) in .demon,docs [docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/pi-proof/after.dracon-platform.pi-files.txt, docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/CLEANUP_MANIFEST.md, docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/pi-proof/after.Junk-Runner-bevy.pi-files.txt] DELTA:+364/-1 | NEW:cleanup-except-pi/CLEANUP_MANIFEST.md,pi-proof/.dracon.pi-files.diff,pi-proof/.dracon.pi-untracked.diff,pi-proof/DraconDev.pi-files.diff,pi-proof/DraconDev.pi-untracked.diff,pi-proof/Junk-Runner-bevy.pi-files.diff,pi-proof/Junk-Runner-bevy.pi-untracked.diff,pi-proof/after..dracon.pi-files.txt,pi-proof/after..dracon.pi-untracked.txt,pi-proof/after.DraconDev.pi-files.txt+71more DEL:keys/owner_age1wz5p.pub
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.dracon-utilities.git.txt:25:250bc181 64 file(s) in docs [docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv, docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/before/inventory.json, docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/pi-proof/before.dracon-platform.pi-files.txt] DELTA:+3508/-0 | NEW:before/inventory.json,before/inventory.tsv,candidates/cleanup-candidates.tsv,per-repo/before..dracon.git.txt,per-repo/before.DraconDev.git.txt,per-repo/before.Junk-Runner-bevy.git.txt,per-repo/before.ai-auto-repo-rot-scanner-todo-agent.git.txt,per-repo/before.ai-auto-writer.git.txt,per-repo/before.avid.git.txt,per-repo/before.browser-extensions-shared.git.txt+54more
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.dracon-utilities.git.txt:27:adf34aea 27 file(s) in docs [docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/inventory.json, docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/browser-extensions-shared.git.txt, docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/dracon-platform.git.txt] DELTA:+1206/-0 | NEW:deps/deps.tsv,public-readiness-funding/hygiene.tsv,public-readiness-funding/inventory.json,public-readiness-funding/inventory.tsv,non-rust/non-rust.tsv,per-repo/.dracon.git.txt,per-repo/DraconDev.git.txt,per-repo/Junk-Runner-bevy.git.txt,per-repo/ai-auto-repo-rot-scanner-todo-agent.git.txt,per-repo/ai-auto-writer.git.txt+17more
docs/audit/2026-06-11-full-repo-audit/per-repo/dracon-platform.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/per-repo/dracon-platform.git.txt:3:1 .M N... 100644 100644 100644 d7ed0401c309f871c7f829c4fb5d57ab8cb4a886 d7ed0401c309f871c7f829c4fb5d57ab8cb4a886 apis/services/ai-api/src/ai/client/mod.rs
docs/audit/2026-06-11-full-repo-audit/per-repo/dracon-platform.git.txt:5:? apis/services/ai-api/tests/streaming.rs
docs/audit/2026-06-11-full-repo-audit/per-repo/dracon-platform.git.txt:6:? web/ai-hub-browser-probe.mjs
docs/audit/2026-06-11-full-repo-audit/per-repo/dracon-platform.git.txt:7:? web/ai-hub-signed-browser-probe.mjs
docs/audit/2026-06-11-full-repo-audit/per-repo/dracon-platform.git.txt:20:  azumi-ver                                11f588f8d chore(goal): ai-hub-audit goal complete (6/6 tasks)
docs/audit/2026-06-11-full-repo-audit/per-repo/dracon-platform.git.txt:22:* main                                     9d330e811 [origin/main] 1 file(s) in apis [apis/services/ai-api/src/ai/client/mod.rs] DELTA:+5/-0
docs/audit/2026-06-11-full-repo-audit/per-repo/dracon-platform.git.txt:25:  phase-1/api-core-lift                    0f5e8e22b [origin/phase-1/api-core-lift] 1 file(s) in apis [apis/ai-api/.env] DELTA:+1/-1 | ENV:
docs/audit/2026-06-11-full-repo-audit/per-repo/dracon-platform.git.txt:26:  phase-2/high-cluster                     7ad8ecca9 [origin/phase-2/high-cluster] 3 file(s) in web [web/ai-hub/src/lib/chrome.config.ts, web/ai-hub/src/routes/+layout.svelte, web/packages/chrome/src/lib/SiteSubNav.svelte] DELTA:+9/-19
docs/audit/2026-06-11-full-repo-audit/per-repo/dracon-platform.git.txt:30:  phase-4/specta-metrics                   d8f6a56e2 [origin/phase-4/specta-metrics] 1 file(s) in web [web/ai-hub/src/lib/icons.ts] DELTA:+4/-2
docs/audit/2026-06-11-full-repo-audit/per-repo/dracon-platform.git.txt:32:9d330e811 1 file(s) in apis [apis/services/ai-api/src/ai/client/mod.rs] DELTA:+5/-0
docs/audit/2026-06-11-full-repo-audit/per-repo/dracon-platform.git.txt:33:e88dbd7ec 19 file(s) in apis,web [apis/services/ai-api/src/handlers/middleware.rs, apis/docs/audits/audit-2026-06-08-findings-status.md, apis/services/ai-api/src/handlers/tests.rs] DELTA:+168/-16 | TEST:13 BIN:7 NEW:audits/audit-2026-06-08-findings-status.md ENV:
docs/audit/2026-06-11-full-repo-audit/per-repo/dracon-platform.git.txt:35:bb39684e6 8 file(s) in apis [apis/services/ai-api/src/specta_export.rs, apis/services/ai-api/src/ai/client/mod.rs, apis/services/ai-api/src/handlers/chat.rs] DELTA:+13/-22
docs/audit/2026-06-11-full-repo-audit/per-repo/youtube-video-uploader.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/per-repo/youtube-video-uploader.git.txt:13:* main 771d422 [origin/main] Merge https://github.com/DraconDev/youtube-video-uploader
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.Junk-Runner-bevy.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.Junk-Runner-bevy.git.txt:14:  main        e1894697f [origin/main] Added SOLID_VS_SVELTE.md
docs/audit/2026-06-11-full-repo-audit/strategy-audit/recommendations.tsv:2:R1	P1	changelog-vs-source	Rewrite CHANGELOG.md so [0.112.4] and [Unreleased] match the actual source. Remove scribe/ai-bumper/generate_commit_message/parse_ai_bump_response entries that no longer exist.	S	none	yes	CHANGELOG.md L80-263 (Unreleased + 0.112.0 block); rg "cfg(feature = ..scribe..)" dracon-sync/src = 0; rg "scribe_update|SimpleAiService" = 0
docs/audit/2026-06-11-full-repo-audit/strategy-audit/recommendations.tsv:6:R5	P2	test-counts	Replace AGENTS.md hard test counts (lines 846-850) with "see latest CI run" or a CI-enforced assertion.	XS	none	no	AGENTS.md L846-850 claim 431/692; release-readiness report says 705/9/22
docs/audit/2026-06-11-full-repo-audit/strategy-audit/recommendations.tsv:12:R11	P3	secret-layout-docs	Document the new ~/.dracon/secrets/{pat,registry,ai,...} layout in AGENTS.md (or a linked section).	S	none	no	AGENTS.md "Tokens & Secrets" still describes the old path; git-auth-prompt report
docs/audit/2026-06-11-full-repo-audit/per-repo/browser-extensions-shared.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/per-repo/browser-extensions-shared.git.txt:18:* main 16736ee5f [origin/main] 4 file(s) in auto-form-filler,job-finder,vidpro-extension [auto-form-filler/AUDIT_REPORT.md, vidpro-extension/AUDIT-2026-06-11.md, job-finder/docs/COMPETITOR_MONETIZATION.md] DELTA:+674/-156 | NEW:vidpro-extension/AUDIT-2026-06-11.md
docs/audit/2026-06-11-full-repo-audit/per-repo/browser-extensions-shared.git.txt:24:066629120 1 file(s) in vidpro-extension [vidpro-extension/docs/NON-AI-GAPS.md] DELTA:+22/-28
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/repos.tsv:8:/home/dracon/Dev/rust-ai-web-auto
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/repos.tsv:10:/home/dracon/Dev/dracon-ai-lib
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/repos.tsv:12:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/repos.tsv:14:/home/dracon/Dev/ai-auto-writer
docs/audit/2026-06-11-full-repo-audit/per-repo/dracon-code.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/per-repo/dracon-code.git.txt:13:  backup-main-20260513                             13262567 security(dependency configuration): Updated dependency configuration in `deny.toml` for security and comp...
docs/audit/2026-06-11-full-repo-audit/per-repo/dracon-code.git.txt:14:  bevy-version                                     ef86290b [gui+src|wip] screenshot viewer, task persistence, fetch denylist UI, gui_refresh_secs poll wiring, ai_actions in plan prompt, dead code cleanup
docs/audit/2026-06-11-full-repo-audit/per-repo/dracon-code.git.txt:18:  egui-version                                     0de221d8 {"schema":"dracon.commit.v2","schema_rev":2,"commit_kind":"sync_event","actor":"dracon-sync","generator":{"name":"dracon-git","version":"0.1.0"},"event_fingerprint":"bcc7462f0ab438a932e8482e31fc41ac25fb3d82d26d0a96f0d53304e52a706b","ts":"1771992484","repo":"dracon-code","branch":"master","files":{"added":0,"modified":3,"deleted":0,"renamed":0,"type_change":0,"unknown":0},"changed_paths_full":["Cargo.lock","Cargo.toml","gui/src/main.rs"],"changed_paths":["Cargo.lock","Cargo.toml","gui/src/main.rs"],"changed_paths_total":3,"changed_paths_truncated":false,"top_level_scopes":[{"key":"Cargo.lock","count":1},{"key":"Cargo.toml","count":1},{"key":"gui","count":1}],"extension_summary":[{"key":"lock","count":1},{"key":"rs","count":1},{"key":"toml","count":1}],"domain_summary":[{"key":"code","count":1},{"key":"config","count":1},{"key":"lockfile","count":1}],"intent_tags":["behavior_change_possible","compiled_or_runtime_code_touched","configuration_update","dependency_lock_changed"],"risk_flags":["build_graph_or_dependency_surface"],"semantic":{"files_analyzed":1,"files_skipped":2,"symbols_total":74,"symbols_truncated":false,"symbols":[{"path":"gui/src/main.rs","language":"rust","name":"main","kind":"function","start_line":11,"end_line":25},{"path":"gui/src/main.rs","language":"rust","name":"GuiRuntimeConfig","kind":"struct","start_line":28,"end_line":33},{"path":"gui/src/main.rs","language":"rust","name":"DraconConfigFile","kind":"struct","start_line":36,"end_line":44},{"path":"gui/src/main.rs","language":"rust","name":"Screen","kind":"enum","start_line":47,"end_line":51},{"path":"gui/src/main.rs","language":"rust","name":"Screen","kind":"impl","start_line":53,"end_line":61},{"path":"gui/src/main.rs","language":"rust","name":"label","kind":"function","start_line":54,"end_line":60},{"path":"gui/src/main.rs","language":"rust","name":"HubSort","kind":"enum","start_line":64,"end_line":70},{"path":"gui/src/main.rs","language":"rust","name":"HubSort","kind":"impl","start_line":72,"end_line":82},{"path":"gui/src/main.rs","language":"rust","name":"label","kind":"function","start_line":73,"end_line":81},{"path":"gui/src/main.rs","language":"rust","name":"HubFilter","kind":"enum","start_line":85,"end_line":89},{"path":"gui/src/main.rs","language":"rust","name":"HubFilter","kind":"impl","start_line":91,"end_line":99},{"path":"gui/src/main.rs","language":"rust","name":"label","kind":"function","start_line":92,"end_line":98},{"path":"gui/src/main.rs","language":"rust","name":"SettingsForm","kind":"struct","start_line":102,"end_line":110},{"path":"gui/src/main.rs","language":"rust","name":"SettingsForm","kind":"impl","start_line":112,"end_line":179},{"path":"gui/src/main.rs","language":"rust","name":"from_body","kind":"function","start_line":113,"end_line":132},{"path":"gui/src/main.rs","language":"rust","name":"apply_to_body","kind":"function","start_line":134,"end_line":178},{"path":"gui/src/main.rs","language":"rust","name":"OperatorApp","kind":"struct","start_line":181,"end_line":198},{"path":"gui/src/main.rs","language":"rust","name":"OperatorApp","kind":"impl","start_line":200,"end_line":708},{"path":"gui/src/main.rs","language":"rust","name":"new","kind":"function","start_line":201,"end_line":240},{"path":"gui/src/main.rs","language":"rust","name":"refresh","kind":"function","start_line":242,"end_line":271},{"path":"gui/src/main.rs","language":"rust","name":"run_action","kind":"function","start_line":273,"end_line":280},{"path":"gui/src/main.rs","language":"rust","name":"save_config","kind":"function","start_line":282,"end_line":304},{"path":"gui/src/main.rs","language":"rust","name":"sorted_hub_rows","kind":"function","start_line":306,"end_line":348},{"path":"gui/src/main.rs","language":"rust","name":"nav_row","kind":"function","start_line":350,"end_line":365},{"path":"gui/src/main.rs","language":"rust","name":"project_screen","kind":"function","start_line":367,"end_line":450},{"path":"gui/src/main.rs","language":"rust","name":"hub_screen","kind":"function","start_line":452,"end_line":533},{"path":"gui/src/main.rs","language":"rust","name":"settings_screen","kind":"function","start_line":535,"end_line":707},{"path":"gui/src/main.rs","language":"rust","name":"OperatorApp","kind":"impl","start_line":710,"end_line":769},{"path":"gui/src/main.rs","language":"rust","name":"update","kind":"function","start_line":711,"end_line":768},{"path":"gui/src/main.rs","language":"rust","name":"apply_theme","kind":"function","start_line":771,"end_line":818},{"path":"gui/src/main.rs","language":"rust","name":"panel","kind":"function","start_line":820,"end_line":835},{"path":"gui/src/main.rs","language":"rust","name":"screen_title","kind":"function","start_line":837,"end_line":851},{"path":"gui/src/main.rs","language":"rust","name":"paint_background","kind":"function","start_line":853,"end_line":888},{"path":"gui/src/main.rs","language":"rust","name":"kv","kind":"function","start_line":890,"end_line":895},{"path":"gui/src/main.rs","language":"rust","name":"status_chip","kind":"function","start_line":897,"end_line":909},{"path":"gui/src/main.rs","language":"rust","name":"action_button","kind":"function","start_line":911,"end_line":928},{"path":"gui/src/main.rs","language":"rust","name":"tab_button","kind":"function","start_line":930,"end_line":952},{"path":"gui/src/main.rs","language":"rust","name":"chip_button","kind":"function","start_line":954,"end_line":969},{"path":"gui/src/main.rs","language":"rust","name":"truncate_middle","kind":"function","start_line":971,"end_line":978},{"path":"gui/src/main.rs","language":"rust","name":"draw_projects_table","kind":"function","start_line":980,"end_line":1065},{"path":"gui/src/main.rs","language":"rust","name":"draw_hub_table","kind":"function","start_line":1067,"end_line":1180},{"path":"gui/src/main.rs","language":"rust","name":"table_header","kind":"function","start_line":1182,"end_line":1190},{"path":"gui/src/main.rs","language":"rust","name":"table_row_bg","kind":"function","start_line":1192,"end_line":1198},{"path":"gui/src/main.rs","language":"rust","name":"is_active_repo","kind":"function","start_line":1200,"end_line":1205},{"path":"gui/src/main.rs","language":"rust","name":"phase_color","kind":"function","start_line":1207,"end_line":1217},{"path":"gui/src/main.rs","language":"rust","name":"trigger_color","kind":"function","start_line":1219,"end_line":1225},{"path":"gui/src/main.rs","language":"rust","name":"git_state_color","kind":"function","start_line":1227,"end_line":1241},{"path":"gui/src/main.rs","language":"rust","name":"FleetView","kind":"struct","start_line":1244,"end_line":1247},{"path":"gui/src/main.rs","language":"rust","name":"FleetRow","kind":"struct","start_line":1250,"end_line":1259},{"path":"gui/src/main.rs","language":"rust","name":"FleetRow","kind":"impl","start_line":1261,"end_line":1273},{"path":"gui/src/main.rs","language":"rust","name":"active_slice_label","kind":"function","start_line":1262,"end_line":1266},{"path":"gui/src/main.rs","language":"rust","name":"updated_label","kind":"function","start_line":1268,"end_line":1272},{"path":"gui/src/main.rs","language":"rust","name":"merge_discovered_repos","kind":"function","start_line":1275,"end_line":1291},{"path":"gui/src/main.rs","language":"rust","name":"compute_git_states","kind":"function","start_line":1293,"end_line":1297},{"path":"gui/src/main.rs","language":"rust","name":"git_state_for_repo","kind":"function","start_line":1299,"end_line":1324},{"path":"gui/src/main.rs","language":"rust","name":"parse_branch_sync","kind":"function","start_line":1326,"end_line":1348},{"path":"gui/src/main.rs","language":"rust","name":"discover_git_repos","kind":"function","start_line":1350,"end_line":1363},{"path":"gui/src/main.rs","language":"rust","name":"walk_for_git_repos","kind":"function","start_line":1365,"end_line":1407},{"path":"gui/src/main.rs","language":"rust","name":"refresh_view","kind":"function","start_line":1409,"end_line":1432},{"path":"gui/src/main.rs","language":"rust","name":"choose_selected_repo","kind":"function","start_line":1434,"end_line":1457},{"path":"gui/src/main.rs","language":"rust","name":"load_fleet_rows","kind":"function","start_line":1459,"end_line":1501},{"path":"gui/src/main.rs","language":"rust","name":"load_fleet_rows_sqlite","kind":"function","start_line":1503,"end_line":1547},{"path":"gui/src/main.rs","language":"rust","name":"load_gui_runtime_config","kind":"function","start_line":1549,"end_line":1580},{"path":"gui/src/main.rs","language":"rust","name":"default_fleet_db_path","kind":"function","start_line":1582,"end_line":1584},{"path":"gui/src/main.rs","language":"rust","name":"expand_tilde","kind":"function","start_line":1586,"end_line":1596},{"path":"gui/src/main.rs","language":"rust","name":"read_text_file","kind":"function","start_line":1598,"end_line":1601},{"path":"gui/src/main.rs","language":"rust","name":"run_json","kind":"function","start_line":1603,"end_line":1611},{"path":"gui/src/main.rs","language":"rust","name":"run_cmd","kind":"function","start_line":1613,"end_line":1619},{"path":"gui/src/main.rs","language":"rust","name":"run_cmd_in","kind":"function","start_line":1621,"end_line":1627},{"path":"gui/src/main.rs","language":"rust","name":"run_cmd_capture","kind":"function","start_line":1629,"end_line":1652},{"path":"gui/src/main.rs","language":"rust","name":"resolve_default_project","kind":"function","start_line":1654,"end_line":1659},{"path":"gui/src/main.rs","language":"rust","name":"append_log","kind":"function","start_line":1661,"end_line":1666},{"path":"gui/src/main.rs","language":"rust","name":"now_secs","kind":"function","start_line":1668,"end_line":1673},{"path":"gui/src/main.rs","language":"rust","name":"format_ts","kind":"function","start_line":1675,"end_line":1677}],"kind_summary":[{"key":"function","count":58},{"key":"impl","count":7},{"key":"struct","count":6},{"key":"enum","count":3}],"language_summary":[{"key":"rust","count":74}]},"status":{"ahead":0,"behind":0,"modified_files":1,"staged_files":0},"policy":{"deterministic":true,"ai_commit_messages":false}}
docs/audit/2026-06-11-full-repo-audit/per-repo/dracon-code.git.txt:20:* main                                             1fc9373f [origin/main] 4 file(s) in docs,plan [docs/AI-LIB-AUDIT.md, docs/README.md, docs/AI-STRATEGY.md] DELTA:+154/-1 | NEW:docs/AI-LIB-AUDIT.md
docs/audit/2026-06-11-full-repo-audit/per-repo/dracon-code.git.txt:22:  temp-main                                        e1eaa26d Reset to origin/master
docs/audit/2026-06-11-full-repo-audit/per-repo/dracon-code.git.txt:25:1fc9373f 4 file(s) in docs,plan [docs/AI-LIB-AUDIT.md, docs/README.md, docs/AI-STRATEGY.md] DELTA:+154/-1 | NEW:docs/AI-LIB-AUDIT.md
docs/audit/2026-06-11-full-repo-audit/per-repo/dracon-code.git.txt:26:e53841a6 1 file(s) in crates [crates/dracon-ai/src/lib.rs] DELTA:+1/-1
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.DraconDev.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.DraconDev.git.txt:13:* main e280732 [origin/main] 3 file(s) [README_SUGGESTED_FORM.md, SUGGESTED_FORM_USAGE.md, SUGGESTED_FORM_BLOCKERS.md] DELTA:+191/-0 | NEW:README_SUGGESTED_FORM.md,SUGGESTED_FORM_BLOCKERS.md,SUGGESTED_FORM_USAGE.md
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/dracon-ai-lib.git.txt:1:REPO=/home/dracon/Dev/dracon-ai-lib
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/dracon-ai-lib.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/dracon-ai-lib.git.txt:5:dracon-ai	https://github.com/dracon-ai/dracon-ai-lib.git (fetch)
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/dracon-ai-lib.git.txt:6:dracon-ai	https://github.com/dracon-ai/dracon-ai-lib.git (push)
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/dracon-ai-lib.git.txt:7:origin	https://github.com/DraconDev/dracon-ai-lib.git (fetch)
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/dracon-ai-lib.git.txt:8:origin	https://github.com/DraconDev/dracon-ai-lib.git (push)
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/dracon-ai-lib.git.txt:10:* main 3acafd9 [origin/main: ahead 15] docs: stage consumer cutover plan and align README to dracon-ai org
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/dracon-ai-lib.git.txt:12:3acafd9 docs: stage consumer cutover plan and align README to dracon-ai org
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/dracon-ai-lib.git.txt:14:cd8bc7f 1 file(s) in crates [crates/ai-lib/src/providers/minimax.rs] DELTA:+48/-37
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/dracon-ai-lib.git.txt:15:209cff3 1 file(s) in crates [crates/ai-lib/src/providers/minimax.rs] DELTA:+1/-1
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/dracon-ai-lib.git.txt:16:d70cf8a 1 file(s) in crates [crates/ai-lib/src/providers/minimax.rs] DELTA:+16/-13
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/REPORT.md:8:- **`public-release` side-branch on `dracon-utilities` is gone.** 10 commits (1 merge + 9 public-release-only) merged into `main` via `--no-ff`. The branch was deleted on all 4 remotes (origin, github, gitlab, codeberg) and locally. The local `remotes/github/public-release` tracking ref was pruned.
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/REPORT.md:10:- **`one-mil-girls`**: 3 cosmetic comment edits (`shim` → `stub`, comment rewording) committed (`6bf75d9`) and pushed to origin/github/codeberg. **GitLab push was rejected by the protected-branch policy** ("You are not allowed to push code to protected branches on this project.") — this is a documented operator-policy block, not a code-side block. The 31 untracked `docs/audit/2026-06-11-cleanup/` files are preserved on disk (not added, not deleted) per the user's earlier "preserve user changes" constraint.
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/REPORT.md:18:repos: 16  ok: 16  warn: 0  concern: 0  failures: 0
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/REPORT.md:26:- `* main` (current) — at `109e110f`
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/REPORT.md:31:- `origin/main`, `github/main`, `gitlab/main`, `gitlab/master`, `codeberg/main`, `codeberg/master`, `origin/scribe-version`, `github/scribe-version`, `gitlab/scribe-version`, `codeberg/scribe-version`
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/REPORT.md:34:`Junk-Runner-bevy` is on `tauri2` with 2604 ahead / 1 behind `origin/main` (logged in evidence, no action).
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/REPORT.md:40:| 1 | `public-release` branch | "Merge to main, delete branch" | Paused daemon, stashed uncommitted, `git checkout main`, `git merge --no-ff public-release`, pushed to all 4 remotes, deleted branch on all 4 remotes, pruned local tracking ref | `pre-merge-state.md`, `post-merge-state.md` |
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/REPORT.md:45:All approval decisions were paired with the action taken in `evidence/approval-log.md`.
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/REPORT.md:47:## Constraints respected
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/REPORT.md:54:- No `master`/`main` conflict on GitLab (each mirror pushed `main` to its own `main`; the existence of a `master` on gitlab/codeberg is a default-branch mismatch but not an action item).
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/REPORT.md:59:`one-mil-girls`'s `main` cannot be pushed to `gitlab` because GitLab has the branch protected (no force-push, no MR-only pushes for the local account). The 711-commit gap between local `main` and `gitlab/main` will not close on its own. Resolving this requires either:
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/REPORT.md:60:- relaxing the protection on GitLab's `main`, or
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/REPORT.md:68:`docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/` contains:
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/REPORT.md:91:- (a) Public-release branch: **resolved** (merged to main, deleted on all 4 remotes).
docs/audit/2026-06-11-full-repo-audit/strategy-audit/cross-ref.tsv:1:theme	strategy_doc	strategy_claim	implementation_evidence	status	note
docs/audit/2026-06-11-full-repo-audit/strategy-audit/cross-ref.tsv:2:ai-scribe-removal	CHANGELOG.md	"Removed scribe_update() and stage_project_state()"	dracon-sync/src/scribe.rs missing; rg "scribe_update|stage_project_state" src → 0 hits; AUDIT.md 1.2 P1	consistency_old_audit_drift	audit flagged P1; now resolved (files removed) but CHANGELOG still lists scribe/ai-bumper feature toggles
docs/audit/2026-06-11-full-repo-audit/strategy-audit/cross-ref.tsv:3:ai-scribe-features	CHANGELOG.md (Unreleased)	"Version bumper prevents double-bump when both scribe and ai-bumper features enabled"	dracon-sync/Cargo.toml [features]: default = [] (no scribe/ai-bumper declared); rg "cfg(feature = ..scribe..)" src → 0 hits	changelog_drift	CHANGELOG references nonexistent Cargo features
docs/audit/2026-06-11-full-repo-audit/strategy-audit/cross-ref.tsv:4:orphan-warden-service	AUDIT.md 1.1 P1	dracon-warden.service references nonexistent daemon subcommand	ls dracon-warden/dracon-warden.service → missing; rg "Command::Daemon" src/main.rs → 0 hits; install.sh → 0 matches for warden.service	resolved	file deleted; no install.sh change needed (never installed it)
docs/audit/2026-06-11-full-repo-audit/strategy-audit/cross-ref.tsv:6:cargo-fmt-invocation	README.md line 212	"cargo fmt --check" as a dev command	For a workspace, that command is invalid (workspace has no root crate source); CI uses `cargo fmt -p dracon-sync -p dracon-system -p dracon-warden -- --check`	doc_drift_p3	README example would fail; AGENTS.md uses the correct per-package form
docs/audit/2026-06-11-full-repo-audit/strategy-audit/cross-ref.tsv:9:indexlock-coordination	ARCHITECTURE.md "Coordination: IndexLock"	O_EXCL on .git/index.lock	Verified: dracon-warden/src/main.rs IndexLock at 946-998; dracon-sync/src/sync.rs:2121-2124; reports confirm working	consistent	audit also confirmed intact
docs/audit/2026-06-11-full-repo-audit/strategy-audit/cross-ref.tsv:10:no-kill-guard	AGENTS.md "CRITICAL INVARIANT: The guard NEVER kills processes"	dracon-system/src/main.rs:586-587 has explicit comment; renice only	rg "SIGKILL|SIGTERM|nix::sys::signal|libc::kill" in dracon-system/src → 0 matches	consistent	audit confirmed
docs/audit/2026-06-11-full-repo-audit/strategy-audit/cross-ref.tsv:14:git-credential-prompt	(raised by user 2026-06-11)	(asked "are we using from the wrong place?")	After PAT-based helper installed, `env -u GH_TOKEN git ls-remote` returns SHA; no keyring popup	resolved	helper at ~/.dracon/secrets/pat/git-credential-github.sh wired as first helper in ~/.gitconfig
docs/audit/2026-06-11-full-repo-audit/strategy-audit/cross-ref.tsv:16:inventory-staleness	docs/audit/2026-06-11 release-readiness/REPORT.md	"no unexplained CONCERN/STUCK_PUSH remains"	Fresh `dracon-sync repos --json --full-path` (now): browser-extensions-shared STUCK AHEAD:1,STUCK_PUSH; folder-auto-banner STUCK AHEAD:1,STUCK_PUSH	stale_report	2 new STUCK repos not in the release-readiness report (created/pushed after that report)
docs/audit/2026-06-11-full-repo-audit/strategy-audit/cross-ref.tsv:23:changelog-vs-source	CHANGELOG.md Unreleased section	Mass-deletion guard, scribe features, ai-bumper features	rg in source: 0 hits for scribe_update, stage_project_state, SimpleAiService	changelog_drift_p2	"Unreleased" reads like a 0.112.0 release that was never finalised; deserves a full rewrite pass
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/folder-auto-banner.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/folder-auto-banner.git.txt:13:* main a963582 [origin/main] 5 file(s) in src [CHANGELOG.md, RELEASE_NOTES_0.6.16.md, Cargo.lock] DELTA:+49/-49 | NEW:RELEASE_NOTES_0.6.16.md
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/browser-extensions-shared.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/browser-extensions-shared.git.txt:20:* main 337c73a3d [origin/main] 2 file(s) in auto-form-filler,wxt-shared [wxt-shared/src/byok/BYOKSettings.tsx, auto-form-filler/entrypoints/options/App.tsx] DELTA:+17/-6
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/dracon-platform.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/dracon-platform.git.txt:15:  azumi-ver                                11f588f8d chore(goal): ai-hub-audit goal complete (6/6 tasks)
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/dracon-platform.git.txt:17:* main                                     de67c98a5 [origin/main] 6 file(s) in web [web/games-hosted/games/junk-runner/assets/index-CURdM7DG.js, web/games-hosted/games/junk-runner/assets/index-DnLya-k-.js, web/games-hosted/games/junk-runner/index.html] DELTA:+14/-14 | BIN:1 NEW:assets/index-Bo_QmhmO.css,assets/index-DnLya-k-.js DEL:assets/index-BiVTZKcp.css,assets/index-CURdM7DG.js
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/dracon-platform.git.txt:20:  phase-1/api-core-lift                    0f5e8e22b [origin/phase-1/api-core-lift] 1 file(s) in apis [apis/ai-api/.env] DELTA:+1/-1 | ENV:
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/dracon-platform.git.txt:21:  phase-2/high-cluster                     7ad8ecca9 [origin/phase-2/high-cluster] 3 file(s) in web [web/ai-hub/src/lib/chrome.config.ts, web/ai-hub/src/routes/+layout.svelte, web/packages/chrome/src/lib/SiteSubNav.svelte] DELTA:+9/-19
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/dracon-platform.git.txt:25:  phase-4/specta-metrics                   d8f6a56e2 [origin/phase-4/specta-metrics] 1 file(s) in web [web/ai-hub/src/lib/icons.ts] DELTA:+4/-2
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/dracon-libs-post-state.md:13:  origin/main: local_ahead=0 remote_ahead=0
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/dracon-libs-post-state.md:14:  github/main: local_ahead=0 remote_ahead=0
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/dracon-libs-post-state.md:15:  gitlab/main: local_ahead=0 remote_ahead=0
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/dracon-libs-post-state.md:16:  codeberg/main: local_ahead=0 remote_ahead=0
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/dracon-libs.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/dracon-libs.git.txt:13:* main 2ff017b [origin/main] 1 file(s) [deny.toml] DELTA:+2/-0
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.ai-auto-repo-rot-scanner-todo-agent.git.txt:1:REPO=/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.ai-auto-repo-rot-scanner-todo-agent.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.ai-auto-repo-rot-scanner-todo-agent.git.txt:4:codeberg	git@codeberg.org:dracondev/ai-auto-repo-rot-scanner-todo-agent.git (fetch)
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.ai-auto-repo-rot-scanner-todo-agent.git.txt:5:codeberg	git@codeberg.org:dracondev/ai-auto-repo-rot-scanner-todo-agent.git (push)
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.ai-auto-repo-rot-scanner-todo-agent.git.txt:6:github	git@github.com:DraconDev/ai-auto-repo-rot-scanner-todo-agent.git (fetch)
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.ai-auto-repo-rot-scanner-todo-agent.git.txt:7:github	git@github.com:DraconDev/ai-auto-repo-rot-scanner-todo-agent.git (push)
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.ai-auto-repo-rot-scanner-todo-agent.git.txt:8:gitlab	git@gitlab.com:dracondev/ai-auto-repo-rot-scanner-todo-agent.git (fetch)
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.ai-auto-repo-rot-scanner-todo-agent.git.txt:9:gitlab	git@gitlab.com:dracondev/ai-auto-repo-rot-scanner-todo-agent.git (push)
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.ai-auto-repo-rot-scanner-todo-agent.git.txt:10:origin	https://github.com/DraconDev/ai-auto-repo-rot-scanner-todo-agent.git (fetch)
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.ai-auto-repo-rot-scanner-todo-agent.git.txt:11:origin	https://github.com/DraconDev/ai-auto-repo-rot-scanner-todo-agent.git (push)
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.ai-auto-repo-rot-scanner-todo-agent.git.txt:13:* main 7132201 [origin/main] 1 file(s) in docs [docs/AUDIT-2026-06-10.md] DELTA:+128/-0
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.ai-auto-repo-rot-scanner-todo-agent.git.txt:19:9cb9641 6 file(s) in src [Cargo.lock, src/ai/mod.rs, src/webhook.rs] DELTA:+502/-1983
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/ai-auto-writer.git.txt:1:REPO=/home/dracon/Dev/ai-auto-writer
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/ai-auto-writer.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/ai-auto-writer.git.txt:4:codeberg	git@codeberg.org:dracondev/ai-auto-writer.git (fetch)
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/ai-auto-writer.git.txt:5:codeberg	git@codeberg.org:dracondev/ai-auto-writer.git (push)
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/ai-auto-writer.git.txt:6:github	git@github.com:DraconDev/ai-auto-writer.git (fetch)
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/ai-auto-writer.git.txt:7:github	git@github.com:DraconDev/ai-auto-writer.git (push)
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/ai-auto-writer.git.txt:8:gitlab	git@gitlab.com:dracondev/ai-auto-writer.git (fetch)
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/ai-auto-writer.git.txt:9:gitlab	git@gitlab.com:dracondev/ai-auto-writer.git (push)
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/ai-auto-writer.git.txt:10:origin	https://github.com/DraconDev/ai-auto-writer.git (fetch)
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/ai-auto-writer.git.txt:11:origin	https://github.com/DraconDev/ai-auto-writer.git (push)
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/ai-auto-writer.git.txt:14:* main                               aa5d0ebb [origin/main] 1 file(s) in src [src/services/dracon.rs] DELTA:+4/-32
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/ai-auto-writer.git.txt:19:9c829b43 Merge https://github.com/DraconDev/ai-auto-writer
docs/audit/2026-06-11-full-repo-audit/strategy-audit/doc-inventory.tsv:12:docs/design/warden-plaintext-sibling.md	3810	81	2026-06-07 01:51:52.740161622 +0100
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/kiki-sassy-desktop-announcer.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/kiki-sassy-desktop-announcer.git.txt:13:* main 0155632 [origin/main] 2 file(s) in src [src/journal.rs, src/daemon.rs] DELTA:+2/-4
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.video-factory.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.video-factory.git.txt:13:* main 698a658 [origin/main] 14 file(s) in crates [crates/api/src/routes.rs, crates/core/src/config.rs, crates/worker/src/ffmpeg.rs] DELTA:+225/-162
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.video-factory.git.txt:17:4215e5f 1 file(s) in src [src/main.rs] DELTA:+3/-3
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.dracon-platform.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.dracon-platform.git.txt:3:1 .M N... 100644 100644 100644 eefde8c443ba243fd7ad46e2b4017d57a4faef74 eefde8c443ba243fd7ad46e2b4017d57a4faef74 apis/services/ai-api/src/ai/client/mod.rs
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.dracon-platform.git.txt:17:  azumi-ver                                11f588f8d chore(goal): ai-hub-audit goal complete (6/6 tasks)
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.dracon-platform.git.txt:19:* main                                     928216dbd [origin/main] 8 file(s) in apis,web [apis/services/ai-api/tests/happy_path.rs, apis/services/ai-api/tests/common/mod.rs, apis/services/ai-api/ai-api-sdk/tests/sdk.rs] DELTA:+160/-120 | TEST:276 BIN:2
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.dracon-platform.git.txt:22:  phase-1/api-core-lift                    0f5e8e22b [origin/phase-1/api-core-lift] 1 file(s) in apis [apis/ai-api/.env] DELTA:+1/-1 | ENV:
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.dracon-platform.git.txt:23:  phase-2/high-cluster                     7ad8ecca9 [origin/phase-2/high-cluster] 3 file(s) in web [web/ai-hub/src/lib/chrome.config.ts, web/ai-hub/src/routes/+layout.svelte, web/packages/chrome/src/lib/SiteSubNav.svelte] DELTA:+9/-19
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.dracon-platform.git.txt:27:  phase-4/specta-metrics                   d8f6a56e2 [origin/phase-4/specta-metrics] 1 file(s) in web [web/ai-hub/src/lib/icons.ts] DELTA:+4/-2
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.dracon-platform.git.txt:29:928216dbd 8 file(s) in apis,web [apis/services/ai-api/tests/happy_path.rs, apis/services/ai-api/tests/common/mod.rs, apis/services/ai-api/ai-api-sdk/tests/sdk.rs] DELTA:+160/-120 | TEST:276 BIN:2
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.dracon-platform.git.txt:33:1ea8a4ae6 6 file(s) in .data,web [web/games-hosted/games/junk-runner/index.html, .data/ai_rankings_cache.json, web/games-hosted/games/junk-runner/assets/{index-Bo_QmhmO.css => index-Dijw7Gvv.css}] DELTA:+5/-5 | BIN:2
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/avid.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/avid.git.txt:14:* main                                     8d1f698 [origin/main] 19 file(s) in examples,src,tests [src/cli.rs, src/ai_gen.rs, src/analyzer.rs] DELTA:+1034/-453 | TEST:81
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/avid.git.txt:16:8d1f698 19 file(s) in examples,src,tests [src/cli.rs, src/ai_gen.rs, src/analyzer.rs] DELTA:+1034/-453 | TEST:81
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/youtube-video-uploader.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/youtube-video-uploader.git.txt:13:* main 771d422 [origin/main] Merge https://github.com/DraconDev/youtube-video-uploader
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/video-uploader.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/video-uploader.git.txt:13:* main 9d5e9f1 [origin/main] 2 file(s) in youtube-uploader-cli [youtube-uploader-cli/tests/cli.rs, youtube-uploader-cli/src/main.rs] DELTA:+5/-3 | TEST:6
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/video-uploader.git.txt:15:9d5e9f1 2 file(s) in youtube-uploader-cli [youtube-uploader-cli/tests/cli.rs, youtube-uploader-cli/src/main.rs] DELTA:+5/-3 | TEST:6
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/video-uploader.git.txt:17:b630d5f 4 file(s) in youtube-uploader,youtube-uploader-cli [youtube-uploader/src/youtube.rs, youtube-uploader/src/config.rs, youtube-uploader-cli/src/main.rs] DELTA:+80/-40
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/pre-inventory.json:8:  "failures": 0,
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/pre-inventory.json:15:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/pre-inventory.json:31:      "hint": "run repair-warns --apply"
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/pre-inventory.json:34:      "repo": "/home/dracon/Dev/rust-ai-web-auto",
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/pre-inventory.json:38:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/pre-inventory.json:54:      "hint": "run repair-warns --apply"
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/pre-inventory.json:57:      "repo": "/home/dracon/Dev/dracon-ai-lib",
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/pre-inventory.json:61:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/pre-inventory.json:84:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/pre-inventory.json:93:      "last_msg": "docs: clarify ai-api BYOK gateway role",
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/pre-inventory.json:130:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/pre-inventory.json:153:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/pre-inventory.json:176:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/pre-inventory.json:199:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/pre-inventory.json:222:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/pre-inventory.json:268:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/pre-inventory.json:291:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/pre-inventory.json:307:      "hint": "run repair-warns --apply"
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/pre-inventory.json:310:      "repo": "/home/dracon/Dev/ai-auto-writer",
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/pre-inventory.json:314:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/pre-inventory.json:333:      "repo": "/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent",
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/pre-inventory.json:337:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/pre-inventory.json:360:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/Junk-Runner-bevy.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/Junk-Runner-bevy.git.txt:14:  main        e1894697f [origin/main] Added SOLID_VS_SVELTE.md
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/dracon-code.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/dracon-code.git.txt:13:  backup-main-20260513                             13262567 security(dependency configuration): Updated dependency configuration in `deny.toml` for security and comp...
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/dracon-code.git.txt:14:  bevy-version                                     ef86290b [gui+src|wip] screenshot viewer, task persistence, fetch denylist UI, gui_refresh_secs poll wiring, ai_actions in plan prompt, dead code cleanup
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/dracon-code.git.txt:18:  egui-version                                     0de221d8 {"schema":"dracon.commit.v2","schema_rev":2,"commit_kind":"sync_event","actor":"dracon-sync","generator":{"name":"dracon-git","version":"0.1.0"},"event_fingerprint":"bcc7462f0ab438a932e8482e31fc41ac25fb3d82d26d0a96f0d53304e52a706b","ts":"1771992484","repo":"dracon-code","branch":"master","files":{"added":0,"modified":3,"deleted":0,"renamed":0,"type_change":0,"unknown":0},"changed_paths_full":["Cargo.lock","Cargo.toml","gui/src/main.rs"],"changed_paths":["Cargo.lock","Cargo.toml","gui/src/main.rs"],"changed_paths_total":3,"changed_paths_truncated":false,"top_level_scopes":[{"key":"Cargo.lock","count":1},{"key":"Cargo.toml","count":1},{"key":"gui","count":1}],"extension_summary":[{"key":"lock","count":1},{"key":"rs","count":1},{"key":"toml","count":1}],"domain_summary":[{"key":"code","count":1},{"key":"config","count":1},{"key":"lockfile","count":1}],"intent_tags":["behavior_change_possible","compiled_or_runtime_code_touched","configuration_update","dependency_lock_changed"],"risk_flags":["build_graph_or_dependency_surface"],"semantic":{"files_analyzed":1,"files_skipped":2,"symbols_total":74,"symbols_truncated":false,"symbols":[{"path":"gui/src/main.rs","language":"rust","name":"main","kind":"function","start_line":11,"end_line":25},{"path":"gui/src/main.rs","language":"rust","name":"GuiRuntimeConfig","kind":"struct","start_line":28,"end_line":33},{"path":"gui/src/main.rs","language":"rust","name":"DraconConfigFile","kind":"struct","start_line":36,"end_line":44},{"path":"gui/src/main.rs","language":"rust","name":"Screen","kind":"enum","start_line":47,"end_line":51},{"path":"gui/src/main.rs","language":"rust","name":"Screen","kind":"impl","start_line":53,"end_line":61},{"path":"gui/src/main.rs","language":"rust","name":"label","kind":"function","start_line":54,"end_line":60},{"path":"gui/src/main.rs","language":"rust","name":"HubSort","kind":"enum","start_line":64,"end_line":70},{"path":"gui/src/main.rs","language":"rust","name":"HubSort","kind":"impl","start_line":72,"end_line":82},{"path":"gui/src/main.rs","language":"rust","name":"label","kind":"function","start_line":73,"end_line":81},{"path":"gui/src/main.rs","language":"rust","name":"HubFilter","kind":"enum","start_line":85,"end_line":89},{"path":"gui/src/main.rs","language":"rust","name":"HubFilter","kind":"impl","start_line":91,"end_line":99},{"path":"gui/src/main.rs","language":"rust","name":"label","kind":"function","start_line":92,"end_line":98},{"path":"gui/src/main.rs","language":"rust","name":"SettingsForm","kind":"struct","start_line":102,"end_line":110},{"path":"gui/src/main.rs","language":"rust","name":"SettingsForm","kind":"impl","start_line":112,"end_line":179},{"path":"gui/src/main.rs","language":"rust","name":"from_body","kind":"function","start_line":113,"end_line":132},{"path":"gui/src/main.rs","language":"rust","name":"apply_to_body","kind":"function","start_line":134,"end_line":178},{"path":"gui/src/main.rs","language":"rust","name":"OperatorApp","kind":"struct","start_line":181,"end_line":198},{"path":"gui/src/main.rs","language":"rust","name":"OperatorApp","kind":"impl","start_line":200,"end_line":708},{"path":"gui/src/main.rs","language":"rust","name":"new","kind":"function","start_line":201,"end_line":240},{"path":"gui/src/main.rs","language":"rust","name":"refresh","kind":"function","start_line":242,"end_line":271},{"path":"gui/src/main.rs","language":"rust","name":"run_action","kind":"function","start_line":273,"end_line":280},{"path":"gui/src/main.rs","language":"rust","name":"save_config","kind":"function","start_line":282,"end_line":304},{"path":"gui/src/main.rs","language":"rust","name":"sorted_hub_rows","kind":"function","start_line":306,"end_line":348},{"path":"gui/src/main.rs","language":"rust","name":"nav_row","kind":"function","start_line":350,"end_line":365},{"path":"gui/src/main.rs","language":"rust","name":"project_screen","kind":"function","start_line":367,"end_line":450},{"path":"gui/src/main.rs","language":"rust","name":"hub_screen","kind":"function","start_line":452,"end_line":533},{"path":"gui/src/main.rs","language":"rust","name":"settings_screen","kind":"function","start_line":535,"end_line":707},{"path":"gui/src/main.rs","language":"rust","name":"OperatorApp","kind":"impl","start_line":710,"end_line":769},{"path":"gui/src/main.rs","language":"rust","name":"update","kind":"function","start_line":711,"end_line":768},{"path":"gui/src/main.rs","language":"rust","name":"apply_theme","kind":"function","start_line":771,"end_line":818},{"path":"gui/src/main.rs","language":"rust","name":"panel","kind":"function","start_line":820,"end_line":835},{"path":"gui/src/main.rs","language":"rust","name":"screen_title","kind":"function","start_line":837,"end_line":851},{"path":"gui/src/main.rs","language":"rust","name":"paint_background","kind":"function","start_line":853,"end_line":888},{"path":"gui/src/main.rs","language":"rust","name":"kv","kind":"function","start_line":890,"end_line":895},{"path":"gui/src/main.rs","language":"rust","name":"status_chip","kind":"function","start_line":897,"end_line":909},{"path":"gui/src/main.rs","language":"rust","name":"action_button","kind":"function","start_line":911,"end_line":928},{"path":"gui/src/main.rs","language":"rust","name":"tab_button","kind":"function","start_line":930,"end_line":952},{"path":"gui/src/main.rs","language":"rust","name":"chip_button","kind":"function","start_line":954,"end_line":969},{"path":"gui/src/main.rs","language":"rust","name":"truncate_middle","kind":"function","start_line":971,"end_line":978},{"path":"gui/src/main.rs","language":"rust","name":"draw_projects_table","kind":"function","start_line":980,"end_line":1065},{"path":"gui/src/main.rs","language":"rust","name":"draw_hub_table","kind":"function","start_line":1067,"end_line":1180},{"path":"gui/src/main.rs","language":"rust","name":"table_header","kind":"function","start_line":1182,"end_line":1190},{"path":"gui/src/main.rs","language":"rust","name":"table_row_bg","kind":"function","start_line":1192,"end_line":1198},{"path":"gui/src/main.rs","language":"rust","name":"is_active_repo","kind":"function","start_line":1200,"end_line":1205},{"path":"gui/src/main.rs","language":"rust","name":"phase_color","kind":"function","start_line":1207,"end_line":1217},{"path":"gui/src/main.rs","language":"rust","name":"trigger_color","kind":"function","start_line":1219,"end_line":1225},{"path":"gui/src/main.rs","language":"rust","name":"git_state_color","kind":"function","start_line":1227,"end_line":1241},{"path":"gui/src/main.rs","language":"rust","name":"FleetView","kind":"struct","start_line":1244,"end_line":1247},{"path":"gui/src/main.rs","language":"rust","name":"FleetRow","kind":"struct","start_line":1250,"end_line":1259},{"path":"gui/src/main.rs","language":"rust","name":"FleetRow","kind":"impl","start_line":1261,"end_line":1273},{"path":"gui/src/main.rs","language":"rust","name":"active_slice_label","kind":"function","start_line":1262,"end_line":1266},{"path":"gui/src/main.rs","language":"rust","name":"updated_label","kind":"function","start_line":1268,"end_line":1272},{"path":"gui/src/main.rs","language":"rust","name":"merge_discovered_repos","kind":"function","start_line":1275,"end_line":1291},{"path":"gui/src/main.rs","language":"rust","name":"compute_git_states","kind":"function","start_line":1293,"end_line":1297},{"path":"gui/src/main.rs","language":"rust","name":"git_state_for_repo","kind":"function","start_line":1299,"end_line":1324},{"path":"gui/src/main.rs","language":"rust","name":"parse_branch_sync","kind":"function","start_line":1326,"end_line":1348},{"path":"gui/src/main.rs","language":"rust","name":"discover_git_repos","kind":"function","start_line":1350,"end_line":1363},{"path":"gui/src/main.rs","language":"rust","name":"walk_for_git_repos","kind":"function","start_line":1365,"end_line":1407},{"path":"gui/src/main.rs","language":"rust","name":"refresh_view","kind":"function","start_line":1409,"end_line":1432},{"path":"gui/src/main.rs","language":"rust","name":"choose_selected_repo","kind":"function","start_line":1434,"end_line":1457},{"path":"gui/src/main.rs","language":"rust","name":"load_fleet_rows","kind":"function","start_line":1459,"end_line":1501},{"path":"gui/src/main.rs","language":"rust","name":"load_fleet_rows_sqlite","kind":"function","start_line":1503,"end_line":1547},{"path":"gui/src/main.rs","language":"rust","name":"load_gui_runtime_config","kind":"function","start_line":1549,"end_line":1580},{"path":"gui/src/main.rs","language":"rust","name":"default_fleet_db_path","kind":"function","start_line":1582,"end_line":1584},{"path":"gui/src/main.rs","language":"rust","name":"expand_tilde","kind":"function","start_line":1586,"end_line":1596},{"path":"gui/src/main.rs","language":"rust","name":"read_text_file","kind":"function","start_line":1598,"end_line":1601},{"path":"gui/src/main.rs","language":"rust","name":"run_json","kind":"function","start_line":1603,"end_line":1611},{"path":"gui/src/main.rs","language":"rust","name":"run_cmd","kind":"function","start_line":1613,"end_line":1619},{"path":"gui/src/main.rs","language":"rust","name":"run_cmd_in","kind":"function","start_line":1621,"end_line":1627},{"path":"gui/src/main.rs","language":"rust","name":"run_cmd_capture","kind":"function","start_line":1629,"end_line":1652},{"path":"gui/src/main.rs","language":"rust","name":"resolve_default_project","kind":"function","start_line":1654,"end_line":1659},{"path":"gui/src/main.rs","language":"rust","name":"append_log","kind":"function","start_line":1661,"end_line":1666},{"path":"gui/src/main.rs","language":"rust","name":"now_secs","kind":"function","start_line":1668,"end_line":1673},{"path":"gui/src/main.rs","language":"rust","name":"format_ts","kind":"function","start_line":1675,"end_line":1677}],"kind_summary":[{"key":"function","count":58},{"key":"impl","count":7},{"key":"struct","count":6},{"key":"enum","count":3}],"language_summary":[{"key":"rust","count":74}]},"status":{"ahead":0,"behind":0,"modified_files":1,"staged_files":0},"policy":{"deterministic":true,"ai_commit_messages":false}}
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/dracon-code.git.txt:20:* main                                             664aba62 [origin/main] 1 file(s) [COMPARATIVE_AUDIT.md] DELTA:+1/-1
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/dracon-code.git.txt:22:  temp-main                                        e1eaa26d Reset to origin/master
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/dracon-code.git.txt:29:1fc9373f 4 file(s) in docs,plan [docs/AI-LIB-AUDIT.md, docs/README.md, docs/AI-STRATEGY.md] DELTA:+154/-1 | NEW:docs/AI-LIB-AUDIT.md
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.browser-extensions-shared.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.browser-extensions-shared.git.txt:24:* main 1034dd58b [origin/main] 17 file(s) in auto-form-filler,job-finder,vidpro-extension [auto-form-filler/.audit-ui/ui-ux-audit-output.json, job-finder/utils/ics.ts, auto-form-filler/.audit-ui/functional-smoke-output.json] DELTA:+462/-40 | TEST:64 BIN:1 NEW:.audit-ui/functional-smoke-output.err,.audit-ui/functional-smoke-output.json,.audit-ui/functional-smoke-output.txt,.audit-ui/ui-ux-audit-output.err,.audit-ui/ui-ux-audit-output.json,tests/ics.test.ts,tests/jobCapture.test.ts,utils/ics.ts
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.browser-extensions-shared.git.txt:29:5d18602f7 4 file(s) in job-finder [job-finder/docs/ROADMAP_TODO.md, job-finder/README.md, job-finder/docs/AI_ERA_STRATEGY.md] DELTA:+353/-0 | NEW:docs/ROADMAP_TODO.md
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.rust-ai-web-auto.git.txt:1:REPO=/home/dracon/Dev/rust-ai-web-auto
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.rust-ai-web-auto.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.rust-ai-web-auto.git.txt:8:codeberg	git@codeberg.org:dracondev/rust-ai-web-auto.git (fetch)
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.rust-ai-web-auto.git.txt:9:codeberg	git@codeberg.org:dracondev/rust-ai-web-auto.git (push)
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.rust-ai-web-auto.git.txt:10:github	git@github.com:DraconDev/rust-ai-web-auto.git (fetch)
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.rust-ai-web-auto.git.txt:11:github	git@github.com:DraconDev/rust-ai-web-auto.git (push)
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.rust-ai-web-auto.git.txt:12:gitlab	git@gitlab.com:dracondev/rust-ai-web-auto.git (fetch)
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.rust-ai-web-auto.git.txt:13:gitlab	git@gitlab.com:dracondev/rust-ai-web-auto.git (push)
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.rust-ai-web-auto.git.txt:14:origin	https://github.com/DraconDev/rust-ai-web-auto.git (fetch)
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.rust-ai-web-auto.git.txt:15:origin	https://github.com/DraconDev/rust-ai-web-auto.git (push)
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.rust-ai-web-auto.git.txt:17:* main f27cc05 [origin/main] 2 file(s) in docs [docs/current-workflows.md, README.md] DELTA:+368/-0 | NEW:docs/current-workflows.md
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/ai-auto-repo-rot-scanner-todo-agent.git.txt:1:REPO=/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/ai-auto-repo-rot-scanner-todo-agent.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/ai-auto-repo-rot-scanner-todo-agent.git.txt:4:codeberg	git@codeberg.org:dracondev/ai-auto-repo-rot-scanner-todo-agent.git (fetch)
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/ai-auto-repo-rot-scanner-todo-agent.git.txt:5:codeberg	git@codeberg.org:dracondev/ai-auto-repo-rot-scanner-todo-agent.git (push)
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/ai-auto-repo-rot-scanner-todo-agent.git.txt:6:github	git@github.com:DraconDev/ai-auto-repo-rot-scanner-todo-agent.git (fetch)
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/ai-auto-repo-rot-scanner-todo-agent.git.txt:7:github	git@github.com:DraconDev/ai-auto-repo-rot-scanner-todo-agent.git (push)
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/ai-auto-repo-rot-scanner-todo-agent.git.txt:8:gitlab	git@gitlab.com:dracondev/ai-auto-repo-rot-scanner-todo-agent.git (fetch)
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/ai-auto-repo-rot-scanner-todo-agent.git.txt:9:gitlab	git@gitlab.com:dracondev/ai-auto-repo-rot-scanner-todo-agent.git (push)
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/ai-auto-repo-rot-scanner-todo-agent.git.txt:10:origin	https://github.com/DraconDev/ai-auto-repo-rot-scanner-todo-agent.git (fetch)
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/ai-auto-repo-rot-scanner-todo-agent.git.txt:11:origin	https://github.com/DraconDev/ai-auto-repo-rot-scanner-todo-agent.git (push)
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/ai-auto-repo-rot-scanner-todo-agent.git.txt:13:* main 9607985 [origin/main] 1 file(s) in docs [docs/AUDIT-2026-06-10.md] DELTA:+527/-0 | NEW:docs/AUDIT-2026-06-10.md
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/ai-auto-repo-rot-scanner-todo-agent.git.txt:16:90c4433 refactor(ai): migrate from archived dracon-ai-lib to ai-api-sdk
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/ai-auto-repo-rot-scanner-todo-agent.git.txt:18:c70e485 1 file(s) in src [src/ai/mod.rs] DELTA:+1/-1
docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/ai-auto-repo-rot-scanner-todo-agent.git.txt:19:eb44a00 1 file(s) in src [src/ai/mod.rs] DELTA:+1/-4
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.folder-auto-banner.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.folder-auto-banner.git.txt:13:* main cf497b7 [origin/main] 1 file(s) [RELEASE_NOTES_0.6.17.md] DELTA:+12/-0 | NEW:RELEASE_NOTES_0.6.17.md
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.dracon-ai-lib.git.txt:1:REPO=/home/dracon/Dev/dracon-ai-lib
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.dracon-ai-lib.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.dracon-ai-lib.git.txt:3:1 .M N... 100644 100644 100644 cca767eb58fa89e9d8691b95fa4f387af29145cb cca767eb58fa89e9d8691b95fa4f387af29145cb crates/ai-lib/src/providers/minimax.rs
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.dracon-ai-lib.git.txt:4:1 .M N... 100644 100644 100644 dcff46013f85496737c582f1aa326eea83dd0a60 dcff46013f85496737c582f1aa326eea83dd0a60 crates/ai-lib/src/providers/openai.rs
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.dracon-ai-lib.git.txt:6:origin	https://github.com/DraconDev/dracon-ai-lib.git (fetch)
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.dracon-ai-lib.git.txt:7:origin	https://github.com/DraconDev/dracon-ai-lib.git (push)
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.dracon-ai-lib.git.txt:9:* main d8846da [origin/main: ahead 21] docs: make crate docs explicit BYOK-library contract
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.dracon-ai-lib.git.txt:15:6882198 simplify: drop the dracon-ai/* cutover theater; use the real repo URL
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:6:tracked	SamAI/.dracon/data/keys/owner_age1f7y5.pub
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:7:tracked	SamAI/.dracon/data/keys/owner_nixos.pub
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:8:tracked	SamAI/.env
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:9:tracked	SamAI/.env.example
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:10:tracked	SamAI/.env.production
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:11:tracked	SamAI/ai-job-finder/.ralph/deep-bug-fix-loop.md
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:12:tracked	SamAI/ai-job-finder/.ralph/deep-bug-fix-loop.state.json
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:13:tracked	SamAI/ai-job-finder/.ralph/fix-review-bugs.md
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:14:tracked	SamAI/ai-job-finder/.ralph/fix-review-bugs.state.json
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:15:tracked	SamAI/ai-job-finder/.ralph/fix-review-round2.md
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:16:tracked	SamAI/ai-job-finder/.ralph/fix-review-round2.state.json
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:17:tracked	SamAI/ai-job-finder/.ralph/full-polish-loop.md
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:18:tracked	SamAI/ai-job-finder/.ralph/full-polish-loop.state.json
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:19:tracked	SamAI/ai-job-finder/.ralph/full-redesign.md
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:20:tracked	SamAI/ai-job-finder/.ralph/full-redesign.state.json
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:21:tracked	SamAI/ai-job-finder/.ralph/loop-todos.md
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:22:tracked	SamAI/ai-job-finder/.ralph/loop-todos.state.json
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:23:tracked	SamAI/ai-job-finder/.ralph/make-it-work.md
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:24:tracked	SamAI/ai-job-finder/.ralph/make-it-work.state.json
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:25:tracked	SamAI/ai-job-finder/.ralph/next-pass-audit-fix.md
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:26:tracked	SamAI/ai-job-finder/.ralph/next-pass-audit-fix.state.json
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:27:tracked	SamAI/ai-job-finder/.ralph/next-phase-features.md
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:28:tracked	SamAI/ai-job-finder/.ralph/next-phase-features.state.json
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:29:tracked	SamAI/ai-job-finder/.ralph/options-overhaul-models.md
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:30:tracked	SamAI/ai-job-finder/.ralph/options-overhaul-models.state.json
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:31:tracked	SamAI/ai-job-finder/.ralph/popup-dark-redesign.md
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:32:tracked	SamAI/ai-job-finder/.ralph/popup-dark-redesign.state.json
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:33:tracked	SamAI/ai-job-finder/.ralph/remove-server-byok-only.md
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:34:tracked	SamAI/ai-job-finder/.ralph/remove-server-byok-only.state.json
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:35:tracked	SamAI/ai-job-finder/.ralph/ui-ux-improvements.md
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:36:tracked	SamAI/ai-job-finder/.ralph/ui-ux-improvements.state.json
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:37:tracked	SamAI/ai-job-finder/public/icon/128.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:38:tracked	SamAI/ai-job-finder/public/icon/16.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:39:tracked	SamAI/ai-job-finder/public/icon/32.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:40:tracked	SamAI/ai-job-finder/public/icon/48.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:41:tracked	SamAI/ai-job-finder/public/icon/96.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:42:tracked	SamAI/ai-job-finder/public/wxt.svg
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:43:tracked	SamAI/ai-job-finder/server/.env.example
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:44:tracked	SamAI/ai-job-finder/src/lib/styles/tokens.css
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:45:tracked	SamAI/assets/unnamed (1).png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:46:tracked	SamAI/assets/unnamed (2).png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:47:tracked	SamAI/assets/unnamed (3).png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:48:tracked	SamAI/assets/unnamed (4).png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:49:tracked	SamAI/assets/unnamed.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:50:tracked	SamAI/coverage/block-navigation.js
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:51:tracked	SamAI/coverage/favicon.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:52:tracked	SamAI/coverage/services/background/handlers/navigation.ts.html
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:53:tracked	SamAI/coverage/sort-arrow-sprite.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:54:tracked	SamAI/coverage/utils/formFiller/profileMapper.ts.html
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:55:tracked	SamAI/coverage/utils/formProfileTemplates.json.html
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:56:tracked	SamAI/coverage/utils/simpleFormProfiles.ts.html
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:57:tracked	SamAI/docs/CHROME_STORE_LISTING.md
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:58:tracked	SamAI/docs/FIREFOX_AMO_GUIDE.md
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:59:tracked	SamAI/docs/FIREFOX_SUBMISSION_GUIDE.md
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:60:tracked	SamAI/docs/assets/screenshots/1.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:61:tracked	SamAI/docs/assets/screenshots/2.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:62:tracked	SamAI/docs/assets/screenshots/3.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:63:tracked	SamAI/docs/assets/screenshots/4.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:64:tracked	SamAI/docs/assets/screenshots/5.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:65:tracked	SamAI/docs/assets/screenshots/SCREENSHOTS.md
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:66:tracked	SamAI/docs/assets/screenshots/t1.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:67:tracked	SamAI/entrypoints/profile-editor-page/index.html
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:68:tracked	SamAI/entrypoints/profile-editor-page/main.tsx
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:69:tracked	SamAI/entrypoints/profiles-page/index.html
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:70:tracked	SamAI/entrypoints/profiles-page/main.tsx
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:71:tracked	SamAI/public/1.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:72:tracked	SamAI/public/2.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:73:tracked	SamAI/public/3.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:74:tracked	SamAI/public/4.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:75:tracked	SamAI/public/440x280.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:76:tracked	SamAI/public/5.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:77:tracked	SamAI/public/icon/128.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:78:tracked	SamAI/public/icon/16.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:79:tracked	SamAI/public/icon/32.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:80:tracked	SamAI/public/icon/48.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:81:tracked	SamAI/public/icon/96.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:82:tracked	SamAI/public/wxt.svg
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:83:tracked	SamAI/services/background/handlers/navigation.ts
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:84:tracked	SamAI/src/content/SearchPanel/components/ProfileEditorPage.css
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:85:tracked	SamAI/src/content/SearchPanel/components/ProfileEditorPage.tsx
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:86:tracked	SamAI/src/content/SearchPanel/components/ProfilesPage.css
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:87:tracked	SamAI/src/content/SearchPanel/components/ProfilesPage.tsx
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:88:tracked	SamAI/src/content/SearchPanel/components/TabNavigation.tsx
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:89:tracked	SamAI/test/profile-editor-page.test.tsx
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:90:tracked	SamAI/test/profileMapper.test.ts
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:91:tracked	SamAI/test/simpleFormProfiles.test.ts
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:92:tracked	SamAI/utils/formFiller/profileMapper.ts
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:93:tracked	SamAI/utils/formProfileTemplates.json
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:94:tracked	SamAI/utils/simpleFormProfiles.ts
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:95:tracked	ai-ats/.dracon/data/keys/owner_age1f7y5.pub
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:96:tracked	ai-ats/.dracon/data/keys/owner_nixos.pub
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:97:tracked	ai-ats/.env
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:98:tracked	ai-ats/.env.example
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:99:tracked	ai-ats/docs/FIREFOX_AMO_GUIDE.md
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:100:tracked	ai-ats/docs/FIREFOX_SUBMISSION_GUIDE.md
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:101:tracked	ai-ats/public/icon/128.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:102:tracked	ai-ats/public/icon/16.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:103:tracked	ai-ats/public/icon/32.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:104:tracked	ai-ats/public/icon/48.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:105:tracked	ai-ats/public/icon/96.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:106:tracked	ai-ats/public/pdf.worker.min.mjs
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:107:tracked	ai-ats/public/wxt.svg
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:108:tracked	ai-ats/test-cvs/cv4_david_kim.txt
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:109:tracked	ai-ats/test-cvs/final/cv4_david_kim.txt
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:110:tracked	ai-ats/test-cvs/mixed/cv4_david_kim.txt
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:111:tracked	ai-ats/test-files/cv-david-kim.txt
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:196:tracked	auto-form-filler/.audit-ui/functional-profile-debug3/Default/Local Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/CURRENT
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:197:tracked	auto-form-filler/.audit-ui/functional-profile-debug3/Default/Local Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOCK
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:198:tracked	auto-form-filler/.audit-ui/functional-profile-debug3/Default/Local Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOG
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:199:tracked	auto-form-filler/.audit-ui/functional-profile-debug3/Default/Local Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOG.old
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:200:tracked	auto-form-filler/.audit-ui/functional-profile-debug3/Default/Local Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/MANIFEST-000001
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:263:tracked	auto-form-filler/.audit-ui/functional-profile-debug3/Default/Sync Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/CURRENT
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:264:tracked	auto-form-filler/.audit-ui/functional-profile-debug3/Default/Sync Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOCK
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:265:tracked	auto-form-filler/.audit-ui/functional-profile-debug3/Default/Sync Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOG
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:266:tracked	auto-form-filler/.audit-ui/functional-profile-debug3/Default/Sync Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOG.old
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:267:tracked	auto-form-filler/.audit-ui/functional-profile-debug3/Default/Sync Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/MANIFEST-000001
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:383:tracked	auto-form-filler/.audit-ui/functional-profile-debug4/Default/Local Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/CURRENT
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:384:tracked	auto-form-filler/.audit-ui/functional-profile-debug4/Default/Local Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOCK
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:385:tracked	auto-form-filler/.audit-ui/functional-profile-debug4/Default/Local Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOG
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:386:tracked	auto-form-filler/.audit-ui/functional-profile-debug4/Default/Local Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/MANIFEST-000001
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:440:tracked	auto-form-filler/.audit-ui/functional-profile-debug4/Default/Sync Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/CURRENT
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:441:tracked	auto-form-filler/.audit-ui/functional-profile-debug4/Default/Sync Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOCK
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:442:tracked	auto-form-filler/.audit-ui/functional-profile-debug4/Default/Sync Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOG
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:443:tracked	auto-form-filler/.audit-ui/functional-profile-debug4/Default/Sync Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/MANIFEST-000001
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:558:tracked	auto-form-filler/.audit-ui/functional-profile-debug5/Default/Local Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/CURRENT
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:559:tracked	auto-form-filler/.audit-ui/functional-profile-debug5/Default/Local Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOCK
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:560:tracked	auto-form-filler/.audit-ui/functional-profile-debug5/Default/Local Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOG
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:561:tracked	auto-form-filler/.audit-ui/functional-profile-debug5/Default/Local Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOG.old
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:562:tracked	auto-form-filler/.audit-ui/functional-profile-debug5/Default/Local Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/MANIFEST-000001
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:627:tracked	auto-form-filler/.audit-ui/functional-profile-debug5/Default/Sync Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/CURRENT
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:628:tracked	auto-form-filler/.audit-ui/functional-profile-debug5/Default/Sync Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOCK
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:629:tracked	auto-form-filler/.audit-ui/functional-profile-debug5/Default/Sync Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOG
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:630:tracked	auto-form-filler/.audit-ui/functional-profile-debug5/Default/Sync Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOG.old
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:631:tracked	auto-form-filler/.audit-ui/functional-profile-debug5/Default/Sync Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/MANIFEST-000001
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:747:tracked	auto-form-filler/.audit-ui/functional-profile-debug6/Default/Local Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/CURRENT
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:748:tracked	auto-form-filler/.audit-ui/functional-profile-debug6/Default/Local Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOCK
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:749:tracked	auto-form-filler/.audit-ui/functional-profile-debug6/Default/Local Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOG
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:750:tracked	auto-form-filler/.audit-ui/functional-profile-debug6/Default/Local Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/MANIFEST-000001
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:804:tracked	auto-form-filler/.audit-ui/functional-profile-debug6/Default/Sync Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/CURRENT
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:805:tracked	auto-form-filler/.audit-ui/functional-profile-debug6/Default/Sync Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOCK
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:806:tracked	auto-form-filler/.audit-ui/functional-profile-debug6/Default/Sync Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOG
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:807:tracked	auto-form-filler/.audit-ui/functional-profile-debug6/Default/Sync Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/MANIFEST-000001
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:916:tracked	auto-form-filler/.audit-ui/functional-profile-debug7/Default/Local Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/CURRENT
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:917:tracked	auto-form-filler/.audit-ui/functional-profile-debug7/Default/Local Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOCK
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:918:tracked	auto-form-filler/.audit-ui/functional-profile-debug7/Default/Local Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOG
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:919:tracked	auto-form-filler/.audit-ui/functional-profile-debug7/Default/Local Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/MANIFEST-000001
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:973:tracked	auto-form-filler/.audit-ui/functional-profile-debug7/Default/Sync Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/CURRENT
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:974:tracked	auto-form-filler/.audit-ui/functional-profile-debug7/Default/Sync Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOCK
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:975:tracked	auto-form-filler/.audit-ui/functional-profile-debug7/Default/Sync Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOG
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:976:tracked	auto-form-filler/.audit-ui/functional-profile-debug7/Default/Sync Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/MANIFEST-000001
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:1082:tracked	auto-form-filler/.audit-ui/functional-profile-debug8/Default/Local Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/CURRENT
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:1083:tracked	auto-form-filler/.audit-ui/functional-profile-debug8/Default/Local Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOCK
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:1084:tracked	auto-form-filler/.audit-ui/functional-profile-debug8/Default/Local Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOG
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:1085:tracked	auto-form-filler/.audit-ui/functional-profile-debug8/Default/Local Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/MANIFEST-000001
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:1139:tracked	auto-form-filler/.audit-ui/functional-profile-debug8/Default/Sync Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/CURRENT
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:1140:tracked	auto-form-filler/.audit-ui/functional-profile-debug8/Default/Sync Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOCK
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:1141:tracked	auto-form-filler/.audit-ui/functional-profile-debug8/Default/Sync Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOG
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:1142:tracked	auto-form-filler/.audit-ui/functional-profile-debug8/Default/Sync Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/MANIFEST-000001
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:1248:tracked	auto-form-filler/.audit-ui/functional-profile-debug9/Default/Local Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/CURRENT
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:1249:tracked	auto-form-filler/.audit-ui/functional-profile-debug9/Default/Local Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOCK
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:1250:tracked	auto-form-filler/.audit-ui/functional-profile-debug9/Default/Local Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOG
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:1251:tracked	auto-form-filler/.audit-ui/functional-profile-debug9/Default/Local Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/MANIFEST-000001
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:1305:tracked	auto-form-filler/.audit-ui/functional-profile-debug9/Default/Sync Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/CURRENT
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:1306:tracked	auto-form-filler/.audit-ui/functional-profile-debug9/Default/Sync Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOCK
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:1307:tracked	auto-form-filler/.audit-ui/functional-profile-debug9/Default/Sync Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOG
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:1308:tracked	auto-form-filler/.audit-ui/functional-profile-debug9/Default/Sync Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/MANIFEST-000001
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:1360:tracked	auto-form-filler/.audit-ui/functional-profile/Default/AutofillAiModelCache/LOCK
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:1361:tracked	auto-form-filler/.audit-ui/functional-profile/Default/AutofillAiModelCache/LOG
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:1436:tracked	auto-form-filler/.audit-ui/functional-profile/Default/Local Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/CURRENT
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:1437:tracked	auto-form-filler/.audit-ui/functional-profile/Default/Local Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOCK
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:1438:tracked	auto-form-filler/.audit-ui/functional-profile/Default/Local Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOG
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:1439:tracked	auto-form-filler/.audit-ui/functional-profile/Default/Local Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOG.old
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:1440:tracked	auto-form-filler/.audit-ui/functional-profile/Default/Local Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/MANIFEST-000001
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:1509:tracked	auto-form-filler/.audit-ui/functional-profile/Default/Sync Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/CURRENT
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:1510:tracked	auto-form-filler/.audit-ui/functional-profile/Default/Sync Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOCK
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:1511:tracked	auto-form-filler/.audit-ui/functional-profile/Default/Sync Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOG
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:1512:tracked	auto-form-filler/.audit-ui/functional-profile/Default/Sync Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOG.old
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:1513:tracked	auto-form-filler/.audit-ui/functional-profile/Default/Sync Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/MANIFEST-000001
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:1631:tracked	auto-form-filler/.audit-ui/functional-profile2/Default/Local Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/CURRENT
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:1632:tracked	auto-form-filler/.audit-ui/functional-profile2/Default/Local Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOCK
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:1633:tracked	auto-form-filler/.audit-ui/functional-profile2/Default/Local Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOG
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:1634:tracked	auto-form-filler/.audit-ui/functional-profile2/Default/Local Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/MANIFEST-000001
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:1688:tracked	auto-form-filler/.audit-ui/functional-profile2/Default/Sync Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/CURRENT
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:1689:tracked	auto-form-filler/.audit-ui/functional-profile2/Default/Sync Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOCK
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:1690:tracked	auto-form-filler/.audit-ui/functional-profile2/Default/Sync Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOG
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:1691:tracked	auto-form-filler/.audit-ui/functional-profile2/Default/Sync Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/MANIFEST-000001
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:1743:tracked	auto-form-filler/.audit-ui/functional-profile3/Default/AutofillAiModelCache/LOCK
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:1744:tracked	auto-form-filler/.audit-ui/functional-profile3/Default/AutofillAiModelCache/LOG
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:1811:tracked	auto-form-filler/.audit-ui/functional-profile3/Default/Local Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/CURRENT
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:1812:tracked	auto-form-filler/.audit-ui/functional-profile3/Default/Local Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOCK
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:1813:tracked	auto-form-filler/.audit-ui/functional-profile3/Default/Local Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOG
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:1814:tracked	auto-form-filler/.audit-ui/functional-profile3/Default/Local Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/MANIFEST-000001
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:1873:tracked	auto-form-filler/.audit-ui/functional-profile3/Default/Sync Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/CURRENT
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:1874:tracked	auto-form-filler/.audit-ui/functional-profile3/Default/Sync Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOCK
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:1875:tracked	auto-form-filler/.audit-ui/functional-profile3/Default/Sync Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOG
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:1876:tracked	auto-form-filler/.audit-ui/functional-profile3/Default/Sync Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/MANIFEST-000001
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:2010:tracked	auto-form-filler/.audit-ui/profile/Default/Local Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/CURRENT
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:2011:tracked	auto-form-filler/.audit-ui/profile/Default/Local Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOCK
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:2012:tracked	auto-form-filler/.audit-ui/profile/Default/Local Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOG
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:2013:tracked	auto-form-filler/.audit-ui/profile/Default/Local Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOG.old
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:2014:tracked	auto-form-filler/.audit-ui/profile/Default/Local Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/MANIFEST-000001
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:2079:tracked	auto-form-filler/.audit-ui/profile/Default/Sync Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/CURRENT
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:2080:tracked	auto-form-filler/.audit-ui/profile/Default/Sync Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOCK
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:2081:tracked	auto-form-filler/.audit-ui/profile/Default/Sync Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOG
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:2082:tracked	auto-form-filler/.audit-ui/profile/Default/Sync Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOG.old
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:2083:tracked	auto-form-filler/.audit-ui/profile/Default/Sync Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/MANIFEST-000001
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:2222:tracked	cursor-style/public/assets/cursors/crosshair-pointer.svg
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:2258:tracked	cursor-style/public/assets/cursors/rain-pointer.svg
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:2408:tracked	death-note-typing-practice/tests/e2e/screenshots/main-menu-after-pause.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:2434:tracked	full-page-screenshot/EXTENSION_CONSTRAINTS.md
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:2443:tracked	full-page-screenshot/entrypoints/editor/main.tsx
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:2447:tracked	full-page-screenshot/entrypoints/popup/main.tsx
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:2461:tracked	full-page-screenshot/tailwind.config.cjs
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:2507:tracked	live-reload-pro/references/jnihajbhpnppcggbcgedagnkighmdlei/IconUnavailable.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:2508:tracked	live-reload-pro/references/jnihajbhpnppcggbcgedagnkighmdlei/IconUnavailable@2x.png
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:2698:untracked	auto-form-filler/.audit-ui/functional-profile-debug10/Default/Local Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/CURRENT
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:2699:untracked	auto-form-filler/.audit-ui/functional-profile-debug10/Default/Local Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOCK
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:2700:untracked	auto-form-filler/.audit-ui/functional-profile-debug10/Default/Local Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOG
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:2701:untracked	auto-form-filler/.audit-ui/functional-profile-debug10/Default/Local Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/MANIFEST-000001
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:2755:untracked	auto-form-filler/.audit-ui/functional-profile-debug10/Default/Sync Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/CURRENT
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:2756:untracked	auto-form-filler/.audit-ui/functional-profile-debug10/Default/Sync Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOCK
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:2757:untracked	auto-form-filler/.audit-ui/functional-profile-debug10/Default/Sync Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOG
docs/audit/2026-06-11-full-repo-audit/final/risk-paths/browser-extensions-shared.risk.tsv:2758:untracked	auto-form-filler/.audit-ui/functional-profile-debug10/Default/Sync Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/MANIFEST-000001
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/post-inventory.json:8:  "failures": 0,
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/post-inventory.json:15:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/post-inventory.json:38:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/post-inventory.json:54:      "hint": "run repair-warns --apply"
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/post-inventory.json:61:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/post-inventory.json:77:      "hint": "run repair-warns --apply"
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/post-inventory.json:80:      "repo": "/home/dracon/Dev/rust-ai-web-auto",
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/post-inventory.json:84:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/post-inventory.json:103:      "repo": "/home/dracon/Dev/dracon-ai-lib",
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/post-inventory.json:107:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/post-inventory.json:130:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/post-inventory.json:139:      "last_msg": "docs: clarify ai-api BYOK gateway role",
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/post-inventory.json:153:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/post-inventory.json:176:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/post-inventory.json:199:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/post-inventory.json:222:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/post-inventory.json:245:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/post-inventory.json:291:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/post-inventory.json:310:      "repo": "/home/dracon/Dev/ai-auto-writer",
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/post-inventory.json:314:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/post-inventory.json:333:      "repo": "/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent",
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/post-inventory.json:337:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/post-inventory.json:360:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.dracon-code.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.dracon-code.git.txt:14:  backup-main-20260513                             13262567 security(dependency configuration): Updated dependency configuration in `deny.toml` for security and comp...
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.dracon-code.git.txt:15:  bevy-version                                     ef86290b [gui+src|wip] screenshot viewer, task persistence, fetch denylist UI, gui_refresh_secs poll wiring, ai_actions in plan prompt, dead code cleanup
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.dracon-code.git.txt:19:  egui-version                                     0de221d8 {"schema":"dracon.commit.v2","schema_rev":2,"commit_kind":"sync_event","actor":"dracon-sync","generator":{"name":"dracon-git","version":"0.1.0"},"event_fingerprint":"bcc7462f0ab438a932e8482e31fc41ac25fb3d82d26d0a96f0d53304e52a706b","ts":"1771992484","repo":"dracon-code","branch":"master","files":{"added":0,"modified":3,"deleted":0,"renamed":0,"type_change":0,"unknown":0},"changed_paths_full":["Cargo.lock","Cargo.toml","gui/src/main.rs"],"changed_paths":["Cargo.lock","Cargo.toml","gui/src/main.rs"],"changed_paths_total":3,"changed_paths_truncated":false,"top_level_scopes":[{"key":"Cargo.lock","count":1},{"key":"Cargo.toml","count":1},{"key":"gui","count":1}],"extension_summary":[{"key":"lock","count":1},{"key":"rs","count":1},{"key":"toml","count":1}],"domain_summary":[{"key":"code","count":1},{"key":"config","count":1},{"key":"lockfile","count":1}],"intent_tags":["behavior_change_possible","compiled_or_runtime_code_touched","configuration_update","dependency_lock_changed"],"risk_flags":["build_graph_or_dependency_surface"],"semantic":{"files_analyzed":1,"files_skipped":2,"symbols_total":74,"symbols_truncated":false,"symbols":[{"path":"gui/src/main.rs","language":"rust","name":"main","kind":"function","start_line":11,"end_line":25},{"path":"gui/src/main.rs","language":"rust","name":"GuiRuntimeConfig","kind":"struct","start_line":28,"end_line":33},{"path":"gui/src/main.rs","language":"rust","name":"DraconConfigFile","kind":"struct","start_line":36,"end_line":44},{"path":"gui/src/main.rs","language":"rust","name":"Screen","kind":"enum","start_line":47,"end_line":51},{"path":"gui/src/main.rs","language":"rust","name":"Screen","kind":"impl","start_line":53,"end_line":61},{"path":"gui/src/main.rs","language":"rust","name":"label","kind":"function","start_line":54,"end_line":60},{"path":"gui/src/main.rs","language":"rust","name":"HubSort","kind":"enum","start_line":64,"end_line":70},{"path":"gui/src/main.rs","language":"rust","name":"HubSort","kind":"impl","start_line":72,"end_line":82},{"path":"gui/src/main.rs","language":"rust","name":"label","kind":"function","start_line":73,"end_line":81},{"path":"gui/src/main.rs","language":"rust","name":"HubFilter","kind":"enum","start_line":85,"end_line":89},{"path":"gui/src/main.rs","language":"rust","name":"HubFilter","kind":"impl","start_line":91,"end_line":99},{"path":"gui/src/main.rs","language":"rust","name":"label","kind":"function","start_line":92,"end_line":98},{"path":"gui/src/main.rs","language":"rust","name":"SettingsForm","kind":"struct","start_line":102,"end_line":110},{"path":"gui/src/main.rs","language":"rust","name":"SettingsForm","kind":"impl","start_line":112,"end_line":179},{"path":"gui/src/main.rs","language":"rust","name":"from_body","kind":"function","start_line":113,"end_line":132},{"path":"gui/src/main.rs","language":"rust","name":"apply_to_body","kind":"function","start_line":134,"end_line":178},{"path":"gui/src/main.rs","language":"rust","name":"OperatorApp","kind":"struct","start_line":181,"end_line":198},{"path":"gui/src/main.rs","language":"rust","name":"OperatorApp","kind":"impl","start_line":200,"end_line":708},{"path":"gui/src/main.rs","language":"rust","name":"new","kind":"function","start_line":201,"end_line":240},{"path":"gui/src/main.rs","language":"rust","name":"refresh","kind":"function","start_line":242,"end_line":271},{"path":"gui/src/main.rs","language":"rust","name":"run_action","kind":"function","start_line":273,"end_line":280},{"path":"gui/src/main.rs","language":"rust","name":"save_config","kind":"function","start_line":282,"end_line":304},{"path":"gui/src/main.rs","language":"rust","name":"sorted_hub_rows","kind":"function","start_line":306,"end_line":348},{"path":"gui/src/main.rs","language":"rust","name":"nav_row","kind":"function","start_line":350,"end_line":365},{"path":"gui/src/main.rs","language":"rust","name":"project_screen","kind":"function","start_line":367,"end_line":450},{"path":"gui/src/main.rs","language":"rust","name":"hub_screen","kind":"function","start_line":452,"end_line":533},{"path":"gui/src/main.rs","language":"rust","name":"settings_screen","kind":"function","start_line":535,"end_line":707},{"path":"gui/src/main.rs","language":"rust","name":"OperatorApp","kind":"impl","start_line":710,"end_line":769},{"path":"gui/src/main.rs","language":"rust","name":"update","kind":"function","start_line":711,"end_line":768},{"path":"gui/src/main.rs","language":"rust","name":"apply_theme","kind":"function","start_line":771,"end_line":818},{"path":"gui/src/main.rs","language":"rust","name":"panel","kind":"function","start_line":820,"end_line":835},{"path":"gui/src/main.rs","language":"rust","name":"screen_title","kind":"function","start_line":837,"end_line":851},{"path":"gui/src/main.rs","language":"rust","name":"paint_background","kind":"function","start_line":853,"end_line":888},{"path":"gui/src/main.rs","language":"rust","name":"kv","kind":"function","start_line":890,"end_line":895},{"path":"gui/src/main.rs","language":"rust","name":"status_chip","kind":"function","start_line":897,"end_line":909},{"path":"gui/src/main.rs","language":"rust","name":"action_button","kind":"function","start_line":911,"end_line":928},{"path":"gui/src/main.rs","language":"rust","name":"tab_button","kind":"function","start_line":930,"end_line":952},{"path":"gui/src/main.rs","language":"rust","name":"chip_button","kind":"function","start_line":954,"end_line":969},{"path":"gui/src/main.rs","language":"rust","name":"truncate_middle","kind":"function","start_line":971,"end_line":978},{"path":"gui/src/main.rs","language":"rust","name":"draw_projects_table","kind":"function","start_line":980,"end_line":1065},{"path":"gui/src/main.rs","language":"rust","name":"draw_hub_table","kind":"function","start_line":1067,"end_line":1180},{"path":"gui/src/main.rs","language":"rust","name":"table_header","kind":"function","start_line":1182,"end_line":1190},{"path":"gui/src/main.rs","language":"rust","name":"table_row_bg","kind":"function","start_line":1192,"end_line":1198},{"path":"gui/src/main.rs","language":"rust","name":"is_active_repo","kind":"function","start_line":1200,"end_line":1205},{"path":"gui/src/main.rs","language":"rust","name":"phase_color","kind":"function","start_line":1207,"end_line":1217},{"path":"gui/src/main.rs","language":"rust","name":"trigger_color","kind":"function","start_line":1219,"end_line":1225},{"path":"gui/src/main.rs","language":"rust","name":"git_state_color","kind":"function","start_line":1227,"end_line":1241},{"path":"gui/src/main.rs","language":"rust","name":"FleetView","kind":"struct","start_line":1244,"end_line":1247},{"path":"gui/src/main.rs","language":"rust","name":"FleetRow","kind":"struct","start_line":1250,"end_line":1259},{"path":"gui/src/main.rs","language":"rust","name":"FleetRow","kind":"impl","start_line":1261,"end_line":1273},{"path":"gui/src/main.rs","language":"rust","name":"active_slice_label","kind":"function","start_line":1262,"end_line":1266},{"path":"gui/src/main.rs","language":"rust","name":"updated_label","kind":"function","start_line":1268,"end_line":1272},{"path":"gui/src/main.rs","language":"rust","name":"merge_discovered_repos","kind":"function","start_line":1275,"end_line":1291},{"path":"gui/src/main.rs","language":"rust","name":"compute_git_states","kind":"function","start_line":1293,"end_line":1297},{"path":"gui/src/main.rs","language":"rust","name":"git_state_for_repo","kind":"function","start_line":1299,"end_line":1324},{"path":"gui/src/main.rs","language":"rust","name":"parse_branch_sync","kind":"function","start_line":1326,"end_line":1348},{"path":"gui/src/main.rs","language":"rust","name":"discover_git_repos","kind":"function","start_line":1350,"end_line":1363},{"path":"gui/src/main.rs","language":"rust","name":"walk_for_git_repos","kind":"function","start_line":1365,"end_line":1407},{"path":"gui/src/main.rs","language":"rust","name":"refresh_view","kind":"function","start_line":1409,"end_line":1432},{"path":"gui/src/main.rs","language":"rust","name":"choose_selected_repo","kind":"function","start_line":1434,"end_line":1457},{"path":"gui/src/main.rs","language":"rust","name":"load_fleet_rows","kind":"function","start_line":1459,"end_line":1501},{"path":"gui/src/main.rs","language":"rust","name":"load_fleet_rows_sqlite","kind":"function","start_line":1503,"end_line":1547},{"path":"gui/src/main.rs","language":"rust","name":"load_gui_runtime_config","kind":"function","start_line":1549,"end_line":1580},{"path":"gui/src/main.rs","language":"rust","name":"default_fleet_db_path","kind":"function","start_line":1582,"end_line":1584},{"path":"gui/src/main.rs","language":"rust","name":"expand_tilde","kind":"function","start_line":1586,"end_line":1596},{"path":"gui/src/main.rs","language":"rust","name":"read_text_file","kind":"function","start_line":1598,"end_line":1601},{"path":"gui/src/main.rs","language":"rust","name":"run_json","kind":"function","start_line":1603,"end_line":1611},{"path":"gui/src/main.rs","language":"rust","name":"run_cmd","kind":"function","start_line":1613,"end_line":1619},{"path":"gui/src/main.rs","language":"rust","name":"run_cmd_in","kind":"function","start_line":1621,"end_line":1627},{"path":"gui/src/main.rs","language":"rust","name":"run_cmd_capture","kind":"function","start_line":1629,"end_line":1652},{"path":"gui/src/main.rs","language":"rust","name":"resolve_default_project","kind":"function","start_line":1654,"end_line":1659},{"path":"gui/src/main.rs","language":"rust","name":"append_log","kind":"function","start_line":1661,"end_line":1666},{"path":"gui/src/main.rs","language":"rust","name":"now_secs","kind":"function","start_line":1668,"end_line":1673},{"path":"gui/src/main.rs","language":"rust","name":"format_ts","kind":"function","start_line":1675,"end_line":1677}],"kind_summary":[{"key":"function","count":58},{"key":"impl","count":7},{"key":"struct","count":6},{"key":"enum","count":3}],"language_summary":[{"key":"rust","count":74}]},"status":{"ahead":0,"behind":0,"modified_files":1,"staged_files":0},"policy":{"deterministic":true,"ai_commit_messages":false}}
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.dracon-code.git.txt:21:* main                                             3e33ca82 [origin/main] 4 file(s) in docs,src [src/plugin_manager.rs, docs/PI2-DEFAULT-PLUGINS-INVENTORY.md, src/app_runtime.rs] DELTA:+320/-15 | NEW:src/plugin_manager.rs
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.dracon-code.git.txt:23:  temp-main                                        e1eaa26d Reset to origin/master
docs/audit/2026-06-11-full-repo-audit/strategy-audit/REPORT.md:4:Scope: the full Dracon project strategy as expressed in the authoritative documents of `dracon-utilities`, cross-referenced against the current implementation, tests, daemon behaviour, recent audit reports, and the fresh sync inventory.
docs/audit/2026-06-11-full-repo-audit/strategy-audit/REPORT.md:20:| `docs/ARCHITECTURE.md` | Service overview, core loop, key design decisions, AI-to-AI commit protocol, IndexLock | Architecture |
docs/audit/2026-06-11-full-repo-audit/strategy-audit/REPORT.md:26:| `docs/design/*.md` | Design notes (CLI print style, warden plaintext sibling, etc.) | Specific design decisions |
docs/audit/2026-06-11-full-repo-audit/strategy-audit/REPORT.md:27:| `AUDIT.md` (720 lines, 2026-06-09) | The full multi-domain audit (the latest, superseded only by the 2026-06-11 partial audits) | Current security & contract posture |
docs/audit/2026-06-11-full-repo-audit/strategy-audit/REPORT.md:41:| 2 | `simple_ai.rs` / `scribe.rs` still compiled | AUDIT.md 1.2 P1 | Both files removed; `rg "scribe_update\|SimpleAiService"` = 0; Cargo features block is now `default = []` with no `scribe`/`ai-bumper` | **Resolved** |
docs/audit/2026-06-11-full-repo-audit/strategy-audit/REPORT.md:47:| 8 | CHANGELOG mentions `scribe` / `ai-bumper` Cargo features | CHANGELOG.md "Unreleased" / 0.112.0 | `dracon-sync/Cargo.toml [features]` has `default = []` only; 0 `#[cfg(feature = "scribe")]` references in source | **CHANGELOG drift P2** |
docs/audit/2026-06-11-full-repo-audit/strategy-audit/REPORT.md:49:| 10 | `IndexLock` coordination | ARCHITECTURE.md / AGENTS.md | `dracon-warden/src/main.rs:946-998`; `dracon-sync/src/sync.rs:2121-2124` | **Consistent** |
docs/audit/2026-06-11-full-repo-audit/strategy-audit/REPORT.md:50:| 11 | No-kill guard | AGENTS.md "CRITICAL INVARIANT" | `dracon-system/src/main.rs:586-587` explicit comment; `rg "SIGKILL\|SIGTERM"` = 0 in `dracon-system/src` | **Consistent** |
docs/audit/2026-06-11-full-repo-audit/strategy-audit/REPORT.md:57:| 18 | Fresh sync inventory | `docs/audit/2026-06-11/release-readiness/REPORT.md` said "no STUCK_PUSH" | Fresh run now shows **2 new STUCK_PUSH** repos: `browser-extensions-shared` (`AHEAD:1,STUCK_PUSH`) and `folder-auto-banner` (`AHEAD:1,STUCK_PUSH`) | **Stale report** (the release-readiness report is from earlier today and does not cover the current state) |
docs/audit/2026-06-11-full-repo-audit/strategy-audit/REPORT.md:63:- **The contract is honest.** The "invisible infrastructure" promise (auto-commit, deterministic messages, no AI at the commit boundary) is implemented and tested. The fingerprint-based scheduling, inactivity delay, IndexLock, mass-deletion guard, and per-URL credential fallback are all real, not aspirational.
docs/audit/2026-06-11-full-repo-audit/strategy-audit/REPORT.md:65:- **Scribe/AI-at-the-commit-boundary is gone.** AUDIT 1.2's P1 was a real risk (LLM-scribed messages and a non-feature-gated LLM client shipped by default). The fix is the right fix: delete the code, align the docs. The only residual is **the CHANGELOG still describing scribe features** (see drift below).
docs/audit/2026-06-11-full-repo-audit/strategy-audit/REPORT.md:68:- **The audit chain is real.** AUDIT.md (720 lines, 2026-06-09) → 8 post-audit reports (2026-06-11) → fresh per-repo evidence on disk → durable. The pattern of writing evidence files alongside the report is the right one and should be the template for any future audits.
docs/audit/2026-06-11-full-repo-audit/strategy-audit/REPORT.md:77:   - `scribe` and `ai-bumper` Cargo features that **do not exist** (`dracon-sync/Cargo.toml [features]` is `default = []`; zero `#[cfg(feature = ...)]` references in source)
docs/audit/2026-06-11-full-repo-audit/strategy-audit/REPORT.md:80:   - `parse_ai_bump_response` and the major-bump cap — code does not exist
docs/audit/2026-06-11-full-repo-audit/strategy-audit/REPORT.md:81:   - `discover_git_repos_recursive` optimization — code path may still exist, but the surrounding "scribe / ai-bumper" context is fictional
docs/audit/2026-06-11-full-repo-audit/strategy-audit/REPORT.md:83:   This is a real P1 because the CHANGELOG is a contractual document: it is what downstream users (and the AI) trust to know what changed. Reading it now, a maintainer would think there is a `scribe` feature flag to test. There is not. The CHANGELOG needs a full rewrite pass to match the actual `0.112.4` release and to keep `[Unreleased]` honest or remove it.
docs/audit/2026-06-11-full-repo-audit/strategy-audit/REPORT.md:85:3. **Stale release-readiness report.** The 2026-06-11 release-readiness report claims "no unexplained CONCERN/STUCK_PUSH remains." The fresh inventory right now shows two new `STUCK_PUSH` repos (`browser-extensions-shared`, `folder-auto-banner`). This is a one-day-stale report, but it is the document the operator is most likely to read first. Recommendation: either (a) re-run and refresh the report, or (b) add a freshness timestamp and a "valid for inventory snapshot YYYY-MM-DDTHH:MM" header so readers know it is point-in-time.
docs/audit/2026-06-11-full-repo-audit/strategy-audit/REPORT.md:89:4. **AGENTS.md test counts are stale** (line 846-850 say 431 / 692, reality is 705 / 9 / 22 suites). The CI workflow does not enforce AGENTS.md accuracy. Recommendation: either drop the hard counts from AGENTS.md (replace with "see latest CI run") or wire a small CI check that fails when AGENTS.md counts and the test binary's own `--list` disagree.
docs/audit/2026-06-11-full-repo-audit/strategy-audit/REPORT.md:105:11. **`docs/design/`** has 8 design notes (cli-print-style, warden-plaintext-sibling, etc.) but `docs/ROADMAP.md` doesn't link to them. Some are historical (the ones that `ROADMAP.md` says are "archived in archive/") but the surviving design notes have no central index.
docs/audit/2026-06-11-full-repo-audit/strategy-audit/REPORT.md:109:13. **AGENTS.md is in the workspace** and tracked; **AUDIT.md** is also in the workspace and tracked. Both are useful but they age. Recommendation: move them under `docs/audit/2026-06-09-full-multi-domain-audit/AUDIT.md` and `docs/strategy/AGENTS.md` (or similar) so the live root is just `README.md`, `CONTRIBUTING.md`, `CHANGELOG.md`, and a thin `docs/STRATEGY.md` that links into dated subdocs. This is the same pattern that already works for the 2026-06-11 audit chain.
docs/audit/2026-06-11-full-repo-audit/strategy-audit/REPORT.md:113:- **No enterprise features.** There is no SSO, no audit log aggregation, no multi-tenant mode, no GUI. This is consistent with the "invisible infrastructure for an AI coder" philosophy and is the right call.
docs/audit/2026-06-11-full-repo-audit/strategy-audit/REPORT.md:114:- **No push-to-deploy.** There is no `webhook`-driven deployment pipeline. The daemon's webhook is one-way (failure notifications). This is consistent with the philosophy.
docs/audit/2026-06-11-full-repo-audit/strategy-audit/REPORT.md:123:| 1 | Rewrite `CHANGELOG.md` so that `0.112.4` and `[Unreleased]` match the actual source. Remove `scribe`/`ai-bumper`/`generate_commit_message`/`parse_ai_bump_response` references that no longer exist. | S | none | **P1** |
docs/audit/2026-06-11-full-repo-audit/strategy-audit/REPORT.md:133:| 11 | Document the new `~/.dracon/secrets/{pat,registry,ai,...}` layout in AGENTS.md (or a linked "Secret layout" section). | S | none | **P3** |
docs/audit/2026-06-11-full-repo-audit/strategy-audit/REPORT.md:136:## Constraints respected
docs/audit/2026-06-11-full-repo-audit/strategy-audit/REPORT.md:153:  - `docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/REPORT.md`
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.kiki-sassy-desktop-announcer.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.kiki-sassy-desktop-announcer.git.txt:13:* main 0155632 [origin/main] 2 file(s) in src [src/journal.rs, src/daemon.rs] DELTA:+2/-4
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after..dracon.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after..dracon.git.txt:13:* main f3daf8503 [origin/main] 3 file(s) in memory,utilities [utilities/sync/dracon-sync.toml, utilities/sync/templates/FUNDING.yml, memory/rag/rag_index.json] DELTA:+8/-7
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.ai-auto-writer.git.txt:1:REPO=/home/dracon/Dev/ai-auto-writer
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.ai-auto-writer.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.ai-auto-writer.git.txt:4:codeberg	git@codeberg.org:dracondev/ai-auto-writer.git (fetch)
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.ai-auto-writer.git.txt:5:codeberg	git@codeberg.org:dracondev/ai-auto-writer.git (push)
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.ai-auto-writer.git.txt:6:github	git@github.com:DraconDev/ai-auto-writer.git (fetch)
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.ai-auto-writer.git.txt:7:github	git@github.com:DraconDev/ai-auto-writer.git (push)
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.ai-auto-writer.git.txt:8:gitlab	git@gitlab.com:dracondev/ai-auto-writer.git (fetch)
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.ai-auto-writer.git.txt:9:gitlab	git@gitlab.com:dracondev/ai-auto-writer.git (push)
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.ai-auto-writer.git.txt:10:origin	https://github.com/DraconDev/ai-auto-writer.git (fetch)
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.ai-auto-writer.git.txt:11:origin	https://github.com/DraconDev/ai-auto-writer.git (push)
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.ai-auto-writer.git.txt:14:* main                               aa5d0ebb [origin/main] 1 file(s) in src [src/services/dracon.rs] DELTA:+4/-32
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.ai-auto-writer.git.txt:19:9c829b43 Merge https://github.com/DraconDev/ai-auto-writer
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.video-factory.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.video-factory.git.txt:13:* main 698a658 [origin/main] 14 file(s) in crates [crates/api/src/routes.rs, crates/core/src/config.rs, crates/worker/src/ffmpeg.rs] DELTA:+225/-162
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.video-factory.git.txt:17:4215e5f 1 file(s) in src [src/main.rs] DELTA:+3/-3
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.video-uploader.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.video-uploader.git.txt:13:* main 9d5e9f1 [origin/main] 2 file(s) in youtube-uploader-cli [youtube-uploader-cli/tests/cli.rs, youtube-uploader-cli/src/main.rs] DELTA:+5/-3 | TEST:6
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.video-uploader.git.txt:15:9d5e9f1 2 file(s) in youtube-uploader-cli [youtube-uploader-cli/tests/cli.rs, youtube-uploader-cli/src/main.rs] DELTA:+5/-3 | TEST:6
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.video-uploader.git.txt:17:b630d5f 4 file(s) in youtube-uploader,youtube-uploader-cli [youtube-uploader/src/youtube.rs, youtube-uploader/src/config.rs, youtube-uploader-cli/src/main.rs] DELTA:+80/-40
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/one-mil-girls-decoded-diff.md:5:plaintext. To inspect real user changes, smudge the HEAD blob and diff.
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/one-mil-girls-decoded-diff.md:11:## main...origin/main
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/one-mil-girls-decoded-diff.md:34: // rather than failing.
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/one-mil-girls-decoded-diff.md:37:-//   - Svelte 5 runes ($state) are not available in raw Bun. We shim $state
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/one-mil-girls-decoded-diff.md:38:+//   - Svelte 5 runes ($state) are not available in raw Bun. Stub $state
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.pully-fully-pull-based-fleet-reconciler.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.pully-fully-pull-based-fleet-reconciler.git.txt:13:* main a2003343 [origin/main] 1 file(s) in fully [fully/bins/fully/src/bootstrap.rs] DELTA:+48/-8
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.pully-fully-pull-based-fleet-reconciler.git.txt:16:f9cc9ffc 8 file(s) in fully,pully,pully-types [fully/bins/fully/src/bootstrap.rs, fully/crates/fully-core/src/fleet_status.rs, fully/bins/fully/src/main.rs] DELTA:+213/-116
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.pully-fully-pull-based-fleet-reconciler.git.txt:17:364735af 6 file(s) in fully,pully [AUDIT_REPORT.md, pully/bins/pully/src/main.rs, pully/README.md] DELTA:+95/-14
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.pully-fully-pull-based-fleet-reconciler.git.txt:18:90575b82 8 file(s) in fully,pully [fully/docs/CLI.md, pully/docs/CLI.md, pully/bins/pully/src/main.rs] DELTA:+436/-203
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.pully-fully-pull-based-fleet-reconciler.git.txt:19:23a92627 5 file(s) in fully,pully [fully/crates/fully-core/src/fleet_status.rs, fully/bins/fully/src/main.rs, pully/crates/pully-core/src/service_reconciler/mod.rs] DELTA:+148/-39
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/inventory.tsv:2:/home/dracon/Dev/one-mil-girls	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/inventory.tsv:3:/home/dracon/Dev/dracon-utilities	main	1	0	1	0	0	DIRTY	OK	run repair-warns --apply
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/inventory.tsv:4:/home/dracon/.dracon	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/inventory.tsv:5:/home/dracon/Dev/browser-extensions-shared	main	9	0	10	0	0	DIRTY	OK	run repair-warns --apply
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/inventory.tsv:6:/home/dracon/Dev/dracon-ai-lib	main	0	0	0	21	0	AHEAD:21,STUCK_PUSH	STUCK	run repair-concerns --apply (push or rewrite)
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/inventory.tsv:7:/home/dracon/Dev/folder-auto-banner	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/inventory.tsv:8:/home/dracon/Dev/dracon-platform	main	7	0	3	0	0	DIRTY	OK	run repair-warns --apply
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/inventory.tsv:10:/home/dracon/Dev/dracon-code	main	3	0	0	0	0	DIRTY	OK	run repair-warns --apply
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/inventory.tsv:11:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/inventory.tsv:12:/home/dracon/Dev/pully-fully-pull-based-fleet-reconciler	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/inventory.tsv:13:/home/dracon/Dev/DraconDev	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/inventory.tsv:14:/home/dracon/Dev/rust-ai-web-auto	main	1	0	1	0	0	DIRTY	OK	run repair-warns --apply
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/inventory.tsv:15:/home/dracon/Dev/ai-auto-writer	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/inventory.tsv:16:/home/dracon/Dev/video-uploader	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/inventory.tsv:17:/home/dracon/Dev/video-factory	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/inventory.tsv:18:/home/dracon/Dev/avid	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/inventory.tsv:19:/home/dracon/Dev/youtube-video-uploader	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/inventory.tsv:20:/home/dracon/Dev/dracon-libs	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/inventory.tsv:21:/home/dracon/Dev/kiki-sassy-desktop-announcer	main	0	0	0	0	0	OK	OK	healthy
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.ai-auto-writer.git.txt:1:REPO=/home/dracon/Dev/ai-auto-writer
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.ai-auto-writer.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.ai-auto-writer.git.txt:4:codeberg	git@codeberg.org:dracondev/ai-auto-writer.git (fetch)
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.ai-auto-writer.git.txt:5:codeberg	git@codeberg.org:dracondev/ai-auto-writer.git (push)
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.ai-auto-writer.git.txt:6:github	git@github.com:DraconDev/ai-auto-writer.git (fetch)
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.ai-auto-writer.git.txt:7:github	git@github.com:DraconDev/ai-auto-writer.git (push)
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.ai-auto-writer.git.txt:8:gitlab	git@gitlab.com:dracondev/ai-auto-writer.git (fetch)
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.ai-auto-writer.git.txt:9:gitlab	git@gitlab.com:dracondev/ai-auto-writer.git (push)
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.ai-auto-writer.git.txt:10:origin	https://github.com/DraconDev/ai-auto-writer.git (fetch)
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.ai-auto-writer.git.txt:11:origin	https://github.com/DraconDev/ai-auto-writer.git (push)
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.ai-auto-writer.git.txt:14:* main                               aa5d0ebb [origin/main] 1 file(s) in src [src/services/dracon.rs] DELTA:+4/-32
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.ai-auto-writer.git.txt:19:9c829b43 Merge https://github.com/DraconDev/ai-auto-writer
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.dracon-libs.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.dracon-libs.git.txt:13:* main 2ff017b [origin/main] 1 file(s) [deny.toml] DELTA:+2/-0
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/one-mil-girls-state.md:7:## main...origin/main
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/one-mil-girls-state.md:31:docs/audit/2026-06-11-cleanup/smoke/11-pause-menu-before-main.png
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/one-mil-girls-state.md:33:docs/audit/2026-06-11-cleanup/smoke/13-main-menu-after-quit.png
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/one-mil-girls-state.md:43:docs/audit/2026-06-11-cleanup/smoke/23-main-menu-before-ending-save.png
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.folder-auto-banner.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.folder-auto-banner.git.txt:13:* main cf497b7 [origin/main] 1 file(s) [RELEASE_NOTES_0.6.17.md] DELTA:+12/-0 | NEW:RELEASE_NOTES_0.6.17.md
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.dracon-ai-lib.git.txt:1:REPO=/home/dracon/Dev/dracon-ai-lib
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.dracon-ai-lib.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.dracon-ai-lib.git.txt:9:origin	https://github.com/DraconDev/dracon-ai-lib.git (fetch)
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.dracon-ai-lib.git.txt:10:origin	https://github.com/DraconDev/dracon-ai-lib.git (push)
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.dracon-ai-lib.git.txt:12:* main a87ab96 [origin/main: ahead 24] 2 file(s) in crates [crates/ai-models-catalog/README.md, crates/ai-lib/README.md] DELTA:+12/-7
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.dracon-ai-lib.git.txt:14:a87ab96 2 file(s) in crates [crates/ai-models-catalog/README.md, crates/ai-lib/README.md] DELTA:+12/-7
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.dracon-ai-lib.git.txt:15:4eccdcc 2 file(s) in crates [crates/ai-lib/src/providers/minimax.rs, crates/ai-lib/src/providers/openai.rs] DELTA:+15/-15
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.dracon-ai-lib.git.txt:16:9cb5103 docs: fix stale ai-lib release tag wording
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/pi-proof/before.Junk-Runner-bevy.pi-files.txt:37:.pi/goals/notes_dev_failu[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBaUDVSbHpuekZDNTVTSWt0bElBMmJRSllDSDRlT3NDNko0STEwRXNHc1hrCndRRFBmZmg2dGtRRHFjTkJUSTZYOGVMT09McXZ5N3JzWTBsRE84NFdVbDQKLT4gWDI1NTE5IDhIZWl3YTNCVWRDL1BzV2VQYXZJaTRyMmJVdnc1bnU3MGZDRnhtSy9IMUkKUEdaUGxJWHh0V0lMaUlnb0l3ejZlOHZVZ29CYlBrZ0RVend5cERGYXo5cwotPiBYMjU1MTkgSHNYRUt4R0JNQ01PRXFsOUVpeVprdzNtN2Nwd3JwdHI5a3A1NnhhcHNrcwp2bFpkNlBXYnFHQVNSbzZSQ2dpbkVsVFFNQ2pQWGxJUkYvSW5POW9zSy9FCi0+IFgyNTUxOSBLWWRIL0FNdGpWb3dMOVEvTEg1bjRYZitmMnN6T1NnUEhGMnVWNWdZdlFNClA2Q3Fpakx6WDZTUVAzcGhUYndNR2dBWnVHd3dJS25Vdi9zUExUaVZPS0EKLT4gWDI1NTE5IG5EQ21CZmx5WlVMY2RUNytZeEh4WWJKZWpLZ0hLcEd1VmNEekc5ejVQaUEKQXhYWlhKS3NvUmtzcjdib2ZoRVJ2MDVraUFRRCszOUd5R3l1cXNPS01zSQotPiBYMjU1MTkgb1ByUUpUZGo5TXJHa3FYVDc4RmxveHk1Y2ZmTWQrZUFvZ3FzczZ4enYzaworcGpRUkhLWlVrQ1U2VlBGQktIa0c4Q3dNQytFU28rQTRtNURlbHJuVE1JCi0+IFgyNTUxOSBFSENiQjhWaHpxQTRKYWkrUEkzQytJNWpsMHMwUkhoMFliTGl6MTllcHlBCkgrSTB6Z1ZQeGQ1bWJQcnZiSEVRRlBjejdWb0o4UTB5SCthM3VJL0dhVXMKLT4gJm0sRCstZ3JlYXNlIDtNIGwsNFhiIHp6aGYgdApJRG96UUl0cHBKM3lKcmdQNU1WN1NibFFqSmhNVlU0OTBMcnhLU2xROE1sVWM1Z1VaUkZSSjlxTDd2Q0g2WGdRCnE5NVdRdWpYcWlvRFVXYTlBWDR5MjM5Nnd1V0EwTU1IbS9jRGlPK01teHlDV2RRCi0tLSBTS3BjU0Nydzd2ZTkyNVNDSHpMUXJYQWloa1p3c09HZlFDcjMwdjFZbkVNCi90vpriiYjvtyQ9S7eqZs6oBMG4Mu1Dqcz14wlWALA/KPuE8sxZ963dO7IjNKiotUWG8QdiG2iCReZwwI8=].md
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/inventory.json:8:  "failures": 0,
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/inventory.json:15:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/inventory.json:38:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/inventory.json:54:      "hint": "run repair-warns --apply"
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/inventory.json:61:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/inventory.json:84:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/inventory.json:100:      "hint": "run repair-warns --apply"
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/inventory.json:103:      "repo": "/home/dracon/Dev/dracon-ai-lib",
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/inventory.json:108:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/inventory.json:121:      "push_error": "ahead=21, push failing",
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/inventory.json:124:      "hint": "run repair-concerns --apply (push or rewrite)"
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/inventory.json:131:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/inventory.json:154:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/inventory.json:170:      "hint": "run repair-warns --apply"
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/inventory.json:200:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/inventory.json:216:      "hint": "run repair-warns --apply"
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/inventory.json:219:      "repo": "/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent",
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/inventory.json:223:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/inventory.json:246:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/inventory.json:255:      "last_msg": "6 file(s) in fully,pully [AUDIT_REPORT.md, pully/bins/pully/src/main.rs…",
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/inventory.json:269:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/inventory.json:288:      "repo": "/home/dracon/Dev/rust-ai-web-auto",
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/inventory.json:292:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/inventory.json:308:      "hint": "run repair-warns --apply"
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/inventory.json:311:      "repo": "/home/dracon/Dev/ai-auto-writer",
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/inventory.json:315:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/inventory.json:338:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/inventory.json:361:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/inventory.json:384:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/inventory.json:393:      "last_msg": "19 file(s) in examples,src,tests [src/cli.rs, src/ai_gen.rs, src/analyz…",
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/inventory.json:407:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/inventory.json:430:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/inventory.json:453:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/dracon-utilities-public-release-state.md:6:## main..public-release
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/dracon-utilities-public-release-state.md:11:7f95a61e deps: pin dracon-ai runtime deps to local dracon-libs
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/dracon-utilities-public-release-state.md:14:## public-release..main
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/dracon-utilities-public-release-state.md:19:codeberg/main
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/dracon-utilities-public-release-state.md:24:github/main
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/dracon-utilities-public-release-state.md:30:gitlab/main
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/dracon-utilities-public-release-state.md:34:main
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/dracon-utilities-public-release-state.md:36:origin/main
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/dracon-utilities-public-release-state.md:42:## diff stat public-release vs main
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/dracon-utilities-public-release-state.md:49: dracon-ai/Cargo.lock                               |  15 +-
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/hygiene.tsv:7:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	76	1	8	1	0	3	1	0
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/hygiene.tsv:9:/home/dracon/Dev/dracon-ai-lib	35	3	14	1	0	31	1	0
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/hygiene.tsv:11:/home/dracon/Dev/rust-ai-web-auto	46	2	2	1	0	27	1	0
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/hygiene.tsv:15:/home/dracon/Dev/ai-auto-writer	217	3	4	80	0	5	1	0
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/pi-proof/before.folder-auto-banner.pi-files.txt:25:.pi/goals/archived/goal_2026060520100089_mq1aifn7-6yezq4.md
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/pi-proof/after.one-mil-girls.pi-files.txt:11:.pi/goals/archived/goal_2026060318184883_mpyaiftl-mv1cfy.md
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/pi-proof/after.folder-auto-banner.pi-files.txt:25:.pi/goals/archived/goal_2026060520100089_mq1aifn7-6yezq4.md
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/one-mil-girls-post-state.md:7:## main...origin/main
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/one-mil-girls-post-state.md:14:  origin/main: 0	0
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/one-mil-girls-post-state.md:15:  github/main: 0	0
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/one-mil-girls-post-state.md:16:  gitlab/main: 711	0
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/one-mil-girls-post-state.md:17:  codeberg/main: 0	0
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/post-merge-state.md:7:main
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/post-merge-state.md:9:## main sha
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/post-merge-state.md:17:  * main
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/post-merge-state.md:26:## main on remotes
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/post-merge-state.md:27:  origin/main: f99961ce385e6836af8f04c34e211af8d0376df7
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/post-merge-state.md:28:  github/main: f99961ce385e6836af8f04c34e211af8d0376df7
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/post-merge-state.md:29:  gitlab/main: f99961ce385e6836af8f04c34e211af8d0376df7
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/post-merge-state.md:30:  codeberg/main: f99961ce385e6836af8f04c34e211af8d0376df7
docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/pi-proof/before.one-mil-girls.pi-files.txt:11:.pi/goals/archived/goal_2026060318184883_mpyaiftl-mv1cfy.md
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/pre-merge-state.md:12:## main sha
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/final-inventory.json:8:  "failures": 0,
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/final-inventory.json:15:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/final-inventory.json:38:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/final-inventory.json:61:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/final-inventory.json:80:      "repo": "/home/dracon/Dev/rust-ai-web-auto",
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/final-inventory.json:84:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/final-inventory.json:103:      "repo": "/home/dracon/Dev/dracon-ai-lib",
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/final-inventory.json:107:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/final-inventory.json:130:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/final-inventory.json:139:      "last_msg": "docs: clarify ai-api BYOK gateway role",
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/final-inventory.json:153:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/final-inventory.json:176:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/final-inventory.json:199:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/final-inventory.json:222:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/final-inventory.json:245:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/final-inventory.json:291:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/final-inventory.json:310:      "repo": "/home/dracon/Dev/ai-auto-writer",
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/final-inventory.json:314:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/final-inventory.json:333:      "repo": "/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent",
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/final-inventory.json:337:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/final-inventory.json:360:      "branch": "main",
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/approval-log.md:6:2026-06-11T18:25+01:00 | public-release | user picked 'merge to main, delete branch' | ask_user_question 1/4 | pending-execute
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/approval-log.md:13:## public-release → merged to main, deleted on all 4 remotes
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/approval-log.md:16:  - git checkout main (was 43d7505d)
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/approval-log.md:19:  - git push origin main: 43d7505d..f99961ce OK
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/approval-log.md:20:  - git push github main: up-to-date (already pushed by daemon)
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/approval-log.md:21:  - git push gitlab main: 43d7505d..f99961ce OK
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/approval-log.md:22:  - git push codeberg main: 43d7505d..f99961ce OK
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/approval-log.md:40:  - git push origin main: 33f1eb1..6bf75d9 OK
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/approval-log.md:41:  - git push github main: up-to-date (daemon already pushed)
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/approval-log.md:42:  - git push codeberg main: 33f1eb1..6bf75d9 OK
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/approval-log.md:43:  - git push gitlab main: REJECTED — 'You are not allowed to push code to protected branches on this project.'
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/approval-log.md:48:  - ahead/behind origin/main: 2604/1 (no local main divergence)
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/approval-log.md:53:  - main on all 4 remotes is at f99961ce (the merge + post-merge evidence commit)
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/approval-log.md:55:  - one-mil-girls main on origin/github/codeberg is at 6bf75d9; gitlab still at 2026-06-05 ancestor
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/folder-auto-banner-state.md:6:ai-auto-repo-rot-scanner-todo-agent
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/folder-auto-banner-state.md:7:ai-auto-writer
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/folder-auto-banner-state.md:10:dracon-ai-lib
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/folder-auto-banner-state.md:20:rust-ai-web-auto
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/hygiene.tsv:6:/home/dracon/Dev/dracon-ai-lib	35	3	14	1	1	0
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/hygiene.tsv:11:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent	77	1	8	1	1	0
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/hygiene.tsv:14:/home/dracon/Dev/rust-ai-web-auto	46	2	2	1	1	0
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/hygiene.tsv:15:/home/dracon/Dev/ai-auto-writer	217	3	4	80	1	0
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/validation-logs/final-validation.tsv:4:ai-auto-repo-rot-scanner-todo-agent	0	ai-auto-repo-rot-scanner-todo-agent.fmt.log	0	ai-auto-repo-rot-scanner-todo-agent.test.log	0	ai-auto-repo-rot-scanner-todo-agent.clippy.log
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/validation-logs/final-validation.tsv:8:ai-auto-writer	0	ai-auto-writer.fmt.log	0	ai-auto-writer.test.log	101	ai-auto-writer.clippy.log
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/validation-logs/final-validation.tsv:10:rust-ai-web-auto	0	rust-ai-web-auto.fmt.log	0	rust-ai-web-auto.test.log	0	rust-ai-web-auto.clippy.log
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/validation-logs/final-validation.tsv:15:dracon-ai-lib	0	dracon-ai-lib.fmt.log	0	dracon-ai-lib.test.log	101	dracon-ai-lib.clippy.log
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/PUBLIC_READINESS.md:10:- The code default remains unchanged: `SyncPolicy::default()` has no `standard_files`, so external users are not forced to receive `FUNDING.yml`.
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/PUBLIC_READINESS.md:28:- The remaining blockers are mostly public-readiness/hygiene, push/remote, and pre-existing user-owned changes — not FUNDING.yml.
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/PUBLIC_READINESS.md:36:3. **`dracon-ai-lib`**: decide remote strategy. It is still AHEAD:21 and push is blocked by the archived remote.
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/PUBLIC_READINESS.md:37:4. **Tracked local state**: approve whether `.pi/goals`, `.ralph`, audit screenshots, and generated artifacts in repos like `one-mil-girls` and `Junk-Runner-bevy` may remain public. If not, they need explicit cleanup/rewrite approval.
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/PUBLIC_READINESS.md:38:5. **Pre-existing clippy warnings**: decide whether public CI will require `-D warnings` for all repos. Several repos still fail clippy under `-D warnings` for pre-existing issues.
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/PUBLIC_READINESS.md:47:- `dracon-platform`: passes fmt/test/clippy, but current inventory shows user-owned changes in AI API tests and hosted web assets; public readiness depends on reviewing/committing those changes and whether tracked local state/artifacts are acceptable.
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/PUBLIC_READINESS.md:49:- `ai-auto-repo-rot-scanner-todo-agent`: passes fmt/test/clippy.
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/PUBLIC_READINESS.md:52:- `rust-ai-web-auto`: passes fmt/test/clippy.
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/PUBLIC_READINESS.md:54:- `one-mil-girls`: non-Rust checks pass; tracked `.pi/goals`/audit artifacts remain.
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/PUBLIC_READINESS.md:55:- `Junk-Runner-bevy/web`: non-Rust checks pass; tracked `.pi/goals`/audit artifacts remain.
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/PUBLIC_READINESS.md:60:- `dracon-ai-lib`: local validation passes, but push is blocked (AHEAD:21, archived remote).
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/PUBLIC_READINESS.md:61:- `dracon-code`: user-owned changes currently cause fmt/clippy failures; preserve them unless the user approves cleanup.
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/PUBLIC_READINESS.md:62:- `ai-auto-writer`, `video-factory`, `youtube-video-uploader`, `video-uploader`, `dracon-ai-lib`, `dracon-libs`: tests pass, but pre-existing clippy warnings remain under `-D warnings`.
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/PUBLIC_READINESS.md:79:The FUNDING.yml issue is resolved: it is Dracon-specific, external users are not forced to receive it, and the behavior is documented. The remaining public-release blockers are hygiene, remote/push, and explicit approval decisions.
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/one-mil-girls.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/one-mil-girls.git.txt:14:* main aa0d7b0 [origin/main] 1 file(s) in docs [docs/audit/2026-06-11-full-audit-v2/baseline.txt] DELTA:+300/-0 | NEW:2026-06-11-full-audit-v2/baseline.txt
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/avid.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/avid.git.txt:14:* main                                     8d1f698 [origin/main] 19 file(s) in examples,src,tests [src/cli.rs, src/ai_gen.rs, src/analyzer.rs] DELTA:+1034/-453 | TEST:81
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/avid.git.txt:16:8d1f698 19 file(s) in examples,src,tests [src/cli.rs, src/ai_gen.rs, src/analyzer.rs] DELTA:+1034/-453 | TEST:81
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/dracon-code.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/dracon-code.git.txt:13:  backup-main-20260513                             13262567 security(dependency configuration): Updated dependency configuration in `deny.toml` for security and comp...
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/dracon-code.git.txt:14:  bevy-version                                     ef86290b [gui+src|wip] screenshot viewer, task persistence, fetch denylist UI, gui_refresh_secs poll wiring, ai_actions in plan prompt, dead code cleanup
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/dracon-code.git.txt:18:  egui-version                                     0de221d8 {"schema":"dracon.commit.v2","schema_rev":2,"commit_kind":"sync_event","actor":"dracon-sync","generator":{"name":"dracon-git","version":"0.1.0"},"event_fingerprint":"bcc7462f0ab438a932e8482e31fc41ac25fb3d82d26d0a96f0d53304e52a706b","ts":"1771992484","repo":"dracon-code","branch":"master","files":{"added":0,"modified":3,"deleted":0,"renamed":0,"type_change":0,"unknown":0},"changed_paths_full":["Cargo.lock","Cargo.toml","gui/src/main.rs"],"changed_paths":["Cargo.lock","Cargo.toml","gui/src/main.rs"],"changed_paths_total":3,"changed_paths_truncated":false,"top_level_scopes":[{"key":"Cargo.lock","count":1},{"key":"Cargo.toml","count":1},{"key":"gui","count":1}],"extension_summary":[{"key":"lock","count":1},{"key":"rs","count":1},{"key":"toml","count":1}],"domain_summary":[{"key":"code","count":1},{"key":"config","count":1},{"key":"lockfile","count":1}],"intent_tags":["behavior_change_possible","compiled_or_runtime_code_touched","configuration_update","dependency_lock_changed"],"risk_flags":["build_graph_or_dependency_surface"],"semantic":{"files_analyzed":1,"files_skipped":2,"symbols_total":74,"symbols_truncated":false,"symbols":[{"path":"gui/src/main.rs","language":"rust","name":"main","kind":"function","start_line":11,"end_line":25},{"path":"gui/src/main.rs","language":"rust","name":"GuiRuntimeConfig","kind":"struct","start_line":28,"end_line":33},{"path":"gui/src/main.rs","language":"rust","name":"DraconConfigFile","kind":"struct","start_line":36,"end_line":44},{"path":"gui/src/main.rs","language":"rust","name":"Screen","kind":"enum","start_line":47,"end_line":51},{"path":"gui/src/main.rs","language":"rust","name":"Screen","kind":"impl","start_line":53,"end_line":61},{"path":"gui/src/main.rs","language":"rust","name":"label","kind":"function","start_line":54,"end_line":60},{"path":"gui/src/main.rs","language":"rust","name":"HubSort","kind":"enum","start_line":64,"end_line":70},{"path":"gui/src/main.rs","language":"rust","name":"HubSort","kind":"impl","start_line":72,"end_line":82},{"path":"gui/src/main.rs","language":"rust","name":"label","kind":"function","start_line":73,"end_line":81},{"path":"gui/src/main.rs","language":"rust","name":"HubFilter","kind":"enum","start_line":85,"end_line":89},{"path":"gui/src/main.rs","language":"rust","name":"HubFilter","kind":"impl","start_line":91,"end_line":99},{"path":"gui/src/main.rs","language":"rust","name":"label","kind":"function","start_line":92,"end_line":98},{"path":"gui/src/main.rs","language":"rust","name":"SettingsForm","kind":"struct","start_line":102,"end_line":110},{"path":"gui/src/main.rs","language":"rust","name":"SettingsForm","kind":"impl","start_line":112,"end_line":179},{"path":"gui/src/main.rs","language":"rust","name":"from_body","kind":"function","start_line":113,"end_line":132},{"path":"gui/src/main.rs","language":"rust","name":"apply_to_body","kind":"function","start_line":134,"end_line":178},{"path":"gui/src/main.rs","language":"rust","name":"OperatorApp","kind":"struct","start_line":181,"end_line":198},{"path":"gui/src/main.rs","language":"rust","name":"OperatorApp","kind":"impl","start_line":200,"end_line":708},{"path":"gui/src/main.rs","language":"rust","name":"new","kind":"function","start_line":201,"end_line":240},{"path":"gui/src/main.rs","language":"rust","name":"refresh","kind":"function","start_line":242,"end_line":271},{"path":"gui/src/main.rs","language":"rust","name":"run_action","kind":"function","start_line":273,"end_line":280},{"path":"gui/src/main.rs","language":"rust","name":"save_config","kind":"function","start_line":282,"end_line":304},{"path":"gui/src/main.rs","language":"rust","name":"sorted_hub_rows","kind":"function","start_line":306,"end_line":348},{"path":"gui/src/main.rs","language":"rust","name":"nav_row","kind":"function","start_line":350,"end_line":365},{"path":"gui/src/main.rs","language":"rust","name":"project_screen","kind":"function","start_line":367,"end_line":450},{"path":"gui/src/main.rs","language":"rust","name":"hub_screen","kind":"function","start_line":452,"end_line":533},{"path":"gui/src/main.rs","language":"rust","name":"settings_screen","kind":"function","start_line":535,"end_line":707},{"path":"gui/src/main.rs","language":"rust","name":"OperatorApp","kind":"impl","start_line":710,"end_line":769},{"path":"gui/src/main.rs","language":"rust","name":"update","kind":"function","start_line":711,"end_line":768},{"path":"gui/src/main.rs","language":"rust","name":"apply_theme","kind":"function","start_line":771,"end_line":818},{"path":"gui/src/main.rs","language":"rust","name":"panel","kind":"function","start_line":820,"end_line":835},{"path":"gui/src/main.rs","language":"rust","name":"screen_title","kind":"function","start_line":837,"end_line":851},{"path":"gui/src/main.rs","language":"rust","name":"paint_background","kind":"function","start_line":853,"end_line":888},{"path":"gui/src/main.rs","language":"rust","name":"kv","kind":"function","start_line":890,"end_line":895},{"path":"gui/src/main.rs","language":"rust","name":"status_chip","kind":"function","start_line":897,"end_line":909},{"path":"gui/src/main.rs","language":"rust","name":"action_button","kind":"function","start_line":911,"end_line":928},{"path":"gui/src/main.rs","language":"rust","name":"tab_button","kind":"function","start_line":930,"end_line":952},{"path":"gui/src/main.rs","language":"rust","name":"chip_button","kind":"function","start_line":954,"end_line":969},{"path":"gui/src/main.rs","language":"rust","name":"truncate_middle","kind":"function","start_line":971,"end_line":978},{"path":"gui/src/main.rs","language":"rust","name":"draw_projects_table","kind":"function","start_line":980,"end_line":1065},{"path":"gui/src/main.rs","language":"rust","name":"draw_hub_table","kind":"function","start_line":1067,"end_line":1180},{"path":"gui/src/main.rs","language":"rust","name":"table_header","kind":"function","start_line":1182,"end_line":1190},{"path":"gui/src/main.rs","language":"rust","name":"table_row_bg","kind":"function","start_line":1192,"end_line":1198},{"path":"gui/src/main.rs","language":"rust","name":"is_active_repo","kind":"function","start_line":1200,"end_line":1205},{"path":"gui/src/main.rs","language":"rust","name":"phase_color","kind":"function","start_line":1207,"end_line":1217},{"path":"gui/src/main.rs","language":"rust","name":"trigger_color","kind":"function","start_line":1219,"end_line":1225},{"path":"gui/src/main.rs","language":"rust","name":"git_state_color","kind":"function","start_line":1227,"end_line":1241},{"path":"gui/src/main.rs","language":"rust","name":"FleetView","kind":"struct","start_line":1244,"end_line":1247},{"path":"gui/src/main.rs","language":"rust","name":"FleetRow","kind":"struct","start_line":1250,"end_line":1259},{"path":"gui/src/main.rs","language":"rust","name":"FleetRow","kind":"impl","start_line":1261,"end_line":1273},{"path":"gui/src/main.rs","language":"rust","name":"active_slice_label","kind":"function","start_line":1262,"end_line":1266},{"path":"gui/src/main.rs","language":"rust","name":"updated_label","kind":"function","start_line":1268,"end_line":1272},{"path":"gui/src/main.rs","language":"rust","name":"merge_discovered_repos","kind":"function","start_line":1275,"end_line":1291},{"path":"gui/src/main.rs","language":"rust","name":"compute_git_states","kind":"function","start_line":1293,"end_line":1297},{"path":"gui/src/main.rs","language":"rust","name":"git_state_for_repo","kind":"function","start_line":1299,"end_line":1324},{"path":"gui/src/main.rs","language":"rust","name":"parse_branch_sync","kind":"function","start_line":1326,"end_line":1348},{"path":"gui/src/main.rs","language":"rust","name":"discover_git_repos","kind":"function","start_line":1350,"end_line":1363},{"path":"gui/src/main.rs","language":"rust","name":"walk_for_git_repos","kind":"function","start_line":1365,"end_line":1407},{"path":"gui/src/main.rs","language":"rust","name":"refresh_view","kind":"function","start_line":1409,"end_line":1432},{"path":"gui/src/main.rs","language":"rust","name":"choose_selected_repo","kind":"function","start_line":1434,"end_line":1457},{"path":"gui/src/main.rs","language":"rust","name":"load_fleet_rows","kind":"function","start_line":1459,"end_line":1501},{"path":"gui/src/main.rs","language":"rust","name":"load_fleet_rows_sqlite","kind":"function","start_line":1503,"end_line":1547},{"path":"gui/src/main.rs","language":"rust","name":"load_gui_runtime_config","kind":"function","start_line":1549,"end_line":1580},{"path":"gui/src/main.rs","language":"rust","name":"default_fleet_db_path","kind":"function","start_line":1582,"end_line":1584},{"path":"gui/src/main.rs","language":"rust","name":"expand_tilde","kind":"function","start_line":1586,"end_line":1596},{"path":"gui/src/main.rs","language":"rust","name":"read_text_file","kind":"function","start_line":1598,"end_line":1601},{"path":"gui/src/main.rs","language":"rust","name":"run_json","kind":"function","start_line":1603,"end_line":1611},{"path":"gui/src/main.rs","language":"rust","name":"run_cmd","kind":"function","start_line":1613,"end_line":1619},{"path":"gui/src/main.rs","language":"rust","name":"run_cmd_in","kind":"function","start_line":1621,"end_line":1627},{"path":"gui/src/main.rs","language":"rust","name":"run_cmd_capture","kind":"function","start_line":1629,"end_line":1652},{"path":"gui/src/main.rs","language":"rust","name":"resolve_default_project","kind":"function","start_line":1654,"end_line":1659},{"path":"gui/src/main.rs","language":"rust","name":"append_log","kind":"function","start_line":1661,"end_line":1666},{"path":"gui/src/main.rs","language":"rust","name":"now_secs","kind":"function","start_line":1668,"end_line":1673},{"path":"gui/src/main.rs","language":"rust","name":"format_ts","kind":"function","start_line":1675,"end_line":1677}],"kind_summary":[{"key":"function","count":58},{"key":"impl","count":7},{"key":"struct","count":6},{"key":"enum","count":3}],"language_summary":[{"key":"rust","count":74}]},"status":{"ahead":0,"behind":0,"modified_files":1,"staged_files":0},"policy":{"deterministic":true,"ai_commit_messages":false}}
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/dracon-code.git.txt:20:* main                                             b205edd2 [origin/main] 1 file(s) in plugins [plugins/default-builtin-tools/src/tools.rs] DELTA:+2/-1
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/dracon-code.git.txt:22:  temp-main                                        e1eaa26d Reset to origin/master
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/junk-runner-bevy-state.md:11:    main
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/junk-runner-bevy-state.md:18:    codeberg/main
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/junk-runner-bevy-state.md:24:    github/main
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/junk-runner-bevy-state.md:29:    gitlab/main
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/junk-runner-bevy-state.md:32:    origin/HEAD -> origin/main
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/junk-runner-bevy-state.md:37:    origin/main
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/junk-runner-bevy-state.md:49:## tauri2..main (commits on main not in tauri2)
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/junk-runner-bevy-state.md:51:## main..tauri2 (commits on tauri2 not in main)
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/junk-runner-bevy-state.md:1149:953ea677a 1 file(s) in assets DELTA:+0/-0 | BIN:1 NEW:sfx/system_failure.wav
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/junk-runner-bevy-state.md:1152:eb45bf4b4 1 file(s) in assets DELTA:+0/-0 | BIN:1 NEW:sfx/repair.wav
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/junk-runner-bevy-state.md:1219:25e216f20 2 file(s) in .pi,web [web/src/main.ts, .pi/goals/active_goal_2026060514355014_mq0yvzib-zgq5hb.md] DELTA:+16/-5
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/junk-runner-bevy-state.md:1352:cdf4fc733 2 file(s) in .pi,web [.pi/goals/active_goal_2026060501484049_mq07hesq-5j0mws.md, web/src/main.ts] DELTA:+6/-6
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/junk-runner-bevy-state.md:1364:96913bdf5 2 file(s) in .pi,web [web/src/main.ts, .pi/goals/active_goal_2026060501484049_mq07hesq-5j0mws.md] DELTA:+21/-153
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/junk-runner-bevy-state.md:1444:a048c2be2 2 file(s) in .pi,web [web/src/main.ts, .pi/goals/active_goal_2026060422111245_mpzzpqvc-gj1jh9.md] DELTA:+23/-5
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/junk-runner-bevy-state.md:1446:3ee4e2539 2 file(s) in .pi,web [web/src/main.ts, .pi/goals/active_goal_2026060422111245_mpzzpqvc-gj1jh9.md] DELTA:+21/-7
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/junk-runner-bevy-state.md:1457:bd3c22928 5 file(s) in .pi,src-tauri,web [web/src/main.ts, web/src/lib/theme.css, .pi/goals/active_goal_2026060422111245_mpzzpqvc-gj1jh9.md] DELTA:+8/-211
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/junk-runner-bevy-state.md:1461:124bc8655 2 file(s) in .pi,web [.pi/goals/active_goal_2026060422111245_mpzzpqvc-gj1jh9.md, web/src/main.ts] DELTA:+9/-5
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/junk-runner-bevy-state.md:1467:524bba010 2 file(s) in .pi,web [web/src/main.ts, .pi/goals/active_goal_2026060422111245_mpzzpqvc-gj1jh9.md] DELTA:+20/-20
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/junk-runner-bevy-state.md:1470:cb2cf0db9 2 file(s) in .pi,web [.pi/goals/active_goal_2026060422111245_mpzzpqvc-gj1jh9.md, web/src/main.ts] DELTA:+6/-5
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/junk-runner-bevy-state.md:1471:837b428f4 2 file(s) in .pi,web [web/src/main.ts, .pi/goals/active_goal_2026060422111245_mpzzpqvc-gj1jh9.md] DELTA:+21/-11
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/junk-runner-bevy-state.md:1474:3210f3ba7 2 file(s) in .pi,web [web/src/main.ts, .pi/goals/active_goal_2026060422111245_mpzzpqvc-gj1jh9.md] DELTA:+34/-5
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/junk-runner-bevy-state.md:1476:31d2375d8 2 file(s) in .pi,web [web/src/main.ts, .pi/goals/active_goal_2026060422111245_mpzzpqvc-gj1jh9.md] DELTA:+24/-6
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/junk-runner-bevy-state.md:1479:361815223 2 file(s) in .pi,web [web/src/main.ts, .pi/goals/active_goal_2026060422111245_mpzzpqvc-gj1jh9.md] DELTA:+35/-5
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/junk-runner-bevy-state.md:1485:a7868e260 2 file(s) in .pi,web [web/src/main.ts, .pi/goals/active_goal_2026060422111245_mpzzpqvc-gj1jh9.md] DELTA:+21/-7
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/junk-runner-bevy-state.md:1495:99fa28a19 2 file(s) in .pi,web [web/src/main.ts, .pi/goals/active_goal_2026060422111245_mpzzpqvc-gj1jh9.md] DELTA:+28/-5
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/junk-runner-bevy-state.md:1500:2b28fd84e 2 file(s) in .pi,web [web/src/main.ts, .pi/goals/active_goal_2026060422111245_mpzzpqvc-gj1jh9.md] DELTA:+60/-5
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/junk-runner-bevy-state.md:1555:d22aa8402 2 file(s) in .pi,web [web/src/main.ts, .pi/goals/active_goal_2026060419170925_mpzthwua-rzc91q.md] DELTA:+19/-55
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/junk-runner-bevy-state.md:1580:f2740fd4c 2 file(s) in .pi,web [web/src/main.ts, .pi/goals/active_goal_2026060419170925_mpzthwua-rzc91q.md] DELTA:+62/-20
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/junk-runner-bevy-state.md:1604:09a398e01 2 file(s) in .pi,web [web/src/main.ts, .pi/goals/active_goal_2026060403142441_mpyv3t7i-vup5bv.md] DELTA:+29/-23
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/junk-runner-bevy-state.md:1607:acb7169f5 3 file(s) in .pi,web [web/vite.config.ts, web/src/main.ts, .pi/goals/active_goal_2026060403142441_mpyv3t7i-vup5bv.md] DELTA:+48/-6
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/junk-runner-bevy-state.md:1620:aeb52986e 2 file(s) in .pi,web [web/src/main.ts, .pi/goals/active_goal_2026060403142441_mpyv3t7i-vup5bv.md] DELTA:+94/-30
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/junk-runner-bevy-state.md:1684:c66cfc7d7 2 file(s) in web [web/src/main.ts, web/src/lib/theme.css] DELTA:+57/-5
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/junk-runner-bevy-state.md:2275:b65590686 2 file(s) in .pi [.pi/goals/notes_dev_failu[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBYbnU1Z2J3OEJkRU11MGkzL3RjeHpodU1xUk9CUkROTWR0QlZnOERNNUhZCkxMR2NtTGVNWURyVWl1NVBOUW9LVkZxWm5CQVVSVmZ6OTF2ZHNaL1QrTmcKLT4gWDI1NTE5IFE1QkRMZmFwTVdVRWtUMlNDaHpOVzRBRkVxR2FKR1g4TnJyWjdtbk45UW8KYU92K25VbDJmUm5DOWZ2NjNoQnJhcEc5c0xkdFdUWndJNEtGOEExQVo2ZwotPiBYMjU1MTkgbFhZb0RhRm5RaFhLQmkvNHJuOTJtSFdXT0VUcVluYWJIbHFENnh4Wm5YWQpKWS82NnNaTFFSWm9USzdNSDMxSi8zb3l0ZnBMUmc0enZyZ3ZxT1EwcG5vCi0+IFgyNTUxOSB1WVU5K3BJWHN6LzBBU2hjZEJSZDFsM3l0dE85bnhKWkRMeWkxSUlOMUZBCmk0b21ja3pxSy9RSkoyekJqOGpXa3FuRHNYTmhnWjI0amhyS20wQlErUkEKLT4gWDI1NTE5IGlCQUtRdlowazZxZHFkZ042elJRejZCMU9XVTdpM0ZHdEdoaHV4ZGNSVEUKSy9QbXhoalI4aDVuS0hROHQ3ZUhubS80OU0rbmxxZlk4aTN3T2UwZVNWYwotPiBYMjU1MTkgTDJiMkQzVDdNdDBrNXd4ZnpVc05xMStYSnhzazNybUtkQmd0eXJDZTUwSQpJM3JLTkJ2UVZLbzNMa2dSY2VVQUJtUzRiQjFOWDV2UW11bkxxelNXWVEwCi0+IFgyNTUxOSBmbkZ1Wi85VWpURm5nWjllL2RYcjVqZGljZE9OS0FWN3BMUUg5eGszQXlRCk1zTnhKbEg3NDR0VndyazlDMXJiWDlMQTNhWWFxTytTdmtjQzBPdmZGUzQKLT4gQXIsNj1kV2ItZ3JlYXNlIFZjIDMKRWdENjNnckJkT2JqcUE3aWxuaGJ0TTQ1QjhOVzJsM0o3U0Z1eEphaW14RklvMS9jMTArVHhMMlRCWXdQCi0tLSA2MjhmcGV6aEl2cFp2K1IxOHdXdFFvaTR3VDBpbUZZK2s5V2c4RXBNT0owCtSxTYUvePVQj7CqC7HNJcgKenDrXwrXcaP9U0B/FxZaIzk5vjay7ZxQwOFx8RTy662v7dP2qzWImZSCy8U=].md, .pi/goals/active_goal_2026060300451889_mpxac84y-4w4ers.md] DELTA:+124/-0 | NEW:goals/active_goal_2026060300451889_mpxac84y-4w4ers.md,goals/notes_dev_failu[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBnWjhuWVFPMHM0eUM2M1RzWFN2QUpXc2prRGdUaWg0LzN4U04xc3BSaEJNCm9jeHZkUmgxaERIZTM5ODBXbzVIbGRWc0lRd0V6NEFRMlVDNTZZalk0dzAKLT4gWDI1NTE5IFYvZ1M0OVBIMDBPam5hT0E4YVNiUnl5NTAyLzZBdDFHL29CWDkyYjhleEkKMzRkOUJzWElINEtRc2VmdldOOFZ0S0g2UzFyNTVkakdxc2Y4L2gyaithNAotPiBYMjU1MTkgaGtkbWlPdDFudWVjY2pTSzdsR0djSHFUM3MwWGViWERhMzZ1OUFXTm1RSQpQOUxhSU1uTjVNb25hV1haeUNvUi9hWVVNS1lSbHNCb2lHaFdSUy93cXBnCi0+IFgyNTUxOSBXZkp6UFQwSDVWc2xzVFVNQnRieWtvR0xZL1RsTnViN3BkUTE5TEFaakJNCkVMU0VqbGFSOEdZQXpnTFBoQTlyOXl0dTRJdFovMUNUYkx6Mit0cnQ1VzQKLT4gWDI1NTE5IFM5NmRxNk9OWVR0SXFsdUtmK1FDSmlvOFErZDh6VzRYQXgraXd5OUlrUzQKc3VsZy92WXBEZ3ZINnNlQmFSeHJ2b0paZmlQaEFBdFFkOHdQcnkrQWEwbwotPiBYMjU1MTkgT2lIY2g2dkh4NkU4ajNxWitxY1lPeCtJcUg0Y3VlQWlVTEk3REFqalhINApqallXa0pxODNSMkp0bG8zK3ZNdE11QzdLMmRuK3lyTzh0Vm0wSjZzaWZJCi0+IFgyNTUxOSA4K2FjZk5IaE9yVmMyL0kzNVdZSWQxQWNHRGNBMjUzV2ttaUFycG93YUZjCkY3OHNwc20wYVdhOU9DZTBVcER1SC9FMExxNWtNeEh5ZFU2YjRuakJUMXcKLT4gWy1ncmVhc2UgYmw5IDJhIGxeUCZDVFAmCkh4b2Nmcm1WbTJNa1dGS2NuWkU3UmFJCi0tLSBkYlNWRm1scmdYV3ZKQ3VJZ3V1ZW85R1FHWmRJZjkxaVdGV2NnTnY0WlFrCrk6xzWksirwBkxhDHuqF9hCA+ISD3XTd7esis03qCoLRollD3cMz0oSw7/26aalML+zRRQXYl6Bc3FuiRs=].md
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/junk-runner-bevy-state.md:2314:09941d5e2 2 file(s) in .pi,web [.pi/goals/active_goal_2026060220413454_mpx1mrww-dclo26.md, web/src/main.ts] DELTA:+99/-0 | NEW:goals/active_goal_2026060220413454_mpx1mrww-dclo26.md
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/junk-runner-bevy-state.md:2320:46ac79a2b 8 file(s) in .pi,src-tauri [src-tauri/src/core/events/trade.rs, pi-session-2026-06-02T08-47-00-003Z_019e8783-c3a3-7f64-9195-785dac6f2701.html, src-tauri/src/core/events/crew.rs] DELTA:+16305/-13 | NEW:pi-session-2026-06-02T08-47-00-003Z_019e8783-c3a3-7f64-9195-785dac6f2701.html,events/chain.rs
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/junk-runner-bevy-state.md:2325:f36ad274b 72 file(s) in .pi,src,web [src/terminal.css, Cargo.lock, src/src/state.rs] DELTA:+845/-7333 | BIN:3 NEW:lib/ThemeToggle.svelte,screens/Menu.svelte,lib/theme.css,lib/theme.ts DEL:src/Cargo.toml,src/Trunk.toml,fonts/jetbrains-mono.zip,fonts/press-start.zip,src/index.html,fonts/JetBrainsMono-Bold.woff2,fonts/JetBrainsMono-Regular.woff2,fonts/PressStart2P-Regular.woff2,src/app.rs,src/audio.rs+51more
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/junk-runner-bevy-state.md:2350:871c97389 6 file(s) in .pi,tmp [tmp/capture-tauri.js, .pi/goals/active_goal_2026053120272930_mpu68ydx-s264wr.md] DELTA:+82/-10 | BIN:4 NEW:tmp/capture-tauri.js,tmp/chromium-screenshot.png,tmp/chromium-wait.png,tmp/spectacle-tauri.png,tmp/tauri-now.png
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/junk-runner-bevy-state.md:2382:7ed1fe3e2 8 file(s) in .pi,src [.pi/goals/active_goal_2026053120272930_mpu68ydx-s264wr.md, src/terminal.css, src/fonts/jetbrains-mono.zip] DELTA:+199/-20 | BIN:3 NEW:goals/active_goal_2026053120272930_mpu68ydx-s264wr.md,fonts/JetBrainsMono-Bold.woff2,fonts/JetBrainsMono-Regular.woff2,fonts/PressStart2P-Regular.woff2,fonts/jetbrains-mono.zip,fonts/press-start.zip
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/junk-runner-bevy-state.md:2467:0739aef97 3 file(s) in src [src/src/lib.rs, src/src/main.rs, src/src/components/cargo_bay.rs] DELTA:+25/-42
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/junk-runner-bevy-state.md:2468:fa9497d99 3 file(s) in src [src/src/main.rs, src/src/lib.rs, src/src/app.rs] DELTA:+62/-79
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/junk-runner-bevy-state.md:2469:672a6cd12 7 file(s) in .github,src [src/src/main.rs, src/src/components/cargo_bay.rs, .github/workflows/test.yml] DELTA:+335/-0 | TEST:52 NEW:workflows/test.yml,components/cargo_bay.rs,components/crew_card.rs+2more
docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/junk-runner-bevy-state.md:2606:5ff9cafa3 Added 2: src-tauri/src/core/crew_dialogue.rs, src-tauri/src/core/portrait_gen.rs
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/folder-auto-banner.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/folder-auto-banner.git.txt:13:* main cf497b7 [origin/main] 1 file(s) [RELEASE_NOTES_0.6.17.md] DELTA:+12/-0 | NEW:RELEASE_NOTES_0.6.17.md
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/dracon-libs.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/dracon-libs.git.txt:13:* main 2ff017b [origin/main] 1 file(s) [deny.toml] DELTA:+2/-0
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/video-uploader.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/video-uploader.git.txt:13:* main 9d5e9f1 [origin/main] 2 file(s) in youtube-uploader-cli [youtube-uploader-cli/tests/cli.rs, youtube-uploader-cli/src/main.rs] DELTA:+5/-3 | TEST:6
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/video-uploader.git.txt:15:9d5e9f1 2 file(s) in youtube-uploader-cli [youtube-uploader-cli/tests/cli.rs, youtube-uploader-cli/src/main.rs] DELTA:+5/-3 | TEST:6
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/video-uploader.git.txt:17:b630d5f 4 file(s) in youtube-uploader,youtube-uploader-cli [youtube-uploader/src/youtube.rs, youtube-uploader/src/config.rs, youtube-uploader-cli/src/main.rs] DELTA:+80/-40
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/dracon-utilities.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/dracon-utilities.git.txt:5:? docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/dracon-ai-lib.git.txt
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/dracon-utilities.git.txt:20:* main                   184c5c33 [origin/main] 4 file(s) in dracon-sync [dracon-sync/src/policy.rs, dracon-sync/dracon-sync.example.toml, dracon-sync/README.md] DELTA:+47/-12
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/ai-auto-repo-rot-scanner-todo-agent.git.txt:1:REPO=/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/ai-auto-repo-rot-scanner-todo-agent.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/ai-auto-repo-rot-scanner-todo-agent.git.txt:4:codeberg	git@codeberg.org:dracondev/ai-auto-repo-rot-scanner-todo-agent.git (fetch)
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/ai-auto-repo-rot-scanner-todo-agent.git.txt:5:codeberg	git@codeberg.org:dracondev/ai-auto-repo-rot-scanner-todo-agent.git (push)
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/ai-auto-repo-rot-scanner-todo-agent.git.txt:6:github	git@github.com:DraconDev/ai-auto-repo-rot-scanner-todo-agent.git (fetch)
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/ai-auto-repo-rot-scanner-todo-agent.git.txt:7:github	git@github.com:DraconDev/ai-auto-repo-rot-scanner-todo-agent.git (push)
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/ai-auto-repo-rot-scanner-todo-agent.git.txt:8:gitlab	git@gitlab.com:dracondev/ai-auto-repo-rot-scanner-todo-agent.git (fetch)
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/ai-auto-repo-rot-scanner-todo-agent.git.txt:9:gitlab	git@gitlab.com:dracondev/ai-auto-repo-rot-scanner-todo-agent.git (push)
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/ai-auto-repo-rot-scanner-todo-agent.git.txt:10:origin	https://github.com/DraconDev/ai-auto-repo-rot-scanner-todo-agent.git (fetch)
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/ai-auto-repo-rot-scanner-todo-agent.git.txt:11:origin	https://github.com/DraconDev/ai-auto-repo-rot-scanner-todo-agent.git (push)
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/ai-auto-repo-rot-scanner-todo-agent.git.txt:13:* main 7132201 [origin/main] 1 file(s) in docs [docs/AUDIT-2026-06-10.md] DELTA:+128/-0
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/ai-auto-repo-rot-scanner-todo-agent.git.txt:19:9cb9641 6 file(s) in src [Cargo.lock, src/ai/mod.rs, src/webhook.rs] DELTA:+502/-1983
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/kiki-sassy-desktop-announcer.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/kiki-sassy-desktop-announcer.git.txt:13:* main 0155632 [origin/main] 2 file(s) in src [src/journal.rs, src/daemon.rs] DELTA:+2/-4
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/pully-fully-pull-based-fleet-reconciler.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/pully-fully-pull-based-fleet-reconciler.git.txt:13:* main 364735af [origin/main] 6 file(s) in fully,pully [AUDIT_REPORT.md, pully/bins/pully/src/main.rs, pully/README.md] DELTA:+95/-14
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/pully-fully-pull-based-fleet-reconciler.git.txt:15:364735af 6 file(s) in fully,pully [AUDIT_REPORT.md, pully/bins/pully/src/main.rs, pully/README.md] DELTA:+95/-14
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/pully-fully-pull-based-fleet-reconciler.git.txt:16:90575b82 8 file(s) in fully,pully [fully/docs/CLI.md, pully/docs/CLI.md, pully/bins/pully/src/main.rs] DELTA:+436/-203
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/pully-fully-pull-based-fleet-reconciler.git.txt:17:23a92627 5 file(s) in fully,pully [fully/crates/fully-core/src/fleet_status.rs, fully/bins/fully/src/main.rs, pully/crates/pully-core/src/service_reconciler/mod.rs] DELTA:+148/-39
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/youtube-video-uploader.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/youtube-video-uploader.git.txt:13:* main 771d422 [origin/main] Merge https://github.com/DraconDev/youtube-video-uploader
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/repos.tsv:1:/home/dracon/Dev/dracon-ai-lib
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/repos.tsv:9:/home/dracon/Dev/ai-auto-repo-rot-scanner-todo-agent
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/repos.tsv:13:/home/dracon/Dev/rust-ai-web-auto
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/repos.tsv:14:/home/dracon/Dev/ai-auto-writer
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/dracon-platform.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/dracon-platform.git.txt:15:  azumi-ver                                11f588f8d chore(goal): ai-hub-audit goal complete (6/6 tasks)
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/dracon-platform.git.txt:17:* main                                     bdb138e70 [origin/main] 1 file(s) in web DELTA:+0/-0 | BIN:1
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/dracon-platform.git.txt:20:  phase-1/api-core-lift                    0f5e8e22b [origin/phase-1/api-core-lift] 1 file(s) in apis [apis/ai-api/.env] DELTA:+1/-1 | ENV:
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/dracon-platform.git.txt:21:  phase-2/high-cluster                     7ad8ecca9 [origin/phase-2/high-cluster] 3 file(s) in web [web/ai-hub/src/lib/chrome.config.ts, web/ai-hub/src/routes/+layout.svelte, web/packages/chrome/src/lib/SiteSubNav.svelte] DELTA:+9/-19
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/dracon-platform.git.txt:25:  phase-4/specta-metrics                   d8f6a56e2 [origin/phase-4/specta-metrics] 1 file(s) in web [web/ai-hub/src/lib/icons.ts] DELTA:+4/-2
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/dracon-platform.git.txt:30:1ea8a4ae6 6 file(s) in .data,web [web/games-hosted/games/junk-runner/index.html, .data/ai_rankings_cache.json, web/games-hosted/games/junk-runner/assets/{index-Bo_QmhmO.css => index-Dijw7Gvv.css}] DELTA:+5/-5 | BIN:2
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/browser-extensions-shared.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/browser-extensions-shared.git.txt:4:1 .M N... 100644 100644 100644 ace41c47b5f25839a3ed6f38898a47bd7010c549 ace41c47b5f25839a3ed6f38898a47bd7010c549 job-finder/docs/AI_ERA_STRATEGY.md
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/browser-extensions-shared.git.txt:67:? auto-form-filler/.audit-ui/ui-ux-audit/Default/Local Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/CURRENT
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/browser-extensions-shared.git.txt:68:? auto-form-filler/.audit-ui/ui-ux-audit/Default/Local Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOCK
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/browser-extensions-shared.git.txt:69:? auto-form-filler/.audit-ui/ui-ux-audit/Default/Local Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOG
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/browser-extensions-shared.git.txt:70:? auto-form-filler/.audit-ui/ui-ux-audit/Default/Local Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/MANIFEST-000001
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/browser-extensions-shared.git.txt:120:? auto-form-filler/.audit-ui/ui-ux-audit/Default/Sync Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/CURRENT
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/browser-extensions-shared.git.txt:121:? auto-form-filler/.audit-ui/ui-ux-audit/Default/Sync Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOCK
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/browser-extensions-shared.git.txt:122:? auto-form-filler/.audit-ui/ui-ux-audit/Default/Sync Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/LOG
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/browser-extensions-shared.git.txt:123:? auto-form-filler/.audit-ui/ui-ux-audit/Default/Sync Extension Settings/eainhgdjdmipbcjhbjbnhgmccchbaegd/MANIFEST-000001
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/browser-extensions-shared.git.txt:185:* main 2cece7133 [origin/main] 3 file(s) in job-finder [job-finder/docs/SCORED_STRATEGY.md, job-finder/docs/MONETIZATION.md, job-finder/docs/COMPETITIVE.md] DELTA:+7/-1
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/browser-extensions-shared.git.txt:188:5d18602f7 4 file(s) in job-finder [job-finder/docs/ROADMAP_TODO.md, job-finder/README.md, job-finder/docs/AI_ERA_STRATEGY.md] DELTA:+353/-0 | NEW:docs/ROADMAP_TODO.md
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/ai-auto-writer.git.txt:1:REPO=/home/dracon/Dev/ai-auto-writer
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/ai-auto-writer.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/ai-auto-writer.git.txt:4:codeberg	git@codeberg.org:dracondev/ai-auto-writer.git (fetch)
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/ai-auto-writer.git.txt:5:codeberg	git@codeberg.org:dracondev/ai-auto-writer.git (push)
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/ai-auto-writer.git.txt:6:github	git@github.com:DraconDev/ai-auto-writer.git (fetch)
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/ai-auto-writer.git.txt:7:github	git@github.com:DraconDev/ai-auto-writer.git (push)
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/ai-auto-writer.git.txt:8:gitlab	git@gitlab.com:dracondev/ai-auto-writer.git (fetch)
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/ai-auto-writer.git.txt:9:gitlab	git@gitlab.com:dracondev/ai-auto-writer.git (push)
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/ai-auto-writer.git.txt:10:origin	https://github.com/DraconDev/ai-auto-writer.git (fetch)
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/ai-auto-writer.git.txt:11:origin	https://github.com/DraconDev/ai-auto-writer.git (push)
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/ai-auto-writer.git.txt:14:* main                               aa5d0ebb [origin/main] 1 file(s) in src [src/services/dracon.rs] DELTA:+4/-32
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/ai-auto-writer.git.txt:19:9c829b43 Merge https://github.com/DraconDev/ai-auto-writer
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/dracon-ai-lib.git.txt:1:REPO=/home/dracon/Dev/dracon-ai-lib
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/dracon-ai-lib.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/dracon-ai-lib.git.txt:4:origin	https://github.com/DraconDev/dracon-ai-lib.git (fetch)
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/dracon-ai-lib.git.txt:5:origin	https://github.com/DraconDev/dracon-ai-lib.git (push)
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/dracon-ai-lib.git.txt:7:* main d8846da [origin/main: ahead 21] docs: make crate docs explicit BYOK-library contract
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/dracon-ai-lib.git.txt:13:6882198 simplify: drop the dracon-ai/* cutover theater; use the real repo URL
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/video-factory.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/video-factory.git.txt:13:* main 698a658 [origin/main] 14 file(s) in crates [crates/api/src/routes.rs, crates/core/src/config.rs, crates/worker/src/ffmpeg.rs] DELTA:+225/-162
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/video-factory.git.txt:17:4215e5f 1 file(s) in src [src/main.rs] DELTA:+3/-3
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/rust-ai-web-auto.git.txt:1:REPO=/home/dracon/Dev/rust-ai-web-auto
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/rust-ai-web-auto.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/rust-ai-web-auto.git.txt:4:codeberg	git@codeberg.org:dracondev/rust-ai-web-auto.git (fetch)
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/rust-ai-web-auto.git.txt:5:codeberg	git@codeberg.org:dracondev/rust-ai-web-auto.git (push)
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/rust-ai-web-auto.git.txt:6:github	git@github.com:DraconDev/rust-ai-web-auto.git (fetch)
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/rust-ai-web-auto.git.txt:7:github	git@github.com:DraconDev/rust-ai-web-auto.git (push)
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/rust-ai-web-auto.git.txt:8:gitlab	git@gitlab.com:dracondev/rust-ai-web-auto.git (fetch)
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/rust-ai-web-auto.git.txt:9:gitlab	git@gitlab.com:dracondev/rust-ai-web-auto.git (push)
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/rust-ai-web-auto.git.txt:10:origin	https://github.com/DraconDev/rust-ai-web-auto.git (fetch)
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/rust-ai-web-auto.git.txt:11:origin	https://github.com/DraconDev/rust-ai-web-auto.git (push)
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/rust-ai-web-auto.git.txt:13:* main e99bc4a [origin/main] ci+reports: add social_bot_e2e to CI; obvious-improvements report
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/rust-ai-web-auto.git.txt:19:996b4ac docs(audit): document Dracon AI lib adoption + Section 7/8/9/10 renumbering
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/DraconDev.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/DraconDev.git.txt:13:* main e280732 [origin/main] 3 file(s) [README_SUGGESTED_FORM.md, SUGGESTED_FORM_USAGE.md, SUGGESTED_FORM_BLOCKERS.md] DELTA:+191/-0 | NEW:README_SUGGESTED_FORM.md,SUGGESTED_FORM_BLOCKERS.md,SUGGESTED_FORM_USAGE.md
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/Junk-Runner-bevy.git.txt:2:--- git status porcelain=v2 ---
docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/Junk-Runner-bevy.git.txt:14:  main        e1894697f [origin/main] Added SOLID_VS_SVELTE.md

## dracon-ai crate tree
dracon-ai/BLUEPRINT.md
dracon-ai/Cargo.lock
dracon-ai/Cargo.toml
dracon-ai/dracon-ai.example.toml
dracon-ai/.gitattributes
dracon-ai/.gitignore
dracon-ai/README.md
dracon-ai/src/main.rs
dracon-ai/target/CACHEDIR.TAG
dracon-ai/target/debug/.cargo-lock
dracon-ai/target/.rustc_info.json

## workspace references to dracon-ai
./UTILITY_BOUNDARIES.md:13:  - `dracon-ai`
./UTILITY_BOUNDARIES.md:16:  - Interactive utility: `dracon-ai`
./UTILITY_BOUNDARIES.md:39:- `dracon-ai`
./UTILITY_BOUNDARIES.md:51:- `dracon-ai`
./UTILITY_BOUNDARIES.md:56:  - May consume `dracon-ai`, but does not own sync/warden/system runtime roles.
./UTILITY_BOUNDARIES.md:71:- Active utility binaries are `dracon-sync`, `dracon-warden`, `dracon-system`, and `dracon-ai`.
./CONTRIBUTING.md:32:└── dracon-ai/
./AUDIT.md:93:- **License metadata:** All 4 packages (`dracon-sync`, `dracon-system`, `dracon-warden`, `dracon-ai`) carry `license = "AGPL-3.0-only"`. `LICENSE` (33 KB) is the AGPL v3 text.
./AUDIT.md:174:- `dracon-sync/Cargo.toml:2`, `dracon-system/Cargo.toml:2`, `dracon-warden/Cargo.toml:2`, `dracon-ai/Cargo.toml:2` all set `license = "AGPL-3.0-only"`. Consistent.
./AUDIT.md:285:### 2.10 P3 — `dracon-ai/` lives in the repo but is not in the workspace
./AUDIT.md:287:- **Evidence:** `Cargo.toml` workspace `members = ["dracon-sync", "dracon-system", "dracon-warden"]`. `dracon-ai/` is a standalone Rust package (`dracon-ai/Cargo.toml`) with its own `Cargo.lock` (101 KB) and `src/main.rs` (77 KB).
./AUDIT.md:288:- **AGENTS.md / README / install.sh:** None mention `dracon-ai` as a project deliverable. The CHANGELOG 0.112.0 explicitly notes: *"`install.sh`: Removed dracon-ai build (not in workspace); fixed nonexistent file references"*.
./AUDIT.md:289:- **Impact:** Confusion for new contributors — the directory looks like a 4th binary, but `cargo build --workspace` does not build it. The 101 KB `dracon-ai/Cargo.lock` is a redundant lockfile.
./AUDIT.md:290:- **Fix:** Either (a) move `dracon-ai/` to its own repo, or (b) add it to the workspace members and fix any cross-deps, or (c) add a `dracon-ai/README.md` clarifying "not built by workspace".
./AUDIT.md:374:- Utility table at the top is correct (3 binaries, no dracon-ai).
./AUDIT.md:405:### 5.3 P3 — `dracon-ai` is a separate package with its own `Cargo.lock` (101 KB) — see finding 2.10
./AUDIT.md:529:12. **Decide `dracon-ai` policy** (finding 2.10). Either move to its own repo, add to workspace, or add a `dracon-ai/README.md` clarifying its standalone status.
./AUDIT.md:703:- Updated `AGENTS.md` test counts, test helper guidance, systemd hardening tables, local-state policy, `dracon-ai/` standalone validation policy, and commit-message guidance.
./AUDIT.md:705:- Fixed `dracon-ai/` standalone dependency paths and updated it to the current `dracon-libs` AI runtime contracts so it validates separately from the main workspace.
./AUDIT.md:713:- Per-crate counts: `dracon-sync` 431 passed, `dracon-system` 83 passed, `dracon-warden` 79 passed, `dracon-security` 99 passed + 6 ignored, `dracon-ai` standalone 7 passed.
./AUDIT.md:714:- `cargo test --manifest-path dracon-ai/Cargo.toml -- --test-threads=1` — passed: **7 passed**, 1 suite.
./AGENTS.md:35:**Workspace policy:** the root Cargo workspace intentionally includes `dracon-sync`, `dracon-system`, and `dracon-warden` only. `dracon-ai/` is a standalone subcrate and must be validated separately when touched; do not fold it into the main workspace without a separate compatibility review.
./AGENTS.md:37:Standalone validation for `dracon-ai/`:
./AGENTS.md:40:cargo test --manifest-path dracon-ai/Cargo.toml -- --test-threads=1
./AGENTS.md:854:- `dracon-ai` standalone: 7 passed, 1 suite (`cargo test --manifest-path dracon-ai/Cargo.toml -- --test-threads=1`).
./docs/public-readiness.md:72:- `cargo test --manifest-path dracon-ai/Cargo.toml -- --test-threads=1`
./docs/archive/MASTER_ROADMAP_2026-06-01.md:83:- **Category D** (1 repo): Data dir only — dracon-ai-lib .dracon/
./docs/archive/STUCK_PUSH_TRIAGE_2026-06-02.md:17:| 5 | dracon-ai-lib | target/debug/examples/basic_chat-* | 78MB | `target/` |
./docs/archive/STUCK_PUSH_TRIAGE_2026-06-02.md:45:- **avid, ai-auto-writer, dracon-code, dracon-ai-lib, rust-ai-web-auto**: `--invert-paths --path target/`
./docs/archive/REPOS_CLEANUP_PLAN_2026-06-01.md:68:#### D1. dracon-ai-lib (branch: main)
./docs/archive/REPOS_CLEANUP_PLAN_2026-06-01.md:102:For Category D (dracon-ai-lib), additionally evaluate whether `.dracon/` should be tracked or added to .gitignore.
./docs/archive/REPOS_CLEANUP_PLAN_2026-06-01.md:142:### Step 4: Investigate dracon-ai-lib .dracon/ Data (D1)
./docs/archive/REPOS_CLEANUP_PLAN_2026-06-01.md:145:cd ~/Dev/dracon-ai-lib
./docs/archive/REPOS_CLEANUP_PLAN_2026-06-01.md:174:- **Low risk**: Investigating dracon-ai-lib .dracon/ — just inspection
./docs/public-release-branch/PUBLIC_RELEASE_PREP.md:157:cargo test --manifest-path dracon-ai/Cargo.toml -- --test-threads=1
./Cargo.toml:8:    "dracon-ai",
./Cargo.toml:31:dracon-ai-runtime-contracts = { path = "../dracon-libs/contracts/crates/ai/dracon-ai-runtime-contracts" }
./docs/public-release-plan.md:227:cargo test --manifest-path dracon-ai/Cargo.toml -- --test-threads=1
./dracon-ai/README.md:1:# dracon-ai
./dracon-ai/README.md:3:`dracon-ai` is the **only** Dracon AI CLI. It is intentionally thin: it does **not** implement provider/model wiring itself.
./dracon-ai/README.md:9:- No direct provider hookup logic in this repo (no OpenRouter/OpenAI/Anthropic “native” client logic in `dracon-ai`).
./dracon-ai/README.md:15:### `dracon-ai` (default)
./dracon-ai/README.md:21:By default, interactive `do` mode is opened in a **new terminal tab** when possible. Use `dracon-ai do --same-terminal` to keep it in the current terminal.
./dracon-ai/README.md:23:### `dracon-ai status`
./dracon-ai/README.md:31:### `dracon-ai do [--plan] [--dangerous] [task...]`
./dracon-ai/README.md:36:- Plan-only: `dracon-ai do --plan ...` (or `DRACON_AI_APPLY=0`).
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
./dracon-ai/README.md:100:`dracon-ai` follows the `dracon-libs` resolution behavior.
./dracon-ai/README.md:114:- `dracon-ai do ...` (plan+execute loop, default)
./dracon-ai/README.md:116:- `dracon-ai cmd ...` (one-shot capture+ask; requires `DRACON_AI_ALLOW_CMD=1`)
./dracon-ai/dracon-ai.example.toml:2:# Path: ~/.dracon/utilities/ai/dracon-ai.toml
./docs/audit/2026-06-11-full-repo-audit/inventory.tsv:17:/home/dracon/Dev/dracon-ai-lib	main	0	0	0	13	0	AHEAD:13,STUCK_PUSH	STUCK	run repair-concerns --apply (push or rewrite)
./docs/audit/2026-06-11-full-repo-audit/final/inventory.tsv:19:/home/dracon/Dev/dracon-ai-lib	main	0	0	0	13	0	AHEAD:13,STUCK_PUSH	STUCK	run repair-concerns --apply (push or rewrite)
./docs/audit/2026-06-11-full-repo-audit/inventory.json:300:      "last_msg": "refactor(ai): migrate from archived dracon-ai-lib to ai-api-sdk",
./docs/audit/2026-06-11-full-repo-audit/inventory.json:356:      "repo": "/home/dracon/Dev/dracon-ai-lib",
./docs/audit/2026-06-11-full-repo-audit/final-validation.tsv:15:dracon-ai-lib	0	0	0	
./docs/audit/2026-06-11-full-repo-audit/final/REPORT.md:4:Scope: every repo reported by `dracon-sync repos --json --full-path`, explicitly including `dracon-ai-lib`.
./docs/audit/2026-06-11-full-repo-audit/final/REPORT.md:17:   - Removed `dracon-ai-lib` from `exclude_repos` in the sync policy so it is included in `dracon-sync repos`.
./docs/audit/2026-06-11-full-repo-audit/final/REPORT.md:20:2. **`dracon-ai-lib`**
./docs/audit/2026-06-11-full-repo-audit/final/REPORT.md:21:   - Fixed invalid origin URL and pointed it at the valid `https://github.com/DraconDev/dracon-ai-lib.git` remote.
./docs/audit/2026-06-11-full-repo-audit/final/REPORT.md:40:   - Fixed stale Dracon AI integration references to unavailable `dracon_ai_contracts` / `dracon_ai_client` APIs.
./docs/audit/2026-06-11-full-repo-audit/final/REPORT.md:68:| `dracon-ai-lib` | 0 | 0 | 0 | pass; push still stuck |
./docs/audit/2026-06-11-full-repo-audit/final/REPORT.md:98:| `dracon-ai-lib` | **Blocked** | Local validation passes, but repo is ahead 13 and push is stuck. Needs explicit remote/recreate/unarchive/rewrite decision. |
./docs/audit/2026-06-11-full-repo-audit/final/REPORT.md:111:2. **`dracon-ai-lib`**
./docs/audit/2026-06-11-full-repo-audit/release-readiness/inventory-current.json:333:      "repo": "/home/dracon/Dev/dracon-ai-lib",
./docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-code.risk.tsv:19:tracked	.ralph/phase3-dracon-ai-extraction.md
./docs/audit/2026-06-11-full-repo-audit/final/risk-paths/dracon-code.risk.tsv:20:tracked	.ralph/phase3-dracon-ai-extraction.state.json
./docs/audit/2026-06-11-full-repo-audit/final/inventory.json:402:      "repo": "/home/dracon/Dev/dracon-ai-lib",
./docs/audit/2026-06-11-full-repo-audit/final/final-validation.tsv:15:dracon-ai-lib	0	0	0	
./docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:16:/home/dracon/Dev/dracon-code	tracked	.ralph/phase3-dracon-ai-extraction.md	blocked-needs-approval	local task/session state; ambiguous user-owned content
./docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:17:/home/dracon/Dev/dracon-code	tracked	.ralph/phase3-dracon-ai-extraction.state.json	blocked-needs-approval	local task/session state; ambiguous user-owned content
./docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1851:/home/dracon/Dev/dracon-ai-lib	tracked	.env	blocked-needs-approval	secret-like file requires rotation/approval before removal
./docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1852:/home/dracon/Dev/dracon-ai-lib	tracked	.env.example	blocked-needs-approval	secret-like file requires rotation/approval before removal
./docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1853:/home/dracon/Dev/dracon-ai-lib	tracked	.pi/goals/archived/goal_2026053121495992_mpu8240y-aobl2r.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
./docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1854:/home/dracon/Dev/dracon-ai-lib	tracked	.pi/goals/archived/goal_2026060102321437_mpugtb4f-82ryaq.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
./docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1855:/home/dracon/Dev/dracon-ai-lib	tracked	.pi/goals/archived/goal_2026060121193146_mpv56bb9-1tm2an.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
./docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1856:/home/dracon/Dev/dracon-ai-lib	tracked	.pi/goals/archived/goal_2026060123165134_mpvrmjhm-pknz4m.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
./docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1857:/home/dracon/Dev/dracon-ai-lib	tracked	.pi/goals/archived/goal_2026060210011760_mpwehb17-9tq3v5.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
./docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1858:/home/dracon/Dev/dracon-ai-lib	tracked	.pi/goals/archived/goal_2026060210385994_mpwfm8m2-o16qqa.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
./docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1859:/home/dracon/Dev/dracon-ai-lib	tracked	.pi/goals/archived/goal_2026060211120567_mpwgv778-gfdgir.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
./docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1860:/home/dracon/Dev/dracon-ai-lib	tracked	.pi/goals/archived/goal_2026060212521240_mpwkld79-u6b4c0.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
./docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1861:/home/dracon/Dev/dracon-ai-lib	tracked	.pi/goals/archived/goal_2026060214212564_mpwm8hys-a2ky77.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
./docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1862:/home/dracon/Dev/dracon-ai-lib	tracked	.pi/goals/archived/goal_2026060215052605_mpwoi3iu-3qlf61.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
./docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1863:/home/dracon/Dev/dracon-ai-lib	tracked	.pi/goals/archived/goal_2026060217222514_mpwu7sxu-c5e4um.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
./docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1864:/home/dracon/Dev/dracon-ai-lib	tracked	.pi/goals/archived/goal_2026060218052396_mpwvp55w-mf378w.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
./docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1865:/home/dracon/Dev/dracon-ai-lib	tracked	.pi/goals/archived/goal_2026060317495547_mpy5dihe-174b31.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
./docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1866:/home/dracon/Dev/dracon-ai-lib	tracked	.pi/goals/archived/goal_2026060415231066_mpzi21xo-6k45hv.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
./docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1867:/home/dracon/Dev/dracon-ai-lib	tracked	.pi/goals/archived/goal_2026060416554220_mpzm5wwy-4v5mkd.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
./docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1868:/home/dracon/Dev/dracon-ai-lib	tracked	.pi/goals/archived/goal_2026060420041353_mpzusmll-tkuavh.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
./docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1869:/home/dracon/Dev/dracon-ai-lib	tracked	.pi/goals/archived/goal_2026060421392982_mpzy1kjm-7ooplq.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
./docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1870:/home/dracon/Dev/dracon-ai-lib	tracked	.pi/goals/archived/goal_2026060422553256_mq00aa8p-q78q3b.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
./docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1871:/home/dracon/Dev/dracon-ai-lib	tracked	.pi/goals/archived/goal_2026060500465297_mq052bj9-1hi21x.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
./docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1872:/home/dracon/Dev/dracon-ai-lib	tracked	.pi/goals/archived/goal_2026060511014963_mq0qqfj2-7mdgp7.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
./docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1873:/home/dracon/Dev/dracon-ai-lib	tracked	.pi/goals/archived/goal_2026060517130651_mq13cixj-o88468.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
./docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1874:/home/dracon/Dev/dracon-ai-lib	tracked	.pi/goals/archived/goal_2026060519131616_mq18k8nb-8z83jr.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
./docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1875:/home/dracon/Dev/dracon-ai-lib	tracked	.pi/goals/archived/goal_2026060600082972_mq1j5294-ibcxfd.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
./docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1876:/home/dracon/Dev/dracon-ai-lib	tracked	.pi/goals/archived/goal_2026060614351979_mq29t6kn-pno183.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
./docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1877:/home/dracon/Dev/dracon-ai-lib	tracked	.pi/goals/archived/goal_2026060621301132_mq2jyqty-gznsws.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
./docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1878:/home/dracon/Dev/dracon-ai-lib	tracked	.pi/goals/archived/goal_2026060723330028_mq4cpmy6-srv0bj.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
./docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1879:/home/dracon/Dev/dracon-ai-lib	tracked	.pi/goals/archived/goal_2026060811353888_mq4gszze-8rib4x.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
./docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1880:/home/dracon/Dev/dracon-ai-lib	tracked	.pi/goals/archived/goal_2026060813080905_mq53j6wk-tc1udv.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
./docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1881:/home/dracon/Dev/dracon-ai-lib	tracked	.pi/goals/archived/goal_2026060816375771_mq5ckllx-58ktk5.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
./docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1882:/home/dracon/Dev/dracon-ai-lib	tracked	.pi/goals/archived/goal_2026060817103628_mq5ebocf-8c0p11.md	blocked-by-pi-exclusion	.pi local task state must not be cleaned
./docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1883:/home/dracon/Dev/dracon-ai-lib	tracked	.pi/goals/goal_events.jsonl	blocked-by-pi-exclusion	.pi local task state must not be cleaned
./docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/candidates/cleanup-candidates.tsv:1884:/home/dracon/Dev/dracon-ai-lib	tracked	crates/client/.env	blocked-needs-approval	secret-like file requires rotation/approval before removal
./docs/audit/2026-06-11-full-repo-audit/final/hygiene.tsv:19:/home/dracon/Dev/dracon-ai-lib	35	3	14	1	0
./CHANGELOG.md:253:- **install.sh**: Removed dracon-ai build (not in workspace); fixed nonexistent file references
./docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/REPORT.md:62:2. **`dracon-ai-lib`**
./docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/REPORT.md:75:   - `dracon-code`, `browser-extensions-shared`, and `dracon-ai-lib` have user-owned changes that were preserved.
./docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/REPORT.md:116:The highest-risk browser profile/cache data and generated coverage were removed. `.pi` was proven unchanged. Remaining public-release blockers are now limited to secret rotation/approval decisions, preserved user-owned content, user-owned changes, and the pre-existing `dracon-ai-lib` push blocker.
./docs/audit/2026-06-11-full-repo-audit/final/per-repo/rust-ai-web-auto.git.txt:18:3a55f5a 2 file(s) in examples,src [examples/dracon_ai_smoke.rs, src/env_keys.rs] DELTA:+12/-7
./docs/audit/2026-06-11-full-repo-audit/final/per-repo/rust-ai-web-auto.git.txt:19:c698705 4 file(s) in examples,src [examples/dracon_ai_smoke.rs, src/doctor.rs, Cargo.lock] DELTA:+169/-1 | NEW:examples/dracon_ai_smoke.rs
./docs/audit/2026-06-11-full-repo-audit/final/per-repo/repos.tsv:18:/home/dracon/Dev/dracon-ai-lib
./docs/audit/2026-06-11-full-repo-audit/final/per-repo/dracon-ai-lib.git.txt:1:REPO=/home/dracon/Dev/dracon-ai-lib
./docs/audit/2026-06-11-full-repo-audit/final/per-repo/dracon-ai-lib.git.txt:4:origin	https://github.com/DraconDev/dracon-ai-lib.git (fetch)
./docs/audit/2026-06-11-full-repo-audit/final/per-repo/dracon-ai-lib.git.txt:5:origin	https://github.com/DraconDev/dracon-ai-lib.git (push)
./dracon-ai/src/main.rs:4:use dracon_ai_contracts::{RoutingTask, SelectionConstraints};
./dracon-ai/src/main.rs:5:use dracon_ai_runtime_contracts::models::{ChatMessage, ChatRequest};
./dracon-ai/src/main.rs:6:use dracon_ai_runtime_contracts::traits::AiProvider;
./dracon-ai/src/main.rs:49:            .join("dracon-ai.toml"),
./dracon-ai/src/main.rs:68:    name = "dracon-ai",
./dracon-ai/src/main.rs:538:    let tool = ansi("1;36", "dracon-ai"); // bold cyan
./dracon-ai/src/main.rs:673:            if status_ok("tmux", &["new-window", "-n", "dracon-ai", &cmd]) {
./dracon-ai/src/main.rs:706:                "dracon-ai",
./dracon-ai/src/main.rs:721:        c.args(["--tab", "--title=dracon-ai", "--", &exe_s])
./dracon-ai/src/main.rs:731:            .args(["--new-tab", "-p", "tabtitle=dracon-ai", "-e", &exe_s])
./dracon-ai/src/main.rs:810:        "You are dracon-ai, a computer-context assistant.",
./dracon-ai/src/main.rs:990:    println!("🔧 dracon-ai setup");
./dracon-ai/src/main.rs:1023:        println!("  dracon-ai setup --refresh");
./dracon-ai/src/main.rs:1092:    println!("Run 'dracon-ai status' to verify.");
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
./docs/audit/2026-06-11-full-repo-audit/final/per-repo/ai-auto-repo-rot-scanner-todo-agent.git.txt:16:90c4433 refactor(ai): migrate from archived dracon-ai-lib to ai-api-sdk
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-before.json:195:      "repo": "/home/dracon/Dev/dracon-ai-lib",
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/git-evidence.txt:4:/home/dracon/Dev/dracon-ai-lib	main	0	0	0	28	0	AHEAD:28,STUCK_PUSH	STUCK	run repair-concerns --apply (push or rewrite)
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/git-evidence.txt:7:origin	https://github.com/DraconDev/dracon-ai-lib.git (fetch)
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/git-evidence.txt:8:origin	https://github.com/DraconDev/dracon-ai-lib.git (push)
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/git-evidence.txt:25:6882198 simplify: drop the dracon-ai/* cutover theater; use the real repo URL
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/git-evidence.txt:27:3acafd9 docs: stage consumer cutover plan and align README to dracon-ai org
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/git-evidence.txt:68:remote.origin.url https://github.com/DraconDev/dracon-ai-lib.git
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/git-evidence.txt:72:fatal: unable to access 'https://github.com/DraconDev/dracon-ai-lib.git/': The requested URL returned error: 403
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/git-evidence.txt:84:{"defaultBranchRef":{"name":"main"},"description":"","isArchived":true,"url":"https://github.com/DraconDev/dracon-ai-lib","visibility":"PRIVATE"}
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/git-evidence.txt:86:{"archived":true,"default_branch":"main","full_name":"DraconDev/dracon-ai-lib","permissions":{"admin":true,"maintain":true,"pull":true,"push":true,"triage":true},"visibility":"private"}
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/git-evidence.txt:99:14397af archive: fix remaining dracon-ai-sdk references to ai-api-sdk
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/git-evidence.txt:120:6882198 simplify: drop the dracon-ai/* cutover theater; use the real repo URL
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/git-evidence.txt:122:3acafd9 docs: stage consumer cutover plan and align README to dracon-ai org
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/git-evidence.txt:135:92d4f2b chore: rename repo to dracon-ai-lib
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/after-fix.txt:3:/home/dracon/Dev/dracon-ai-lib	main	1	0	0	28	0	DIRTY,AHEAD:28,STUCK_PUSH	STUCK	run repair-concerns --apply (push or rewrite)
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-final-audit.tsv:2:/home/dracon/Dev/dracon-ai-lib	main	0	0	0	29	0	AHEAD:29,STUCK_PUSH	STUCK	run repair-concerns --apply (push or rewrite)
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:1:# `dracon-ai-lib` stuck-push investigation
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:7:`dracon-ai-lib` is marked `CONCERN` because it is clean locally but cannot push its local `main` branch to GitHub. The remote repository is archived/read-only, so `git push` fails with HTTP 403.
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:11:- Repo: `/home/dracon/Dev/dracon-ai-lib`
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:18:- Root cause: `DraconDev/dracon-ai-lib` is archived and read-only on GitHub.
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:26:`/home/dracon/Dev/dracon-utilities/docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/`
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:44:/home/dracon/Dev/dracon-ai-lib	main	0	0	0	28	0	AHEAD:28,STUCK_PUSH	STUCK	run repair-concerns --apply (push or rewrite)
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:51:origin	https://github.com/DraconDev/dracon-ai-lib.git (fetch)
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:52:origin	https://github.com/DraconDev/dracon-ai-lib.git (push)
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:73:fatal: unable to access 'https://github.com/DraconDev/dracon-ai-lib.git/': The requested URL returned error: 403
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:76:`gh repo view DraconDev/dracon-ai-lib --json isArchived,visibility,defaultBranchRef,url,description` reported:
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:83:  "url": "https://github.com/DraconDev/dracon-ai-lib",
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:88:`gh api repos/DraconDev/dracon-ai-lib --jq '{full_name,archived,visibility,default_branch,permissions}'` reported:
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:94:  "full_name": "DraconDev/dracon-ai-lib",
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:123:The incident ledger contains repeated `concern` entries for `/home/dracon/Dev/dracon-ai-lib` with:
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:130:details=fatal: unable to access 'https://github.com/DraconDev/dracon-ai-lib.git/': The requested URL returned error: 403
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:147:- `cargo test --manifest-path dracon-ai-lib/Cargo.toml -- --test-threads=1` → **181 passed, 0 failed**
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:148:- `cargo clippy --manifest-path dracon-ai-lib/Cargo.toml --workspace -- -D warnings` → pass
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:162:/home/dracon/Dev/dracon-ai-lib	main	0	0	0	29	0	AHEAD:29,STUCK_PUSH	STUCK	run repair-concerns --apply (push or rewrite)
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:169:origin	https://github.com/DraconDev/dracon-ai-lib.git (fetch)
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:170:origin	https://github.com/DraconDev/dracon-ai-lib.git (push)
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:180:fatal: unable to access 'https://github.com/DraconDev/dracon-ai-lib.git/': The requested URL returned error: 403
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:185:`dracon-sync` is correct to mark `dracon-ai-lib` as a concern.
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:187:The repo is not unhealthy locally: it is clean, tests pass, and clippy passes. The concern is external to the working tree: GitHub has archived `DraconDev/dracon-ai-lib`, making the origin read-only. Because local `main` is 29 commits ahead of `origin/main`, every push attempt fails and the repo remains `AHEAD:29,STUCK_PUSH`.
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:200:1. **If `dracon-ai-lib` should continue to be the canonical repo**
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:201:   - Unarchive `DraconDev/dracon-ai-lib` on GitHub.
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/REPORT.md:205:2. **If `dracon-ai-lib` should move to a new active repo**
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-after.tsv:2:/home/dracon/Dev/dracon-ai-lib	main	1	0	0	28	0	DIRTY,AHEAD:28,STUCK_PUSH	STUCK	run repair-concerns --apply (push or rewrite)
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/final-git-evidence.txt:3:origin	https://github.com/DraconDev/dracon-ai-lib.git (fetch)
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/final-git-evidence.txt:4:origin	https://github.com/DraconDev/dracon-ai-lib.git (push)
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/final-git-evidence.txt:15:fatal: unable to access 'https://github.com/DraconDev/dracon-ai-lib.git/': The requested URL returned error: 403
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/sync-evidence.txt:14:--- incident ledger recent dracon-ai-lib ---
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/sync-evidence.txt:15:{"ts_unix":1781178421,"scope":"concern","repo":"/home/dracon/Dev/dracon-ai-lib","reason":"AHEAD:28,STUCK_PUSH","action":"push_origin_head","backup_branch":null,"result":"fail","details":"git push failed in /home/dracon/Dev/dracon-ai-lib with status exit status: 128: remote: This repository was archived so it is read-only.\nfatal: unable to access 'https://github.com/DraconDev/dracon-ai-lib.git/': The requested URL returned error: 403"}
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/sync-evidence.txt:16:{"ts_unix":1781178558,"scope":"concern","repo":"/home/dracon/Dev/dracon-ai-lib","reason":"AHEAD:28,STUCK_PUSH","action":"push_origin_head","backup_branch":null,"result":"fail","details":"git push failed in /home/dracon/Dev/dracon-ai-lib with status exit status: 128: remote: This repository was archived so it is read-only.\nfatal: unable to access 'https://github.com/DraconDev/dracon-ai-lib.git/': The requested URL returned error: 403"}
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/sync-evidence.txt:17:{"ts_unix":1781178675,"scope":"concern","repo":"/home/dracon/Dev/dracon-ai-lib","reason":"AHEAD:28,STUCK_PUSH","action":"push_origin_head","backup_branch":null,"result":"fail","details":"git push failed in /home/dracon/Dev/dracon-ai-lib with status exit status: 128: remote: This repository was archived so it is read-only.\nfatal: unable to access 'https://github.com/DraconDev/dracon-ai-lib.git/': The requested URL returned error: 403"}
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/sync-evidence.txt:18:{"ts_unix":1781178791,"scope":"concern","repo":"/home/dracon/Dev/dracon-ai-lib","reason":"AHEAD:28,STUCK_PUSH","action":"push_origin_head","backup_branch":null,"result":"fail","details":"git push failed in /home/dracon/Dev/dracon-ai-lib with status exit status: 128: remote: This repository was archived so it is read-only.\nfatal: unable to access 'https://github.com/DraconDev/dracon-ai-lib.git/': The requested URL returned error: 403"}
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/sync-evidence.txt:19:{"ts_unix":1781178888,"scope":"concern","repo":"/home/dracon/Dev/dracon-ai-lib","reason":"AHEAD:28,STUCK_PUSH","action":"push_origin_head","backup_branch":null,"result":"fail","details":"git push failed in /home/dracon/Dev/dracon-ai-lib with status exit status: 128: remote: This repository was archived so it is read-only.\nfatal: unable to access 'https://github.com/DraconDev/dracon-ai-lib.git/': The requested URL returned error: 403"}
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/sync-evidence.txt:20:{"ts_unix":1781178999,"scope":"concern","repo":"/home/dracon/Dev/dracon-ai-lib","reason":"AHEAD:28,STUCK_PUSH","action":"push_origin_head","backup_branch":null,"result":"fail","details":"git push failed in /home/dracon/Dev/dracon-ai-lib with status exit status: 128: remote: This repository was archived so it is read-only.\nfatal: unable to access 'https://github.com/DraconDev/dracon-ai-lib.git/': The requested URL returned error: 403"}
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/sync-evidence.txt:21:{"ts_unix":1781179120,"scope":"concern","repo":"/home/dracon/Dev/dracon-ai-lib","reason":"AHEAD:28,STUCK_PUSH","action":"push_origin_head","backup_branch":null,"result":"fail","details":"git push failed in /home/dracon/Dev/dracon-ai-lib with status exit status: 128: remote: This repository was archived so it is read-only.\nfatal: unable to access 'https://github.com/DraconDev/dracon-ai-lib.git/': The requested URL returned error: 403"}
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/sync-evidence.txt:22:{"ts_unix":1781179248,"scope":"concern","repo":"/home/dracon/Dev/dracon-ai-lib","reason":"AHEAD:28,STUCK_PUSH","action":"push_origin_head","backup_branch":null,"result":"fail","details":"git push failed in /home/dracon/Dev/dracon-ai-lib with status exit status: 128: remote: This repository was archived so it is read-only.\nfatal: unable to access 'https://github.com/DraconDev/dracon-ai-lib.git/': The requested URL returned error: 403"}
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/sync-evidence.txt:23:{"ts_unix":1781179377,"scope":"concern","repo":"/home/dracon/Dev/dracon-ai-lib","reason":"AHEAD:28,STUCK_PUSH","action":"push_origin_head","backup_branch":null,"result":"fail","details":"git push failed in /home/dracon/Dev/dracon-ai-lib with status exit status: 128: remote: This repository was archived so it is read-only.\nfatal: unable to access 'https://github.com/DraconDev/dracon-ai-lib.git/': The requested URL returned error: 403"}
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/sync-evidence.txt:24:{"ts_unix":1781179511,"scope":"concern","repo":"/home/dracon/Dev/dracon-ai-lib","reason":"AHEAD:28,STUCK_PUSH","action":"push_origin_head","backup_branch":null,"result":"fail","details":"git push failed in /home/dracon/Dev/dracon-ai-lib with status exit status: 128: remote: This repository was archived so it is read-only.\nfatal: unable to access 'https://github.com/DraconDev/dracon-ai-lib.git/': The requested URL returned error: 403"}
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/sync-evidence.txt:25:{"ts_unix":1781179616,"scope":"concern","repo":"/home/dracon/Dev/dracon-ai-lib","reason":"AHEAD:28,STUCK_PUSH","action":"push_origin_head","backup_branch":null,"result":"fail","details":"git push failed in /home/dracon/Dev/dracon-ai-lib with status exit status: 128: remote: This repository was archived so it is read-only.\nfatal: unable to access 'https://github.com/DraconDev/dracon-ai-lib.git/': The requested URL returned error: 403"}
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/sync-evidence.txt:26:{"ts_unix":1781179717,"scope":"concern","repo":"/home/dracon/Dev/dracon-ai-lib","reason":"AHEAD:28,STUCK_PUSH","action":"push_origin_head","backup_branch":null,"result":"fail","details":"git push failed in /home/dracon/Dev/dracon-ai-lib with status exit status: 128: remote: This repository was archived so it is read-only.\nfatal: unable to access 'https://github.com/DraconDev/dracon-ai-lib.git/': The requested URL returned error: 403"}
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/sync-evidence.txt:27:{"ts_unix":1781179811,"scope":"concern","repo":"/home/dracon/Dev/dracon-ai-lib","reason":"AHEAD:28,STUCK_PUSH","action":"push_origin_head","backup_branch":null,"result":"fail","details":"git push failed in /home/dracon/Dev/dracon-ai-lib with status exit status: 128: remote: This repository was archived so it is read-only.\nfatal: unable to access 'https://github.com/DraconDev/dracon-ai-lib.git/': The requested URL returned error: 403"}
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/sync-evidence.txt:28:{"ts_unix":1781180066,"scope":"concern","repo":"/home/dracon/Dev/dracon-ai-lib","reason":"AHEAD:28,STUCK_PUSH","action":"push_origin_head","backup_branch":null,"result":"fail","details":"git push failed in /home/dracon/Dev/dracon-ai-lib with status exit status: 128: remote: This repository was archived so it is read-only.\nfatal: unable to access 'https://github.com/DraconDev/dracon-ai-lib.git/': The requested URL returned error: 403"}
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/sync-evidence.txt:29:{"ts_unix":1781180455,"scope":"concern","repo":"/home/dracon/Dev/dracon-ai-lib","reason":"AHEAD:28,STUCK_PUSH","action":"push_origin_head","backup_branch":null,"result":"fail","details":"git push failed in /home/dracon/Dev/dracon-ai-lib with status exit status: 128: remote: This repository was archived so it is read-only.\nfatal: unable to access 'https://github.com/DraconDev/dracon-ai-lib.git/': The requested URL returned error: 403"}
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/sync-evidence.txt:30:{"ts_unix":1781180627,"scope":"concern","repo":"/home/dracon/Dev/dracon-ai-lib","reason":"AHEAD:28,STUCK_PUSH","action":"push_origin_head","backup_branch":null,"result":"fail","details":"git push failed in /home/dracon/Dev/dracon-ai-lib with status exit status: 128: remote: This repository was archived so it is read-only.\nfatal: unable to access 'https://github.com/DraconDev/dracon-ai-lib.git/': The requested URL returned error: 403"}
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/sync-evidence.txt:31:{"ts_unix":1781180729,"scope":"concern","repo":"/home/dracon/Dev/dracon-ai-lib","reason":"AHEAD:28,STUCK_PUSH","action":"push_origin_head","backup_branch":null,"result":"fail","details":"git push failed in /home/dracon/Dev/dracon-ai-lib with status exit status: 128: remote: This repository was archived so it is read-only.\nfatal: unable to access 'https://github.com/DraconDev/dracon-ai-lib.git/': The requested URL returned error: 403"}
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/sync-evidence.txt:32:{"ts_unix":1781181111,"scope":"concern","repo":"/home/dracon/Dev/dracon-ai-lib","reason":"AHEAD:28,STUCK_PUSH","action":"push_origin_head","backup_branch":null,"result":"fail","details":"git push failed in /home/dracon/Dev/dracon-ai-lib with status exit status: 128: remote: This repository was archived so it is read-only.\nfatal: unable to access 'https://github.com/DraconDev/dracon-ai-lib.git/': The requested URL returned error: 403"}
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/sync-evidence.txt:33:{"ts_unix":1781181219,"scope":"concern","repo":"/home/dracon/Dev/dracon-ai-lib","reason":"AHEAD:28,STUCK_PUSH","action":"push_origin_head","backup_branch":null,"result":"fail","details":"git push failed in /home/dracon/Dev/dracon-ai-lib with status exit status: 128: remote: This repository was archived so it is read-only.\nfatal: unable to access 'https://github.com/DraconDev/dracon-ai-lib.git/': The requested URL returned error: 403"}
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/sync-evidence.txt:34:{"ts_unix":1781181320,"scope":"concern","repo":"/home/dracon/Dev/dracon-ai-lib","reason":"AHEAD:28,STUCK_PUSH","action":"push_origin_head","backup_branch":null,"result":"fail","details":"git push failed in /home/dracon/Dev/dracon-ai-lib with status exit status: 128: remote: This repository was archived so it is read-only.\nfatal: unable to access 'https://github.com/DraconDev/dracon-ai-lib.git/': The requested URL returned error: 403"}
./dracon-ai/Cargo.lock:20: "dracon-ai-contracts",
./dracon-ai/Cargo.lock:21: "dracon-ai-runtime-contracts",
./dracon-ai/Cargo.lock:31: "dracon-ai-runtime-contracts",
./dracon-ai/Cargo.lock:285:name = "dracon-ai"
./dracon-ai/Cargo.lock:294: "dracon-ai-contracts",
./dracon-ai/Cargo.lock:295: "dracon-ai-runtime-contracts",
./dracon-ai/Cargo.lock:308:name = "dracon-ai-contracts"
./dracon-ai/Cargo.lock:315:name = "dracon-ai-runtime-contracts"
./dracon-ai/Cargo.lock:320: "dracon-ai-contracts",
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-before.tsv:2:/home/dracon/Dev/dracon-ai-lib	main	0	0	0	28	0	AHEAD:28,STUCK_PUSH	STUCK	run repair-concerns --apply (push or rewrite)
./dracon-ai/Cargo.toml:3:name = "dracon-ai"
./dracon-ai/Cargo.toml:24:dracon-ai-contracts = { path = "../../dracon-libs/contracts/crates/ai/dracon-ai-contracts" }
./dracon-ai/Cargo.toml:25:dracon-ai-runtime-contracts = { path = "../../dracon-libs/contracts/crates/ai/dracon-ai-runtime-contracts" }
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-after-final.json:11:      "repo": "/home/dracon/Dev/dracon-ai-lib",
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-after-final.json:48:      "last_msg": "4 file(s) in docs [docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-…",
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-after.json:47:      "last_msg": "3 file(s) in docs [docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-…",
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-after.json:195:      "repo": "/home/dracon/Dev/dracon-ai-lib",
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-final-audit.json:24:      "last_msg": "3 file(s) in docs [docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-…",
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-final-audit.json:57:      "repo": "/home/dracon/Dev/dracon-ai-lib",
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-stuck-investigation/inventory-after-final.tsv:2:/home/dracon/Dev/dracon-ai-lib	main	0	0	0	29	0	AHEAD:29,STUCK_PUSH	STUCK	run repair-concerns --apply (push or rewrite)
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-before.json:80:      "repo": "/home/dracon/Dev/dracon-ai-lib",
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-before.json:186:      "last_msg": "3 file(s) in docs [docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-…",
./docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/after/inventory.tsv:5:/home/dracon/Dev/dracon-ai-lib	main	0	0	0	28	0	AHEAD:28,STUCK_PUSH	STUCK	run repair-concerns --apply (push or rewrite)
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:3:/home/dracon/Dev/dracon-ai-lib	main	0	0	0	30	0	AHEAD:30,STUCK_PUSH	STUCK	run repair-concerns --apply (push or rewrite)
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:5:{"archivedAt":"2026-06-08T20:06:42Z","createdAt":"2026-05-31T20:31:51Z","defaultBranchRef":{"name":"main"},"description":"","isArchived":true,"updatedAt":"2026-06-08T20:06:42Z","url":"https://github.com/DraconDev/dracon-ai-lib","visibility":"PRIVATE"}
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:7:{"archived":true,"archived_at":null,"default_branch":"main","description":null,"full_name":"DraconDev/dracon-ai-lib","html_url":"https://github.com/DraconDev/dracon-ai-lib","permissions":{"admin":true,"maintain":true,"pull":true,"push":true,"triage":true},"visibility":"private"}
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:10:origin	https://github.com/DraconDev/dracon-ai-lib.git (fetch)
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:11:origin	https://github.com/DraconDev/dracon-ai-lib.git (push)
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:21:6882198 simplify: drop the dracon-ai/* cutover theater; use the real repo URL
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:23:14397af archive: fix remaining dracon-ai-sdk references to ai-api-sdk
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:67:    archive: fix remaining dracon-ai-sdk references to ai-api-sdk
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:92:docs/audit/2026-06-11-full-repo-audit/per-repo/ai-auto-repo-rot-scanner-todo-agent.git.txt:14:* main 90c4433 [origin/main] refactor(ai): migrate from archived dracon-ai-lib to ai-api-sdk
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:93:docs/audit/2026-06-11-full-repo-audit/per-repo/ai-auto-repo-rot-scanner-todo-agent.git.txt:16:90c4433 refactor(ai): migrate from archived dracon-ai-lib to ai-api-sdk
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:96:docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/ai-auto-repo-rot-scanner-todo-agent.git.txt:16:90c4433 refactor(ai): migrate from archived dracon-ai-lib to ai-api-sdk
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:128:docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/dracon-ai-lib.git.txt:7:* main d8846da [origin/main: ahead 21] docs: make crate docs explicit BYOK-library contract
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:129:docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/dracon-ai-lib.git.txt:9:d8846da docs: make crate docs explicit BYOK-library contract
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:130:docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/dracon-ai-lib.git.txt:10:4b70129 4 file(s) in docs [docs/archive/legacy-key-management-design.md, docs/consumer-getting-started.md, README.md] DELTA:+26/-8
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:131:docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/dracon-ai-lib.git.txt:11:210c250 1 file(s) in docs [docs/{key-management-design.md => archive/legacy-key-management-design.md}] DELTA:+0/-0
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:148:docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/PUBLIC_READINESS.md:36:3. **`dracon-ai-lib`**: decide remote strategy. It is still AHEAD:21 and push is blocked by the archived remote.
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:149:docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/PUBLIC_READINESS.md:60:- `dracon-ai-lib`: local validation passes, but push is blocked (AHEAD:21, archived remote).
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:163:docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.dracon-ai-lib.git.txt:9:* main d8846da [origin/main: ahead 21] docs: make crate docs explicit BYOK-library contract
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:164:docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.dracon-ai-lib.git.txt:11:d8846da docs: make crate docs explicit BYOK-library contract
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:165:docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.dracon-ai-lib.git.txt:12:4b70129 4 file(s) in docs [docs/archive/legacy-key-management-design.md, docs/consumer-getting-started.md, README.md] DELTA:+26/-8
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:166:docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.dracon-ai-lib.git.txt:13:210c250 1 file(s) in docs [docs/{key-management-design.md => archive/legacy-key-management-design.md}] DELTA:+0/-0
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:167:docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.dracon-ai-lib.git.txt:17:d8846da docs: make crate docs explicit BYOK-library contract
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/rationale-evidence.txt:168:docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.dracon-ai-lib.git.txt:18:4b70129 4 file(s) in docs [docs/archive/legacy-key-management-design.md, docs/consumer-getting-started.md, README.md] DELTA:+26/-8
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-final-audit.tsv:2:/home/dracon/Dev/dracon-ai-lib	main	0	0	0	0	0	OK	OK	healthy
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/REPORT.md:1:# `dracon-ai-lib` unarchive and push recovery
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/REPORT.md:7:`DraconDev/dracon-ai-lib` has been unarchived and `dracon-ai-lib` push health has been restored.
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/REPORT.md:12:repo: /home/dracon/Dev/dracon-ai-lib
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/REPORT.md:30:- Follow-up commit: `14397af archive: fix remaining dracon-ai-sdk references to ai-api-sdk`
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/REPORT.md:40:Decision: unarchive and keep `dracon-ai-lib` active for direct BYOK Rust consumers, while preserving the guidance that `ai-api-sdk` is the right path for shared gateway/multi-consumer deployments.
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/REPORT.md:47:gh api -X PATCH repos/DraconDev/dracon-ai-lib \
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/REPORT.md:55:{"archived":true,"default_branch":"main","full_name":"DraconDev/dracon-ai-lib","visibility":"private"}
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/REPORT.md:61:{"archived":false,"default_branch":"main","description":"Standalone Rust workspace for an importable BYOK AI client library.","full_name":"DraconDev/dracon-ai-lib","visibility":"private"}
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/REPORT.md:74:  "url": "https://github.com/DraconDev/dracon-ai-lib",
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/REPORT.md:89:- `dracon-ai-lib` is active for direct BYOK Rust consumers.
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/REPORT.md:106:📝 committed 3 file(s) in /home/dracon/Dev/dracon-ai-lib
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/REPORT.md:131:cargo test --manifest-path dracon-ai-lib/Cargo.toml -- --test-threads=1
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/REPORT.md:134:cargo clippy --manifest-path dracon-ai-lib/Cargo.toml --workspace -- -D warnings
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/REPORT.md:144:/home/dracon/Dev/dracon-ai-lib	main	0	0	0	0	0	OK	OK	healthy
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/REPORT.md:165:`/home/dracon/Dev/dracon-utilities/docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/`
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-after.tsv:2:/home/dracon/Dev/dracon-ai-lib	main	0	0	0	0	0	OK	OK	healthy
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:2:# dracon-ai-lib
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:48:ai-lib = { git = "https://github.com/DraconDev/dracon-ai-lib", tag = "v0.2.0" }
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:97:ai-lib = { git = "https://github.com/DraconDev/dracon-ai-lib", tag = "v0.2.0" }
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:108:# dracon-ai-lib — Consumer Guide
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:110:> **⚠️ ARCHIVED — Use [`ai-api-sdk`](https://github.com/DraconDev/dracon-ai-platform/tree/main/crates/ai-api-sdk) in the `dracon-ai-platform` repo instead. This lib is frozen at v0.2.0.**
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:118:| **One consumer, one set of keys, no sharing** (solo dev) | The lib directly (`dracon-ai-client`) |
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:138:| **Direct — `dracon-ai-client` (this lib)** | `AiClient::from_env()` reads `AI_KEY_*` from your env | Solo dev. One consumer, one key set, no sharing. |
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:186:> **Going multi-consumer? Skip these patterns.** Use `ai-api-sdk` against an `ai-api` server with BYOK instead. See the [ai-api-sdk README](https://github.com/DraconDev/dracon-ai-platform) for the BYOK flow.
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:198:dracon-ai-client = { git = "https://github.com/DraconDev/dracon-ai-lib.git", tag = "v0.2.0" }
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:204:dracon-ai-contracts = { git = "https://github.com/DraconDev/dracon-ai-lib.git", tag = "v0.2.0" }
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:218:use dracon_ai_client::AiClient;
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:219:use dracon_ai_contracts::ChatMessage;
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:241:If you want to use the lib's default keys (the ones in `dracon-ai-lib/.env`), you must explicitly opt in by calling `load_lib_env()`:
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:244:use dracon_ai_client::{AiClient, load_lib_env};
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:245:use dracon_ai_contracts::ChatMessage;
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:264:- Finds the lib's `.env` (via `DRACON_AI_LIB_ENV` env var, manifest dir, or `./dracon-ai-lib/.env`)
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:350:dracon-ai-client
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:351:dracon-ai-contracts
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:352:dracon-ai-core
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/current-state.txt:353:dracon-ai-providers
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-before.tsv:2:/home/dracon/Dev/dracon-ai-lib	main	0	0	0	30	0	AHEAD:30,STUCK_PUSH	STUCK	run repair-concerns --apply (push or rewrite)
./docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/after/inventory.json:80:      "repo": "/home/dracon/Dev/dracon-ai-lib",
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-after.json:34:      "repo": "/home/dracon/Dev/dracon-ai-lib",
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-after.json:162:      "last_msg": "4 file(s) in docs [docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-…",
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-final-audit.json:24:      "last_msg": "4 file(s) in docs [docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-…",
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/inventory-final-audit.json:81:      "repo": "/home/dracon/Dev/dracon-ai-lib",
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/post-verification.txt:2:{"archivedAt":null,"createdAt":"2026-05-31T20:31:51Z","defaultBranchRef":{"name":"main"},"description":"Standalone Rust workspace for an importable BYOK AI client library.","isArchived":false,"updatedAt":"2026-06-11T13:11:47Z","url":"https://github.com/DraconDev/dracon-ai-lib","visibility":"PRIVATE"}
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/post-verification.txt:4:{"archived":false,"default_branch":"main","description":"Standalone Rust workspace for an importable BYOK AI client library.","full_name":"DraconDev/dracon-ai-lib","html_url":"https://github.com/DraconDev/dracon-ai-lib","visibility":"private"}
./docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-unarchive/post-verification.txt:7:/home/dracon/Dev/dracon-ai-lib	main	0	0	0	0	0	OK	OK	healthy
./docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/before/inventory.tsv:11:/home/dracon/Dev/dracon-ai-lib	main	2	0	0	21	0	DIRTY,AHEAD:21,STUCK_PUSH	STUCK	run repair-concerns --apply (push or rewrite)
./docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/before/inventory.json:218:      "repo": "/home/dracon/Dev/dracon-ai-lib",
./docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-code.risk.tsv:19:tracked	.ralph/phase3-dracon-ai-extraction.md
./docs/audit/2026-06-11-full-repo-audit/risk-paths/dracon-code.risk.tsv:20:tracked	.ralph/phase3-dracon-ai-extraction.state.json
./docs/audit/2026-06-11-full-repo-audit/hygiene.tsv:17:/home/dracon/Dev/dracon-ai-lib	35	3	14	1	0
./docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-before.json:141:      "last_msg": "2 file(s) in docs [docs/audit/2026-06-11-full-repo-audit/dracon-ai-lib-…",
./docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-before.json:174:      "repo": "/home/dracon/Dev/dracon-ai-lib",
./docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-after-remediation.json:70:      "last_msg": "5 file(s) in crates,plugins [crates/dracon-ai/src/ai_client.rs, crates/…",
./docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-after-remediation.json:231:      "last_msg": "8 file(s) in dracon-ai-sdk [dracon-ai-sdk/src/lib.rs, dracon-ai-sdk/tes…",
./docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-after-remediation.json:287:      "repo": "/home/dracon/Dev/dracon-ai-lib",
./docs/audit/2026-06-11-full-repo-audit/per-repo/rust-ai-web-auto.git.txt:17:3a55f5a 2 file(s) in examples,src [examples/dracon_ai_smoke.rs, src/env_keys.rs] DELTA:+12/-7
./docs/audit/2026-06-11-full-repo-audit/per-repo/rust-ai-web-auto.git.txt:18:c698705 4 file(s) in examples,src [examples/dracon_ai_smoke.rs, src/doctor.rs, Cargo.lock] DELTA:+169/-1 | NEW:examples/dracon_ai_smoke.rs
./docs/audit/2026-06-11-full-repo-audit/per-repo/repos.tsv:16:/home/dracon/Dev/dracon-ai-lib
./docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-post-tests.json:163:      "last_msg": "5 file(s) in crates,plugins [crates/dracon-ai/src/ai_client.rs, crates/…",
./docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-post-tests.json:232:      "last_msg": "8 file(s) in dracon-ai-sdk [dracon-ai-sdk/src/lib.rs, dracon-ai-sdk/tes…",
./docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-post-tests.json:288:      "repo": "/home/dracon/Dev/dracon-ai-lib",
./docs/audit/2026-06-11-full-repo-audit/per-repo/dracon-ai-lib.git.txt:1:REPO=/home/dracon/Dev/dracon-ai-lib
./docs/audit/2026-06-11-full-repo-audit/per-repo/dracon-ai-lib.git.txt:4:origin	https://github.com/DraconDev/dracon-ai-lib.git (fetch)
./docs/audit/2026-06-11-full-repo-audit/per-repo/dracon-ai-lib.git.txt:5:origin	https://github.com/DraconDev/dracon-ai-lib.git (push)
./docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/REPORT.md:241:Current status includes a user change under `crates/dracon-ai/src/ai_client.rs`:
./docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/REPORT.md:244: M crates/dracon-ai/src/ai_client.rs
./docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/REPORT.md:298:- `dracon-ai-lib`: archived-remote blocker was handled in the prior investigation; current push is OK.
./docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-final.json:300:      "last_msg": "8 file(s) in dracon-ai-sdk [dracon-ai-sdk/src/lib.rs, dracon-ai-sdk/tes…",
./docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-final.json:333:      "repo": "/home/dracon/Dev/dracon-ai-lib",
./docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-after-rust-clean.json:162:      "last_msg": "5 file(s) in docs,dracon-ai [docs/audit/2026-06-11-full-repo-audit/rema…",
./docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-after-rust-clean.json:219:      "repo": "/home/dracon/Dev/dracon-ai-lib",
./docs/audit/2026-06-11-full-repo-audit/per-repo/dracon-code.git.txt:26:e53841a6 1 file(s) in crates [crates/dracon-ai/src/lib.rs] DELTA:+1/-1
./docs/audit/2026-06-11-full-repo-audit/per-repo/ai-auto-repo-rot-scanner-todo-agent.git.txt:14:* main 90c4433 [origin/main] refactor(ai): migrate from archived dracon-ai-lib to ai-api-sdk
./docs/audit/2026-06-11-full-repo-audit/per-repo/ai-auto-repo-rot-scanner-todo-agent.git.txt:16:90c4433 refactor(ai): migrate from archived dracon-ai-lib to ai-api-sdk
./docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/hygiene.tsv:9:/home/dracon/Dev/dracon-ai-lib	35	3	14	1	0	31	1	0
./docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/pre-inventory.json:57:      "repo": "/home/dracon/Dev/dracon-ai-lib",
./docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/post-inventory.json:103:      "repo": "/home/dracon/Dev/dracon-ai-lib",
./docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/dracon-utilities-public-release-state.md:11:7f95a61e deps: pin dracon-ai runtime deps to local dracon-libs
./docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/dracon-utilities-public-release-state.md:49: dracon-ai/Cargo.lock                               |  15 +-
./docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/final-inventory.json:103:      "repo": "/home/dracon/Dev/dracon-ai-lib",
./docs/audit/2026-06-11-full-repo-audit/branch-reconciliation/evidence/folder-auto-banner-state.md:10:dracon-ai-lib
./docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.dracon-ai-lib.git.txt:1:REPO=/home/dracon/Dev/dracon-ai-lib
./docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.dracon-ai-lib.git.txt:6:origin	https://github.com/DraconDev/dracon-ai-lib.git (fetch)
./docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.dracon-ai-lib.git.txt:7:origin	https://github.com/DraconDev/dracon-ai-lib.git (push)
./docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/before.dracon-ai-lib.git.txt:15:6882198 simplify: drop the dracon-ai/* cutover theater; use the real repo URL
./docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/repos.tsv:10:/home/dracon/Dev/dracon-ai-lib
./docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.dracon-ai-lib.git.txt:1:REPO=/home/dracon/Dev/dracon-ai-lib
./docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.dracon-ai-lib.git.txt:9:origin	https://github.com/DraconDev/dracon-ai-lib.git (fetch)
./docs/audit/2026-06-11-full-repo-audit/cleanup-except-pi/per-repo/after.dracon-ai-lib.git.txt:10:origin	https://github.com/DraconDev/dracon-ai-lib.git (push)
./docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/inventory.tsv:6:/home/dracon/Dev/dracon-ai-lib	main	0	0	0	21	0	AHEAD:21,STUCK_PUSH	STUCK	run repair-concerns --apply (push or rewrite)
./docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/PUBLIC_READINESS.md:36:3. **`dracon-ai-lib`**: decide remote strategy. It is still AHEAD:21 and push is blocked by the archived remote.
./docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/PUBLIC_READINESS.md:60:- `dracon-ai-lib`: local validation passes, but push is blocked (AHEAD:21, archived remote).
./docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/PUBLIC_READINESS.md:62:- `ai-auto-writer`, `video-factory`, `youtube-video-uploader`, `video-uploader`, `dracon-ai-lib`, `dracon-libs`: tests pass, but pre-existing clippy warnings remain under `-D warnings`.
./docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/inventory.json:103:      "repo": "/home/dracon/Dev/dracon-ai-lib",
./docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/validation-logs/final-validation.tsv:15:dracon-ai-lib	0	dracon-ai-lib.fmt.log	0	dracon-ai-lib.test.log	101	dracon-ai-lib.clippy.log
./docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/hygiene.tsv:6:/home/dracon/Dev/dracon-ai-lib	35	3	14	1	1	0
./docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/dracon-utilities.git.txt:5:? docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/dracon-ai-lib.git.txt
./docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/repos.tsv:1:/home/dracon/Dev/dracon-ai-lib
./docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/dracon-ai-lib.git.txt:1:REPO=/home/dracon/Dev/dracon-ai-lib
./docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/dracon-ai-lib.git.txt:4:origin	https://github.com/DraconDev/dracon-ai-lib.git (fetch)
./docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/dracon-ai-lib.git.txt:5:origin	https://github.com/DraconDev/dracon-ai-lib.git (push)
./docs/audit/2026-06-11-full-repo-audit/public-readiness-funding/per-repo/dracon-ai-lib.git.txt:13:6882198 simplify: drop the dracon-ai/* cutover theater; use the real repo URL
./docs/audit/2026-06-11-full-repo-audit/post-funding/inventory.tsv:14:/home/dracon/Dev/dracon-ai-lib	main	0	0	0	18	0	AHEAD:18,STUCK_PUSH	STUCK	run repair-concerns --apply (push or rewrite)
./docs/audit/2026-06-11-full-repo-audit/post-funding/REPORT.md:17:- Every Dracon-managed repo now has a `.github/FUNDING.yml`. 19 of 20 already had it (committed manually by the operator with `github: [DraconDev]`). 1 (`dracon-ai-lib`) was missing; the standard-files flow scaffolded the empty default template into it. No existing `FUNDING.yml` content was overwritten.
./docs/audit/2026-06-11-full-repo-audit/post-funding/REPORT.md:36:   - `dracon-sync scaffold --repo /home/dracon/Dev/dracon-ai-lib --files '.github/FUNDING.yml'` → 1 file copied. No other repos touched.
./docs/audit/2026-06-11-full-repo-audit/post-funding/REPORT.md:60:| `dracon-ai-lib` | 0 | 0 | 101 | fmt/test pass; clippy reports pre-existing `.filter_map(..)` → `.map(..)` (unchanged from prior audit) |
./docs/audit/2026-06-11-full-repo-audit/post-funding/REPORT.md:113:- `dracon-ai-lib` — local validation passes; push remains blocked (now AHEAD:15 after the FUNDING.yml commit).
./docs/audit/2026-06-11-full-repo-audit/post-funding/REPORT.md:120:2. **`dracon-ai-lib`** — AHEAD:15; push blocked. Needs explicit remote/recreate/rewrite decision.
./docs/audit/2026-06-11-full-repo-audit/post-funding/REPORT.md:121:3. **`ai-auto-writer`, `video-factory`, `youtube-video-uploader`, `video-uploader`, `dracon-ai-lib`, `dracon-libs`** — pre-existing clippy warnings (unchanged by this change). Not blockers for the FUNDING.yml goal; tracked separately.
./docs/audit/2026-06-11-full-repo-audit/post-funding/inventory.json:287:      "repo": "/home/dracon/Dev/dracon-ai-lib",
./docs/audit/2026-06-11-full-repo-audit/post-funding/hygiene.tsv:10:/home/dracon/Dev/dracon-ai-lib	35	3	14	1	1	0
./docs/audit/2026-06-11-full-repo-audit/post-funding/validation-logs/final-validation.tsv:15:dracon-ai-lib	0	dracon-ai-lib.fmt.log	0	dracon-ai-lib.test.log	101	dracon-ai-lib.clippy.log
./docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/rust-ai-web-auto.git.txt:20:3a55f5a 2 file(s) in examples,src [examples/dracon_ai_smoke.rs, src/env_keys.rs] DELTA:+12/-7
./docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/ai-auto-repo-rot-scanner-todo-agent.git.txt:16:90c4433 refactor(ai): migrate from archived dracon-ai-lib to ai-api-sdk
./docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/repos.tsv:9:/home/dracon/Dev/dracon-ai-lib
./docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/dracon-ai-lib.git.txt:1:REPO=/home/dracon/Dev/dracon-ai-lib
./docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/dracon-ai-lib.git.txt:5:dracon-ai	https://github.com/dracon-ai/dracon-ai-lib.git (fetch)
./docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/dracon-ai-lib.git.txt:6:dracon-ai	https://github.com/dracon-ai/dracon-ai-lib.git (push)
./docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/dracon-ai-lib.git.txt:7:origin	https://github.com/DraconDev/dracon-ai-lib.git (fetch)
./docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/dracon-ai-lib.git.txt:8:origin	https://github.com/DraconDev/dracon-ai-lib.git (push)
./docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/dracon-ai-lib.git.txt:10:* main 3acafd9 [origin/main: ahead 15] docs: stage consumer cutover plan and align README to dracon-ai org
./docs/audit/2026-06-11-full-repo-audit/post-funding/per-repo/dracon-ai-lib.git.txt:12:3acafd9 docs: stage consumer cutover plan and align README to dracon-ai org
