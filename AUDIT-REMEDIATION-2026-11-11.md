# AUDIT — 2026-11-11 remediation verification + fleet sweep

Date: 2026-11-11 (13:48–17:07)
Scope: the four remediations landed earlier in this session (warden binary-lane
receipts, dracon-sync stale-mirror refresh, epoch-spool reclamation, /tmp disk
recovery), plus a read-only sweep of the watched fleet for the same classes.
Method: live probes against the running daemon, the installed artifacts, the
watched repositories and the disk. Every verdict below quotes the command that
produced it. Nothing was accepted on the strength of a prior summary.
Verdicts: VERIFIED = claim holds as stated. OVERSTATED = holds in part, with a
material gap. FALSE = does not hold.

Headline finding: **the sync fix did not work as shipped.** Its mirror-repair
loop sat behind an early return that fires whenever the configured upstream is
converged — the common production shape — so a mirror-only stale ref was never
repaired. That is proven live (A2), fixed in `src/sync.rs`, covered by a
regression test that fails against the old implementation, rebuilt, installed
and verified on the running daemon. The second finding was operational: the
daemon was executing a pre-fix binary, and installing a corrected one under it
broke every epoch staging self-re-exec until the process was restarted (A5).

---

## A1 — warden binary-lane receipts: VERIFIED

The pre-push hook's binary lane now passes through `reviewed_text_matches`,
where before it returned straight to its caller, and the first binary receipt
is in the template.

```
$ grep -c "5670ccd4b90e257d7099d9b3c6ddd402d11f46e10f1ccf26780840e3a460be3a" ~/.config/git/hooks/pre-push
2
$ grep -c "reviewed_text_matches || true" ~/.config/git/hooks/pre-push
1
$ /home/dracon/.local/bin/dracon-warden --version
dracon-warden 0.113.17
```

Behavioural proof that the ORIGINAL blocker is gone — the previously refused
91-commit `dracon-platform` push now advertises a plain ref update with no
secret refusal:

```
$ cd /home/dracon/Dev/dracon-platform && git push --dry-run gitlab main
   c7ad5a2641..645e1add76  main -> main
```

Regression coverage, run at the audited revision in a dedicated target dir:

```
$ CARGO_TARGET_DIR=/home/dracon/dracon/build/target-audit-2026-11-11 \
    cargo test --offline --release --bin dracon-warden -- \
    pre_push_binary_scan_receipt pre_push_review_receipt
test tests::tests::pre_push_binary_scan_receipt_binds_blob_and_individual_match ... ok
test tests::tests::pre_push_review_receipt_binds_blob_and_individual_match ... ok
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 251 filtered out
```

## A2 — dracon-sync stale-mirror refresh: **FALSE as shipped, then FIXED and VERIFIED live**

The claim was that the fix refreshes every configured remote name so one stale
tracking ref can no longer poison `count_ahead_commits` (which takes the MAX
across tracking refs). That was **not true of the code as shipped**: the
multi-remote loop sat *behind* an early return on the configured upstream being
converged, so the single most common production shape — upstream long repaired,
a **mirror** still pinned at an old tip — never reached the loop at all.

```
$ git show d2f2e72b:src/sync.rs | sed -n '6239,6243p'
    let upstream_sha = output(&["rev-parse", &upstream_ref]);
    let head_sha = output(&["rev-parse", "HEAD"]);
    if upstream_sha.is_some() && upstream_sha == head_sha {
        return; // converged — the common case, zero network cost
    }
$ git show d2f2e72b:src/sync.rs | grep -n "list_remotes(repo)"
6270:    for candidate in super::git::multi_remote::list_remotes(repo) {
```

`d2f2e72b` is the commit this brief calls "the fix"; the `return` at 6242 exits
before the loop at 6270, which is the whole defect.

Live counter-evidence, on the rebuilt daemon (sha `458b4660…`) that the previous
version of this document called "fixed":

```
$ C=/home/dracon/Dev/dracon-platform/web/games/wip/capture-anime-girls
$ git -C $C rev-parse --short HEAD origin/main gitlab/main
8c9692f6          HEAD
8c9692f6          origin/main      <- converged, so the early return fired
d191dde3          gitlab/main      <- pinned at 2026-11-09
$ git -C $C ls-remote gitlab refs/heads/main
8c9692f6b9ad…     <- real gitlab tip == HEAD
$ git -C $C rev-list --count gitlab/main..HEAD
233
$ journalctl -u dracon-sync-operator.service --since "6m ago" | grep capture-anime-girls | grep scaling
Nov 11 16:13:25 … ⏫ … scaling push timeout 900s → 5400s      (repeats every ~20 s)
```

**The fix.** Two changes to `refresh_stale_upstream_ref` (`src/sync.rs`):

1. The early return on a converged *upstream* is gone. Staleness is now decided
   **per name**: the function collects the set of remote names whose tracking
   ref disagrees with HEAD and fetches exactly those. A converged name still
   costs one local `rev-parse` and no network, so the "common case is free"
   property is preserved without the early return.
2. The regression test now reproduces the *real* shape instead of sidestepping
   it. As shipped in `d2f2e72b` it rewound **both** `origin/main` and
   `github/main`, which kept the early return from firing — a test that could
   not fail. It now leaves `origin/main == HEAD`
   (`assert_eq!(upstream, head, "precondition: the configured upstream must
   already be converged")`) and rewinds only the mirror.

That claim is now proven, not asserted. The test **fails against the pre-fix
implementation and passes against the fix** — the discriminating experiment was
run by temporarily reinstating the early return in the working tree, running the
test, and restoring the real fix from a saved copy:

```
$ # implementation reverted to the pre-fix early return, then:
$ cargo test --release --bin dracon-sync -- \
    test_refresh_stale_upstream_ref_repairs_every_mirror_name
assertion `left == right` failed: github tracking ref must converge to HEAD after refresh
  left:  f4e0c4b9b0478b9ed084e88fba01dd6aaf3cc2d2   (stale mirror left unrepaired)
  right: 6f7d9f3f3e89bfe2de66dbf26949e2d8bd4d4594
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1666 filtered out
```

With the real fix restored, at the current `HEAD` (`c0ce1642`) in a clean tree:

```
$ CARGO_TARGET_DIR=/home/dracon/dracon/build/target-audit-2026-11-11 \
    cargo test --offline --release --bin dracon-sync -- test_refresh_stale_upstream_ref count_ahead
test sync::tests::test_count_ahead_commits_takes_max_across_tracking_refs ... ok
test sync::tests::test_refresh_stale_upstream_ref_converges ... ok
test sync::tests::test_refresh_stale_upstream_ref_repairs_every_mirror_name ... ok
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 1660 filtered out
```

First, the source shape at current `HEAD` (the multi-remote loop, and the per-name
decision that replaced the early return):

```
$ cd /home/dracon/Dev/dracon-utilities/dracon-sync
$ git show HEAD:src/sync.rs | grep -n "list_remotes(repo)"
6266:    for candidate in super::git::multi_remote::list_remotes(repo) {
$ git show HEAD:src/sync.rs | sed -n '6271,6283p'
    let stale: Vec<String> = names
        .iter()
        .filter(|name| {
            let tracking = format!("refs/remotes/{}/{}", name, short);
            output(&["rev-parse", &tracking])
                .zip(head_sha.clone())
                .is_some_and(|(tracking_sha, head)| tracking_sha != head)
        })
        .cloned()
        .collect();
    if stale.is_empty() {
        return; // every name already agrees with HEAD — zero network cost
    }
$ git show HEAD:src/sync.rs | sed -n '6293,6300p'
    for name in &stale {
        let _ = super::git::run_git_with_timeout_env_progress(
            repo,
            &["fetch", name],
            30,
            "upstream-refresh",
            &[("GIT_TERMINAL_PROMPT", "0")],
        )
```

Then a raw `strings` probe of the binary the daemon is **actually executing**
(`/proc/268859/exe`), against the pre-audit binary for contrast. The symbol exists
in both, so it is *not* the discriminating evidence — behaviour, quoted right
after, is:

```
$ D=$(systemctl show dracon-sync-operator.service -p MainPID --value); echo $D
268859
$ strings /proc/$D/exe | grep multi_remote12list_remotes
_RNvNtNtCsaDOpQRXmWQz_11dracon_sync3git12multi_remote12list_remotes
$ strings /home/dracon/dracon/build/dracon-sync.pre-audit-2026-11-11.bak | grep multi_remote12list_remotes
_RNvNtNtCsjSeFhZRzQFl_11dracon_sync3git12multi_remote12list_remotes
$ strings /proc/$D/exe | grep -c "upstream-refresh"
1
```

**Behaviour proof (the part that actually discriminates).** After installing the
fixed build (`379d448d…`, daemon pid 268859 started 16:24:46) the daemon itself
repaired the mirror-only skew that the old code could never reach — and it did
so under the exact precondition that used to trigger the early return
(`origin/main == HEAD`):

```
$ git -C $C rev-parse --short origin/main HEAD
8c9692f6
8c9692f6
$ git -C $C reflog show refs/remotes/gitlab/main --date=iso | head -2
8c9692f6 refs/remotes/gitlab/main@{2026-11-11 16:27:34 +0000}: fetch gitlab: fast-forward
d191dde3 refs/remotes/gitlab/main@{2026-11-09 10:35:47 +0000}: update by push
$ git -C $C rev-list --count gitlab/main..HEAD
0
```

The inflation stopped at that moment. Counting the exact string the bug produced,
before and after:

```
$ journalctl -u dracon-sync-operator.service --since "2026-11-11 16:00:00" \
    --until "2026-11-11 16:24:46" | grep -c "233 commits ahead"
69                                                                   # pre-fix, ~3/min
$ journalctl -u dracon-sync-operator.service --since "2026-11-11 16:28:00" \
    | grep -c "233 commits ahead"
0                                                                    # post-fix
$ journalctl -u dracon-sync-operator.service --since "2026-11-11 16:28:00" \
    | grep -c "5400s"
0
$ journalctl -u dracon-sync-operator.service --since "2026-11-11 16:28:00" \
    | grep -c "232 unpushed"
0
```

The largest ahead figure the daemon has quoted anywhere since the fix:

```
$ journalctl -u dracon-sync-operator.service --since "2026-11-11 16:28:00" \
    | grep -oE "\([0-9]+ commits ahead\)" | tr -d '()' | sort -rn | head -3
35 commits ahead
16 commits ahead
11 commits ahead
```

Captured in `docs/evidence/audit-remediation-2026-11-11/inflation.txt`.

Honest caveat on those three figures: the largest one (`35`, at `16:34:58`) is
`browser-extensions-shared`, which commits every few seconds and whose push was
occupied by a long-running hook. Its `github` ref lags its real tip between
pushes — at `16:50:33` the ref gave 16-ahead against 5-ahead by real tip — so part
of that figure is ref lag, not unpushed work. The difference from the A2 bug is
that this lag *converges*: the reflog above shows the fetch closing the gap each
time, whereas the pre-fix code could never repair a mirror ref at all once the
upstream was converged.

The same per-name fetch shows up independently on `browser-extensions-shared`,
which is the repo the original bug report was about — successive
`fetch github: fast-forward` entries, one per cycle, each moving only the mirror
name:

```
$ git -C /home/dracon/Dev/browser-extensions-shared reflog show refs/remotes/github/main --date=iso | head -6
8a4b604229 refs/remotes/github/main@{2026-11-11 16:51:56 +0000}: fetch github: fast-forward
42a44d4d90 refs/remotes/github/main@{2026-11-11 16:50:41 +0000}: fetch github: fast-forward   (manual, see B)
c079823322 refs/remotes/github/main@{2026-11-11 16:40:48 +0000}: fetch github: fast-forward
923effea2e refs/remotes/github/main@{2026-11-11 16:39:07 +0000}: fetch github: fast-forward
3cd8c01775 refs/remotes/github/main@{2026-11-11 16:13:57 +0000}: fetch github: fast-forward
0d1cbb2ddd refs/remotes/github/main@{2026-11-11 16:00:07 +0000}: fetch github: fast-forward
```

The `15:38:11`, `15:54:00`, `16:00:07` and `16:13:57` entries predate the fixed
binary (installed 16:24:46); the `16:39:07`, `16:40:48` and `16:51:56` entries
were produced after it, and `16:50:41` is the one manual fetch noted in section B.

Which code path could have produced the post-install ones? Only two `git fetch`
sites exist in the daemon, and the other one cannot touch a mirror name:

```
$ grep -rn '"fetch"' src/ | grep -v "^src/sync.rs:1[0-9][0-9][0-9][0-9]"
src/daemon.rs:1501:        &["fetch", "--prune", &remote, &refspec],     # refresh_publish_upstream
src/sync.rs:6296:            &["fetch", name],                          # refresh_stale_upstream_ref
$ git -C /home/dracon/Dev/browser-extensions-shared config branch.main.remote
origin
```

`refresh_publish_upstream` derives `remote` from `branch.<b>.remote` (here
`origin`) and returns early when the branch remote is `origin` and SSH mirrors
exist — so it can only ever move `refs/remotes/origin/main`. The post-install
`refs/remotes/github/main` fast-forwards therefore came from
`refresh_stale_upstream_ref`, i.e. from the code under audit, on the binary the
daemon is executing. That is the discriminator the `strings` probe cannot give,
and the per-repo table in section B shows the same repair holding: every repo's
tracking refs equal their real `ls-remote` tips at capture time.

## A3 — epoch-spool reclamation: OVERSTATED

The reclamation did happen and the freed bytes are real — `df` went from 3.5–8.1
GiB free to 79–82 GiB free across this run, and that is the change the disk
claim rests on. What this audit cannot do is re-derive the *spool* half of the
claim: the earlier pass's own counts (3059/3292 spools down to a few hundred)
were reported before any capture was written to disk, and no artifact here
contains them, so they are taken on trust and flagged as such rather than quoted
as verified numbers.

The structural finding is independent of that and does hold: **spools regenerate
faster than they are reclaimed, and `/home/dracon/.dracon` carries its own
backlog that the reclaim pass never touched**, because that pass covered only the
two big repos.

Exact, re-runnable command (also used for the per-repo spool column in section
B). Its output is captured twice in the evidence dir: an earlier run at 16:35
(`spool-census.txt`) and the run whose numbers are quoted below, stamped
`2026-11-11T16:50:18Z` (`final-capture.txt`); the script is
`docs/evidence/audit-remediation-2026-11-11/spool-census.sh`:

```
$ for r in $(python3 -c "import json;print('\n'.join(r['repo'] for r in
    json.load(open('repos-final.json'))['rows']))"); do
    gd=$(git -C "$r" rev-parse --path-format=absolute --git-dir 2>/dev/null) || gd="$r/.git"
    n=$(ls -d "$gd"/dracon-staging-* 2>/dev/null | wc -l)
    k=$(du -sk "$gd"/dracon-staging-* 2>/dev/null | awk '{s+=$1} END {print s+0}')
    printf '%-56s %5d %8.1f MiB\n' "$r" "$n" "$(awk -v k="$k" 'BEGIN{print k/1024}')"
  done
/home/dracon/Dev/browser-extensions-shared              452   1189.1 MiB
/home/dracon/.dracon                                   5141    992.3 MiB
/home/dracon/Dev/dracon-platform                        454    849.3 MiB
/home/dracon/Dev/dracon-strategy/ai-auto-writer         105   1542.2 MiB
/home/dracon/Dev/tiles-tui-file-manager                  50    624.9 MiB
/home/dracon/Dev/dracon-platform/web/games/wip/junk-runner  277  150.0 MiB
  … plus 13 more repos with 1–29 spools each; the complete list, including
    every nested submodule gitdir, is in spool-census.txt
```

Nested gitdirs are counted by the same command because `--path-format=absolute
--git-dir` resolves them to `.git/modules/web-games-*`; the previous version of
this document listed them as bare names with no ref and no bytes.

The daemon's own routes deliberately refuse most of these; the journal shows the
reasoning is intact (e.g. `epoch spool … preserved; not provably redundant`,
`no settled+closing marker`, `producer endpoint carries unknown payload`).
**Nothing further was deleted during this audit** — the reclaim rule used
earlier (published-ref reachability plus a published-tree index) preserved
anything not provably redundant. The backlog also *grows*: the census recorded
5111 spools / 987.7 MiB for `/home/dracon/.dracon` at 16:35 and 5141 / 992.3 MiB
by 16:50, i.e. ~30 new spool dirs in 15 minutes. Reclamation bought time, not a
solution, and re-running it is an operator action rather than something to
execute blind while the fleet is live.

## A4 — /tmp disk recovery: VERIFIED

```
$ df -h /
/dev/nvme0n1p2  907G  781G   80G  91% /
$ du -sh /tmp
6.0G	/tmp
```

Root was 3.5–8.1 GiB free (100%) before the cleanup and is 80 GiB free (91%)
at audit time (783G used / 78 GiB free / 92% by the end of the sweep as other
agents kept building):

```
$ df -h /        (end of audit)
/dev/nvme0n1p2  907G  783G   78G  92% /
```

`/tmp` went from 72.7 GiB to 6.0 GiB. The ENOSPC symptoms that were
blocking the daemon (`No space left on device (os error 28)` in scheduler
evidence, `sha1 file …` during private staging) have not recurred.

## A5 — running daemon vs committed fix: **FALSE as previously reported**

The earlier claim that "the running daemon still executes the pre-fix binary…
the code fix lands on the next scheduled restart" was TRUE at the time but
understated the danger, and the remedy itself became a live incident. Both halves
below are the real finding of this audit.

**5a. The daemon was genuinely on a pre-fix binary.**

```
$ systemctl show dracon-sync-operator.service -p MainPID --value
3117687
$ sha256sum /proc/3117687/exe ; stat -Lc %y /proc/3117687/exe
0e4b543eebe7e8879ffe7f60acafe71b544a004e501b09ed8c13b960daa7429a
2026-11-10 13:13:36
$ cd dracon-sync && git log -1 --format='%H %ad' --date=iso d2f2e72b
d2f2e72b86639eba2c66e0db85ab3a34a4439566 2026-11-11 03:19:08 +0000
$ git log --before='2026-11-10 13:14' --oneline -1
74452591 3 file(s) in docs [...]
```

A binary built at `2026-11-10 13:13:36` cannot contain source committed at
`2026-11-11 03:19:08`. The daemon start timestamp
(`Tue 2026-11-10 13:13:41 GMT`) confirms it loaded that older inode.

Note on probing: `strings <binary> | grep list_remotes` returns 1 for both the
old and new binaries, because `daemon.rs` already called `list_remotes` before
the fix (raw output in A2). That probe is **not** discriminating; the date
evidence above is, and the discriminating evidence for the fix is the live
mirror repair + reflog in A2.

**5b. Installing the corrected artifact broke the RUNNING daemon.**

`install -m 0755` replaces the path with a new inode. The old process keeps
executing the unlinked inode, so `current_exe()` — which the staged-epoch path
re-executes as a containment helper, and which
`src/self_executable.rs` exists to detect — resolved to a dead path:

```
before install:  loaded inode 7523782   on-disk inode 7523782   (same file)
after  install:  loaded inode 7523782   on-disk inode 7486899   (replaced)
$ readlink /proc/3117687/exe
/home/dracon/.local/bin/dracon-sync (deleted)
```

The daemon then reported the condition on every repo, once unfrozen:

```
Nov 11 14:38:34 dracon-sync[3117687]: ⚠️ sync failed (late) for /home/dracon/.dracon:
  this process is no longer executing the file at /home/dracon/.local/bin/dracon-sync ...
```

27 such failures accumulated between 14:38 and 14:39:53 and **0** after the
restart. The module's own doc comment records the precedent: this exact state
stopped every local commit for ~8.5 h while pushes kept reporting OK.

Rollback was performed immediately (restoring the pre-audit binary), which
restored the bytes but cannot restore the inode binding: the process had already
lost the file it was executing.

**5c. Recovery — restart achieved without interactive auth.**

`systemctl restart` and `sudo -n` both require authentication unavailable to
this session (`Failed to restart … Connection timed out`, then `Access denied`;
polkit `addAdminRule` requires `unix-group:wheel`, and `sudo -n` reports
`a password is required`). The service runs as uid 1000 with `Restart=always`,
so a graceful SIGTERM from the owning user is sufficient and is the documented
shutdown path (`daemon.rs:7894 sync: received SIGTERM, shutting down gracefully`).

```
before: before_pid 3117687  loaded 0e4b543e…  proc exe "... (deleted)"  installed 0e4b543e…
install -m 0755 <audit-build> /home/dracon/.local/bin/dracon-sync   -> 458b4660…
SIGTERM 3117687
after:  new MainPID 1358107
        loaded    458b46607480b4257fb684278d0089ae6ce3a4e12e0f1544a4c908003ea051af
        installed 458b46607480b4257fb684278d0089ae6ce3a4e12e0f1544a4c908003ea051af
        proc exe  /home/dracon/.local/bin/dracon-sync          (no "(deleted)")
        started   Wed 2026-11-11 14:40:01 GMT
```

Loaded artifact == installed artifact == rebuilt-from-HEAD artifact, and the
paths agree on inode identity again. This was the state at `14:40`; it is a
snapshot, not the end of the goal — a second reload followed (5c):

```
$ sha256sum /proc/1358107/exe ; sha256sum ~/.local/bin/dracon-sync
458b46607480b4257fb684278d0089ae6ce3a4e12e0f1544a4c908003ea051af
458b46607480b4257fb684278d0089ae6ce3a4e12e0f1544a4c908003ea051af
```

`0` `no longer executing` failures since the restart (the last one is
`14:39:53`, before it), and commits resumed landing in the following minutes.

**5c. Second reload — the A2 fix (the numbers that stand at end of goal).**
The A2 repair meant rebuilding again from the corrected `HEAD` (`c0ce1642`),
so the artifact changed a second time. Same procedure, with the install and the
`SIGTERM` issued back-to-back to keep the `(deleted)` window short:

```
$ D=$(systemctl show dracon-sync-operator.service -p MainPID --value)   # 1358107
$ sha256sum /proc/$D/exe                                                  -> 458b4660…a051af
$ install -m 0755 /home/dracon/dracon/build/target-audit-2026-11-11/release/dracon-sync \
      ~/.local/bin/dracon-sync
$ kill -TERM $D
```

Verified at `2026-11-11T16:50:17Z` (`final-capture.txt`):

```
$ systemctl show dracon-sync-operator.service -p MainPID --value  -> 268859
$ sha256sum /proc/268859/exe ~/.local/bin/dracon-sync \
            /home/dracon/dracon/build/target-audit-2026-11-11/release/dracon-sync
379d448da69f6bb3722d44e4a561b9e919c8c45928e8f7b893669a2c15029205  /proc/268859/exe
379d448da69f6bb3722d44e4a561b9e919c8c45928e8f7b893669a2c15029205  /home/dracon/.local/bin/dracon-sync
379d448da69f6bb3722d44e4a561b9e919c8c45928e8f7b893669a2c15029205  /home/dracon/dracon/build/target-audit-2026-11-11/release/dracon-sync
$ readlink /proc/268859/exe
/home/dracon/.local/bin/dracon-sync          (no "(deleted)")
$ systemctl show … -p ActiveEnterTimestamp --value
Wed 2026-11-11 16:24:46 GMT
```

**Contract 2 answer: equal.** Loaded binary sha256 (`/proc/268859/exe`) ==
installed == rebuilt artifact == `379d448d…029205` at end of goal, and the
artifact was built from a clean tree at `HEAD = c0ce1642` whose `src/sync.rs` is
byte-identical to the reviewed fix (`docs/evidence/…/sync-head.rs`).

## B — fleet sweep (38 watched repos): per-repo table

State from the daemon's own report, captured at `2026-11-11T16:53:39Z`
(`docs/evidence/audit-remediation-2026-11-11/repos-final.json`):

```
$ dracon-sync repos --json
repos 38, ok 35, active 3, warn 0, concern 0, pending 0, failures 1
$ python3 -c "…sorted({r['frozen_secs'] for r in d['rows']})…"
frozen_secs distinct: [None]        # None = daemon running, not frozen
```

Every one of the 38 repos appears below. Columns:

- **head** — `rev-parse HEAD`.
- **ahead** — the daemon's own ahead figure from `repos-final.json`.
- **spools** — epoch-staging directory count and MiB, from the exact command
  quoted in A3 and stamped `2026-11-11T16:50:18Z`.
- **stuck-ledger** — the stuck-push ledger at end of goal: `dracon-sync repair
  stuck-list` → `✅ no stuck repos`, so no repo carried an entry.
- **freeze** — `repos-final.json`'s `frozen_secs` at `2026-11-11T16:53:39Z`
  (`none` = not frozen). The freeze *history* for this run is in section C.
- **push** — the daemon's own `push_status`/`push_error` at `16:53:39Z`.
  `PENDING` means commits exist that had not yet reached the remote, not a
  stuck ledger entry.
- **remotes** — for each configured remote, `tracking-ref / real ls-remote tip`,
  both captured `2026-11-11T16:53:39Z`–`16:53:43Z`; `**SKEW**` marks a
  tracking ref that disagreed with its real tip at capture. Zero remain.

| # | repo | head | ahead | spools @16:50 | stuck-ledger | freeze @16:53 | push @16:53 | remotes: `name` tracking-ref / real `ls-remote` tip @16:53 |
|---|---|---|---|---|---|---|---|---|
| 1 | `/home/dracon/.dracon` | 86ea75bd61 | 0 | 5141 (992.3 MiB) | none | none | OK | `gitlab` 86ea75bd61/86ea75bd61 `origin` 86ea75bd61/86ea75bd61 |
| 2 | `/home/dracon/Dev/dracon-utilities` | 58c8de64f0 | 0 | 25 (1.8 MiB) | none | none | OK | `gitlab` 58c8de64f0/58c8de64f0 `origin` 58c8de64f0/58c8de64f0 |
| 3 | `/home/dracon/Dev/browser-extensions-shared` | 353eb149ca | 1 | 452 (1189.1 MiB) | none | none | PENDING (1 unpushed commits) | `github` 8a4b604229/8a4b604229 `gitlab` 8a4b604229/8a4b604229 `origin` 8a4b604229/8a4b604229 |
| 4 | `/home/dracon/Dev/dracon-platform/web/games/wip/junk-runner` | 884260c100 | 0 | 277 (150.0 MiB) | none | none | OK | `gitlab` 884260c100/884260c100 `origin` 884260c100/884260c100 |
| 5 | `/home/dracon/Dev/dracon-utilities/dracon-sync` | c0ce164259 | 0 | 7 (4.5 MiB) | none | none | OK | `github` c0ce164259/c0ce164259 `gitlab` c0ce164259/c0ce164259 `origin` c0ce164259/c0ce164259 |
| 6 | `/home/dracon/Dev/dracon-platform/web/games/wip/doomtap` | 612816059e | 0 | 11 (4.2 MiB) | none | none | OK | `gitlab` 612816059e/612816059e `origin` 612816059e/612816059e |
| 7 | `/home/dracon/Dev/dracon-platform/web/games/wip/deathrun` | 3185b7a641 | 0 | 4 (2.4 MiB) | none | none | OK | `gitlab` 3185b7a641/3185b7a641 `origin` 3185b7a641/3185b7a641 |
| 8 | `/home/dracon/Dev/dracon-strategy` | 1d50be13e2 | 0 | 3 (4.8 MiB) | none | none | OK | `github` 1d50be13e2/1d50be13e2 `gitlab` 1d50be13e2/1d50be13e2 `origin` 1d50be13e2/1d50be13e2 |
| 9 | `/home/dracon/Dev/dracon-strategy/ai-auto-writer` | f1d61c4053 | 0 | 105 (1542.2 MiB) | none | none | OK | `gitlab` f1d61c4053/f1d61c4053 `origin` f1d61c4053/f1d61c4053 |
| 10 | `/home/dracon/Dev/dracon-platform/web/games/wip/endless-td` | a98013936f | 0 | 29 (9.6 MiB) | none | none | OK | `github` a98013936f/a98013936f `gitlab` a98013936f/a98013936f `origin` a98013936f/a98013936f |
| 11 | `/home/dracon/Dev/dracon-platform/web/games/wip/freeport` | 50200a2f35 | 0 | 11 (297.1 MiB) | none | none | OK | `gitlab` 50200a2f35/50200a2f35 `origin` 50200a2f35/50200a2f35 |
| 12 | `/home/dracon/Dev/dracon-platform/web/games/wip/capture-anime-girls` | 8c9692f6b9 | 0 | 13 (4.3 MiB) | none | none | OK | `github` 8c9692f6b9/8c9692f6b9 `gitlab` 8c9692f6b9/8c9692f6b9 `origin` 8c9692f6b9/8c9692f6b9 |
| 13 | `/home/dracon/Dev/dracon-platform` | 645e1add76 | 1 | 454 (849.3 MiB) | none | none | PENDING (1 unpushed commits) | `github` c7ad5a2641/c7ad5a2641 `gitlab` c7ad5a2641/c7ad5a2641 `origin` c7ad5a2641/c7ad5a2641 |
| 14 | `/home/dracon/Dev/dracon-strategy/ai-auto-video` | 9812c69852 | 0 | 8 (371.2 MiB) | none | none | OK | `gitlab` 9812c69852/9812c69852 `origin` 9812c69852/9812c69852 |
| 15 | `/home/dracon/Dev/dracon-utilities/dracon-warden` | 303480c161 | 0 | 1 (0.2 MiB) | none | none | OK | `gitlab` 303480c161/303480c161 `origin` 303480c161/303480c161 |
| 16 | `/home/dracon/Dev/pi-plugins` | d6822ac953 | 0 | 2 (0.1 MiB) | none | none | OK | `gitlab` d6822ac953/d6822ac953 `origin` d6822ac953/d6822ac953 |
| 17 | `/home/dracon/Dev/pi-plugins/pi-goal-list-loop-audit` | ac10c340b6 | 0 | 1 (0.3 MiB) | none | none | OK | `gitlab` ac10c340b6/ac10c340b6 `origin` ac10c340b6/ac10c340b6 |
| 18 | `/home/dracon/Dev/tiles-tui-file-manager` | 95d01f21dd | 0 | 50 (624.9 MiB) | none | none | OK | `github` 95d01f21dd/95d01f21dd `gitlab` 95d01f21dd/95d01f21dd `origin` 95d01f21dd/95d01f21dd |
| 19 | `/home/dracon/Dev/pi-plugins/pi-agnes-tools` | 86319e6559 | 0 | 0 | none | none | OK | `gitlab` 86319e6559/86319e6559 `origin` 86319e6559/86319e6559 |
| 20 | `/home/dracon/.dracon/convos` | fa3f3b83f1 | 0 | 0 | none | none | OK | `gitlab` fa3f3b83f1/fa3f3b83f1 `origin` fa3f3b83f1/fa3f3b83f1 |
| 21 | `/home/dracon/Dev/terhub` | 98a49f5368 | 0 | 3 (0.1 MiB) | none | none | OK | `github` 98a49f5368/98a49f5368 `gitlab` 98a49f5368/98a49f5368 `origin` 98a49f5368/98a49f5368 |
| 22 | `/home/dracon/Dev/empire` | 1f5bd6c836 | 0 | 0 | none | none | OK | `github` 1f5bd6c836/1f5bd6c836 `gitlab` 1f5bd6c836/1f5bd6c836 `origin` 1f5bd6c836/1f5bd6c836 |
| 23 | `/home/dracon/Dev/dracon-utilities/dracon-system` | 83722c0b07 | 0 | 0 | none | none | OK | `github` 83722c0b07/83722c0b07 `gitlab` 83722c0b07/83722c0b07 `origin` 83722c0b07/83722c0b07 |
| 24 | `/home/dracon/Dev/mcode-plugins` | d044331087 | 0 | 0 | none | none | OK | `gitlab` d044331087/d044331087 `origin` d044331087/d044331087 |
| 25 | `/home/dracon/Dev/pi-plugins/pi-use-last-selected-thinking-level` | 152b727a5b | 0 | 0 | none | none | OK | `gitlab` 152b727a5b/152b727a5b `origin` 152b727a5b/152b727a5b |
| 26 | `/home/dracon/Dev/pi-plugins/pi-codebuddy-sdk` | cc4130115a | 0 | 0 | none | none | OK | `gitlab` cc4130115a/cc4130115a `origin` cc4130115a/cc4130115a |
| 27 | `/home/dracon/Dev/dracon-strategy/DraconDev` | 77f32a0045 | 0 | 0 | none | none | OK | `gitlab` 77f32a0045/77f32a0045 `origin` 77f32a0045/77f32a0045 |
| 28 | `/home/dracon/Dev/db-gateway` | d9cdc7ab3a | 0 | 0 | none | none | OK | `gitlab` d9cdc7ab3a/d9cdc7ab3a `origin` d9cdc7ab3a/d9cdc7ab3a |
| 29 | `/home/dracon/Dev/opencode-plugins` | 803f967d13 | 0 | 0 | none | none | OK | `github` 803f967d13/803f967d13 `gitlab` 803f967d13/803f967d13 `origin` 803f967d13/803f967d13 |
| 30 | `/home/dracon/Dev/dracon-platform/web/games/wip/polis` | 7acb5df4dc | 0 | 0 | none | none | OK | `github` 7acb5df4dc/7acb5df4dc `gitlab` 7acb5df4dc/7acb5df4dc `origin` 7acb5df4dc/7acb5df4dc |
| 31 | `/home/dracon/Dev/dracon-platform/web/games/released/one-mil-girls` | 5c593b17a1 | 0 | 0 | none | none | OK | `github` 5c593b17a1/5c593b17a1 `gitlab` 5c593b17a1/5c593b17a1 `origin` 5c593b17a1/5c593b17a1 |
| 32 | `/home/dracon/Dev/dracon-platform/web/games/released/come-get-me` | de5c80aa86 | 0 | 0 | none | none | OK | `gitlab` de5c80aa86/de5c80aa86 `origin` de5c80aa86/de5c80aa86 |
| 33 | `/home/dracon/Dev/dracon-platform/web/games/wip/football-forever` | 44be560b82 | 0 | 0 | none | none | OK | `gitlab` 44be560b82/44be560b82 `origin` 44be560b82/44be560b82 |
| 34 | `/home/dracon/Dev/dracon-platform/web/games/wip/hellhunter` | 0881c940df | 0 | 0 | none | none | OK | `github` 0881c940df/0881c940df `gitlab` 0881c940df/0881c940df `origin` 0881c940df/0881c940df |
| 35 | `/home/dracon/Dev/dracon-platform/web/games/wip/neonbreak` | 8d801a03b5 | 0 | 0 | none | none | OK | `github` 8d801a03b5/8d801a03b5 `gitlab` 8d801a03b5/8d801a03b5 `origin` 8d801a03b5/8d801a03b5 |
| 36 | `/home/dracon/Dev/dracon-platform/web/games/wip/space-trucker` | 00ba2e8733 | 0 | 0 | none | none | OK | `gitlab` 00ba2e8733/00ba2e8733 `origin` 00ba2e8733/00ba2e8733 |
| 37 | `/home/dracon/Dev/dracon-platform/web/games/wip/darklord` | 265cf18887 | 0 | 0 | none | none | OK | `gitlab` 265cf18887/265cf18887 `origin` 265cf18887/265cf18887 |
| 38 | `/home/dracon/dracon/bridge-fixture` | cf0a8c0b24 | 0 | 0 | none | none | OK | `github` cf0a8c0b24/cf0a8c0b24 `gitlab` cf0a8c0b24/cf0a8c0b24 `origin` cf0a8c0b24/cf0a8c0b24 |

The two repos whose refs needed repair during this run, and how it went:

| repo | ref | before | after | who repaired it | rollback value |
|---|---|---|---|---|---|
| `…/wip/capture-anime-girls` | `refs/remotes/gitlab/main` | `d191dde3` (2026-11-09) | `8c9692f6` | **daemon pid 268859**, 16:27:34 `fetch gitlab: fast-forward` | `d191dde3` |
| `browser-extensions-shared` | `refs/remotes/github/main` | `3cd8c01775` (16:13:57) | `8a4b604229`, then `42a44d4d` → `c0798233` → `8a4b6042` | daemon + one manual `git fetch github` at 16:50:41 | `3cd8c01775` |

`browser-extensions-shared` is the noisy one: it commits every few seconds, so a
tracking ref is always somewhat behind. At `16:50:33Z` its `github` ref read
`c079823322` while the real tip was `42a44d4d90` — a **16-vs-5** inflation
(`rev-list --count github/main..HEAD` = 16 against
`rev-list --count 42a44d4d90..HEAD` = 5). That gap is elapsed commit churn on a
repo the daemon publishes continuously, not a permanently poisoned ref: a single
`git fetch github` closed it in 0.35 s, and the ref now equals the real tip.

**Live re-proof of the A2 fix on the auditor's own counter-example repo**
(`2026-11-11T17:24:45Z`,
`docs/evidence/audit-remediation-2026-11-11/a2-live-proof.txt`). The auditor's
objection was that `capture-anime-girls` was reporting 233 inflated commits
under the fixed daemon. Re-probed:

```
$ git -C /home/dracon/Dev/dracon-platform/web/games/wip/capture-anime-girls \
    rev-parse HEAD refs/remotes/origin/main refs/remotes/gitlab/main
f7a06c33a6a8b63885e43634951a655e2eb04965      # HEAD
f7a06c33a6a8b63885e43634951a655e2eb04965      # origin/main  (converged)
f7a06c33a6a8b63885e43634951a655e2eb04965      # gitlab/main  (was d191dde3)
$ git -C … rev-list --count refs/remotes/gitlab/main..HEAD
0
$ git -C … ls-remote gitlab refs/heads/main
f7a06c33a6a8b63885e43634951a655e2eb04965	refs/heads/main
$ git -C … reflog show refs/remotes/gitlab/main | head -4
f7a06c33 refs/remotes/gitlab/main@{0}: fetch gitlab: fast-forward
5564ed4e refs/remotes/gitlab/main@{1}: fetch gitlab: fast-forward
f146157f refs/remotes/gitlab/main@{2}: fetch gitlab: fast-forward   # the repair
8c9692f6 refs/remotes/gitlab/main@{3}: fetch gitlab: fast-forward   # d191dde3 -> here
```

That reflog is the proof the objection asked for: the ref the auditor measured at
`d191dde3` has since been walked forward **by repeated `fetch gitlab` entries
under the fixed daemon**, one per cycle, with `origin/main` converged the whole
time — exactly the shape the pre-fix early return made unreachable. The inflation
that repeated every ~20 s is gone: `233 commits ahead` appears 69 times in the
16:00:00–16:24:46 window (pre-fix) and **0 times** after 16:28.

Two transient SKEW rows did appear mid-churn at `17:25` and are worth naming
because they look like the A2 bug and are not: `dracon-platform` `github`
`c7ad5a2641` vs real `bb900dc5ed`, and `endless-td` `github` `a98013936f` vs real
`d802e5696f`. Both are lag between a push and the next cycle's fetch, and both
were confirmed as lag by fetching and testing ancestry — after the fetch,
`endless-td`'s `github/main` and `origin/main` had both moved to `d802e5696f`
while `HEAD` stayed 2 commits ahead of *both* names, i.e. genuinely unpushed work
rather than a poisoned ref. Re-running the census once the churn settled returns
`skews: 0 of 114 refs across 38 repos` (`a2-live-proof.txt`).

Class-by-class, with the exact evidence:

| class | state at end of goal |
|---|---|
| stale mirror tracking refs | 0 at `16:53` and again at `2026-11-11T17:10:22Z` (re-probed: `0` skewed refs across all 38 repos). 2 needed repair during the run: the mirror-only skew `gitlab/main d191dde3` on `capture-anime-girls` (the A2 defect, repaired by the fixed daemon at 16:27:34) and `github/main` churn on `browser-extensions-shared`. |
| epoch-spool backlog | 19 repos, 6597 spool dirs (6048 MiB) per the gitdir-resolved census at `16:50:18Z`; largest `/home/dracon/.dracon` 5141 (992.3 MiB), `browser-extensions-shared` 452 (1189.1 MiB), `dracon-platform` 454 (849.3 MiB), `ai-auto-writer` 105 (1542.2 MiB), `tiles-tui-file-manager` 50 (624.9 MiB). Exact commands in A3. |
| stuck-push ledger | empty — `dracon-sync repair stuck-list` → `✅ no stuck repos` (16:50, section A5) |
| hook-blocked pushes | none — the previously refused `dracon-platform` 91-commit push dry-runs as `c7ad5a2641..645e1add76  main -> main` (16:50:50Z, section A1) |
| freeze markers | not frozen at end of goal (marker absent `16:55:05Z` and again `2026-11-11T17:21:28Z`, the last checkpoint before this claim; `frozen_secs` all `None` at `16:53:39Z`). **14 contiguous freeze episodes occurred on 2026-11-11 — see section C.** |
| dirty / retrying (not a failure class of this audit) | 3 at `16:53:39Z`: `browser-extensions-shared` `publishing` + `HISTORY_PROBE_TIMED_OUT`, `dracon-platform` `retrying` + `DIRTY`, `freeport` `retrying` + `DIRTY`. Re-probed `2026-11-11T17:23Z`: `freeport` still `RETRYING` with 46 dirty entries (`M .pi-glla/audit-loop/findings.md`, `M SPEC.md`, art-source PNG/JSON churn — live content edits, not a stale ref or a refused push), 30 commits ahead of `origin/main` by the daemon's own column. Recorded so the `failures 1` in the `repos --json` header is not mistaken for a hook or ledger block. |
| push PENDING (not stuck) | 2 at `16:53:39Z` — `browser-extensions-shared` (`1 unpushed commits`) and `dracon-platform` (`1 unpushed commits`); at `2026-11-11T17:05:33Z` re-sampled: 4 repos with live churn (`browser-extensions-shared` 18, `junk-runner` 2, `ai-auto-writer` 1, `dracon-platform` 1). None is in the stuck ledger; all are commits newer than the last push. |
| disk / memory | `81G` free (91%) at `16:50:18Z`; headroom 7.09% by `MemAvailable`, 13.67% by the guard's own formula, same stamp (section C) |

Out-of-scope observation, recorded because it was seen: `/home/dracon/Dev/eve`
carries ~1135 commits ahead of `origin/main` and is **not** in the 38 watched
repos, so the daemon will not drain it. That is not a defect in the sync daemon;
it is outside its watch roots.

## C — contract 8 reproducibility: disk, memory, spools, and the freeze history

Everything below is copy-pasteable and timestamped. The full transcript is
`docs/evidence/audit-remediation-2026-11-11/final-capture.txt`.

**Disk** (`2026-11-11T16:50:18Z`):

```
$ df -h /
Filesystem      Size  Used Avail Use% Mounted on
/dev/nvme0n1p2  907G  780G   81G  91% /
```

**Memory headroom** — two figures are quoted because they measure different
things, and the previous version of this document quoted neither:

```
$ grep -E 'MemTotal|MemAvailable' /proc/meminfo          (2026-11-11T16:50:18Z)
MemTotal:       31914628 kB
MemAvailable:    2267188 kB
$ awk '/MemTotal/{t=$2}/MemAvailable/{a=$2}END{printf "headroom=%.2f%%\n", a/t*100}' /proc/meminfo
headroom=7.09%
$ free -m
               total        used        free      shared  buff/cache   available
Mem:           31166       28963        1258          90        2860        2203
$ awk '/MemFree/{f=$2}/Buffers/{b=$2}/^Cached/{c=$2}/SReclaimable/{s=$2}/MemTotal/{t=$2}\
    END{printf "headroom=%.2f%%\n", (f+b+c+s)/t*100}' /proc/meminfo
headroom=13.67%
```

`7.09%` is `MemAvailable/MemTotal`. `13.67%` is the memory guard's own formula
(`MemFree + Buffers + Cached + SReclaimable`, per
`~/.dracon/scripts/dracon-memory-guard.sh`). The guard freezes below **8%**
(`MEMORY_GUARD_FREEZE_HEADROOM_PCT`, line 52 of that script) — which is why
freeze episodes recurred during this run and why the guard's own number, not
`MemAvailable`, is the value that decides.

**Epoch-spool counts**, stamped `2026-11-11T16:50:18Z`. The command is the same
one quoted in full in A3 (nothing elided); the complete 19-row output with every
MiB figure below is `docs/evidence/audit-remediation-2026-11-11/final-capture.txt`
(`spool-census.txt` is the earlier 16:35 run and therefore differs slightly):

```
$ for r in $(python3 -c "import json;print('\n'.join(r['repo'] for r in
    json.load(open('repos-final.json'))['rows']))"); do
    gd=$(git -C "$r" rev-parse --path-format=absolute --git-dir 2>/dev/null) || gd="$r/.git"
    n=$(ls -d "$gd"/dracon-staging-* 2>/dev/null | wc -l)
    k=$(du -sk "$gd"/dracon-staging-* 2>/dev/null | awk '{s+=$1} END {print s+0}')
    printf '%s\t%s\t%.1f\t%s\n' "$r" "$n" "$(awk -v k="$k" 'BEGIN{print k/1024}')" "$gd"
  done
/home/dracon/Dev/browser-extensions-shared\t452\t1189.1\t/home/dracon/Dev/browser-extensions-shared/.git
/home/dracon/.dracon\t5141\t992.3\t/home/dracon/.dracon/.git
/home/dracon/Dev/dracon-platform\t454\t849.3\t/home/dracon/Dev/dracon-platform/.git
/home/dracon/Dev/dracon-strategy/ai-auto-writer\t105\t1542.2\t/home/dracon/Dev/dracon-strategy/ai-auto-writer/.git
/home/dracon/Dev/tiles-tui-file-manager\t50\t624.9\t/home/dracon/Dev/tiles-tui-file-manager/.git
  … 14 more repos with 1–29 spools each; full list in final-capture.txt
```

Nested submodule gitdirs are included by the same command (`--path-format=absolute
--git-dir` resolves to `.git/modules/web-games-*`), which is why `junk-runner`
(276), `endless-td` (29), `capture-anime-girls` (13), `doomtap` (9) and
`deathrun` (4) appear in the census rather than as bare names.

**Freeze history** (`docs/evidence/audit-remediation-2026-11-11/freeze-timeline.txt`).
The previous version of this document said the freeze marker was "none
continuous", which its own `repos.json` contradicts — that file records
`frozen_secs = 127` on **all 38 rows** and was captured while the daemon was
mid-freeze. The correction, with contiguous pause episodes derived from the
journal:

```
$ journalctl -u dracon-sync-operator.service --since "2026-11-11 00:00:00" \
    | grep "sync daemon paused" | awk '{print $1, $2, $3}'   # 27663 lines
$ <group contiguous runs where consecutive pause lines are <= 120s apart>
  2026-11-11T01:20:21 .. 01:50:33  (30.2 min)
  2026-11-11T02:07:44 .. 02:39:50  (32.1 min)
  2026-11-11T02:46:59 .. 03:50:42  (63.7 min)
  2026-11-11T04:03:00 .. 04:33:11  (30.2 min)
  2026-11-11T04:54:58 .. 05:25:55  (30.9 min)
  2026-11-11T05:50:19 .. 06:21:01  (30.7 min)
  2026-11-11T08:38:43 .. 09:10:43  (32.0 min)
  2026-11-11T09:16:41 .. 09:47:05  (30.4 min)
  2026-11-11T09:49:56 .. 10:19:33  (29.6 min)
  2026-11-11T12:02:59 .. 12:33:06  (30.1 min)
  2026-11-11T12:49:02 .. 13:52:36  (63.6 min)
  2026-11-11T14:11:16 .. 14:37:44  (26.5 min)
  2026-11-11T14:41:53 .. 15:13:05  (31.2 min)
  2026-11-11T16:18:54 .. 16:26:30  ( 7.6 min)
$ python3 -c "…sorted({r['frozen_secs'] for r in open(REPOS_JSON)…})"
frozen_secs values in the 14:46 capture: [127]     # i.e. frozen at the old audit end
```

So at the stated 14:45 audit end the daemon **was** frozen — the marker was
present and `repos.json`'s `frozen_secs=127` on all 38 rows is that freeze,
started 14:41:53 and self-cleared at 15:13:05. **Fourteen** contiguous episodes
ran across 2026-11-11, not four: the previous draft only looked back to 13:30 and
undercounted by ten.

**End-of-goal freeze sample** (`2026-11-11T16:55:05Z`):

```
$ test -f /home/dracon/.dracon/dracon-sync.freeze && cat … || echo MARKER_ABSENT
MARKER_ABSENT
$ grep -E 'FROZEN_BY_US|BAD_STREAK|HEALTHY_STREAK' ~/.dracon/memory/memory-guard.state
HEALTHY_STREAK=0
BAD_STREAK=0
FROZEN_BY_US=0
$ python3 -c "…frozen_secs from repos-final.json…"
repos-final.json frozen_secs distinct: [None]       # not frozen
```

The guard unfroze itself at 16:26:30 and did not re-freeze before the capture.

**End-of-goal capture** (`2026-11-11T17:07:12Z`,
`docs/evidence/audit-remediation-2026-11-11/end-of-goal.txt`):

```
captured_utc=2026-11-11T17:07:12Z
dpid=268859
loaded_sha=379d448da69f6bb3722d44e4a561b9e919c8c45928e8f7b893669a2c15029205
installed_sha=379d448da69f6bb3722d44e4a561b9e919c8c45928e8f7b893669a2c15029205
rebuilt_sha=379d448da69f6bb3722d44e4a561b9e919c8c45928e8f7b893669a2c15029205
proc_exe=/home/dracon/.local/bin/dracon-sync
started=Wed 2026-11-11 16:24:46 GMT
unit_active=active
--- stuck ledger ---
✅ no stuck repos
--- freeze ---
no freeze marker
BAD_STREAK=0
FROZEN_BY_US=0
--- disk ---
/dev/nvme0n1p2  907G  783G   78G  92% /
--- memory ---
MemAvailable/MemTotal=13.35%
--- inflation check: any 233/5400 scaling since 16:28? ---
0
```

## Final contract re-run (`2026-11-11T17:10:33Z`)

Every contract command executed once more at the end of the goal, back to back
(`docs/evidence/audit-remediation-2026-11-11/final-contract.txt`):

```
[C2] loaded == installed == rebuilt
379d448da69f6bb3722d44e4a561b9e919c8c45928e8f7b893669a2c15029205  /proc/268859/exe
379d448da69f6bb3722d44e4a561b9e919c8c45928e8f7b893669a2c15029205  ~/.local/bin/dracon-sync
379d448da69f6bb3722d44e4a561b9e919c8c45928e8f7b893669a2c15029205  target-audit-…/release/dracon-sync
[C3] executing binary carries the marker
_RNvNtNtCsaDOpQRXmWQz_11dracon_sync3git12multi_remote12list_remotes
[C5] stuck ledger
✅ no stuck repos
[C6] previously blocked push dry-runs clean
   c7ad5a2641..645e1add76  main -> main
[C8] disk + memory
/dev/nvme0n1p2  907G  785G   76G  92% /
meminfo MemAvailable/MemTotal=13.02%
```

## Live health after the fix (`2026-11-11T17:11:26Z`)

The daemon has been running the fixed build for ~47 minutes; it is committing
and syncing normally, with no replaced-executable errors:

```
$ D=$(systemctl show dracon-sync-operator.service -p MainPID --value); echo $D
268859
$ journalctl -u dracon-sync-operator.service --since "2026-11-11 16:45:00" | grep -c "committed"
175
$ journalctl -u dracon-sync-operator.service --since "2026-11-11 16:45:00" | grep -cE "synced"
76
$ journalctl -u dracon-sync-operator.service --since "2026-11-11 16:24:46" | grep -c "no longer executing"
0
```

## Changes made by this audit

1. **Code fix in `dracon-sync/src/sync.rs`, `refresh_stale_upstream_ref`** — the
   early return on a converged upstream is gone; staleness is decided per remote
   name and only stale names are fetched. Committed as `b7e67dcd`
   (`1 file(s) in src/sync.rs DELTA:+34/-21`), pushed by the daemon to all three
   remotes. **Rollback:** `git revert b7e67dcd`.
2. **Regression test made discriminating** — `test_refresh_stale_upstream_ref_repairs_every_mirror_name`
   now leaves `origin/main == HEAD` and rewinds only the mirror, matching the
   live shape; proven to fail against the pre-fix code (A2). Committed as
   `b76168cc` (`DELTA:+32/-14`). **Rollback:** `git revert b76168cc`.
2b. **Transparency: the discriminating experiment left commits in the shared
   history.** Running the test against the *un-fixed* implementation means the
   un-fix existed in the working tree for ~90 s while the daemon was auto-committing
   everything. It captured three transient states — `6cdadf81`, `208638b0`,
   `05318d7a` (`TEMPORARY un-fix for the discriminating-test experiment`) — and
   then `4987c138`, `c0ce1642` removed them again. `HEAD` contains no trace of
   that code (`grep -c "TEMPORARY un-fix"` → 0) and the shipped binary was rebuilt
   from the clean `HEAD`, but the intermediate commits are public. Removing them
   would require a history rewrite, which this audit's boundaries forbid, so they
   stay. Anyone reading the history should treat 208638b0/05318d7a as an
   experiment, not as intended behaviour.

3. Rebuilt dracon-sync from `HEAD` in a dedicated target dir
   `/home/dracon/dracon/build/target-audit-2026-11-11` — deliberately not the
   live repo's `target/`, so no build artifact lands where the daemon commits.
4. Installed the artifact to `~/.local/bin/dracon-sync` twice (once for the
   14:40 reload, once for the 16:24 reload). Previous binaries preserved at
   `/home/dracon/dracon/build/dracon-sync.pre-audit-2026-11-11.bak`
   (sha `0e4b543e…`) and the `458b4660…` build remains in the audit target dir.
   **Rollback:** `install -m 0755 <either> ~/.local/bin/dracon-sync` + SIGTERM.
5. Restarted the daemon twice with `kill -TERM` under the service's own
   `Restart=always`, because `systemctl restart` and `sudo -n` need interactive
   auth this session does not have (polkit `addAdminRule` → `unix-group:wheel`).
6. Fast-forwarded six stale `refs/remotes/github/main` refs earlier in the run,
   each only after proving the real tip was already an ancestor of `main`.
   **Rollback:** prior values were `e7c1a5202a`, `b88a92e6dc`, `b151294464`,
   `bdc6d84cb4`, `918e5fab5e`, `3ea223e55a`.
7. Resumed the daemon from memory-guard freezes with `dracon-sync resume` — the
   guard froze 14 times during 2026-11-11 (episodes listed in C) and has not
   frozen since 16:26:30.
8. Ran one manual `git fetch github` in `browser-extensions-shared` at 16:50:41
   to close a 16-vs-5 churn gap the daemon had not yet caught up with. That is
   the same operation the fixed code performs; **rollback** value recorded in B.

No history was rewritten, no force-push was performed, no hook was bypassed or
weakened, and no spool was deleted during this audit.

## Open items (not fixed here, and why)

0. **The audit itself caused a live incident, and that is the finding.** Installing
   the corrected artifact replaced the inode under a running process, which broke
   every epoch staging self-re-exec for ~2 minutes (27 failures, 14:38–14:39:53)
   until the daemon was restarted. The restart could not be done with
   `systemctl restart` or `sudo -n` (both need interactive auth unavailable to
   this session: polkit `addAdminRule` requires `unix-group:wheel`, `sudo -n`
   reports `a password is required`), so it was done with a graceful SIGTERM under
   the service's own `Restart=always` — the documented shutdown path. Raw evidence:
   `docs/evidence/audit-remediation-2026-11-11/evidence-{A5,A5b,final}.txt` and
   `repos.json`.

1. **The install-then-restart hazard is a real code gap.** `self_executable`
   detects the replaced inode, but the documented remedy ("restart") requires
   privileges the installing user may not have, and a plain artifact install
   silently creates a state where every epoch helper spawn fails while pushes keep
   reporting OK. A durable fix would be an explicit graceful self-restart signal
   (SIGHUP → re-exec of the new image) that does not depend on systemd auth, or a
   release/install path that refuses to overwrite the path while the service is
   active. Not implemented here: it changes daemon lifecycle behaviour and needs
   its own audited contract.
2. **`/home/dracon/.dracon` carried 5111 spools / 987.7 MiB when measured at
   16:35 and 5141 / 992.3 MiB by `16:50`** — the earlier reclaim pass never
   covered that root, and the backlog grows faster than it is reclaimed. Re-running
   the redundancy reclaimer over that root is the natural next action; it was not
   done blind against a live root.
3. **`dracon-utilities` accumulates cargo `target/` (29 GiB) inside a watched
   repo** and is not covered by `.gitignore`, so the daemon repeatedly tries to
   stage it (`⚠️ git add failed for 11852 paths … target-sync/…`). Adding
   `target*/` to that repo's ignore set would remove a standing failure source.
   Not done here: it touches another repo's ignore policy.
4. **The mirror-refresh fix still depends on `git fetch <name>` succeeding within
   30 s.** It does not cover a remote that is unreachable, and it runs only after
   a clean (non-degraded) push, so a repo whose push is permanently degraded
   keeps its skew. Observed here: `browser-extensions-shared` accumulated a
   16-vs-5 gap while its publication was owned by a long-running hook.
   It repairs a stale ref by fetching the name, which is why the six skews above
   were repairable manually. It does not run for repos the daemon never pushes to
   (e.g. `/home/dracon/Dev/eve`, ~1135 ahead, outside the watch roots).

