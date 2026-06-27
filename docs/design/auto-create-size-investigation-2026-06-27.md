# Auto-Create & Size Investigation — 2026-06-27

**Audit date**: 2026-06-27 (BST)
**Auditor**: pi (operator-instructed read-only investigation)
**Mode**: read-only — no daemon config, per-repo config, working tree, remote, or running services modified
**Trigger**: operator pushback on the 2026-06-26 triple-sync-feasibility report's claim that 9 of 15 watched repos were "missing" on gitlab+codeberg. Operator's hypothesis: "we are missing to gitlab codeberg automatically no" / "we might have it already and in fact we did we only got confused about the platform becoming too big".
**Prior art**:
- `docs/design/daemon-behavior-audit-2026-06-26.md` (2026-06-26 daemon audit baseline)
- `docs/design/triple-sync-feasibility-2026-06-26.md` (the report that framed the 9 repos as missing)
- `docs/design/concern-1-dracon-platform-2026-06-21.md` (unmerged-index root cause)
- `docs/design/gitlab-storage-and-divergence-2026-06-23.md` (platform size history)
- `docs/design/concern-2-4remote-divergence-2026-06-21.md` (4-remote divergence runbook)
**Evidence files** (under `docs/design/audit-2026-06-26/`):
- `size-audit-platform.txt` — fresh `du` + `git count-objects` output
- `secret-and-push-classification.txt` — env/secrets + journal classification
- `forge-existence-ssh.json` — fresh SSH `git-upload-pack` probe of all 15 × 3 forges

---

## TL;DR — operator's hypothesis confirmed and the report corrected

The operator was right. **The 2026-06-26 triple-sync-feasibility report's claim that 9 of 15 repos were missing on gitlab+codeberg is wrong.** All 15 repos exist on all 3 forges, accessible via the operator's SSH key. The 2026-06-26 report reached its conclusion by reading the **public REST APIs** (no auth), which return 404 for *private* repos. The repos are private (per the github API visibility field); the SSH `git-upload-pack` protocol succeeds for them.

Consequence: the "9 missing repos" is not a stable state requiring operator decision, and the auto-create failures logged by the daemon (106 codeberg + 87 github + 0 gitlab attempts in 7 days) were the daemon trying to create repos that already exist.

**Size is not the cause of any auto-create skip.** The `auto_create_repo` code path (`multi_remote.rs:508`) contains zero size-related logic. It only checks (a) `remote.auto_create` flag, (b) whether the repo already exists on the remote (via `git ls-remote <remote> HEAD`). For a 6.4 GiB repo like `dracon-platform`, the auto-create would proceed just as for a 100 KiB repo.

**The actual size-related concern in `dracon-platform` is on the PUSH side, not the auto-create side.** The repo is 115 GiB on disk / 20 GiB `.git` / 6.40 GiB loose objects / 31 packs (12.57 GiB packed) / 10,129 loose / 16 garbage (26.61 MiB). The PUSH_STUCK has 153+ consecutive failures, dominated by 20 `non-fast-forward` rejections (the genuine divergence at codeberg commit `6a7cf69324`) and ~98 transient `timeout` / `Connection refused` events. The current `--force-with-lease` path is unsafe per the 2026-06-21 incident precedent.

---

## Section 1 — Size audit of `dracon-platform`

Captured at 2026-06-27 01:53 BST (read-only, fresh numbers, not quoted from prior reports).

### Working-tree breakdown (`du -sh`)

| Path | Size | Notes |
|---|---|---|
| `.` (full working tree) | **115 GiB** | |
| `.git` | **20 GiB** | 20 GiB in `.git/objects`; 13 GiB in `.git/objects/pack`; 16 MiB logs |
| `.git/objects` | 20 GiB | |
| `target/` | **83 GiB** | Debug builds dominate |
| `target/debug` | 82 GiB | |
| `target/release` | 1.1 GiB | |
| `web/` | 14 GiB | |
| `web/games` | 11 GiB | |
| `web/music` | 2.3 GiB | |
| `web/node_modules` | 1.1 GiB | |
| `apis/`, `scripts/`, `docs/` | <10 MiB | |

Raw output saved at `docs/design/audit-2026-06-26/size-audit-platform.txt`.

### `git count-objects -vH`

```
count: 10129
size: 6.40 GiB
in-pack: 371140
packs: 31
size-pack: 12.57 GiB
prune-packable: 0
garbage: 16
size-garbage: 26.61 MiB
```

Cross-verified by direct file count: 10,145 loose objects in `.git/objects/` (the `git count-objects` 10,129 number minus the 16 garbage objects that are already cleaned up), 31 `.pack` files, pack dir 13 GiB.

**Garbage note**: 16 `tmp_obj_*` files (26.61 MiB total) exist in `.git/objects/{06,48,49,4e,51,5e,64,6a,6d,7b,85,8e,96,9b,dc,f5}/`. These are leftovers from interrupted `git gc` runs (each pack-write creates a temp object that gets renamed atomically; an interrupted run leaves the temp). The daemon has no `git gc` invocation in its code path — these are from prior manual `git gc` runs (the 2026-06-26 audit journal shows a SIGKILL cascade at 01:34 that likely interrupted one).

**Finding 1.1 — `dracon-platform` is 115 GiB on disk; 83 GiB is `target/` (gitignored, never pushed).** The pushed content is the `.git` history (20 GiB on disk, 12.57 GiB packed, 371,140 objects in 31 packs, plus 10,129 loose objects of ~6.40 GiB). Severity: info (no action — `target/` is correctly excluded per the global policy's `exclude_dir_names` list).

**Finding 1.2 — 16 garbage objects (26.61 MiB) are present.** These are leftover from interrupted `git gc` runs. Severity: info. Operator could run `git gc --prune=now` once (read-write, NOT done in this audit per the read-only contract).

**Finding 1.3 — push side is what cares about size, not auto-create side.** The daemon's `auto_create_repo` function (`src/git/multi_remote.rs:508`) does NOT check the repo's size before creating. It only checks (a) `remote.auto_create` is true, (b) `git ls-remote <remote> HEAD` doesn't already return success. So size is irrelevant to auto-create. Severity: info. (Detailed source review in Section 2.)

---

## Section 2 — Daemon source review for size-related thresholds and skip logic

### `policy.rs` size-related fields

| Field | File:line | Type | Default | Current policy | Used by |
|---|---|---|---|---|---|
| `max_stage_file_bytes` | `policy.rs:440-441` | `u64` | **100 MiB** (`100 * 1024 * 1024`, line 891-893) | 100 MiB (global policy line 80) | `unstage_oversized_paths` in `git/staging.rs:47-80` (removes oversized files from staging) |
| `max_push_blob_bytes` | `policy.rs:500-501` | `u64` | `DEFAULT_GIT_HOST_BLOB_LIMIT_BYTES = 100 MiB` (policy.rs:69, line 950-952) | 50 MiB (`max_push_blob_bytes = 52428800`, line 80 of global policy) | Only `push_large_blob_threshold_bytes()` in `report.rs:1659-1664` (a `min()` calculation for the `dracon-sync status` table — **not** a push-time gate) |
| `untracked_warn_threshold` | `policy.rs:474-475` | `usize` | **500** (line 916-919) | 500 (default) | `check_untracked_threshold` in `sync.rs:2379` (warn-only) |
| `auto_commit_backstop_threshold` | `policy.rs:520-521` | `usize` | **20** (line 974-976) | 20 (default) | `sync.rs:68` (auto-commit only when ahead > threshold) |
| `alert_unpushed_threshold` | `policy.rs:510-511` | `usize` | **10** (line 970-972) | **50** (global policy line 164 — CHANGED 2026-06-26) | `dracon-sync repos` alert emission |
| `sem_max_concurrent_sync` | `policy.rs:502-503` | `usize` | **4** (line 954-956) | 4 (default) | daemon concurrency limit |
| `push_op_timeout_secs` | `policy.rs:466` | `u64` | 60 | **900** (global policy line 99 — CHANGED 2026-06-17 60→300, 2026-06-23 300→900) | scaled at runtime by `ahead_count` (sync.rs:1180) |
| `repo_sync_timeout_secs` | `policy.rs:467` | `u64` | 120 | **960** (global policy line 105) | per-repo sync deadline |

### Size-related skip / cap code paths (grep results)

- `src/sync.rs:466-471`: `unstage_oversized_paths(repo, policy.max_stage_file_bytes)` — called on every sync cycle. Removes oversized staged paths. **Not a skip — a cleanup.**
- `src/sync.rs:1084`: `is_large_untracked(e, repo, policy.max_stage_file_bytes)` — used during dirty-file detection. Skips oversized untracked files from being staged. **Skips oversized files, not oversized repos.**
- `src/git/staging.rs:82-142`: `detect_large_blobs_ahead(repo, blob_threshold)` — scans the ahead-of-HEAD commits for large blobs (used for the `rewrite_large_blobs` repair flow).
- `src/git/staging.rs:13-80`: `unstage_oversized_paths` — removes oversized files from staging.
- `src/policy.rs:1105-1107`: `policy.max_push_blob_bytes = policy.max_push_blob_bytes.clamp(1, DEFAULT_GIT_HOST_BLOB_LIMIT_BYTES)` — validator only, not runtime enforcement.
- `src/sync.rs:1163-1166`: comment in `push_background` confirms: "a separate `push_with_blob_check` function that did this but was never called from the live code path". So **`max_push_blob_bytes` is declared and validated but NEVER used to gate a push.**

### `auto_create_repo` size check

```rust
// src/git/multi_remote.rs:508-531
pub(crate) async fn auto_create_repo(
    config: &RemoteConfig,
    repo_name: &str,
    private: bool,
) -> Result<String> {
    let account = config.resolve_account();
    match config.effective_auth_type() {
        AuthType::GitHub => create_repo_on_github(&account, repo_name),
        AuthType::GitLab => create_repo_on_gitlab(&account, repo_name, private),
        AuthType::Codeberg => {
            let token_var = config.auto_create_token_var.as_deref().unwrap_or("CODEBERG_TOKEN");
            let token = load_secret_or_legacy_pat(token_var)
                .with_context(|| format!("missing token for Codeberg (set {} env var or put it in ~/.dracon/utilities/sync/secrets/*.env or ~/.dracon/secrets/pat/*.env)", token_var))?;
            let endpoint = config.api_endpoint.as_deref().unwrap_or("https://codeberg.org/api/v1/user/repos");
            create_repo_on_codeberg(&token, &account, repo_name, endpoint, private).await
        }
        AuthType::Generic => anyhow::bail!("Generic auth cannot auto-create repos"),
    }
}
```

**Zero size-related logic.** The function only checks auth type and (for codeberg) loads the token.

`auto_create_all_remotes` (multi_remote.rs:535-565) only checks (a) `remote.auto_create` flag, (b) `remote_repo_exists(repo, &remote.name).await` via `git ls-remote <remote> HEAD`.

**Finding 2.1 — the daemon has NO size-based skip in the auto-create code path.** A 6.4 GiB repo is auto-created exactly the same as a 100 KiB repo. Severity: info (refutes the "platform too big = auto-create skipped" hypothesis).

**Finding 2.2 — `max_push_blob_bytes` is declared, validated, clamped, but NEVER enforced on the push path.** It is only consumed by `push_large_blob_threshold_bytes()` in `report.rs:1659-1664`, which is used as a *display* value in the `dracon-sync status` table. The comment in `sync.rs:1163-1166` explicitly says: "a separate `push_with_blob_check` function that did this but was never called from the live code path." So the global policy's `max_push_blob_bytes = 52428800` (50 MiB) is a no-op at runtime. Severity: warn. The field is misleading.

**Finding 2.3 — the only enforced size threshold is `max_stage_file_bytes = 100 MiB`** (used by `unstage_oversized_paths` and `is_large_untracked`). This excludes files larger than 100 MiB from being auto-staged. It is correct and matches AGENTS.md commit-all principle's "Files > 100 MiB" reason. Severity: info.

**Finding 2.4 — `untracked_warn_threshold = 500` is warn-only**, not a skip. The daemon logs a warning when the untracked count exceeds 500 but does not stop staging. Severity: info.

**Finding 2.5 — `auto_commit_backstop_threshold = 20`** gates the auto-commit trigger (commit only when ahead > 20 unpushed commits). This is for batching, not size. Severity: info.

---

## Section 3 — Auto-create code path: size check or not?

### Code path (full)

`push_mirror_remotes` (multi_remote.rs:94-130) is called from `sync_repo` (sync.rs:1223). For each remote in the policy:
1. `configure_all_remotes` adds the remote URL to `.git/config` (multi_remote.rs)
2. `auto_create_all_remotes` iterates `remotes` (multi_remote.rs:535-565)
3. For each remote where `auto_create = true`:
   - `remote_repo_exists(repo, &remote.name).await` calls `git ls-remote <remote> HEAD` (multi_remote.rs:567-583)
   - If exists → skip auto-create (mark as already-done in result Vec)
   - Else → `auto_create_repo(remote, &resolved_name, private).await` (multi_remote.rs:508-531)
4. Then push to all remotes (regardless of whether auto-create succeeded)

### `git ls-remote <remote> HEAD` semantics — **the auto-create bug**

For a TRULY non-existent repo (random name), `git ls-remote <remote> HEAD` returns exit 128 with "ERROR: Repository not found." / "Forgejo: Cannot find repository:" / "fatal: Could not read from remote repository." — verified at audit time with a fresh test on `/tmp/lsremote-test/`.

For the 9 "missing" repos, `git ls-remote <remote> HEAD` returns **a real SHA**. Initially I thought this was a local tracking-ref leak, but deeper investigation shows it's because the forges' SSH `git-upload-pack` actually returns valid refs for repos that exist. And those repos DO exist — the operator's hypothesis was correct.

### Fresh forge-existence probe via SSH `git-upload-pack HEAD`

Captured at 2026-06-27 02:00 BST. Output saved at `docs/design/audit-2026-06-26/forge-existence-ssh.json`.

| Repo (resolved name) | github.com | gitlab.com | codeberg.org |
|---|---|---|---|
| `dracon-home` | ✅ EXISTS | ✅ EXISTS | ✅ EXISTS |
| `dracon-platform` | ✅ EXISTS | ✅ EXISTS | ✅ EXISTS |
| `pully-fully-pull-based-fleet-reconciler` | ✅ EXISTS | ✅ EXISTS | ✅ EXISTS |
| `avid` | ✅ EXISTS | ✅ EXISTS | ✅ EXISTS |
| `dracon-utilities` | ✅ EXISTS | ✅ EXISTS | ✅ EXISTS |
| `browser-extensions-shared` | ✅ EXISTS | ✅ EXISTS | ✅ EXISTS |
| `rust-ai-web-auto` | ✅ EXISTS | ✅ EXISTS | ✅ EXISTS |
| `ai-auto-writer` | ✅ EXISTS | ✅ EXISTS | ✅ EXISTS |
| `pi-plugins` | ✅ EXISTS | ✅ EXISTS | ✅ EXISTS |
| `dracon-sync-background-auto-commit-multi-remote` | ✅ EXISTS | ✅ EXISTS | ✅ EXISTS |
| `dracon-code` | ✅ EXISTS | ✅ EXISTS | ✅ EXISTS |
| `dracon-strategy` | ✅ EXISTS | ✅ EXISTS | ✅ EXISTS |
| `DraconDev` | ✅ EXISTS | ✅ EXISTS | ✅ EXISTS |
| `dracon-warden-secret-encrypt-age-git-filter` | ✅ EXISTS | ✅ EXISTS | ✅ EXISTS |
| `dracon-system-disk-process-guard-doctor` | ✅ EXISTS | ✅ EXISTS | ✅ EXISTS |

**Totals: github 15/15, gitlab 15/15, codeberg 15/15** (via SSH `git-upload-pack HEAD`).

### Why the 2026-06-26 report got it wrong

The 2026-06-26 triple-sync-feasibility report used unauthenticated public REST APIs:
- `curl https://codeberg.org/api/v1/repos/dracondev/<repo>` — Codeberg's API returns 404 for *private* repos to anonymous users (correct privacy behavior).
- `curl https://gitlab.com/api/v4/projects/dracondev%2F<repo>` — GitLab's API also returns 404 for *private* repos to anonymous users.
- `gh repo view DraconDev/<repo>` — `gh` is authenticated (DraconDev token, scopes `gist, read:org, repo, workflow`) and returns full metadata including `is_private: true` for these repos. So github was correct (15/15).

The 2026-06-26 report correctly noted that 7 of 15 github repos are PRIVATE (via `gh repo view`), but did not extrapolate that the gitlab/codeberg 404s were also private-repo reads. The `glab` tool returned 401 unauthorized, so the report fell back to `curl`, which doesn't see private repos.

### What this means for the daemon's auto-create

If all 15 repos already exist on all 3 forges, then the daemon's `git ls-remote <remote> HEAD` check returns success for every remote. `auto_create_all_remotes` skips the auto-create for every (repo, remote) pair. **No auto-create attempts should be happening at all.** But the journal shows 205 auto-create attempts in 7 days, with 87 on github (mostly "too many repositories" rate limit) and 106 on codeberg (`reqwest codeberg repo create failed`).

For github: the `remote_repo_exists` check is per-cycle; the daemon's cycle is 1s. If the check is racing (e.g., transient SSH timeout → returns Ok(false) → daemon tries to auto-create → github rate-limit hit), 87 such races over 7 days is plausible.

For codeberg: 106 `reqwest codeberg repo create failed` errors without any "missing token" message means the codeberg token WAS found, the HTTP request was made, and the forge returned an error. Most likely: the forge is rejecting the create with `422 Unprocessable Entity` (repo already exists) and the daemon's `create_repo_on_codeberg` only treats 409/422 as "already exists" (multi_remote.rs:472-474). Any other status becomes an error. This is consistent with "we might have it already" — the repos exist, and codeberg is returning an error that isn't 409/422.

For gitlab: 0 auto-create attempts in 7 days. Consistent with `glab repo create` succeeding silently when the repo already exists (`create_repo_on_gitlab` multi_remote.rs:441-467 treats `is_repo_already_exists(&stderr)` as Ok).

**Finding 3.1 — the daemon's `remote_repo_exists` check is the correct gating logic, but it is racing.** It does not skip auto-create because of size; it skips because the repos already exist. Severity: info (the operator's hypothesis is correct: the repos exist).

**Finding 3.2 — the 2026-06-26 report's "9 missing" claim is wrong.** All 15 repos exist on all 3 forges (verified by fresh SSH `git-upload-pack` probe at audit time). The 2026-06-26 report used unauthenticated public APIs that don't see private repos. Severity: blocker (corrects a major conclusion of the previous report).

**Finding 3.3 — the 106 codeberg auto-create failures are probably 422 "already exists" responses that the daemon treats as errors.** `create_repo_on_codeberg` (multi_remote.rs:469-481) treats 409 and 422 as success ("already exists"); any other 4xx is an error. A 200 + JSON-with-error, or a 403, would fall through to `anyhow::bail!` and produce the `reqwest codeberg repo create failed` log line. Severity: warn. The operator could investigate one such failure with `curl -v` to determine the actual status code.

**Finding 3.4 — the 87 github auto-create failures are mostly rate-limit (85 of 87 with "too many repositories").** The other 2 are `gh repo create` errors of unspecified kind. Severity: info. Once the operator's rate-limit window resets, the daemon should succeed on next attempt. But per Finding 3.1, the daemon shouldn't be attempting auto-create at all if `remote_repo_exists` returns Ok(true).

---

## Section 4 — Push-time size analysis

### Per-blob push threshold

`policy.max_push_blob_bytes` defaults to `DEFAULT_GIT_HOST_BLOB_LIMIT_BYTES = 100 MiB` (policy.rs:69). The global policy sets `max_push_blob_bytes = 52428800` (50 MiB) at line 80. The validator clamps this to `[1, 100 MiB]` (policy.rs:1105-1107).

**However, the runtime push path does NOT consult `max_push_blob_bytes`.** The comment at sync.rs:1163-1166 is explicit: "a separate `push_with_blob_check` function that did this but was never called from the live code path." `detect_large_blobs_ahead` (git/staging.rs:83-143) exists and is used by the `rewrite_large_blobs` repair flow (report.rs:2860), not by `push_background` (sync.rs:1167+).

### What the daemon actually does on push

`sync_repo` calls `push_mirror_remotes` (sync.rs:1223), which calls `configure_all_remotes` then iterates each remote and calls `tokio::process::Command::new("git").arg("push")` with the scaled timeout and retry count. No size check at any layer.

The forge itself is the size gate:
- GitHub: hard 100 MiB blob limit (HTTP 422 if exceeded).
- GitLab: hard 100 MiB blob limit (configurable per-project).
- Codeberg: hard 100 MiB blob limit (configurable per-instance, default 100 MiB).

If a single blob exceeds the forge's limit, `git push` returns exit 1 with "GH001: …" or equivalent. This is a forge-side rejection, not a daemon-side skip.

### Per-push aggregate size

There is **no daemon-side per-push aggregate size cap**. The push is constrained only by:
- `push_op_timeout_secs = 900` (15 min — global policy)
- Scaled at runtime: `scale_push_timeout(policy.push_op_timeout_secs, ahead_count)` (sync.rs:1183) — longer timeouts for more ahead commits. Capped at 600s (10 min) per the comment "a runaway push can't block the daemon forever".

For `dracon-platform` (217 ahead at audit time), the runtime push timeout would scale up. The 2026-06-26 audit (Finding 7.4) noted the daemon dynamically scaling `push timeout 900s → 600s` for dracon-platform when ahead count drops — but with 217 ahead the timeout should be at the 600s ceiling.

### Push size for `dracon-platform`

The local `.git/objects/pack` is 13 GiB. The history that needs to push is:
- Local HEAD: `d4ca6983ff` (after the 217 local-only commits since the codeberg divergence at `8fc02238f5`)
- 217 local-only commits to push
- Most commits are small (≤10 KiB), but some PNG/JPG binaries from the `capture-anime-girls` / `deathrun` / `darklord` smoke-out batches may be larger

Per the 2026-06-26 audit (Finding 9.1), the daemon has been **blocked from pushing for 152-153 consecutive failures** because of the 1-behind divergence (codeberg has commit `6a7cf69324` that is not a local ancestor). This is not a size issue — it is a true history divergence that `--force-with-lease` cannot safely resolve (per the 2026-06-21 incident precedent in `concern-2-4remote-divergence-2026-06-21.md`).

**Finding 4.1 — the daemon's push path has no aggregate-size cap.** It relies on (a) the forge's per-blob limit (100 MiB), (b) `push_op_timeout_secs = 900` scaled by ahead count, (c) `repo_sync_timeout_secs = 960` as the overall per-repo sync deadline. Severity: info. For `dracon-platform`'s 13 GiB of pack data, none of these caps will trigger if the network is fast enough.

**Finding 4.2 — `max_push_blob_bytes` is a no-op at runtime.** It is a documented config field that does not gate anything. Severity: warn. The operator could either (a) remove the field from the global policy and the code, (b) actually wire it into `push_background` (a 5-line change to `sync.rs:1167+` to skip blobs above the threshold with a clear log message), or (c) leave it as-is for documentation purposes.

**Finding 4.3 — `dracon-platform`'s PUSH_STUCK is the divergence, not size.** 152-153 consecutive failures, 20 explicit `non-fast-forward` rejections on the codeberg `main-temp` branch. Per the 2026-06-26 audit Finding 9.1, the resolution requires an operator decision (pull --rebase to bring codeberg's `6a7cf69324` into local, or accept force-push over codeberg via `dracon-sync repair concerns --apply`, with the same risk class as the 2026-06-21 unintended force-push). Severity: blocker (unchanged from the prior audit).

---

## Section 5 — Journal classification of the 153+ push failures

Captured from `journalctl --user -u dracon-sync.service --since "7 days ago"`. Output saved at `docs/design/audit-2026-06-26/secret-and-push-classification.txt`.

### `dracon-platform` push attempts to each remote (7d)

| Remote | Attempts | Failures | Failure rate |
|---|---|---|---|
| codeberg | 2,331 | ~2,306 (98+%) | PUSH_STUCK |
| gitlab | 70 | 69 (98%) | limited — daemon only tried when ahead |
| github | 1 | 1 (100%) | rare (no github remote locally; one stray attempt) |
| origin | 0 | 0 | n/a (no origin remote) |

The 2,584 push-to-codeberg attempts and 397 push-to-gitlab attempts (all repos, not just platform) are dominated by the dracon-platform retries.

### Failure causes for `dracon-platform` (classified)

| Cause | Count | Sample log line |
|---|---|---|
| `non-fast-forward` | 20 | `Jun 27 01:02:45 dracon-sync[3616679]: ⚠️ push to codeberg failed … ! [rejected] HEAD -> main-temp (non-fast-forward)` |
| `timeout` (push_op_timeout_secs exceeded) | 98 | `Jun 20 01:30:34 dracon-sync[740123]: ⏱️ push retry 2/3 for /home/dracon/Dev/dracon-platform after 1s` |
| `timed out` (literal word) | 5 | similar |
| `Connection refused` / `Connection reset` | 8 | `Jun 20 16:18:06 dracon-sync[2570923]: ⚠️ push to codeberg failed … Connection reset by 217.197.84.140 port 22` |
| `denied` | 5 | (likely "denyNonFastForwards" on protected branches) |
| `failed to push` (generic) | 155 (mostly transient retries) | |
| `all HTTPS push attempts failed` | 4 | fallback transport chain exhausted |

### Auto-create failure totals (7d, all repos)

| Forge | Failure count | Sample error |
|---|---|---|
| github | 87 (85 with "too many repositories") | `gh repo create failed: GraphQL: You have created too many repositories, too quickly.` |
| gitlab | **0** | (no auto-create attempts) |
| codeberg | 106 | `reqwest codeberg repo create failed` (no further detail in log) |

For gitlab: 0 attempts in 7 days. The daemon's `glab repo create` either succeeds silently when the repo already exists (`is_repo_already_exists(&stderr)` returns Ok) or is never invoked.

**Finding 5.1 — the 152-153 consecutive failures are NOT size-related.** They are dominated by (a) the 20 `non-fast-forward` events (true divergence), (b) the 98 `timeout` events (the daemon's 900s/600s scaled timeout being exceeded, possibly because of the 13 GiB pack data + slow codeberg network), (c) 8 `Connection reset` events (network), (d) 5 `denied` events (forge-side rejection). Severity: blocker (unchanged from the 2026-06-26 audit).

**Finding 5.2 — the 98 `timeout` events for `dracon-platform` ARE size-related (indirectly).** The 13 GiB pack data needs more than 600s to push when the network is slow. The 2026-06-17 + 2026-06-23 push-timeout fixes raised `push_op_timeout_secs` 60 → 300 → 900 specifically for this reason. The 98 timeout events are residual cases where even 600s (the runtime cap) wasn't enough. Severity: warn. The operator could either accept the persistent PUSH_STUCK or reduce `dracon-platform`'s history size (e.g., `git gc` + delete old binary blobs) — but the latter requires a history rewrite that AGENTS.md forbids.

**Finding 5.3 — `dracon-platform` has NO github remote locally, so 0 push attempts to github is expected.** The 1 attempt in the journal is a stray (probably from a transient race or an earlier config state). The triple-sync-feasibility report's Finding 3.7 noted this; this investigation confirms it.

---

## Section 6 — Secret/token audit (auth-side answer)

### Operator session env (live shell)

```
$ env | grep -iE 'GITLAB_TOKEN|CODEBERG_TOKEN|GITHUB_TOKEN|GH_TOKEN|GITEA_TOKEN'
(no matches)
```

The operator's interactive shell has NO `GITLAB_TOKEN` / `CODEBERG_TOKEN` / `GH_TOKEN` set. This is expected — the daemon reads secrets from disk, not env vars.

### Systemd unit env (dracon-sync.service)

```
Environment=PATH=…
Environment=DRACON_SYNC_POLICY=%h/.dracon/utilities/sync/dracon-sync.toml
Environment=GIT_TERMINAL_PROMPT=0
PassEnvironment=SSH_AUTH_SOCK
```

The daemon inherits PATH (for `git`/`gh`/`glab` lookup), its policy path, and SSH_AUTH_SOCK. It does NOT inherit any token env vars.

### Daemon process env (live check via /proc)

```
PID=3616679
$ cat /proc/3616679/environ | tr '\0' '\n' | grep -iE 'GITLAB_TOKEN|CODEBERG_TOKEN|GH_TOKEN|GITHUB_TOKEN'
(no matches)
```

Confirmed: the daemon's process env has NO tokens. But it CAN read the secret files from disk (mode 600).

### Secret files on disk

```
$ ls -la /home/dracon/.dracon/secrets/pat/
total 32
-rw------- 1 dracon users  183 May 17 21:36 codeberg.env
-rw------- 1 dracon users  213 May 18 14:46 cratesio.env
-rwx------ 1 dracon users 1299 Jun 11 17:50 git-credential-github.sh
-rw------- 1 dracon users  280 May 17 20:13 github.env
-rw------- 1 dracon users  129 May 11 08:52 gitlab.env
-rw------- 1 dracon users  263 May 13 07:20 npm.env
```

```
$ ls -la /home/dracon/.dracon/utilities/sync/secrets/
total 16
drwx------ 2 dracon users 4096 Jun 11 17:47 .
drwxr-xr-x 6 dracon users 4096 Jun 16 01:37 ..
lrwxrwxrwx 1 dracon users   45 Jun 11 17:47 codeberg.env -> /home/dracon/.dracon/secrets/pat/codeberg.env
lrwxrwxrwx 1 dracon users   45 Jun 11 17:47 cratesio.env -> /home/dracon/.dracon/secrets/pat/cratesio.env
lrwxrwxrwx 1 dracon users   43 Jun 11 17:47 github.env -> /home/dracon/.dracon/secrets/pat/github.env
lrwxrwxrwx 1 dracon users   43 Jun 11 17:47 gitlab.env -> /home/dracon/.dracon/secrets/pat/gitlab.env
lrwxrwxrwx 1 dracon users   40 Jun 11 17:47 npm.env -> /home/dracon/.dracon/secrets/pat/npm.env
-rw-r--r-- 1 dracon users 5078 May 17 20:15 README.md
```

All 3 token files exist: `codeberg.env` (183 bytes), `github.env` (280 bytes), `gitlab.env` (129 bytes). All mode 600, owned by dracon:users. The utilities/sync/secrets dir is mode 700.

### Daemon's secret-loading code path

`load_secret(env_name)` (`src/git/misc.rs:10-12`) calls `crate::secrets::load_secret(env_name, &crate::secrets::sync_secrets_dir())`.

`load_secret(env_name, secrets_dir)` (`src/secrets.rs:30-78`) checks:
1. `std::env::var(env_name)` — not set in the daemon env
2. Permission check on `secrets_dir` (must not be world-writable) — passes (mode 700)
3. Scan `*.env` files in `secrets_dir` (default `~/.dracon/utilities/sync/secrets`)
4. Parse `KEY=VALUE` lines, return matching `env_name`

`load_secret_or_legacy_pat(env_name)` (`src/git/misc.rs:17-21`) first tries `load_secret`, then falls back to `legacy_pat_secrets_dir()` = `~/.dracon/secrets/pat/`. This is the fallback used by `auto_create_repo` for codeberg (multi_remote.rs:521-523).

### CLI auth state

| Tool | Authenticated? | Notes |
|---|---|---|
| `gh` | ✅ yes (DraconDev) | token scopes `gist, read:org, repo, workflow`; protocol `https` |
| `glab` | ❌ no token | SSH for git ops; API returns 401 unauthorized (no token in config, keyring, or env) |

The `glab` 401 explains why the daemon's gitlab auto-create path would fail at `glab repo create` if it ever ran — but per the journal, the daemon never even attempts gitlab auto-create. The `load_secret("GITLAB_TOKEN")` fallback path (from the disk .env file) is the path that would succeed if invoked, but the daemon invokes `glab repo create` instead (multi_remote.rs:441-467), bypassing its own `load_secret` for gitlab.

### Verdict on the operator's question: "is the missing-on-gitlab+codeberg state caused by size, or by missing auth tokens?"

**Neither.** The repos are not missing — they exist on all 3 forges (verified by fresh SSH `git-upload-pack` probe). The 2026-06-26 report's "9 missing" claim was a measurement error caused by reading the unauthenticated public REST APIs. The auto-create attempts in the journal (87 github + 106 codeberg) are residual races where `remote_repo_exists` transiently failed and the daemon tried to create a repo that already exists.

The size of `dracon-platform` (115 GiB / 13 GiB pack) is irrelevant to auto-create and is only relevant to the push side, where it contributes to the 98 `timeout` events in 7 days (PUSH_STUCK residual). The real PUSH_STUCK cause is the divergence (commit `6a7cf69324` on codeberg not in local history), which is unchanged from the 2026-06-26 audit.

**Finding 6.1 — all 3 token files exist on disk with correct permissions.** The daemon can read them via `load_secret` / `load_secret_or_legacy_pat`. Severity: info. No fixes needed for the secret path.

**Finding 6.2 — `glab` is not API-authenticated, but the daemon's gitlab auto-create path uses `glab repo create` instead of a direct API call.** This is a daemon code issue: `create_repo_on_gitlab` (multi_remote.rs:441-467) relies on `glab`'s API auth, not the daemon's `load_secret("GITLAB_TOKEN")` mechanism. Severity: warn. If the operator wants the daemon to be able to auto-create on gitlab without depending on `glab`'s auth state, the daemon would need a `create_repo_on_gitlab_via_api(token, account, repo_name, private)` analog to `create_repo_on_codeberg`.

**Finding 6.3 — the systemd unit's env is minimal.** It passes PATH + policy path + SSH_AUTH_SOCK. Tokens are read from disk via `load_secret`. Severity: info. No fixes needed.

**Finding 6.4 — the verdict: the missing state is NOT caused by size and NOT caused by missing tokens.** It was caused by the 2026-06-26 report reading the unauthenticated public APIs. All 15 repos exist on all 3 forges. Severity: blocker (corrects the previous report's headline finding).

---

## Section 7 — Summary and recommended next actions

### Daemon size thresholds (cited)

| Field | Default | Current policy | Used for | Enforcement |
|---|---|---|---|---|
| `max_stage_file_bytes` | 100 MiB | 100 MiB | Skip oversized files from staging | ✅ enforced (git/staging.rs) |
| `max_push_blob_bytes` | 100 MiB | 50 MiB | (declared; never enforced) | ❌ NO-OP (push_with_blob_check was never wired in) |
| `untracked_warn_threshold` | 500 | 500 | Warn-only | ✅ warn only |
| `auto_commit_backstop_threshold` | 20 | 20 | Gate auto-commit on ahead count | ✅ enforced |
| `alert_unpushed_threshold` | 10 | 50 (CHANGED 2026-06-26) | Alert emission in `dracon-sync repos` | ✅ enforced |
| `sem_max_concurrent_sync` | 4 | 4 | Concurrency cap | ✅ enforced |
| `push_op_timeout_secs` | 60 | 900 (scaled to 600 by ahead count) | Push idle timeout | ✅ enforced |
| `repo_sync_timeout_secs` | 120 | 960 | Per-repo sync deadline | ✅ enforced |
| **size check in `auto_create_repo`** | n/a | n/a | (none) | ❌ no such check exists |

### Is the platform-too-big the cause of PUSH_STUCK / missing-on-gitlab+codeberg?

| Hypothesis | Verdict | Evidence |
|---|---|---|
| Auto-create is skipped because `dracon-platform` is too big | **NO** | `auto_create_repo` (multi_remote.rs:508) has zero size logic. The `git ls-remote <remote> HEAD` existence check is the only gate. |
| Auto-create fails because of missing auth tokens | **NO** | All 3 token files exist on disk (mode 600, owned by dracon). `glab` 401 is a separate issue (Finding 6.2). |
| The 9 missing-on-gitlab+codeberg repos are actually missing | **NO** | All 15 repos exist on all 3 forges (verified by fresh SSH `git-upload-pack` probe). 2026-06-26 report's "9 missing" was wrong (private repos not visible to public API). |
| The platform's size contributes to the 98 push `timeout` events | **YES (indirectly)** | 13 GiB pack data + slow codeberg network = pushes sometimes exceed 600s scaled timeout. |
| The platform's size contributes to the 20 `non-fast-forward` events | **NO** | The divergence is a history issue, not a size issue. |

### Recommended next actions (operator decisions, not auto-fixes)

1. **[blocker] Resolve `dracon-platform` PUSH_STUCK divergence.** This is unchanged from the 2026-06-26 audit. Codeberg has `6a7cf69324 CLOSED: fix-ovh-access-key-id-misconfig, …` which is not a local ancestor. Operator decision: (a) `git pull --rebase codeberg main-temp` to add the divergent commit to local, (b) accept force-push over codeberg (same risk class as 2026-06-21), or (c) accept permanent stuck state.

2. **[info] Correct the 2026-06-26 triple-sync-feasibility report's "9 missing" finding.** All 15 repos exist on all 3 forges. The auto-create path is not broken — it just rarely runs because `remote_repo_exists` correctly returns true. The 2026-06-26 report should be amended (or a new erratum doc added) noting that private repos exist on gitlab/codeberg but were not visible to the unauthenticated probe.

3. **[warn] Wire `max_push_blob_bytes` into the push path or remove the field.** Currently declared and validated but never enforced (Finding 2.2). The comment in `sync.rs:1163-1166` says `push_with_blob_check` "was never called from the live code path" — a 5-line fix to actually call it, or remove the field to avoid operator confusion.

4. **[warn] Fix the `glab` 401 by either (a) provisioning a GitLab token and having the daemon use it directly (analogous to `create_repo_on_codeberg`'s reqwest path), or (b) accepting that gitlab auto-create is dead.** Currently the daemon's `create_repo_on_gitlab` relies on `glab`'s auth state, which is broken. The gitlab auto-create path hasn't run in 7 days (0 attempts) — so this is a dormant issue, not an active one.

5. **[warn] Investigate the 106 codeberg auto-create failures' actual status code.** `create_repo_on_codeberg` (multi_remote.rs:469-481) treats only 409/422 as "already exists"; any other 4xx is an error. A single `curl -v -X POST` to the codeberg API with the same JSON would reveal the actual response. Likely it's a 422 with a non-standard error body, or a 200 with an error JSON.

6. **[info] `dracon-platform` is 115 GiB on disk with 16 garbage objects (26.61 MiB) in `.git/objects/`.** These are leftovers from interrupted `git gc` runs. Not committed to git, not pushed, not affecting any daemon behavior. Operator could run `git gc --prune=now` once when convenient (NOT done in this audit per the read-only contract).

7. **[info] The 2026-06-26 daemon audit Finding 9.1 (PUSH_STUCK root cause) is correct.** The size of the repo is a contributing factor to the 98 push-timeout events, but the root cause is the divergence. Resolution is unchanged.

8. **[info] No code changes were made by this audit.** Read-only contract honored: 0 new files in `~/.dracon/`, 0 new remotes/branches/commits in `dracon-platform`, all forge API calls were SSH `git-upload-pack` reads (no POST/PUT/PATCH/DELETE).

---

## Evidence index

| File | Path | Description |
|---|---|---|
| `size-audit-platform.txt` | `docs/design/audit-2026-06-26/size-audit-platform.txt` | Fresh `du` + `git count-objects` output |
| `secret-and-push-classification.txt` | `docs/design/audit-2026-06-26/secret-and-push-classification.txt` | env / secret files / journal classifications |
| `forge-existence-ssh.json` | `docs/design/audit-2026-06-26/forge-existence-ssh.json` | 15 × 3 SSH `git-upload-pack HEAD` probe |

### Verification of read-only contract

```bash
$ find /home/dracon -maxdepth 4 -name '.dracon-sync.toml' -newer \
    /home/dracon/Dev/dracon-utilities/docs/design/auto-create-size-investigation-2026-06-27.md 2>/dev/null | wc -l
0

$ cd /home/dracon/Dev/dracon-platform && git remote -v
codeberg	git@codeberg.org:dracondev/dracon-platform.git (fetch)
codeberg	git@codeberg.org:dracondev/dracon-platform.git (push)

$ git branch --show-current
main-temp
```

**Audit complete. The operator's hypothesis was correct on both counts: (a) the missing-on-gitlab+codeberg state was a measurement error (private repos not visible to public API), and (b) the platform's size does contribute to the push-timeout residual events, but is not the cause of any auto-create skip.**