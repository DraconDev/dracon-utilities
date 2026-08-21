# Utility checkout disappearance — investigation 2026-08-21

## Summary

Between **2026-08-19 02:07** and **2026-08-21 15:40** the three nested
utility repositories (`dracon-sync/`, `dracon-system/`, `dracon-warden/`)
were deleted from their canonical location
`~/Dev/dracon-utilities/` without any log, Trash entry, or shell-history
trace. They were restored on 2026-08-21 by re-cloning from the GitHub
remotes (all three had zero unpushed commits at restore time — no data was
lost). This doc records the timeline, the forensic evidence, the root-cause
analysis, and prevention.

## Timeline (evidence-backed)

| Time | Event | Evidence |
|---|---|---|
| 2026-08-14 22:46 | Release dry-run work happens in `/tmp` (`dracon-sync-release-dry-run-PYVi2O` mtime) | `/tmp` dir mtimes |
| 2026-08-15 00:00–08:19 | Daemon actively committing all three nested repos; Aug-15 audit verifies local checkouts exist (`check-nested-pins.py --check-local` passed) | `journalctl -u dracon-sync`; `AUDIT_FULL_2026-08-15.md` |
| 2026-08-17 20:41 | `/tmp/dracon-sync-source` created by a FRESH `git clone` (reflog entry `clone:`), not by moving the canonical checkout — the canonical copy still existed at this point | `/tmp/dracon-sync-source` reflog HEAD@{2026-08-17 20:41:21} |
| 2026-08-17 20:58 – 08-18 07:08 | Six post-0.113.51 fixes committed in the `/tmp` clone (mirror-push fix, symlink staging skips, REM column, queued-vs-active push label) | same reflog |
| 2026-08-18 23:09 | Meta-repo `.gitignore` hygiene commit (`1d837ed65`) | meta git log |
| 2026-08-18 23:10 | LAST daemon sighting of `~/Dev/dracon-utilities/dracon-sync` ("synced (late)") | `journalctl -u dracon-sync` |
| 2026-08-19 02:07:46 | LAST daemon sighting of `~/Dev/dracon-utilities/dracon-warden` (8-file commit, then clean push) | `journalctl -u dracon-sync` |
| 2026-08-19 02:09 | `/tmp/dracon-warden-0.113.4-pre-0.113.5` backup binary written — tail end of the warden v0.113.5-RC session | `/tmp` mtime; CHANGELOG "[Unreleased] warden v0.113.5 local RC (2026-08-19)" |
| 2026-08-19 02:08–02:16 | Final commits/syncs of that session land cleanly; after 02:16 the journal contains ZERO further mentions of any of the three paths | `journalctl -u dracon-sync` (grep count = 0 from 02:08 onward) |
| 2026-08-21 15:40 | Fresh audit discovers all three checkouts gone; only registry sources + the `/tmp` sync clone remain | audit transcript |

Note: the differing "last seen" times (system Aug-15, sync Aug-18,
warden Aug-19) are lower bounds on existence, not deletion times — the
daemon only logs repos with activity. All three were still present when
the Aug-19 02:07 warden commit succeeded; the earliest consistent
deletion window is therefore **Aug-19 02:08 → Aug-21**, most plausibly at
the very end of the 02:08 session as post-release "cleanup".

## Forensic findings

1. **No Trash entries**: `~/.local/share/Trash/info/*.trashinfo` contains
   zero entries matching the three names. Deletion used `rm -rf`
   (or equivalent) which bypasses Trash.
2. **No shell history**: interactive histories contain no
   mv/rm/clone touching the canonical paths. Agent sessions execute via
   non-interactive shells which are not recorded in
   `~/.zsh_history` — so agent-driven deletions are inherently unwitnessed.
3. **No daemon alert**: `dracon-sync` silently stopped listing the three
   repos once their directories vanished. There is no
   "previously-watched repo disappeared" concern or alert — the fleet row
   count simply dropped (31 discovered on Aug-15 per the audit doc; 23 on
   Aug-21 before restoration) with no warning line in the journal.
4. **No incident-ledger entry**: zero matches in
   `dracon-sync-incidents.jsonl` for the three paths around the window.
5. **No data loss**: all three remotes carried every commit
   (verified 2026-08-21: the suspected "stranded" commit `76f00bd` was
   already on origin/main; warden's v0.113.5-RC docs are on the remote).
   The auto-commit daemon had pushed everything before the deletion.

## Root cause

A deletion performed by an (unidentified) agent session at/after the end
of the 2026-08-19 02:08 utilities work session — most likely routine
cleanup by an agent that believed the `/tmp` clones plus the forges made
the canonical checkouts redundant. The deletion was *possible* and
*invisible* because of four compounding architectural gaps:

- **G1**: The nested repos are untracked by the meta-repo parent
  (by design — meta-only repo), so parent `git status` shows them only as
  `?? <dir>`; deleting them leaves no diff, no commit, no revert path.
- **G2**: The sync daemon drops vanished watch paths silently — no
  concern, alert, or ledger entry.
- **G3**: Nothing local periodically asserts "the three canonical
  checkouts exist and match the CI pins"; `check-nested-pins.py
  --check-local` runs only in CI where checkouts are materialized fresh.
- **G4**: Agent-run destructive commands leave no audit trail on this
  host (non-interactive shells skip history).

## Prevention

1. **Daemon watchdog (recommended, code change)**: dracon-sync should
   raise a persistent concern/alert when a repo that was discovered in a
   previous scan disappears from its watch root ("watched-repo-vanished"),
   naming the path and last-seen time. This alone would have turned a
   2-day silent absence into a 2-minute signal.
2. **Local pin check timer**: run
   `python3 scripts/check-nested-pins.py --check-local` under a daily
   systemd user timer in the meta repo so missing/mismatched canonical
   checkouts surface within 24h independent of CI.
3. **Policy line in AGENTS.md**: agent loops must treat the three
   canonical checkouts as permanent fixtures — never `rm -rf`, move, or
   "clean up" them; work happens in-place (or via `dracon-sync maintenance
   --` for git surgery), mirroring the existing history-rewrite ban.
4. **Optional belt-and-braces**: nightly `git bundle create` of the three
   repos into `~/dracon/backups/utilities-daily/` (retention ~14d), so a
   future disappearance costs one clone-from-local-bundle instead of
   depending on remote availability.

Items 1 and 2 are follow-up candidates; items 3 and 4 are operator-policy
and can be adopted immediately.

## Restoration record

Restored 2026-08-21 ~20:30 BST by cloning:

- `https://github.com/DraconDev/dracon-sync-background-auto-commit-multi-remote.git`
- `https://github.com/DraconDev/dracon-system-disk-process-guard-doctor.git`
- `https://github.com/DraconDev/dracon-warden-secret-encrypt-age-git-filter.git`

into `~/Dev/dracon-utilities/<name>` (main, clean, synced). Warden filter
re-applied via `dracon-warden once` in dracon-sync post-clone. Daemon
re-discovered all three (fleet 23 → 27 rows).
