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
