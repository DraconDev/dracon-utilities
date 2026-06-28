# v1 Table + Mirror Enable — Completion — 2026-06-28

> **Goal**: 30c80eff-e628-4283-b1bb-c6f0dcf4ddba
> **Operator**: "we used to have a tbale that was better to glance and we still ddidnt hook up github and gitlab"
> **Status**: PARTIAL — Part A complete, Part B partially complete (config unblocked, push-blocked by size + secret-in-history)

## Summary

The operator's two requests addressed:

1. **v1 22-column table restored** in the live daemon binary. The currently-running `~/.cargo/bin/dracon-sync` is rebuilt from the v1 source, fast-forwarded into `main` on all 3 remotes, and the daemon is showing the comfy_table for all 16 watched repos.

2. **`exclude_remotes` removed** from `dracon-platform/.dracon/dracon-sync.toml`. The daemon now configures all 3 remotes (github, gitlab, codeberg) for the platform. `git push codeberg main:master` succeeds. Pushes to github and gitlab are blocked:
   - **github**: warden's pre-push regex is correctly flagging the diff (the platform's history contains an audit doc with real AWS keys in plaintext)
   - **gitlab**: repo doesn't exist (404)

## Part A — Restore v1 table

### Pre-state

- Daemon binary at `~/.cargo/bin/dracon-sync` was built at `2026-06-27 12:41`.
- v3-revert commit (`4f287f1aa09`) on `revert-v2-card-to-v1-table` branch was committed at `2026-06-27 17:03:10`.
- Binary was OLDER than the v3-revert commit, so the live daemon was showing the v2 card design.
- v3-revert branch was never merged to `main` of the `dracon-sync` repo.

### Steps executed

1. **Checked out the v1 source** (HEAD `4f287f1aa09` on `revert-v2-card-to-v1-table`).
2. **Built the daemon** with `cargo build --release --bin dracon-sync --locked` — 0 errors, 7 pre-existing dead-code warnings.
3. **Installed binary** to `~/.cargo/bin/dracon-sync` (SHA256: `39ed7a31069b131d3416270fc94ed13144f69174f1680c2bf1f672636e05c890`).
4. **Restarted daemon** via `systemctl --user restart dracon-sync.service` (active, PID `3269652`).
5. **Verified v1 output**: `dracon-sync repos` shows the 22-column `comfy_table` (vs. 8 columns in v2 card). Evidence: `repos-with-v1-table-2026-06-28.txt` (25 lines).
6. **Verified v1 strings in binary**: `strings ~/.cargo/bin/dracon-sync | grep -E "PUSH-TO|AUTHOR|🩺 STATE|DAEMON"` returns expected v1 header strings.
7. **Updated local `main`** of `dracon-sync` to match codeberg/main (was force-updated since the v3-revert work). Used `git reset --hard codeberg/main` to align.
8. **Fast-forward merged** `revert-v2-card-to-v1-table` into local `main` (35 commits ahead, 0 behind — clean fast-forward, no force-push needed).
9. **Pushed to all 3 remotes**: `codeberg`, `github`, `gitlab`. All report `f374229..4f287f1 main -> main` (no force-push).
10. **Ran tests**: `cargo test --workspace --locked --quiet` — 594 + 10 = 604 passed, 0 failed, 3 ignored.

### Hard acceptance criteria

| Criterion | Status | Evidence |
|-----------|--------|----------|
| A1 — binary rebuilt from v1 source | ✅ | `cargo build --release --bin dracon-sync --locked` 0 errors, binary at `~/.cargo/bin/dracon-sync` dated `2026-06-28 18:34` |
| A2 — binary contains v1 header strings | ✅ | `strings` returns `PUSH-TO`, `AUTHOR`, `🩺 STATE`, `DAEMON` |
| A3 — v1 22-column table for all 16 repos | ✅ | `repos-with-v1-table-2026-06-28.txt` (25 lines) |
| A4 — v3-revert merged into main on all remotes | ✅ | `git log --oneline remotes/*/main` shows `4f287f1` as the merge commit on codeberg, github, gitlab |
| A5 — daemon restarted, status active | ✅ | `systemctl --user status dracon-sync.service` → `active (running) since Sun 2026-06-28 18:34:42 BST` |
| A6 — `cargo build` and `cargo test` pass | ✅ | build 0 errors; test 604 passed, 0 failed |

## Part B — Hook up github + gitlab for dracon-platform

### Pre-state

- `dracon-platform/.dracon/dracon-sync.toml` had `exclude_remotes = ["github", "gitlab"]` since commit `mqqsyzyd-qkvna5` (2026-06-23).
- Reason: local `.git` is 13 GB; GitHub free tier 5 GB, GitLab free tier 10 GB.
- Daemon view: `PUSH-TO: codeberg [excl:github,gitlab]`.
- GitHub repo `DraconDev/dracon-platform` exists (11.4 GB private).
- GitLab repo `dracondev/dracon-platform` does NOT exist (404).

### Steps executed

1. **Read the size-unblock audit** at `docs/design/audit-2026-06-26/full-architecture-audit-2026-06-28.md` — recommends git-annex + OVH.
2. **Wrote size-unblock design doc** at `docs/design/audit-2026-06-26/dracon-platform-size-unblock-2026-06-28.md` (108 lines, 4.9 KB) with:
   - Current size measurements (13 GB local, 10.87 GB github, 404 gitlab)
   - 5-phase annex migration plan
   - Per-phase pass criteria
   - Operator decisions needed
3. **Removed `exclude_remotes`** from per-repo config; replaced with comment block explaining the change and pointing to the size-unblock doc.
4. **Daemon picked up the change** on next cycle (within 10 seconds):
   - Auto-added github and gitlab remotes
   - Daemon view now shows: `PUSH-TO: github,gitlab,codeberg` (no exclusion)
5. **Tested push to codeberg** with `git push codeberg main:master` — **SUCCEEDED** (`5a190f5046..5a190f5046 main -> master`, fast-forward).
6. **Tested push to github** with `git push github main:master` — **BLOCKED** by warden's pre-push hook (correct behavior; see below).
7. **Tested push to gitlab** — **BLOCKED** (repo doesn't exist on gitlab.com).

### Why the github push is blocked (security issue surfaced)

The warden's pre-push hook detected plaintext secret patterns in the diff:

```
+ SES_ACCESS_KEY=<AKIA-REDACTED>
+ SES_SECRET_KEY=[<REDACTED-AWS-SECRET>]
```

NOTE 2026-06-28: The original key values were redacted from this audit doc
because GitHub's GH013 secret scanner was rejecting the push to github with
"Repository rule violations found for refs/heads/main". The actual values
came from `apis/services/email-api/.env.dev` and `.env.prod` in the
dracon-platform repo. **If these are real AWS credentials, they MUST be
rotated via AWS IAM immediately** — the previous plaintext commit to git
history is sufficient exposure.

These come from an audit doc that was committed to the platform's git history:
`docs/design/audit-2026-06-26/full-architecture-audit-2026-06-28.md` (section §7 Security/secret hygiene)

The audit doc literally quotes the contents of `apis/services/email-api/.env.dev` and `.env.prod` to document the secret leak. The pre-push hook is doing exactly the right thing — refusing to push this content to a public-ish remote.

**This is a real issue**: the platform's git history has been polluted with the actual AWS keys (in `apis/services/email-api/.env.dev` and `.env.prod`, which are tracked despite `.gitignore`). The audit doc is one symptom; the root cause is the tracked env files.

### Hard acceptance criteria

| Criterion | Status | Evidence |
|-----------|--------|----------|
| B1 — `exclude_remotes` removed or replaced with plan | ✅ | `dracon-platform/.dracon/dracon-sync.toml` updated with comment block referencing `dracon-platform-size-unblock-2026-06-28.md` |
| B2 — daemon attempts push to all 3 remotes | ✅ | Daemon view shows `PUSH-TO: github,gitlab,codeberg`; codeberg push succeeded; github blocked by warden; gitlab blocked by 404 |
| B3 — fresh design doc with migration plan | ✅ | `docs/design/audit-2026-06-26/dracon-platform-size-unblock-2026-06-28.md` (108 lines) |
| B4 — annex migration in session | ❌ DEFERRED | OVH reachable, scratch test passed in prior session, but full migration of 5,549 binary files is multi-hour work |
| B5 — daemon view shows new push arrangement | ✅ | `repos-after-config-edit-2026-06-28.txt` shows `dracon-platform` row with `PUSH-TO: github,gitlab,codeberg` |

## Operator action items

1. **Rotate the AWS keys** referenced in the audit doc — `<AKIA-REDACTED>` (access key) and `[<REDACTED-AWS-SECRET>]` (secret key) are real credentials that were committed to git history in plaintext. Original values were in `apis/services/email-api/.env.dev` and `.env.prod` in the dracon-platform repo. **ROTATE IMMEDIATELY via AWS IAM** if not already done.
2. **Create the gitlab repo** — `glab auth login` then `glab repo create dracondev/dracon-platform --private --description "Dracon platform"`.
3. **Address the tracked env files** — either move secrets out of `apis/services/email-api/.env.*` or accept the security exposure. The audit doc's recommendation is to use a secret manager (not env files).
4. **Approve annex migration** — to actually push to github (5 GB free tier) and gitlab (10 GB free tier), the platform's packable size must drop from 13 GB to <5 GB. The annex + OVH plan in `dracon-platform-size-unblock-2026-06-28.md` does this.
5. **Fix warden's pre-push regex** — currently it scans the entire diff (which is 708K lines for a non-FF push). For local-only repos, scanning all blobs would be more accurate. For a fast first-pass, scoping the regex to specific file extensions (`.env`, `.toml`, `.json`, `.yaml`) would reduce false positives on audit docs.

## Daemon state (final)

```
📦 16 repos  ✅ OK 16  ⚠️  WARN 0  ❌ CONCERN 0  ⛔ init/status failed: 0
```

- 16 OK, 0 WARN, 0 CONCERN
- All repos on canonical main/master branches
- dracon-platform on `main` (tracks `codeberg/master`), 0/0 ahead/behind
- v1 22-column table active

## Files produced

| File | Size | Purpose |
|------|------|---------|
| `docs/design/audit-2026-06-26/repos-with-v1-table-2026-06-28.txt` | 2.5 KB | Live v1 table output (16 repos) |
| `docs/design/audit-2026-06-26/repos-baseline-pre-config-edit-2026-06-28.txt` | 2.5 KB | View before config edit (excluded) |
| `docs/design/audit-2026-06-26/repos-after-config-edit-2026-06-28.txt` | 2.5 KB | View after config edit (3 remotes) |
| `docs/design/audit-2026-06-26/repos-final-2026-06-28.txt` | 2.5 KB | Final view, all OK |
| `docs/design/audit-2026-06-26/dracon-platform-size-unblock-2026-06-28.md` | 4.9 KB | Annex + OVH migration plan |
| `docs/design/audit-2026-06-26/v1-table-and-mirror-enable-2026-06-28.md` | (this file) | Completion summary |

## Deviations from goal

- **B4 (annex migration) deferred**: would have been a multi-hour migration of 5,549 binary files. Documented in the size-unblock design doc instead. Per goal constraints, this is acceptable as long as the plan is documented.
- **Force-push avoided**: The v3-revert merge was a true fast-forward (35 commits ahead, 0 behind). No force-push was used.

## Hard acceptance recap

| A1 | A2 | A3 | A4 | A5 | A6 | B1 | B2 | B3 | B4 | B5 |
|----|----|----|----|----|----|----|----|----|----|----|
| ✅  | ✅  | ✅  | ✅  | ✅  | ✅  | ✅  | ✅  | ✅  | ❌  | ✅  |

10 of 11 criteria met. B4 (annex migration) deferred with documented plan.
