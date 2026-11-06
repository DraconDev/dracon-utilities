# Fleet epoch-spool cleanup + performance remediation — tasklist

**Created**: 2026-11-06
**Trigger**: host unresponsive, load average 87 → 14, `MemAvailable` 74 MiB of
30 GiB, 65 GiB swap in flight, `/` at 96% (826G/907G), PSI io `full avg10=84.3`
and mem `full avg10=77.5`.
**Owner**: operator
**Status**: NOT STARTED — every destructive step below needs explicit approval.

---

## Evidence gathered 2026-11-06

### The three pressures, ranked

| # | Cause | Magnitude | Fixable here |
|---|---|---|---|
| 1 | Epoch spool leak (`.git/dracon-staging-*`) | 19,707 dirs / **72.2 GiB** | yes (cleanup), no (root cause) |
| 2 | Orphaned pi-session script (37 GiB VmPeak / 35.6 GiB swap) | one-off, exited | yes (guard install) |
| 3 | 35 concurrent pi agents + 83 chrome procs | 5.2 GiB + 2.6 GiB | operator habit |

### Why the daemon never reclaimed them

`dracon-sync 0.113.96` allocates one full-tree spool per sync epoch. The
reaper runs but removes nothing:

```
journal (whole):  preserved 16308   removed 0
reasons:  unknown non-empty payload              11524
          spool index does not describe a tree    3654
          legacy ownership uncertain               943
          source inactivity not proven              165
```

Two independent blockers:

1. `dracon-sync/src/git/epoch_spool.rs:10` — `LOSSLESS_RECLAIM_QUALIFIED: bool
   = false`, with the comment *"Qualification remains disabled until
   regression/fault/non-vacuity gates pass. ... keep live reclamation disabled
   pending renewed qualification."* This is a deliberate **safety hold**, not a
   bug in itself.
2. `epoch_spool_redundant.rs:585` — `only_known_files()` bails on any file it
   does not recognise as payload-free. `producer-io-*` endpoints must name only
   published object ids or a retained pack stem; in practice many hold a
   revision expression or full-width hash that fails the check, so the whole
   spool is preserved even when every object is published.

### Verified redundancy audit (bounded-memory, `git show-index` + `--git-dir`)

Per spool, every object in its retained pack was classified:
**proven** = reachable from a live ref; **unreach** = in the main object store
but unreachable; **UNIQUE** = missing from the main store entirely.

| Repo | Spools | Size | proven | unreach | UNIQUE |
|---|---|---|---|---|---|
| dracon-strategy/ai-auto-writer | 2273 | 29.8 GiB | 2248 | 21 (283 MiB) | 0 |
| dracon-strategy/ai-auto-video | 667 | 24.8 GiB | 654 | 9 (415 MiB) | 0 |
| dracon-platform | 4660 | 8.5 GiB | 4593 | 49 (109 MiB) | 18 (37 MiB) |
| browser-extensions-shared | 1643 | 2.9 GiB | 1630 | 4 (7 MiB) | 0 |
| dracon-strategy | 759 | 2.4 GiB | 753 | 0 | 0 |
| dracon-platform/.git/modules/junk-runner | 2106 | 1.0 GiB | 2102 | 2 (1 MiB) | 0 |
| dracon-platform/.git/modules/polis | 420 | 646 MiB | 412 | 6 (280 MiB) | **2 (110 MiB)** |
| dracon-platform/web/games/wip/freeport | 388 | 496 MiB | 387 | 1 (16 MiB) | 0 |
| dracon-platform/.git/modules/deathrun | 694 | 411 MiB | 693 | 1 (2 MiB) | 0 |
| **dracon-utilities/dracon-system** | 263 | 341 MiB | 4 | 0 | **259 (241 MiB)** |
| empire | 3920 | 311 MiB | 3920 | 0 | 0 |
| dracon-platform/.git/modules/doomtap | 506 | 224 MiB | 506 | 0 | 0 |
| dracon-platform/.git/modules/endless-td | 482 | 193 MiB | 480 | 1 (0) | 0 |
| dracon-platform/.git/modules/capture-anime-girls | 326 | 192 MiB | 326 | 0 | 0 |
| pi-plugins/pi-goal-list-loop-audit | 93 | 25 MiB | 93 | 0 | 0 |
| db-gateway | 202 | 12 MiB | 57 | 145 (8 MiB) | 0 |
| dracon-utilities | 78 | 5 MiB | 78 | 0 | 0 |
| dracon-utilities/dracon-sync | 63 | 34 MiB | 61 | 0 | 0 |
| opencode-plugins | 44 | 1 MiB | 43 | 1 (0) | 0 |
| pi-plugins | 92 | 3 MiB | 92 | 0 | 0 |
| terhub | 28 | 1 MiB | 28 | 0 | 0 |
| **TOTAL** | **19,707** | **72.2 GiB** | **19,160 (70.5 GiB)** | **240 (1.1 GiB)** | **279 (387 MiB)** |

A sample of 5 spools per large repo was lock-tested with
`flock -n reclaim-owner.lock` — **all free**, i.e. no live producer holds them.

### Are they stale?

- **Proven tier (97%):** yes — every byte is already reachable from a live ref.
  Deleting loses nothing. Age floor is met anyway (only 28 spools are <1h old).
- **Unreach tier (1.5%):** yes, *today* — the objects exist in the main store,
  so the spool is a duplicate. Caveat: they are unreachable, so a future
  `git gc --prune=now` would drop the main-store copy too. Delete is safe for
  the spool file, but if you want that content you should rescue it, not rely
  on the store.
- **UNIQUE tier (0.5%, 279 dirs):** **NO — do not blind-delete.** These hold
  objects that exist *nowhere else* (`git cat-file -e` fails). Draining them
  removes the only copy. Concentrated in `dracon-system` (241 MiB) and
  `web-games-polis` (110 MiB).

---

## Tasklist

### Phase 0 — safety rails (do first, ~5 min)

- [ ] **0.1** Snapshot the current audit as the before-state:
      `cp /tmp/audit/fleet_audit.json ~/dracon/backups/spool-audit-20261106.json`
- [ ] **0.2** Confirm root filesystem headroom and record it:
      `df -h /` (currently 826G/907G, 35G free — **do not start at 100%**)
- [ ] **0.3** Pause the daemon with the sanctioned wrapper, NOT `systemctl stop`:
      `dracon-sync pause` (1h hard TTL, 30m watchdog auto-resume)
- [ ] **0.4** Verify the pause took: `ls ~/.dracon/freeze/` and
      `dracon-sync health`

### Phase 1 — rescue the UNIQUE tier (~15 min, non-destructive)

- [ ] **1.1** For each of the 279 UNIQUE spools, bundle its retained pack
      before anything is deleted:
      `git --git-dir=<gitdir> bundle create <backup>/<repo>-<spool>.bundle --all`
      is NOT correct here (the objects are unreachable). Instead copy the
      `.pack`/`.idx` pair verbatim into
      `~/dracon/backups/spool-unique-20261106/<repo>/<spool>/`.
- [ ] **1.2** Priority order: `dracon-utilities/dracon-system` (259 dirs),
      `dracon-platform/.git/modules/web-games-polis` (2 dirs), then
      `dracon-platform` (18 dirs).
- [ ] **1.3** Verify each copied pack opens:
      `git verify-pack -v <copy>/retained-*.idx | tail -1` must say `ok`.
- [ ] **1.4** Record the rescued object ids in the backup dir
      (`objects.txt` per repo) so they can be `git index-pack`-ed back into
      any repo later.

### Phase 2 — delete the proven tier (~70.5 GiB reclaimed)

- [ ] **2.1** Re-run the audit immediately before deleting (daemon is paused,
      but verify no new spools appeared):
      `nice -n 15 python3 /tmp/audit/fleet_audit.py`
- [ ] **2.2** Delete only spools the audit classified `proven`, oldest first.
      Reuse the same classifier — **do not** use an age-only `find -delete`.
- [ ] **2.3** Start with the two giants to prove the win early:
      `ai-auto-writer` (2248 dirs / 29.5 GiB) and `ai-auto-video`
      (654 dirs / 24.2 GiB).
- [ ] **2.4** Then the remaining repos in the table above.
- [ ] **2.5** After each repo, `git count-objects -v` + `df -h /` to confirm
      the expected reclaim and that nothing else grew.

### Phase 3 — delete the unreachable tier (~1.1 GiB)

- [ ] **3.1** Only if Phase 1 rescue is complete (or the content is judged
      worthless). These are duplicates today but unreachable, so the main-store
      copy is gc-fragile.
- [ ] **3.2** Largest: `ai-auto-video` 415 MiB, `ai-auto-writer` 283 MiB,
      `web-games-polis` 280 MiB, `dracon-platform` 109 MiB.

### Phase 4 — verify the reclaim

- [ ] **4.1** `df -h /` — expect ~753G used (~87%), up from 35G → ~108G free
- [ ] **4.2** `find ~/Dev -maxdepth 5 -type d -name 'dracon-staging-*' | wc -l`
      — expect ~0-300 (only the 279 UNIQUE + 28 young if retained)
- [ ] **4.3** `du -sh` the four biggest `.git` dirs; confirm the drop
- [ ] **4.4** Resume: `dracon-sync resume`, then `dracon-sync health`

### Phase 5 — install the missing guard (prevents a repeat of cause #2)

- [ ] **5.1** `dracon-system-guard.service` is **inactive** and
      `dracon-system-guard-watchdog.timer` is **not-found** on this host —
      nothing capped or reniced the 37 GiB python. Install and enable the
      watchdog (shipped in-repo since audit M8; `install.sh` wires it up).
- [ ] **5.2** Confirm `auto_renice_on_memory` and `bias_oom_on_pressure` are
      true in `~/.dracon/utilities/system/dracon-system.toml`.
- [ ] **5.3** Confirm the guard has `CAP_SYS_NICE` (AGENTS.md 2026-08-10: an
      unprivileged guard can lower priority but not restore it, and will
      silently leave processes untouched).
- [ ] **5.4** Verify: `systemctl status dracon-system-guard.service` active,
      `systemctl list-timers 'dracon-*'` shows the guard watchdog.

### Phase 6 — file the daemon defect (root cause; cleanup alone is a treadmill)

- [ ] **6.1** New design doc:
      `dracon-utilities/docs/design/epoch-spool-reaper-never-reclaims-2026-11-06.md`
      capturing: 16308 preserved / 0 removed, the reason histogram, and this
      audit's tiers.
- [ ] **6.2** Decide the `only_known_files()` policy. It currently treats an
      unrecognised non-empty file as "unknown payload" and preserves the spool.
      For the 11,524 `unknown non-empty payload` cases the object set was
      independently proven published, so the file content is diagnostic, not
      payload. Proposal: when every object in the retained pack is reachable
      from a published ref, the spool's own file payload is redundant by
      definition and the whole-spool veto is unnecessary.
- [ ] **6.3** Decide the `LOSSLESS_RECLAIM_QUALIFIED` hold. It is a deliberate
      safety gate; do NOT flip it to `true` casually. The redundancy route
      (`reap_redundant_spools`, added 2026-11-06) was meant to be the safe
      alternative — 6.2 is what makes it actually fire.
- [ ] **6.4** Add a tripwire: if `preserved > removed` for N consecutive
      passes and the spool count grows, log a WARN. Today this leaked 72 GiB
      and 19.7k directories in complete silence until the host fell over.
- [ ] **6.5** `cargo test --workspace --locked`, `cargo build --release
      --locked`, `cargo clippy --workspace --locked -- -D warnings`,
      `cargo deny check` (AGENTS.md test discipline).
- [ ] **6.6** Also fix: `dracon-sync repair spools --apply` reports the
      inventory but removes nothing (both routes decline), which reads as
      success. It should say so explicitly.

### Phase 7 — operator hygiene (cause #3)

- [ ] **7.1** 35 resident pi processes = 5.2 GiB + 34 `systemd-inhibit`
      sleep-blockers. Prune idle sessions.
- [ ] **7.2** 83 chrome/chromium procs = 2.6 GiB. Restart the browser.
- [ ] **7.3** Ban ad-hoc herd scripts over `.git` from inside pi sessions.
      The one that caused this read 1,637 spool indexes against 65,473 commits
      and built a dict of every tree — 37 GiB VmPeak, single-threaded, orphaned
      when its parent died. That kind of scan belongs in the daemon or a
      bounded CLI.
- [ ] **7.4** Consider a swap headroom review: 124 GiB swap (60.9 G zram +
      64 G file at `/mnt/data`) with 65 GiB used means the host routinely
      exists in swap. Note `/mnt/data` is only 68% used (136 G free) so the
      swap file has room, but zram costs real RAM for its metadata.

---

## Do-not-do list

- **Do not** `find .git -name 'dracon-staging-*' -delete`. Age is not evidence;
  the UNIQUE tier would lose its only copy.
- **Do not** `rm -rf` inside any `.git` while the daemon is unpaused — a live
  epoch's spool would be removed under a running producer.
- **Do not** use `systemctl --user stop dracon-sync.service` (AGENTS.md: banned
  for remediation; the unit is masked anyway). Use `dracon-sync pause` or
  `dracon-sync maintenance --`.
- **Do not** flip `LOSSLESS_RECLAIM_QUALIFIED` to `true` to make the numbers
  move. Fix `only_known_files()` instead (6.2).
- **Do not** delete the canonical nested checkouts (`dracon-sync/`,
  `dracon-system/`, `dracon-warden/`) — AGENTS.md forbids it.

## Expected end state

| Metric | Now | After |
|---|---|---|
| `/` used | 826G (96%) | ~753G (87%) |
| Spool dirs | 19,707 | ~300 (UNIQUE + young) |
| Spool bytes | 72.2 GiB | ~0.4 GiB |
| Guard service | inactive, no watchdog | active + watchdog + renice/OOM bias |
| Spool leak | unbounded, silent | tripwire + working redundancy reaper |

## Audit tool

`/tmp/audit/fleet_audit.py` — bounded-memory fleet auditor. Reuse it for
Phase 2.2 instead of ad-hoc shell: it resolves reachability with
`--git-dir` (correct for `.git/modules/*` and nested repos — an earlier
version compared against the parent repo and produced wildly wrong numbers),
reads packs with `git show-index`, and classifies every object into
proven / unreach / UNIQUE. Persist it to the repo before this work is
considered done.
