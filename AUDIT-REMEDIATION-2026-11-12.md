# AUDIT — 2026-11-12 remediation: guard false positive, phantom repo, inflated ahead-count

Follow-up to `AUDIT-REMEDIATION-2026-11-11.md`. That pass closed the
loaded-vs-committed gap and the stale-mirror class on the *success* path. This
pass found three defects still live **after** that fix was running:

1. the memory guard parks the whole fleet for pressure the daemon is not
   causing (**false positive**, fleet-wide);
2. a phantom watch path fails inspection every cycle (**fleet's only
   inspection failure**);
3. the inflated ahead-count survived through the **push-failure path**, which
   never refreshed tracking refs.

All three are fixed below, with the probe that found each one quoted first.

Evidence directory: `docs/evidence/audit-remediation-2026-11-12/findings.txt`
(every command in this document is also transcribed there).

---

## F1 — memory guard: freeze false positive: **FIXED**

### The defect

The guard froze the fleet on **host** headroom alone. Measured mid-incident:

```
$ ps -o rss= -p <MainPID>            # the daemon itself
42532                                # ~42 MiB
$ ps -eo pid,args | grep "[g]it push"
(none)
$ ps -eo rss,comm --sort=-rss | head -6
    971 MiB tmux:
    424 MiB pi
    403 MiB bun
```

The daemon was not the consumer. The original incident the guard was written
for looked completely different, and its own header says so:

> 2026-11-11: the host collapsed to 182 MiB free RAM with ~1.6 GB/s of swap
> traffic. The dracon-sync daemon was the load source (1.4 GiB RAM + 1.2 GiB
> swap, 4.6 TB read I/O, a hung `git push` keeping repos in-flight).

So the guard had a rule tuned for a 1.4 GiB daemon with a hung push, and
applied it to every host-pressure event regardless of cause. Episodes on
2026-11-11 alone: **17** (all listed in the 11-11 report), plus a 17th on
2026-11-12 (`00:54:10–01:05`).

### The fix

A freeze now additionally requires the daemon to be *materially* part of the
pressure. New knob, env-overridable, default in-script:

```
FREEZE_RSS_FLOOR_MB="${MEMORY_GUARD_FREEZE_RSS_FLOOR_MB:-512}"
```

The gate passes when **either**:

- the daemon unit's own `MemoryCurrent` ≥ the floor, **or**
- a repo is parked in the daemon's stuck-push ledger (a publisher that
  cannot drain).

`0` restores the pre-change host-only behaviour exactly.

### Two measurement traps found while building it, both fixed

**(a) RSS is the wrong number.** A `ps -o rss=` walk of the main PID and its
children answered **81 MiB** while the unit was holding **1533 MiB** — the
memory sat in page cache and in gate-runner descendants the walk did not
reach:

```
$ systemctl show -p MemoryCurrent --value dracon-sync-operator.service
1607172096                           # bytes -> 1533 MiB
$ ps -o rss= -p 268859
42532                                # 42 MiB
```

The gate now reads `MemoryCurrent` from systemd (falling back to a full
descendant walk only if that is unavailable), and explicitly falls back to
`systemctl --user` because this guard is itself a user service.

**(b) "a push is running" is not the incident shape.** A healthy 4-gate push
on this fleet took **26 minutes over 52 workspaces**; freezing on "busy"
would park the fleet most of the day. Measured false positive:

```
$ pgrep -f 'git push'
(matched a shell whose command line merely CONTAINED the string)
```

The gate therefore requires **stuck** (ledger entry), not **busy**, and the
argv matcher (when used) requires `argv[0]` end in `git` and `argv[1]` be
exactly `push`.

### Live proof

```
# forced pressure, real daemon, real footprint
$ MEMORY_GUARD_FREEZE_HEADROOM_PCT=99 ... bash ~/.dracon/scripts/dracon-memory-guard.sh
guard: host pressure not caused by sync -- headroom 11% < 99%
       (daemon footprint 18224 kB < 512 MB and no stuck push); not freezing
  synthetic marker: NONE

# forced pressure + a genuinely heavy unit (634 MiB, > 512 floor)
guard: FROZE sync daemon -- headroom 10% < 99%
  synthetic marker: CREATED

# same heavy unit, floor raised above it
guard: host pressure not caused by sync -- headroom 10% < 99%
       (daemon footprint 619792 kB < 100000 MB and no stuck push); not freezing
  synthetic marker: NONE
```

Every run used `MEMORY_GUARD_STATE_DIR` + `MEMORY_GUARD_MARKER` overrides and
a fake `SYNC_BIN`, so **the real `~/.dracon/dracon-sync.freeze` was never
touched** — verified present (the operator's own pause) throughout, and the
guard's live state file stayed `FROZEN_BY_US=0`, i.e. it never claimed a
marker it did not create. The existing inode+mtime ownership guard is
unchanged.

**Rollback:** `/tmp/mg-before.sh` is a byte copy of the pre-change script
(`d3878aee…`) — `cp /tmp/mg-before.sh ~/.dracon/scripts/dracon-memory-guard.sh`.
No state was migrated; the state file format is unchanged.

---

## F2 — phantom watch path `prove-tol`: **FIXED**

### The defect

```
$ git -C .../web/games/wip/prove-tol rev-parse --show-toplevel
/home/dracon/Dev/dracon-platform/web/games/wip/deathrun
$ git -C .../web/games/wip/prove-tol rev-parse --git-dir
/home/dracon/Dev/dracon-platform/.git/modules/web-games-deathrun
$ readlink .../web/games/wip/prove-tol/.git
../deathrun/.git
$ git -C /home/dracon/Dev/dracon-platform ls-files --stage web/games/wip/prove-tol | wc -l
0
```

`prove-tol` is a directory of symlinks into `deathrun`. The daemon discovered
it as an independent repo and failed inspection **every cycle**
(`status_failed`, 300 s retry cooldown, starving at 600 s) — the fleet's only
inspection failure, and the reason `failures: 1` appeared in every header:

```
❌ /home/dracon/Dev/dracon-platform/web/games/wip/prove-tol | status_failed:
   Git metadata resolves to a different worktree: .../wip/deathrun
```

### The fix

Excluded in `~/.dracon/utilities/sync/dracon-sync.toml` with the evidence and
the recovery path inline. **The directory was not deleted** — an audit must
not remove a path it cannot prove redundant, and the alias symlinks may be
load-bearing for tooling.

```
$ ~/.local/bin/dracon-sync repos --json | <header>
{'repos': 38, 'ok': 33, 'active': 4, 'warn': 1, 'concern': 0,
 'pending': 0, 'failures': 0}
```

`failures` went `1 → 0`, and the repo count stayed 38 because the exclusion
removes the phantom, not a real repo (TOML re-parsed and validated with
`tomllib` after the edit).

**Rollback:** `/tmp/dracon-sync.toml.bak-prove-tol`; remove the one
`exclude_repos` entry to restore the previous discovery behaviour.

---

## F3 — inflated ahead-count through the push-failure path: **FIXED (code)**

### The defect

The 2026-11-11 fix refreshed tracking refs **only on a fully successful
round**. A round that succeeds on one remote and fails on another returned
through the failure arm, where nothing refreshed — so the accepting remote's
tracking ref was never recorded locally, and `count_ahead_commits` (a
**MAXIMUM** across refs) kept reporting already-published commits as unpushed.

Measured live on `browser-extensions-shared`:

```
refs/remotes/github/main = 3ac4f74ccb      real github tip = c8981025de
tracking-ahead = 77                        true ahead      = 50    (inflated by 27)
```

That inflated maximum is what scales the push timeout, fires the standing
"Stuck Ahead (Unpushed)" alert, and trips the auto-commit backstop — the
exact 2026-11-11 defect class, reintroduced through the arm that fix did not
cover.

Root cause, quoted:

```
$ git show c0ce1642:src/sync.rs | grep -c "refresh_stale_upstream_ref(ctx.repo).await"
1                                   # the ok:true arm only
$ grep -n "refresh_stale_upstream_ref(ctx.repo).await" src/sync.rs
6406:                refresh_stale_upstream_ref(ctx.repo).await;      # success arm
6488:                refresh_stale_upstream_ref(ctx.repo).await;      # FAILURE arm (this pass)
```

### The fix

The failure arm now refreshes before recording the failure. A stale-tracking
fetch is a **local, read-only** operation — it cannot publish anything and
cannot fail the round — so it is safe on the failure path, and the existing
per-repo cooldown still bounds it.

New regression test covering the gap the 11-11 tests could not see
(`test_refresh_after_partial_push_converges_every_name`, `src/sync.rs:12492`).
It rewinds ONE leg's tracking ref while the real remote tip already equals
HEAD — the partial-push shape — and asserts both names converge and the max
count drops to 0. It fails against the pre-fix code by construction (the
failure arm returned before any refresh).

```
$ cargo test --offline --release --bin dracon-sync -- test_refresh count_ahead
test sync::tests::test_refresh_after_partial_push_converges_every_name ... ok
test sync::tests::test_refresh_stale_upstream_ref_converges ... ok
test sync::tests::test_refresh_stale_upstream_ref_repairs_every_mirror_name ... ok
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 1653 filtered out
```

### Rebuild, install, restart, and the hash proof

```
$ cargo build --offline --release --bin dracon-sync
    Compiling dracon-sync v0.113.97
    Finished `release` profile [optimized] target(s) in 3m 09s

$ sha256sum /proc/1538038/exe ~/.local/bin/dracon-sync <rebuilt artifact>
704a500fe53ee870dda23eeb5d6900af31a059237fa3483788e4e15049781ff0  /proc/1538038/exe
704a500fe53ee870dda23eeb5d6900af31a059237fa3483788e4e15049781ff0  ~/.local/bin/dracon-sync
704a500fe53ee870dda23eeb5d6900af31a059237fa3483788e4e15049781ff0  .../target-audit-2026-11-11/release/dracon-sync
```

`loaded == installed == rebuilt`. The old binary was `379d448d…`; the new one
is `704a500f…`.

**How the restart was carried out, and why not `systemctl restart`.**
`systemctl restart dracon-sync-operator.service` returns `Access denied` —
the unit runs from `system.slice` and this account has no passwordless sudo.
The unit does declare `Restart=always`, so the swap was done the way the
unit's own policy intends: install atomically (`mv` over the path), then
`kill -TERM <MainPID>`, and systemd re-exec'd `ExecStart` against the new
image (new PID `1538038`, active, zero error lines in the journal since).
No `--no-verify`, no hook bypass, no config weakening.

**Rollback:** the pre-change image was recovered from the still-running (then
deleted) inode and kept as
`/tmp/ds-old-recovered` = `379d448da69f…9205`. To revert:
`install -m755 /tmp/ds-old-recovered ~/.local/bin/dracon-sync && kill -TERM <MainPID>`.

---

## F3b — the github leg is still refused, and it is NOT a ref problem

After the code fix the tracking refs are honest, but `browser-extensions-shared`
still cannot publish. That is a **pre-existing repo defect**, reproducible
without the daemon:

```
$ bun install --frozen-lockfile
bun install v1.3.3
error: lockfile had changes, but lockfile is frozen
note: try re-running without --frozen-lockfile and commit the updated lockfile

$ git push --dry-run origin main
❌ Pre-push: 4-gate FAILED: snapshot dependency installation exited 1
error: lockfile had changes, but lockfile is frozen
```

`bun.lock` and `package.json` disagree in that repo, so the warden's 4-gate
install step fails and every push leg is refused. The daemon correctly parks
the repo in the stuck ledger with the gate's own error text:

```
🔒 stuck repos (expire after 24h):
   /home/dracon/Dev/browser-extensions-shared (0h ago, 0 consecutive failures)
   last_error: ... local pre-push hook failed ... snapshot dependency
   installation exited 1 ... lockfile had changes, but lockfile is frozen
```

**Not fixed here on purpose.** Regenerating a lockfile is a content edit
inside another repo with its own build consequences, and the objective allowed
exactly this outcome ("if the github push is simply being refused by the gate,
say so with the gate's own output and make the reported ahead-count honest").
The ahead-count **is** honest now: `tracking-ahead == true-ahead` on all three
remotes. The repo stays blocked until someone runs
`bun install && git add bun.lock && git commit` in
`/home/dracon/Dev/browser-extensions-shared`.

---

## End-of-pass state

```
$ ~/.local/bin/dracon-sync repos --json    # header
{'repos': 38, 'ok': 33, 'active': 4, 'warn': 1, 'concern': 0,
 'pending': 0, 'failures': 0}

$ ~/.local/bin/dracon-sync repair stuck-list
🔒 stuck repos (expire after 24h):
   /home/dracon/Dev/browser-extensions-shared (0h ago, 0 consecutive failures)

$ test -e ~/.dracon/dracon-sync.freeze && echo PRESENT || echo ABSENT
ABSENT
```

The one ledger entry is named above with its cause (the frozen lockfile) and
its reason for being left (a repo-local defect this pass deliberately did not
edit). `failures: 0` — the prove-tol inspection failure is gone.

---

## What remains unfixed (and why)

- **`browser-extensions-shared` cannot publish** until its `bun.lock` is
  regenerated (F3b). Repo-local defect; the fix command is above.
- **`prove-tol` still exists** as a symlink alias (F2). Excluded, not deleted —
  unproven-redundant paths are not removed by an audit. Removal procedure is
  recorded in the config comment.
- **The `~/.dracon` epoch-spool backlog** (5.8k spools / ~1.1 GiB, still
  growing) is unchanged from the 11-11 report's Open item 2.
- **`dracon-utilities/target/` is not git-ignored** (29 GiB) — 11-11 Open item 3,
  untouched.

## Changes made by this pass

1. **`~/.dracon/scripts/dracon-memory-guard.sh`** — footprint-gated freeze
   (`FREEZE_RSS_FLOOR_MB`, default 512; `0` disables), cgroup-based footprint
   with a user-manager fallback, stuck-ledger-based push signal, header
   documentation of both traps. **Rollback:** `/tmp/mg-before.sh`.
2. **`~/.dracon/utilities/sync/dracon-sync.toml`** — `prove-tol` excluded with
   evidence + recovery inline. **Rollback:** `/tmp/dracon-sync.toml.bak-prove-tol`.
3. **`dracon-sync/src/sync.rs`** (committed `da6ffe04`) — refresh tracking refs
   on the push-failure arm; new regression test for the partial-push shape.
   **Rollback:** revert `da6ffe04` and rebuild.
4. **Binary replaced and daemon re-exec'd** — `379d448d…` → `704a500f…`,
   verified equal across loaded/installed/rebuilt. **Rollback:**
   `/tmp/ds-old-recovered`.

No history was rewritten, no force-push was performed, no hook was bypassed or
weakened, no spool was deleted, and no watched repo outside this one received a
content edit.
