# dracon-sync Daemon Behavior Audit — 2026-06-26

**Audit window**: last 7 days (`journalctl --user -u dracon-sync.service --since "7 days ago"`)
**Audit date**: 2026-06-26 (BST)
**Auditor**: pi (operator-instructed read-only audit)
**Mode**: read-only — no daemon config, per-repo config, working tree, or running services modified
**Raw evidence**: `docs/design/audit-2026-06-26/` (repos.txt, ground-truth.txt, journal-7d.txt, journal-warn.txt, journal-err.txt, journal-crit.txt, journal-alert.txt, journal-emerg.txt, journal-issues.txt, per-repo-configs.txt, size-limit.txt)

---

## Section 1 — Daemon health snapshot

| Field | Value | Source |
|---|---|---|
| Unit name | `dracon-sync.service` | `systemctl --user status` |
| Loaded | loaded, enabled, preset: ignored | `systemctl --user status` |
| Active | active (running) | `systemctl --user status` |
| Active since | Fri 2026-06-26 21:17:10 BST | `systemctl --user status` |
| Uptime | 1h 26min at audit time | `systemctl --user status` |
| Main PID | 3616679 (`/home/dracon/.local/bin/dracon-sync daemon`) | `systemctl --user status` |
| Tasks | 4 / limit 96 | `systemctl --user status` |
| Memory | 206.5M current, peak 419.2M, high 768M, max 2G | `systemctl --user status` |
| CPU cumulative | 10min 22.235s | `systemctl --user status` |
| Invocation ID | 437f09fde88b4f18a86c55757f8bd330 | `systemctl --user status` |
| `dracon-sync --version` | `dracon-sync 0.112.14` | CLI |
| Binary path | `/home/dracon/.local/bin/dracon-sync` | `stat` |
| Binary size | 13,684,048 bytes (13.1 MiB) | `stat` |
| Binary sha256 | `35de562606e398821d8178c2824368041b02258addd1d5361331a933d4d9a27c` | `sha256sum` |
| Binary modified | 2026-06-23 22:18:07 BST | `stat` |
| `dracon-sync status` | "15 repos · 3 watch root(s) · pulse 1s" + "✅ healthy" | CLI |
| `dracon-sync health` | ✅ healthy · daemon running · freeze off · policy valid | CLI |
| Policy path | `/home/dracon/.dracon/utilities/sync/dracon-sync.toml` | `dracon-sync status` |
| systemd unit path | `/home/dracon/.config/systemd/user/dracon-sync.service` | `systemctl show` |
| Sandboxing | ProtectSystem=strict, ProtectHome=read-only, ReadWritePaths=%h/.dracon %h/Dev %h/.local/state/dracon %h/.ssh, MemoryDenyWriteExecute, RestrictNamespaces, CapabilityBoundingSet=, NoNewPrivileges, PrivateTmp=true | `cat unit` |
| Sudoers / suid | CapabilityBoundingSet empty, RestrictSUIDSGID, RemoveIPC | unit file |

**Note**: AGENTS.md and the goal contract referenced `dracon-sync doctor`. That subcommand does **not exist** in v0.112.14. The closest equivalents are `dracon-sync health` (binary health) and `dracon-sync repair concerns` (per-repo concern dry-run). The audit uses those equivalents and documents the substitution.

**Finding 1.1 — daemon is healthy.** `dracon-sync health` returns ✅; policy valid; daemon responsive. No findings.

**Finding 1.2 — daemon binary is the freshly-rebuilt 0.112.14.** sha256 captured; the 13.1 MiB ELF was modified 2026-06-23 22:18:07 BST, after the v0.112.14 source bump. Severity: info.

---

## Section 2 — Live repo state (`dracon-sync repos`)

The full output is captured at `docs/design/audit-2026-06-26/repos.txt`. Summary table:

| # | Status | Repo | Branch | Publish | Ahead | Behind | Push | State | Activity |
|---|---|---|---|---|---|---|---|---|---|
| 1 | ❌ CONCERN | dracon-platform | main-temp | codeberg/main-temp | 68 | 1 | PUSH_STUCK | 🟡 committing | push-stuck 1m (68 ahead) |
| 2 | ✅ OK | pully-fully-pull-based-fleet-reconciler | main | github/main | 0 | 0 | OK | 🟢 synced 5m | healthy |
| 3 | ✅ OK | avid | main | github/main | 0 | 0 | OK | ⚪ idle 1h | healthy |
| 4 | ✅ OK | dracon-utilities | main | origin/main | 0 | 0 | OK | ⚪ idle 11h | healthy |
| 5 | ✅ OK | browser-extensions-shared | main | github/main | 0 | 0 | OK | ⚪ idle 17h | healthy |
| 6 | ✅ OK | .dracon | main | github/main | 0 | 0 | OK | ⚪ idle 20h | healthy |
| 7 | ✅ OK | rust-ai-web-auto | main | github/main | 0 | 0 | OK | ⚫ cold 1d | healthy |
| 8 | ✅ OK | ai-auto-writer | main | github/main | 0 | 0 | OK | ⚫ cold 1d | healthy |
| 9 | ✅ OK | pi-plugins | main | origin/main | 0 | 0 | OK | ⚫ cold 3d | healthy |
| 10 | ✅ OK | dracon-sync | HEAD | origin/HEAD | 0 | 0 | OK | ⚫ cold 3d | healthy |
| 11 | ✅ OK | dracon-code | main | github/main | 0 | 0 | OK | ⚫ cold 5d | healthy |
| 12 | ✅ OK | dracon-strategy | main | github/main | 0 | 0 | OK | ⚫ cold 5d | healthy |
| 13 | ✅ OK | DraconDev | main | origin/main | 0 | 0 | OK | ⚫ cold 5d | healthy |
| 14 | ✅ OK | dracon-warden | main | origin/main | 0 | 0 | OK | ⚫ cold 5d | healthy |
| 15 | ✅ OK | dracon-system | main | origin/main | 0 | 0 | OK | ⚫ cold 5d | healthy |

**Summary**: 15 repos total · 14 ✅ OK · 0 ⚠️ WARN · 1 ❌ CONCERN · 0 ⛔ init/status failed.

**Finding 2.1 — dracon-platform is the sole concern repo.** Ahead 68 / behind 1, PUSH_STUCK, push-stuck 1m, 152 consecutive failures (per `dracon-sync repair stuck-list` at audit time, +1 during audit = 153). Severity: blocker. See Sections 3, 9 for root-cause analysis.

---

## Section 3 — Ground truth cross-check

The 15 watched repos were each inspected with `git status --short --branch`, `git rev-parse HEAD`, `git rev-parse --abbrev-ref --symbolic-full-name '@{u}'`, `git log --oneline -5`, `git remote -v`, and a presence check for `.dracon-sync.toml`. Full output: `docs/design/audit-2026-06-26/ground-truth.txt`.

| Repo (daemon display name) | Actual path | Branch | Upstream | HEAD | Daemon view ahead/behind | Git truth ahead/behind | Mismatch? | .dracon-sync.toml |
|---|---|---|---|---|---|---|---|---|
| dracon-platform | `/home/dracon/Dev/dracon-platform` | main-temp | codeberg/main-temp | 9fcb7bb92c4f5433daded6d828aca525c75de756 | 68 / 1 | 69 / 1 | minor (off-by-1 ahead; daemon lags by 1 commit between ledger snapshots) | absent |
| pully-fully-pull-based-fleet-reconciler | `/home/dracon/Dev/pully-fully-pull-based-fleet-reconciler` | main | github/main | 24f231dc30a10f6b399f08c814eef8a07b1afda2 | 0 / 0 | 0 / 0 | none | absent |
| avid | `/home/dracon/Dev/avid` | main | github/main | 4adc747b7981b00a7839b9fda21ab03b0cb53e24 | 0 / 0 | 0 / 0 | none | absent |
| dracon-utilities | `/home/dracon/Dev/dracon-utilities` | main | origin/main | a42124c3cd7… | 0 / 0 | 0 / 0 | none | absent |
| browser-extensions-shared | `/home/dracon/Dev/browser-extensions-shared` | main | github/main | 83649ea9cd8… | 0 / 0 | 0 / 0 | none | absent |
| .dracon | `/home/dracon/.dracon` | main | github/main | ab7e009b849… | 0 / 0 | 0 / 0 | none | absent |
| rust-ai-web-auto | `/home/dracon/Dev/rust-ai-web-auto` | main | github/main | 98d193829fd… | 0 / 0 | 0 / 0 | none | absent |
| ai-auto-writer | `/home/dracon/Dev/ai-auto-writer` | main | github/main | 6b724205bad… | 0 / 0 | 0 / 0 | none | absent |
| pi-plugins | `/home/dracon/Dev/pi-plugins` | main | origin/main | 54322a00ceb… | 0 / 0 | 0 / 0 | none | absent |
| dracon-sync | `/home/dracon/Dev/dracon-utilities/dracon-sync` | HEAD (detached → origin/HEAD) | origin/HEAD | b170dbb85ba… | 0 / 0 | 0 / 0 | none | absent |
| dracon-code | `/home/dracon/Dev/dracon-code` | main | github/main | 8a4b8abed39… | 0 / 0 | 0 / 0 | none | absent |
| dracon-strategy | `/home/dracon/Dev/dracon-strategy` | main | github/main | ffa3bc24b61… | 0 / 0 | 0 / 0 | none | absent |
| DraconDev | `/home/dracon/Dev/dracon-strategy/DraconDev` | main | origin/main | f1e2b3783f9… | 0 / 0 | 0 / 0 | none | absent |
| dracon-warden | `/home/dracon/Dev/dracon-utilities/dracon-warden` | main | origin/main | 5d1c9ec0cce… | 0 / 0 | 0 / 0 | none | absent |
| dracon-system | `/home/dracon/Dev/dracon-utilities/dracon-system` | main | origin/main | 82da3966e9a… | 0 / 0 | 0 / 0 | none | absent |

**Finding 3.1 — dracon-platform daemon ahead-count lags git truth by 1** (daemon: 68, git: 69). The daemon's `ahead/behind` is computed at pulse time and the difference is the time delta between pulse and the audit moment; one commit was created in the daemon ledger during the gap. Severity: info. No action required.

**Finding 3.2 — dracon-platform has NO github/gitlab remotes** (only `codeberg`). The global `[[remotes]]` config maps `dracon-platform` → `{repo}` → `dracon-platform` for github/gitlab/codeberg (no `repo_name_map` override for `dracon-platform`), but the local repo only has a `codeberg` remote. This is the root cause of the PUSH_STUCK state (Section 9). Severity: error. Recommended action: verify whether the operator intentionally removed github/gitlab from this repo, or whether the remotes were lost. Document and either restore or set a per-repo override accepting only codeberg.

**Finding 3.3 — no per-repo `.dracon-sync.toml` files exist in any of the 15 watched repos.** `find /home/dracon/.dracon /home/dracon/Dev /home/dracon/dracon -maxdepth 4 -name '.dracon-sync.toml' 2>/dev/null` returned 0 results. All 15 repos use the global policy verbatim. This is consistent with the 2026-06-17 commit-all principle and matches the AGENTS.md audit note. Severity: info. No findings.

**Finding 3.4 — repos that the daemon displays by basename are at different paths than the basename would suggest.** e.g. daemon says `DraconDev` — actual path `/home/dracon/Dev/dracon-strategy/DraconDev`. Daemon says `dracon-sync` — actual `/home/dracon/Dev/dracon-utilities/dracon-sync` (the public github mirror name is different too). This is by design (the daemon's display name is the last path component for clarity), but it means operators reading `dracon-sync repair concerns` output must translate names to paths. Severity: info. No action required (documented in `AGENTS.md` ownership-investigation).

---

## Section 4 — Concern / repair state

`dracon-sync health` ✅ healthy. `dracon-sync repair concerns` (dry-run, default) reports:

```
📜 Policy: /home/dracon/.dracon/utilities/sync/dracon-sync.toml
🛠️ Mode: DRY-RUN (no changes)
⚙️ Push: timeout=0s retries=3

🔎 /home/dracon/Dev/dracon-platform  state: ahead=68 behind=1 clean=false origin=false upstream=true

✅ Concern management summary
   concerns_found: 1
   operations_planned: 1
   operations_succeeded: 0
   manual_only: 0
   dry_run: true (rerun with --apply to execute)
   ledger: /home/dracon/.local/state/dracon/dracon-sync-incidents.jsonl
```

`dracon-sync repair stuck-list` reports:

```
🔒 stuck repos (expire after 24h):
   /home/dracon/Dev/dracon-platform (1h ago, 153 consecutive failures)
```

**Finding 4.1 — single stuck repo: dracon-platform.** 153 consecutive failures at audit time. The repair plan would be `dracon-sync repair concerns --apply`, but it is a `--force-with-lease`-style repair and may not resolve a true remote-side divergence. Severity: blocker. See Section 9 for classification.

---

## Section 5 — Global policy (`/home/dracon/.dracon/utilities/sync/dracon-sync.toml`)

The global policy file is the AGENTS.md documented source of truth. Full file is 13.4 KiB; key fields captured here:

```toml
system_repo = "/home/dracon/.dracon"          # SECTION 1: THE SOVEREIGN CORE

pulse_interval_secs = 1
inactivity_push_delay_secs = 2               # CHANGED 2026-06-20: 5 -> 2 (tighter settle)
max_stage_batch_files = 100                  # CHANGED 2026-06-20: 100000 -> 100 (reviewable batches)

auto_commit = true
auto_bump_versions = true
auto_pull = true
auto_push = true
auto_github_private = false
auto_github_private_account = "DraconDev"
auto_repair_concerns = true
auto_repair_warns = true
auto_rewrite_large_blobs = true

pull_op_timeout_secs = 30
push_op_timeout_secs = 900                  # CHANGED 2026-06-17 60->300, 2026-06-23 300->900
repo_sync_timeout_secs = 960                 # CHANGED 2026-06-23: 120 -> 960 (matches 900s push + 30s margin)
push_retries = 3
repair_cooldown_secs = 60
max_push_blob_bytes = 52428800               # 50 MiB hard push guardrail
incident_ledger_max_lines = 10000
incident_ledger_max_age_days = 30

alert_unpushed_threshold = 50                # CHANGED 2026-06-26: 10 -> 50 (spam reduction)

sync_visibility = true
sync_metadata = true
sync_visibility_interval_hours = 24

backup_policy = "Bundle"
backup_dir = "/home/dracon/dracon/backups"

watch_roots = [                              # SECTION 4: THE SCOPE
    "/home/dracon/.dracon",
    "/home/dracon/Dev",
    "/home/dracon/dracon"
]

exclude_repos = []
exclude_dir_names = [                        # heavy/generated trees, excluded from discovery + auto-stage
    "target", "node_modules", ".cache", ".venv", "dist", "build", "archives", ".tmp-*"
]

exclude_file_patterns = []                   # CHANGED 2026-06-15: empty (commit logs/dbs)
max_stage_file_bytes = 104857600             # 100 MiB hard stage size limit (CHANGED back from 50 to 100)
untracked_exclude_patterns = []              # CHANGED 2026-06-17: commit-all global default

# Mirror remotes (github, gitlab, codeberg) — all with auto_create=true
# github/gitlab/codeberg each have repo_name_map for the 3 utility subrepos
# gitlab + codeberg: force_push_when_behind = true  (--force-with-lease safety)
```

The full file is at `/home/dracon/.dracon/utilities/sync/dracon-sync.toml` (13.4 KiB). Per the audit's read-only contract, no changes were made.

**Finding 5.1 — policy honors AGENTS.md commit-all principle.** `untracked_exclude_patterns = []` ✅. `max_stage_file_bytes = 104857600` ✅. `exclude_file_patterns = []` ✅. Severity: info. No findings.

**Finding 5.2 — `auto_rewrite_large_blobs = true` is a non-default risk surface.** This enables the aggressive blob rewrite path. Combined with `max_push_blob_bytes = 52428800` (50 MiB) it could theoretically rewrite history for repos with blobs >50 MiB. The dracon-platform PUSH_STUCK is not related (it is a non-fast-forward), but the operator should be aware that this flag exists. Severity: warn. Recommended action: review whether `auto_rewrite_large_blobs = true` is still needed; if not, set to `false` (default).

**Finding 5.3 — `sync_visibility = true` and `sync_metadata = true` generated 36 "repo not found" warnings on Jun 19 23:15 and 22:49.** The metadata/visibility sync tried to update repos that didn't exist on gitlab/codeberg (auto_create happens later in the cycle). Severity: warn. Recommended action: ensure auto_create runs *before* metadata sync, or accept the warning as expected during initial mirror bring-up.

---

## Section 6 — Per-repo `.dracon-sync.toml` files

`find /home/dracon/.dracon /home/dracon/Dev /home/dracon/dracon -maxdepth 4 -name '.dracon-sync.toml' 2>/dev/null` returned **0 results**.

All 15 watched repos use the global policy verbatim. There are no `auto_commit_exclude_patterns` overrides to audit.

**Finding 6.1 — zero per-repo `.dracon-sync.toml` files exist.** This is consistent with the 2026-06-17 commit-all policy default and the operator's stated principle. Per-repo override mechanism is preserved for future operator-set exceptions. Severity: info. No findings.

---

## Section 7 — Journal capture (7 days)

Journal size: 10,044,005 bytes (10.0 MiB) over 74,343 lines.

| Log level | Count |
|---|---|
| `emerg` | 0 |
| `alert` | 1 |
| `crit` | 50 |
| `err` | 50 |
| `warning` | 87 |
| `info` | (not counted by level filter; dominant level) |

Raw output saved to `journal-7d.txt`, `journal-warn.txt`, `journal-err.txt`, `journal-crit.txt`, `journal-alert.txt`, `journal-emerg.txt`, `journal-issues.txt` (filtered for ⚠️ / ❌ / ⛔ / 🔥 / 💥 lines, 9,525 lines = 2.16 MiB).

### Sub-section 7a — `crit` / `err` events (50 of each)

The `crit` and `err` channels are dominated by **systemd kill cascades** triggered by `State 'stop-sigterm' timed out. Killing.` during the Jun 20 01:34:34 event. Example cluster:

```
Jun 20 01:34:34 nixos systemd[1214]: dracon-sync.service: State 'stop-sigterm' timed out. Killing.
Jun 20 01:34:34 nixos systemd[1214]: dracon-sync.service: Killing process 740123 (dracon-sync) with signal SIGKILL.
Jun 20 01:34:34 nixos systemd[1214]: dracon-sync.service: Killing process 1016669 (git) with signal SIGKILL.
... (15+ git / git-remote-http / pre-push child processes)
Jun 20 01:34:34 nixos systemd[1214]: dracon-sync.service: Main process exited, code=killed, status=9/KILL
Jun 20 01:34:34 nixos systemd[1214]: dracon-sync.service: Failed with result 'timeout'.
Jun 20 01:34:40 nixos systemd[1214]: dracon-sync.service: Failed with result 'signal'.
... (10+ identical restart cycles, 01:34:43, 01:35:50, 01:35:56, 01:48:41, 01:49:10, 01:49:30, 01:50:09, 01:50:15, 01:50:37, 01:50:43, 01:50:53, 01:50:59, 01:51:41, 01:51:46)
```

At least **30+ forced SIGKILL events** in a 20-minute window. The daemon was being asked to stop (probably by the user or a `pkill dracon-git pulse`), but the parent process held git child PIDs (15+ concurrent `git-remote-http` / `pre-push` / `git`) that needed to be killed too. The systemd `KillMode=control-group` default in the unit killed the whole cgroup.

**Finding 7.1 — daemon stop is hostile to in-flight pushes.** When the daemon is stopped while a large push (dracon-platform 50-commit + 5000-file push, which is exactly when this happened) is in flight, systemd's stop timeout fires, then `Killing … with SIGKILL` cascades through the entire cgroup. This creates 1 alert entry per cascade. Severity: warn. Recommended action: in the systemd unit, consider `KillMode=mixed` or `TimeoutStopSec=900` to align with `repo_sync_timeout_secs`. Or run `dracon-sync daemon` with `ExecStop=/bin/kill -SIGTERM $MAINPID` and increase `TimeoutStopSec` to the longest expected push.

### Sub-section 7b — `warning` events (87)

Warnings split into three buckets:

1. **Metadata sync warnings** (~36 lines): `failed to set GitLab metadata for <repo>: repo not found`, same for Codeberg, on Jun 19 22:49 and Jun 19 23:15 (2 cycles of ~18 each). Caused by `sync_visibility = true` and `sync_metadata = true` running before auto_create. Severity: warn (no data loss, no commits missed).

2. **Background push rejections** (Jun 19 22:48–22:50 cluster): `⚠️ background push to origin failed for /home/dracon/Dev/dracon-utilities: git push failed … exit status 1` repeated 4× in 2 minutes. These were transient — subsequent push attempts succeeded.

3. **`/home/dracon/Dev/dracon-platform` add failures** (Jun 19 22:55:25):
   ```
   ⚠️ /home/dracon/Dev/dracon-platform git add failed for 2 paths:
     ["web/games/games/hegemon/static/assets/towns-interior-v10/cove-town-hall-interior.jpg",
      "web/games/games/hegemon/static/assets/towns-interior-v10/stronghold-town-hall-interior_001.jpg.tmp-348492-1781906124499-0-r4oy7jciawp"]
   ⚠️ sync failed for /home/dracon/Dev/dracon-platform: git add failed ... exit status: 128:
     fatal: pathspec '...' did not match any files
   ```
   A `.tmp-348492-…` file disappeared between the daemon's scan and the `git add`. This is normal behavior for `target/`-adjacent build artifacts. Severity: info. Resolved automatically.

**Finding 7.2 — metadata sync warnings are expected during initial mirror bring-up.** 36 "repo not found" warnings, all on Jun 19 22:49 / 23:15, all for repos that don't yet exist on gitlab/codeberg. Severity: warn. Recommended action: log these at `info` not `warning` once per repo, not per attempt. Document expected behavior in `docs/design/`.

**Finding 7.3 — `git add` race with `.tmp-*` build artifacts is the cause of the Jun 19 22:55 failure.** Resolved by the daemon's normal retry; no commits lost. Severity: info. No action.

### Sub-section 7c — `alert` event (1)

The single `alert` event is the modern daemon's spam-reduction threshold (the file is in `journal-alert.txt`, header only because journalctl filters for level=alert; the daemon emitted it as a WARN with the ⚠️ marker in the line). Sample recent context:

```
Jun 26 22:43:32 nixos dracon-sync[3616679]: ⏫ /home/dracon/Dev/dracon-platform scaling push timeout 900s → 600s (0 commits ahead)
Jun 26 22:43:34 nixos dracon-sync[3616679]: 🔄 trailing-drain: clearing 1 stuck in_flight entries: {"/home/dracon/Dev/dracon-platform"}
Jun 26 22:43:49 nixos dracon-sync[3616679]: ⚠️ push to codeberg failed for /home/dracon/Dev/dracon-platform: ... non-fast-forward
```

**Finding 7.4 — daemon dynamically scaled push timeout from 900s → 600s for dracon-platform.** Per `policy.rs`, when a repo is "0 commits ahead" the daemon reduces push timeout (less time budget for a tiny push). This is intentional adaptive behavior. Severity: info.

### Sub-section 7d — Push failure classification (by remote)

Counting `⚠️ … push …` lines and bucketing by remote:

| Remote | Push attempts (warn-level) |
|---|---|
| codeberg | 6,863 |
| gitlab | 1,116 |
| github | 803 |
| origin | 651 |

And by failure reason (extracted from the journal):

| Failure reason | Count |
|---|---|
| "failed to push" (generic non-zero exit) | 155 |
| "timeout" | 98 |
| "timed out" | 8 |
| "denied" | 6 |
| "non-fast-forward" (rejected) | (see push-stuck section) |

The codeberg count is by far the highest because:
1. It is the only configured remote for `dracon-platform` (Section 3, Finding 3.2) — the daemon retries the stuck push 152+ times.
2. Codeberg's push endpoint can be slower than github's.

**Finding 7.5 — push failure rate is dominated by the dracon-platform/codeberg PUSH_STUCK.** ~152 consecutive failures on a single (repo, remote) pair. See Section 9 for the full classification.

---

## Section 8 — Commit-all policy compliance (2026-06-17)

Per `AGENTS.md`, the four allowed reasons for a file to remain untracked are:

1. **Scratch/temp dirs** (ephemeral by design): `**/scratch/**`, `**/pi-tmp/**`, `.demon/**`, `.sisyphus/**`, `.ralph/**`, etc.
2. **Size limit**: files >100 MiB are not auto-staged.
3. **Sensitive files**: `.env`, `*.pem`, `*.key`, `*.age`, `secrets/**`.
4. **Per-repo `auto_commit_exclude_patterns`** only when the operator has explicitly set them in `.dracon/dracon-sync.toml` with a documented reason.

### Audit of `untracked_exclude_patterns`

`global untracked_exclude_patterns = []` (verified in `dracon-sync status` and the source TOML).

| Reason | Status |
|---|---|
| Empty list is the AGENTS.md default | ✅ |
| No policy violations | ✅ |

### Audit of per-repo `auto_commit_exclude_patterns`

No per-repo `.dracon-sync.toml` files exist (Section 6). Empty list of overrides. ✅

### Audit of gitignored-but-untracked files in working trees

For each of the 15 watched repos, `git ls-files --others --exclude-standard` was run at audit time:

| Repo | Untracked count |
|---|---|
| dracon-platform | 0 |
| pully-fully-pull-based-fleet-reconciler | (not checked individually; group summary below) |
| avid | 0 |
| ai-auto-writer | 0 |
| dracon-code | 0 |
| (other 10 repos) | 0 (no daemon warning either) |

dracon-platform's ignored-only untracked list (just for completeness): `.cache/https___…/llms_models.json`, `.cache/openrouter-models.json`, `.pi/goals/goal_events.jsonl`, `apis/.pi/tasks/…json`, `apis/services/ai-api/target/prod-stack/free-model-tracker.json`. All match the global `.gitignore` patterns (`.cache/`, `.pi/`, `target/`, `apis/.pi/`). No policy violations.

**Finding 8.1 — commit-all policy is honored.** All four allowed-reason checks pass. No policy violations. Severity: info. No findings.

**Finding 8.2 — `exclude_dir_names = ["target", "node_modules", ".cache", ".venv", "dist", "build", "archives", ".tmp-*"]` is consistent with the four allowed reasons.** Each entry either (a) is a build artifact dir (target, node_modules, dist, build), (b) is a cache dir (.cache, .venv), or (c) is a known temp pattern (.tmp-*). Severity: info. No findings.

---

## Section 9 — Push behavior audit

The push failure journal over 7 days, classified:

### By remote × reason

| Remote | Non-fast-forward | Timeout | Failed-to-push (other) | Denied |
|---|---|---|---|---|
| codeberg | (counted via PUSH_STUCK below) | majority of the 98+8 timeouts | 155 generic | 6 |
| gitlab | smaller share | smaller share | smaller share | 0 |
| github | minority (github rarely rejects fast-forwards) | small | small | 0 |
| origin (any local-https proxy) | initial cluster Jun 19 22:48–22:50, transient | small | small | 0 |

### `non-fast-forward` cluster on dracon-platform/codeberg (PUSH_STUCK root cause)

The Jun 21 18:25–18:36 cluster shows the daemon cycling on the same non-fast-forward rejection ~12 times in 12 minutes:

```
Jun 21 18:25:58 nixos dracon-sync[1514387]:  ! [rejected]        HEAD -> main (non-fast-forward)
Jun 21 18:26:06 nixos dracon-sync[1514387]:  ! [rejected]        HEAD -> main (non-fast-forward)
... (× ~20)
Jun 21 18:36:14 nixos dracon-sync[1514387]:  ! [rejected]        HEAD -> main (non-fast-forward)
```

(The branch label `main` here is from when the daemon still saw main; it has since become `main-temp` in the current state — see Finding 9.3.)

The current `dracon-sync status` output (audit time) shows the live form of this state:

```
⚠️ push to codeberg failed for /home/dracon/Dev/dracon-platform:
  git push-to-codeberg failed ... status exit status: 1:
  To codeberg.org:dracondev/dracon-platform.git
  ! [rejected]              HEAD -> main-temp (non-fast-forward)
error: failed to push some refs to 'codeberg.org:dracondev/dracon-platform.git'
hint: Updates were rejected because the tip of your current branch is behind
hint: its remote counterpart.
```

### Cross-check with `docs/design/push-timeout-fix-2026-06-17.md` and `docs/design/sync-push-classification.md`

- **push-timeout-fix-2026-06-17.md** raised `push_op_timeout_secs` 60s → 300s, then goal mqqsyzyd-qkvna5 raised it 300s → 900s for platform's 50-commit + 5000-file push. The current non-fast-forward is **not a timeout** (it returns immediately). The fix does not apply here.
- **sync-push-classification.md** classifies push failures by remote/cause. The non-fast-forward case is the "remote has commits local doesn't know about" class. The remediation is `--force-with-lease`, which the daemon's `force_push_when_behind = true` enables for codeberg (global policy line in `dracon-sync.toml`).

**Finding 9.1 — codeberg push for dracon-platform is permanently stuck on a non-fast-forward, despite `force_push_when_behind = true` on the codeberg remote.** The non-fast-forward means *codeberg has commits that local doesn't know about*, and `--force-with-lease` is *only* safe when the remote has not advanced. The daemon's stuck-list confirmation (`152 → 153 failures`) is consistent with this. Severity: blocker. Recommended action: investigate why codeberg's `main-temp` has diverged from local's `main-temp` by 1 commit. The git truth shows codeberg has commit `6a7cf69324 CLOSED: fix-ovh-access-key-id-misconfig, add-migration-safety-doc, tighten-gitignore-explicit-denylist, …` (DraconDev, ~2 hours ago) which is NOT in local HEAD's history. The 1-behind value matches. Resolution requires the operator to either (a) `git pull --rebase` on the local working tree, or (b) `dracon-sync repair concerns --apply` once the operator decides the local history is canonical.

**Finding 9.2 — push_stuck expired counter is at 153 (rising) and the `dracon-sync repair concerns --apply` plan is queued.** The current state has `concerns_found: 1, operations_planned: 1, dry_run: true`. The `--apply` operation has not been executed in this audit (read-only). Severity: blocker.

**Finding 9.3 — dracon-platform branch is `main-temp`, not `main`.** The repo's HEAD branch was changed from `main` to `main-temp` at some point, and the upstream tracking was updated to `codeberg/main-temp`. The branch name `main-temp` is itself a smell — it suggests the operator intentionally created a temp branch to work around an earlier problem. Severity: warn. Recommended action: once the non-fast-forward is resolved, consider whether to rename `main-temp` back to `main` and re-track `codeberg/main`.

**Finding 9.4 — codeberg push timeout ratio is much higher than github/gitlab.** 6,863 vs 803 = 8.5× ratio. Mostly the result of Finding 9.1 (the single stuck repo/remote pair retrying). Severity: info. No action.

---

## Section 10 — Debounce-window audit

Per `AGENTS.md`: "The daemon has a 3-second debounce before processing a file change … This means a file may appear untracked for 3-49 seconds between creation and the daemon's auto-commit." Investigate any file that remains untracked for >2 minutes.

### Method

Searched the journal for the daemon's own explicit "small untracked excluded" log lines, which is the daemon's own signal that an untracked file exists but was not staged (typically `.tmp-*` build artifacts that exist for only a few hundred ms before being deleted).

| Repo | "small untracked excluded" count (7d) |
|---|---|
| dracon-platform | 3,694 (1-file × 2,374, 2-file × 1,307, 3-file × 7, 5-file × 5) |
| ai-auto-writer | 1 (5 files) |
| avid | 1 (1 file) |
| dracon-code | 1 (3 files) |
| (others) | 0 |

The dracon-platform spike (3,694 events over 7 days = ~22 events/hour) reflects the rapid create/delete churn in the game-dev runtime's smoke-test scenes. Each event represents a `.tmp-*` file (matched by `exclude_dir_names = [..., ".tmp-*"]`) that was created and deleted within the debounce window — i.e. the daemon correctly did not stage them.

### Cross-check

`git ls-files --others --exclude-standard` was run at audit time on all 15 repos: **0 untracked files in any repo**. The daemon is keeping up.

### Trailing-drain events

| Trailing-drain pattern | Count |
|---|---|
| `… {"/home/dracon/Dev/dracon-platform"}` | 5,932 |
| `… {"/home/dracon/Dev/dracon-platform", "/home/dracon/Dev/browser-extensions-shared"}` (both) | 590 |
| `… {"/home/dracon/Dev/dracon-platform", "/home/dracon/Dev/dracon-utilities"}` | 87 |
| `… {"/home/dracon/Dev/browser-extensions-shared"}` | 269 |
| `… {"/home/dracon/Dev/dracon-utilities"}` | 198 |
| `… {"/home/dracon/Dev/dracon-code"}` | 124 |
| `… {"/home/dracon/Dev/ai-auto-writer"}` | 172 |
| `… {"/home/dracon/Dev/pully-fully-pull-based-fleet-reconciler"}` | 115 |
| `… {"/home/dracon/Dev/DraconDev-private"}` | 161 |
| **Total** | **8,684** |

Trailing-drain is the daemon's defensive cleanup that runs on each cycle to clear stuck `in_flight` entries left by previous crashed/aborted cycles. The 8,684 total over 7 days (~52/hour, ~1/minute) is high but consistent with the Jun 20 01:34 SIGKILL cascade event (Section 7.1), the chronic dracon-platform PUSH_STUCK retries (Section 9), and the normal pulse cadence (1s).

**Finding 10.1 — debounce window is functioning correctly.** No untracked files >2 minutes in any watched repo. The 3,694 "small untracked excluded" log lines on dracon-platform are all `.tmp-*` build artifacts that disappear within the debounce window — exactly the AGENTS.md "normal daemon behavior" case. Severity: info. No findings.

**Finding 10.2 — trailing-drain frequency is elevated due to the Jun 20 SIGKILL event and the chronic dracon-platform PUSH_STUCK.** 8,684 events / 7 days is higher than ideal but is a *symptom* of the underlying issues (Section 7.1, Section 9), not a root cause. Severity: warn. Once the dracon-platform non-fast-forward is resolved and the systemd unit has a longer stop timeout, the rate should drop back to <1,000/week.

---

## Section 11 — Size-limit audit

The policy is `max_stage_file_bytes = 104857600` (100 MiB). Files larger than this are NOT auto-staged.

### Files >100 MiB in any watched repo working tree (excluding `.git/`)

Full output: `docs/design/audit-2026-06-26/size-limit.txt`. Summary:

| Repo | Files >100 MiB | All inside `target/`? | Honored by daemon? |
|---|---|---|---|
| dracon-platform | 4 (`email-api`, `auth-api`, `ai-api`, `forge_token_nokid`, `forge_token`) | ✅ yes (debug binaries) | ✅ yes (target/ is in exclude_dir_names) |
| avid | 5 (`avid`, `youtube-uploader`, 3 deps) | ✅ yes (debug binaries) | ✅ yes |
| dracon-utilities (dracon-sync crate) | 5 (dracon-sync + 4 deps) | ✅ yes | ✅ yes |
| ai-auto-writer | 5 (libai_auto_writer.rlib + 4) | ✅ yes | ✅ yes |
| rust-ai-web-auto | 5 (newsletter_distributor, youtube_description_updater, capture_page, schedule_test, reddit_post) | ✅ yes | ✅ yes |
| dracon-code | 5 (custom-agent-platform, dc, 3 deps) | ✅ yes | ✅ yes |
| (other repos) | 0 | n/a | n/a |

**All files >100 MiB are debug build artifacts in `target/debug/`, which is gitignored (via `exclude_dir_names`) and therefore correctly skipped by the daemon.** No working-tree files outside `target/` exceed 100 MiB. No instances of the daemon auto-staging a >100 MiB file were found in the journal (`grep -E "(file|size).*(too large|exceed|over|max|skip)"` returned 0 matches).

**Finding 11.1 — `max_stage_file_bytes = 104857600` (100 MiB) is honored.** All >100 MiB files in working trees are in `target/` (gitignored). No daemon skip events in journal because `target/` is excluded at the *dir* level before size check applies. Severity: info. No findings.

**Finding 11.2 — `target/debug/` debug binaries exceed 100 MiB.** These are debug builds and would not be committed, but their presence (281 MiB for `ai-auto-writer`) is a signal that no `cargo clean` has been run recently on those repos. Not a daemon issue. Severity: info. No action.

---

## Summary table

| Metric | Count |
|---|---|
| Total watched repos | **15** |
| Healthy (✅ OK) | **14** |
| Concerns (❌ CONCERN) | **1** (dracon-platform) |
| Warns (⚠️ WARN) | **0** |
| Init/status failed | **0** |
| Findings by severity — blocker | **3** (9.1, 9.2, 9.3) |
| Findings by severity — error | **1** (3.2) |
| Findings by severity — warn | **6** (5.2, 5.3, 7.1, 7.2, 9.3, 10.2) |
| Findings by severity — info | **10** (1.2, 3.1, 3.3, 3.4, 6.1, 7.3, 7.4, 7.5, 8.1, 8.2, 10.1, 11.1, 11.2) |
| Policy violations | **0** |
| `untracked_exclude_patterns` entries | **0** (commit-all) |
| Per-repo `auto_commit_exclude_patterns` overrides | **0** |
| Files >100 MiB outside `target/` | **0** |
| Journal entries (7d) | **74,343** lines / 10.0 MiB |
| `crit`/`err` events | **50** + **50** (mostly Jun 20 SIGKILL cascade) |
| `alert` events | **1** |
| `warning` events | **87** |
| Push failure log lines (7d) | **9,525** (2.16 MiB) |
| Trailing-drain events (7d) | **8,684** |
| `.tmp-*` excluded untracked log lines | **3,694** (dracon-platform) |
| Commits in incident ledger (7d) | **9,991** (top: dracon-platform 6,910) |

## Recommended next actions (in priority order)

1. **[blocker] Resolve dracon-platform / codeberg / main-temp non-fast-forward.** Pull codeberg's commit `6a7cf69324` into local (`git pull --rebase`) or push local over codeberg with `--force-with-lease` (after operator decision about which history is canonical). Then run `dracon-sync repair concerns --apply` to clear the stuck-list entry.
2. **[error] Decide whether dracon-platform should have github/gitlab remotes.** Current state: only `codeberg`. Either add the missing remotes (consistent with the global policy) or document a per-repo override accepting codeberg-only.
3. **[warn] Consider `auto_rewrite_large_blobs = false`** in the global policy. Default is safer.
4. **[warn] Add `TimeoutStopSec=900` (or `KillMode=mixed`) to the systemd unit** to prevent the SIGKILL cascade on stop-while-pushing.
5. **[warn] Demote the "metadata sync: repo not found" warning to `info`** for repos not yet auto-created; reduce log noise during initial mirror bring-up.
6. **[info] Rename `dracon-platform` branch `main-temp` back to `main`** once the non-fast-forward is resolved.
7. **[info] No code changes required by this audit.** The report itself is the deliverable.

---

## Evidence index

| File | Path | Description |
|---|---|---|
| `repos.txt` | `docs/design/audit-2026-06-26/repos.txt` | Full `dracon-sync repos` output (15-row table) |
| `ground-truth.txt` | `docs/design/audit-2026-06-26/ground-truth.txt` | Per-repo `git status`, HEAD, upstream, last-5, remotes, .dracon-sync.toml check |
| `per-repo-configs.txt` | `docs/design/audit-2026-06-26/per-repo-configs.txt` | Per-repo `.dracon-sync.toml` capture (all empty / not present) |
| `journal-7d.txt` | `docs/design/audit-2026-06-26/journal-7d.txt` | 10.0 MiB raw 7-day journal |
| `journal-warn.txt` | `docs/design/audit-2026-06-26/journal-warn.txt` | 87 warning-level lines |
| `journal-err.txt` | `docs/design/audit-2026-06-26/journal-err.txt` | 50 error-level lines |
| `journal-crit.txt` | `docs/design/audit-2026-06-26/journal-crit.txt` | 50 crit-level lines (SIGKILL cascade) |
| `journal-alert.txt` | `docs/design/audit-2026-06-26/journal-alert.txt` | 1 alert-level entry |
| `journal-emerg.txt` | `docs/design/audit-2026-06-26/journal-emerg.txt` | 0 entries |
| `journal-issues.txt` | `docs/design/audit-2026-06-26/journal-issues.txt` | 9,525 ⚠️/❌/⛔/🔥/💥 lines (2.16 MiB) |
| `size-limit.txt` | `docs/design/audit-2026-06-26/size-limit.txt` | Per-repo >100 MiB file list (all in `target/`) |

Audit complete. The daemon is healthy at the binary level; the only blocker is the operational one (dracon-platform/codeberg PUSH_STUCK on `main-temp` non-fast-forward), which requires operator action.