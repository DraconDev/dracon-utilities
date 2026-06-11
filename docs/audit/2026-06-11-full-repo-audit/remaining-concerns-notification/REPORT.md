# Remaining Dracon Sync Concerns and Notification Audit

Date: 2026-06-11  
Scope: `dracon-sync` WARN/CONCERN/STUCK inventory and manual-action notification gap.

## Executive Summary

- `rust-ai-web-auto` WARN was not a sync blocker. Its initial dirty state was clean/smudge filter / line-ending-only churn; after recheck it became OK. Later WARNs are ordinary user changes with `push_status=OK`.
- `dracon-platform` CONCERN/STUCK was caused by the installed global Warden pre-push hook, not by the repo history. The hook was outdated and blocked a vendored sample private key because it lacked the `.plaintext` sibling escape hatch. Updating hooks and adding the intentional `.plaintext` marker restored push health.
- There is no remaining `CONCERN` or `STUCK_PUSH` in the latest inventory. Remaining rows are `WARN`/`DIRTY` with `push_status=OK`, caused by preserved user changes.
- The notification gap was real: sync had incident-ledger entries for `dracon-platform`, but there was no persistent alert ledger and stuck repos skipped on retry did not reliably surface repeated manual-action alerts. I added a persistent `dracon-sync-alerts.jsonl`, stderr alert lines, critical desktop notifications, and retry alerts for stuck repos.

## Latest Inventory

Command:

```bash
DRACON_SYNC_GIT_BIN=${DRACON_SYNC_GIT_BIN:-/run/current-system/sw/bin/git} \
  dracon-sync repos --json --full-path
```

Evidence: `inventory-final.json`, `inventory-final.tsv`.

Current non-OK rows from the latest run:

```text
repo                         modified staged untracked ahead push_status state_flags hint
one-mil-girls                1        0      0         0     OK          DIRTY     run repair-warns --apply
dracon-platform              2        0      0         0     OK          DIRTY     run repair-warns --apply
browser-extensions-shared    0        0      2         0     OK          DIRTY     healthy
dracon-utilities             1        0      0         0     OK          DIRTY     run repair-warns --apply
dracon-code                  1        0      0         0     OK          DIRTY     run repair-warns --apply
```

No `CONCERN` or `STUCK_PUSH` remains.

## Root Cause: `rust-ai-web-auto`

Initial evidence:

- `per-repo-before.rust-ai-web-auto.txt`
- `inventory-before.tsv`
- `inventory-after-rust-clean.tsv`

Findings:

- `git diff` showed only line-ending-like changes in:
  - `scripts/fill_form.rs`
  - `scripts/inventory_monitor.rs`
- `git diff --ignore-space-at-eol` showed no textual difference.
- The files are under Dracon clean/smudge filters.
- After recheck, the repo became clean/OK.

Current state:

- Remaining WARN is normal user change tracking, not push failure.
- `git -C /home/dracon/Dev/rust-ai-web-auto push --dry-run origin main` returned `Everything up-to-date`.

Validation:

```text
cargo fmt --all --check          pass
cargo clippy --workspace -- -D warnings   pass
cargo test --workspace -- --test-threads=1  145 passed, 9 ignored
```

## Root Cause: `dracon-platform`

Initial evidence:

- `per-repo-before.dracon-platform.txt`
- `dracon-platform-diff-before-warden.txt`
- `dracon-platform-diff-after-warden.txt`
- `dracon-platform-warden-once.log`
- `dracon-platform-incident-lines.jsonl`

Findings:

- Working tree contained user changes/deletions under `web/games-hosted/games/junk-runner/...`.
- `dracon-warden once /home/dracon/Dev/dracon-platform` reported no hardening changes needed.
- `git push --dry-run origin main` failed in the pre-push hook with:

```text
Possible plaintext secrets detected in push.
```

- The blocked diff was not the user asset change. It was the vendored sample private key:

```text
vendor/hyper-rustls-0.25-patched/examples/sample.rsa
```

- The installed global hook was outdated and lacked the `.plaintext` sibling escape hatch.

Evidence for the current hook implementation:

- `dracon-warden/src/main.rs:2203-2235` documents and implements `<path>.plaintext` skip behavior in the pre-push hook.

Fix applied:

```bash
dracon-warden setup-hooks --global
```

Then added the intentional plaintext marker:

```text
vendor/hyper-rustls-0.25-patched/examples/sample.rsa.plaintext
```

After `git fetch origin main`:

- `main` and `origin/main` are aligned.
- `git push --dry-run origin main` returned `Everything up-to-date`.

Validation:

```text
cargo fmt --all --check          pass
cargo clippy --workspace -- -D warnings   pass (cargo wrapper reported 0 errors, 2 warnings)
cargo test --workspace -- --test-threads=1  268 passed, 6 ignored
scripts/check-env-encryption.sh  pass: All 13 tracked .env* file(s) are encrypted.
git push --dry-run origin main   Everything up-to-date
```

Current WARN is preserved user changes, including `web/ai-hub/src/routes/ai-hub/directory/+page.svelte`.

## Root Cause: Notification Gap

Evidence reviewed:

- `dracon-sync/src/daemon.rs` stuck-push retry path.
- `dracon-sync/src/report.rs` desktop notification path.
- `~/.local/state/dracon/dracon-sync-incidents.jsonl`
- `journalctl --user -u dracon-sync.service`
- `~/.dracon/utilities/sync/dracon-sync.toml`

Findings:

1. `dracon-platform` did create incident-ledger entries. Example extracted evidence is in `dracon-platform-incident-lines.jsonl`.
2. The daemon only attempted desktop notifications through `notify_rust`. There was no persistent alert ledger that an operator could inspect after the fact.
3. Stuck repos can be removed from active sync processing and retried later. That retry path did not create a fresh alert, so a repo could remain stuck without a visible repeated notification.
4. No `webhook_url` is configured in `~/.dracon/utilities/sync/dracon-sync.toml`, so webhook notification was not available for this environment.
5. The latest daemon run now produced an alert-ledger entry for a separate transient timeout:

```json
{"ts_unix":1781185784,"repo":"/home/dracon/Dev/one-mil-girls","reason":"Sync Timeout","details":"exceeded 120s limit"}
```

## Notification Fix

Changed files:

- `dracon-sync/src/report.rs`
- `dracon-sync/src/daemon.rs`
- `dracon-sync/README.md`
- `AGENTS.md`

What changed:

- Added `~/.local/state/dracon/dracon-sync-alerts.jsonl` with JSONL entries containing:
  - `ts_unix`
  - `repo`
  - `reason`
  - `details`
- Desktop notifications now use `notify_rust::Urgency::Critical`.
- Alert attempts are also written to stderr with a `🔔 sync alert:` prefix.
- Stuck-push retry path now writes a `Stuck Push Retry` alert once per repo per 30 minutes.
- Docs now tell operators to check both:
  - `~/.local/state/dracon/dracon-sync-incidents.jsonl`
  - `~/.local/state/dracon/dracon-sync-alerts.jsonl`

Tests added:

- `test_sync_alert_ledger_path_uses_state_dir`
- `test_record_sync_alert_appends_jsonl`

## Binary Installation

Installed updated binaries and restarted services:

```bash
./install.sh --binaries-only --upgrade
systemctl --user start dracon-sync.service dracon-system-guard.service
```

Evidence: `install-binaries-upgrade.log`.

The active `dracon-sync.service` is running the installed binary and emitted the new alert-ledger line for the `one-mil-girls` timeout.

## Remaining WARNs

These are not push blockers. They are preserved user changes with `push_status=OK`.

### `one-mil-girls`

Current status includes a generated/working-tree change:

```text
 M .svelte-kit/ambient.d.ts
```

Action: preserve user/generated change unless explicitly approved. Push is OK after fetch; the earlier timeout was transient and produced a new alert-ledger entry.

### `dracon-platform`

Current status includes user changes under `web/ai-hub/`:

```text
 M web/ai-hub/src/routes/ai-hub/+page.server.ts
 M web/ai-hub/src/routes/ai-hub/+page.svelte
```

Action: preserve user changes unless explicitly approved. Push is OK.

### `dracon-utilities`

Current status includes the live audit inventory file:

```text
 M docs/audit/2026-06-11-full-repo-audit/remaining-concerns-notification/inventory-final.json
```

Action: preserve; sync will commit once fingerprint stabilizes.

### `browser-extensions-shared`

Current status:

```text
?? <two untracked files>
```

Action: preserve user files unless explicitly approved.

### `dracon-code`

Current status includes a user change under `crates/dracon-ai/src/ai_client.rs`:

```text
 M crates/dracon-ai/src/ai_client.rs
```

Action: preserve user refactor/change unless explicitly approved. `git push --dry-run origin main` is up-to-date because this is a local change.

## Validation Evidence

`dracon-utilities`:

```text
cargo fmt --all --check                         pass
cargo clippy --workspace -- -D warnings         pass
cargo test --workspace -- --test-threads=1      705 passed, 9 ignored
cargo deny check                                advisories ok, bans ok, licenses ok, sources ok
scripts/verify-spec.sh                          PASS
dracon-sync config validate                     Policy is valid
dracon-sync scaffold --dry-run                  No standard files to scaffold
```

`rust-ai-web-auto`:

```text
cargo fmt --all --check                         pass
cargo clippy --workspace -- -D warnings         pass
cargo test --workspace -- --test-threads=1      145 passed, 9 ignored
git push --dry-run origin main                  Everything up-to-date
```

`dracon-platform`:

```text
cargo fmt --all --check                         pass
cargo clippy --workspace -- -D warnings         pass (0 errors; cargo wrapper reported 2 warnings)
cargo test --workspace -- --test-threads=1      268 passed, 6 ignored
scripts/check-env-encryption.sh                 All 13 tracked .env* file(s) are encrypted
git push --dry-run origin main                  Everything up-to-date
```

Other affected push checks:

```text
git -C /home/dracon/Dev/one-mil-girls push --dry-run origin main       Everything up-to-date
git -C /home/dracon/Dev/browser-extensions-shared push --dry-run origin main  Everything up-to-date
git -C /home/dracon/Dev/rust-ai-web-auto push --dry-run origin main    Everything up-to-date
git -C /home/dracon/Dev/dracon-utilities push --dry-run origin main    Everything up-to-date
```

## Conclusion

The remaining sync concerns are explained and either fixed or intentionally preserved:

- `rust-ai-web-auto`: no sync blocker; WARNs are user changes.
- `dracon-platform`: push blocker fixed by updating Warden hooks and adding `.plaintext` marker.
- `dracon-ai-lib`: archived-remote blocker was handled in the prior investigation; current push is OK.
- `one-mil-girls`: transient push timeout resolved.
- Notification gap: fixed with persistent alert ledger, critical desktop notifications, stderr alert lines, and stuck-retry alerts.

I did not delete, rebase, force-push, rewrite history, rotate secrets, or discard user changes.
