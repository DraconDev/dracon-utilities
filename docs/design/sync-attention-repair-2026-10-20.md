# Sync attention repairs — 2026-10-20

## Doomtap: index/worktree cancellation

The daemon repeatedly reported empty classification on a dirty status.
SPEC.md had staged edits and inverse unstaged edits; the worktree matched
HEAD exactly. Under `dracon-sync maintenance`, restored only that path's
index from HEAD. The worktree was preserved. Git status became clean and
HEAD matched origin/main. No content commit was necessary.

## Browser extensions: leaked test mounts

The tabmon pre-push test log showed add-all stopped at 38/40 species by its
20-second polling deadline. ui-redesign-states.test.ts removed DOM nodes
without unmounting Svelte instances: storage subscriptions and timers survived
across tests. Keep mount handles and unmount every instance after each test.
The polling helper now throws at its deadline rather than returning as though
it succeeded. Production storage/catch timing and the full pre-push gate stay
intact. The focused file passes all 54 tests; the full tabmon suite passes
all 918 tests across 59 files. sync-now committed the pending repair and
existing operator edits using the normal daemon staging path.

## db-gateway: historical synthetic fixture

Dry-run push reproduced the warden refusal on the added tests/secret_scan.rs
blob in 1c14b46c. The current source already uses split fixture fragments and
has no matches, but a forward change cannot remove a match from an earlier
commit. Verified programmatically that the historical assignment equals the
current synthetic fixture. Added one complete-blob/individual-match receipt
in the warden hook template, with its review record in the warden repo's
reviewed-hook-matches-2026-10-17.md. No file exemption, scan relaxation,
plaintext sibling, or history rewrite.

## Runtime

The live daemon is system-level dracon-sync-operator.service, UID 1000,
active since October 19. The masked user dracon-sync.service is obsolete;
its inactive status is not evidence the live daemon is down.

## Verification and deployment

Warden pre-push regression suite: 36 passed; specific receipt-boundary test:
1 passed. Workspace release build, cargo deny check and clippy with warnings
denied all passed. Release warden installed atomically with binary/hook backup
at ~/.local/state/dracon-sync/deployments/20261020T133559Z-warden-fixture-receipt;
setup-hooks --global installed the receipt. No service restart required.

After stuck-unstuck and sync-now, db-gateway HEAD, origin/main and gitlab/main
all equal 1d52ff491eea40ff8c5ef780d1795d6a6f884867; push-history records
succeeded=true at 1792503399. Doomtap report now classifies OK (zero concerns).

The browser-extension repair commit is bb926e2b1253f9e8bc1b5467d3f1047ada36ffd8.
The actual pre-push gate passed all 918 tabmon tests, plus build and lint,
and all 701 youtube-filter tests. GitHub accepted the previously queued
25 commits; GitLab runs its gates independently.

Db-gateway mirror-degraded notifications retained pre-repair daemon memory
after the CLI push. A local upstream tracking-ref refresh was attempted
under maintenance (remote branch untouched), then restored with ordinary
fetch. Added a repository-local repair note as durable evidence; the daemon
can commit and push it to demonstrate its own recovery.

## Final test hardening under load

One concurrent daemon push still reported a tabmon test refusal, and another
push overwrote its fixed-name test log with a passing result. Rather than
rely on the remaining wall-clock polling deadline, the bulk-add/clear
scenario now uses Vitest fake timers and vi.waitFor while retaining the real
storage code and all assertions. The obsolete polling helper was removed.
The final focused file passes all 54 tests. Svelte mount cleanup remains.

The unbounded-worker final rerun timed out in content-script setup and the
ffmpeg loudness test under severe host load; the repaired UI file passed.
Bounded tabmon to two workers in vitest.config.ts for normal hooks as well
as the verification run. This keeps the full assertions and test selection.
GitLab accepted the 25-commit backlog at bb926e2b; later test hardening is
fb53a493 and the worker configuration follows through ordinary daemon sync.
Rust sync unit suites passed (144 library + 1356 binary tests); integration
passed (10 tests) and storage-clean passed (14 tests, 3 operational ignores).

Final bounded-worker run: all 918 tests / 59 files passed (438 seconds under
host pressure), including the virtual-clock UI scenario and ffmpeg tests.
Final config commit: 1e7d9f7fd2. A fresh browser-extensions report shows
AHEAD:3 with zero concerns, rather than the original stalled push failure.
Db-gateway repair note commit 1bc8fd5 is clean and matches both mirror refs.
