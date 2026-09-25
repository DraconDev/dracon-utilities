# Sync convergence evidence

This directory records the forward-only convergence run selected on 2026-09-25.

## Files

- `evidence.json` — generated run state and final verification input.
- `evidence.schema.json` — checked-in structural contract for the evidence document.

## Safety boundary

The evidence tools are read-only with respect to selected repositories. They do not commit, pull, merge, push, resume the daemon, delete work, or rewrite history. The operator keeps the sanctioned freeze active until the quiescent snapshot, reviewed adoption, and pre-resume gates are complete.

Permitted final exceptions are limited to provider outage, authentication/permission denial, or an unavailable remote after at least three recorded attempts. Dirty state, ahead/behind state, protected-branch rejection, a failed local test, and divergence are never external blockers.

## Regeneration

1. Keep `dracon-sync pause` active.
2. Run `scripts/capture-sync-convergence.py` with every selected `--repo` and `--evidence audit/sync-convergence-2026-09-25/evidence.json`.
3. Review and adopt the snapshotted work using normal commits and fast-forward-only Git operations.
4. Record each action and quality gate with `scripts/manage-sync-convergence.py`; use its `record-blocker` subcommand only after the verifier independently classifies three fresh remote attempts as an allowed provider/auth failure.
5. After pre-resume validation, run `dracon-sync resume` and record that action.
6. Run `scripts/manage-sync-convergence.py ... finalize`, then the independent contract command `python3 scripts/verify-sync-convergence.py --evidence audit/sync-convergence-2026-09-25/evidence.json`.

The verifier independently re-reads every selected repository, checks the daemon JSON reports, queries every effective remote, verifies parent/child gitlinks, and rejects prohibited reflog actions or non-forward ancestry.
