# AUDIT FULL — ROUND5 (2026-10-19)

Date: 2026-10-19
Scope: `dracon-utilities` parent meta repo + the three nested standalone utility repos
(`dracon-sync` 0.113.95, `dracon-system` 0.112.45, `dracon-warden` 0.113.16, `security` 0.4.0),
plus the live `dracon-sync` daemon state and the `dracon-sync repos` fleet table.

Trigger: operator request — "audit this project also check out Detail: `dracon-sync repos <name>`"
for the repo the fleet table flagged as needing attention (`bridge-fixture`), cross-read against
the full key (`dracon-sync repos --legend`) and the rest of the supplied table — which turned
up a second row carrying a failing push that the summary line had not named
(`mcode-plugins`, row 18).

Method: mechanical gate execution (build/test/clippy/fmt/deny/doctor/health) + targeted source
review of code introduced since ROUND4 (2026-10-03) + live daemon/host supervision inspection +
live fleet triage of every row the supplied table marked with a failing push. Every finding
below is backed by a command output or a `file:line` reference read in source.
No repository state was mutated by this audit: no commit, no push, no service change, no
freeze, no file written outside this report and `/tmp`.

Severity: CRITICAL = arbitrary code execution or credential exposure; HIGH = broken
security control or red gate; MEDIUM = correctness/operational gap; LOW = docs, drift, noise.

## Counts (16 findings)

| Severity | Count | Ids |
|---|---|---|
| CRITICAL | 1 | A5-01 |
| HIGH | 5 | A5-02, A5-03, A5-04, A5-14, A5-16 |
| MEDIUM | 6 | A5-05 … A5-08, A5-13, A5-17 |
| LOW | 4 | A5-09 … A5-12 |
| **Total** | **16** | |

## Requested item — `dracon-sync repos bridge-fixture` (resolved)

The fleet table flagged exactly one repo: `bridge-fixture`, `❔ UNKNOWN`, with the hint
*"path-owned identity/origin warning: ✓ owned (path_owned:untrusted_origin); GitHub pack check
unknown — use repos --deep"*.

**The pack check was a false alarm.** `dracon-sync repos bridge-fixture --deep` measured it live:

```
1. 🟡 QUEUED  /home/dracon/dracon/bridge-fixture
   git size:  416 KiB (local history; own+nested, +? unknown)
   hint:      ⚠️ path-owned identity/origin warning: ✓ owned (path_owned:untrusted_origin);
              GitHub pack guard clear (measured this report); 2 GiB/push, not repo size
```

The pack component of the hint was only unmeasured, and is now measured clear; a later
non-deep run shows it cached (`GitHub pack guard clear (cached 711s ago)`). The `❔ UNKNOWN`
that remains in the **status/activity** columns is *not* about the pack check at all — it is
the activity classifier having no reading for a guard-blocked repo. The
`path_owned:untrusted_origin` warning is expected and benign under the AGENTS.md ownership rule
(path-owned repos warn on a foreign `origin`; they do not gate commit/push) — `origin` here is
the local state mirror `/home/dracon/.dracon/state/bridge-fixture-remote.git` while the real
remotes are `github`/`gitlab`.

**The real, unreported blocker is different.** The repo is permanently un-committable:

```
journalctl --user -u dracon-sync.service | grep bridge-fixture
⚠️ /home/dracon/dracon/bridge-fixture bucket runner bridge refused; no commit:
   bucket-runner status refused: refused
🔔 sync alert: … Sync Blocked (>30 min): blocked by a guard or needs manual intervention
🔔 sync alert: … Changes Piling Up: oldest committable change is 49037s old … 1 committable
   entries; oldest: public/qualification-20261018/ovh-live-2fc91627ddb9.png
```

The untracked PNG is **not** an accidental leftover: `bucket-runner.json` lists it in
`allowedCandidates`, and the HEAD commit is *"Stage live retry-qualification fixture (local
transport failure injected)"* — the refusal is the deliberate state under qualification. The
daemon correctly refuses to commit. The defects here are diagnostic, not behavioural
(A5-08, A5-12).

### The second attention item the footer never named: `mcode-plugins`

The supplied table's row 18 read:

```
│ 18  ┆ ✅ CLEAN  ┆ mcode-plugins ┆ … ┆ ✅ OK  … ┆ ❌ FAIL ┆ … ┆ 3 ┆ 3 ┆ 3 ┆ 2.4 MiB ┆ DraconDev ┆
```

`❌ FAIL` in the PUSH column, `✅ CLEAN` in STATUS — and the footer still said
*"💡 1 repo requiring attention"*, naming only `bridge-fixture`. `dracon-sync repos
mcode-plugins` gives the detail:

```
1. ✅ CLEAN  /home/dracon/Dev/mcode-plugins
   publish:   ⚠️ origin/main …
   changes:   0 mod, 0 stg, 0 ut
   ahead/behind: 0/0
   push:      ❌ FAIL 30m
   activity:  ⚪ idle      state: ⚪ idle
   hint:      🛑 push-stuck (5 failures): git push returned non-zero (remotes: github,
              gitlab, origin…) — run dracon-sync repair concerns --apply
```

Tracing it produced three findings that the table's own summary line had suppressed:

- **A5-14** — the repo has **25 commits and an empty remote**, yet the view reports
  `0/0` and `✅ CLEAN`. The report never applies the never-pushed ahead fallback that the
  daemon has used since v0.112.31; git omits `# branch.ab` when there is no tracking ref, so
  the parser leaves ahead at 0.
- **A5-16** — the push fails because the repo's own pre-push guard refuses to publish a
  **plaintext `password=` credential** embedded in a 6.4 MB `.orig.bak` bundle that is still
  in the pending push range. The remote (`git@github.com:DraconDev/mcode-plugins.git`) exists
  and SSH auth works — it simply has **zero branches**, so nothing has ever been published.
- **A5-17** — because the ahead count is 0, `current_health::classify` returns
  `Current::Converged` and the `PUSH_STUCK → Stalled` branch is unreachable, so the row is
  **omitted from the "requiring attention" footer** even though it carries a push-stuck hint
  and the daemon had fired a `Push Stuck (budget exhausted)` alert for it.

While pushes are paused the daemon stays in "commit-only mode" and keeps committing —
`📝 committed 1 file(s) in /home/dracon/Dev/mcode-plugins` appears every ~20-60 s from 14:43
to 14:52 — so the unpublished pile grows without bound. This is a live credential exposure
guarded by exactly one local hook, and it needs an operator decision (rotate the credential;
then the sanctioned-slimming rewrite or a fresh push of a sanitised `main`).

## CRITICAL

### A5-01 — CRITICAL — a tracked file in ANY watched repo yields arbitrary code execution as the operator user

**Evidence** (`dracon-sync/src/bucket_runner_bridge.rs`):

- `:116-127` `struct Config` — deserialized from the **repo's own tracked** `bucket-runner.json`
  (`:40` `CONFIG_RELATIVE = "bucket-runner.json"`; `:243-253` `discover_config` reads
  `repo / "bucket-runner.json"`). It carries `profile_path: String`.
- `:497` `let profile_path = validate_profile_path(&config.profile_path)?;`
- `:206-214` `validate_profile_path` checks only: path is absolute, and `is_file()`. **No
  owner check, no mode check, no symlink check, no fixed operator-owned location.**
- `:515-517` the profile is read and parsed into `struct Profile { runner: ProfileRunner }`,
  where `ProfileRunner` (`:135-137`) has **only** a `path: String` field.
- `:518-524` the only constraint on that path is a *filename suffix* check
  (`/runner.bundle.mjs` or `/runner.bundle.cjs`) plus `is_file()`.
- `:436-444` `spawn_bundle` → `Command::new("node").arg(bundle)` — the profile-selected file
  is **executed**, as the daemon user, with the daemon's environment and sandbox.
- `src/sync.rs:5434` calls `bucket_runner_bridge::prepare(repo, dry_run, policy)` for every
  repo in a sync cycle.
- `src/policy.rs` contains **no** `bucket` knob at all — there is no global policy gate, no
  operator allowlist of repos, and no opt-in outside the repo's own file.

**Attack path** (no access outside the repository is required): a watched repo commits

- `bucket-runner.json` → `{"schemaVersion":1,"enabled":true,"profilePath":"<repo>/p.json", …}`
- `p.json` → `{"runner":{"path":"<repo>/x/runner.bundle.mjs"}}`
- `x/runner.bundle.mjs` → arbitrary code

On the daemon's next cycle for that repo, the bridge validates both paths as *regular files in
the repo*, passes the suffix check, and spawns `node <repo>/x/runner.bundle.mjs`.

**Impact.** Code execution as `dracon` (uid 1000) from the fleet's watch tree, with the
daemon's sandbox (`ProtectSystem=strict`, `ProtectHome=read-only`,
`ReadWritePaths=/home/dracon/.dracon /home/dracon/Dev /home/dracon/.local/state/dracon
/home/dracon/.ssh /home/dracon/dracon/bridge-fixture`) and its environment. That grants read
access to the fleet's private material — `~/.dracon/secrets/**` (including
`~/.dracon/secrets/ovh/dracon-lfs-uploader.env`, the OVH S3 uploader credential),
`~/.dracon/data/keys/**`, `~/.ssh/**` — and write access to `~/.dracon` and `~/.ssh`. The
bundle can also return `commitEligible: true` and a matching `manifestSha256`, so the daemon
will then commit attacker content on the operator's behalf.

**This violates the project's own stated design.** `dracon-sync/evidence/
auto-registration-2026-10-16/integration-seams-next.md` item 2 requires:

> "Add Rust trusted profile loader/runner bridge: **operator-owned fixed profile path** (or
> explicit global policy authority), validate file ownership/private modes, symlink-free paths,
> full runner hash before invocation. **Repository config must never select uploader
> executable/credentials.**"

Three of the four stated controls are absent, and the one that is present (a filename suffix)
is not one of them. `dracon-sync/evidence/bucket-live-qual-2026-10-18/bindings.mjs:179`'s
`allowedAssetTrees` check does not compensate: the allowlist lives *inside the profile*, and the
profile is the attacker-selected file.

**Fix.** Make the profile location operator-owned and outside the repo: resolve
`profilePath` from the global sync policy (or a fixed, validated location) and treat any
repo-supplied `profilePath` as a refusal, not a hint. Independently: enforce
`profile.runner.sha256` against the bytes on disk (A5-02), check the profile file's owner and
`0600` mode, refuse symlinked components, and add a global allowlist of repositories permitted
to use the bridge at all.

## HIGH

### A5-02 — HIGH — the operator's pinned runner hash is silently dropped and never verified

**Evidence.** The live profile
`/home/dracon/.dracon/state/bucket-runner/profile.json` pins the runner:

```json
"runner": { "path": "/home/dracon/.dracon/bin/runner.bundle.mjs",
            "sha256": "352e4de0a452991fa9ed99c9de9afecb633b8399478912a5169c043823893020",
            "version": 1 }
```

`struct ProfileRunner` (`bucket_runner_bridge.rs:135-137`) declares **only** `path: String`.
Serde ignores unknown fields by default, so `sha256` and `version` are **dropped at
deserialization and never compared to the bytes on disk**. A repo-wide search finds no
comparison anywhere in `src/`: the only `sha256` references are manifest digests
(`:583-602`), the `storage_core` LFS paths, and test fixtures. The live bundle's hash does
happen to match today (`sha256sum` → `352e4de0…`), but nothing checks that.

**Impact.** The only integrity control the operator has over the executable that receives the
storage credentials is decorative. Replacing `/home/dracon/.dracon/bin/runner.bundle.mjs` (or
editing the profile) is not detected. This is the "full runner hash before invocation"
requirement of `integration-seams-next.md` item 2, unimplemented — and it is the control that
would otherwise blunt A5-01.

**Fix.** Add `sha256: String` to `ProfileRunner` (and `version`), hash the bundle bytes
immediately before spawn, and refuse on mismatch.

### A5-03 — HIGH — the CI lint gate is red: `cargo fmt --all -- --check` fails

**Evidence.** `.github/workflows/ci.yml` (lint job) runs `cargo fmt --all -- --check`.

At the revisions CI actually checks out (`.github/workflows/ci.yml` pins), the gate fails:

| Nested repo | CI pin | `rustfmt --check` at pin |
|---|---|---|
| dracon-sync | `2759b8c8` | exit 0 (clean) |
| dracon-system | `93b0edd9` | **exit 1 — 4 diffs** |
| dracon-warden | `2388bee4` | exit 0 (clean) |

The four `dracon-system` diffs: `src/main.rs:7865` (`TerminateOutcome::Signalled` pattern),
`src/main.rs:7893` (`reaped[..5].iter().map(detail)…` chain), `src/policy_tests.rs:1651`
(`std::path::Path::new("/tmp").join(…)`), `src/reap_tests.rs:1152` (`assert_eq!(reaped.len(), 2, …)`).

At the **local** HEADs the same command is far worse — `cargo fmt --all --check` exits 1 with
**138 diffs across 20 files**, including `dracon-sync/src/git/push.rs`,
`dracon-sync/src/sync.rs`, `dracon-sync/src/daemon.rs`, `dracon-sync/src/report.rs`,
`dracon-sync/src/lfs_classify.rs`, the whole `bucket_runner_bridge` module,
`dracon-system/src/main.rs`, `dracon-warden/src/tests.rs`. So re-pinning the CI pins (A5-04)
without formatting will not make the gate green either.

**Impact.** The lint job is the first gate in CI and is failing on `main`. Everything after it
in that job (clippy `-D warnings`, strict rustdoc) is not evaluated, so a broken lint job
silently suppresses three other gates.

**Fix.** Run `cargo fmt --all` over the three nested repos, commit, and re-pin. Then make the
gate non-skippable.

### A5-04 — HIGH — the CI pins are stale by 754/31/47 commits; the drift guard is red

**Evidence.**

```
$ python3 scripts/check-nested-pins.py --check-local
FAIL: dracon-sync: local HEAD 9d2fb2274f5c0721c2854657396f6de5d9dce5d0
      != pinned revision 2759b8c8b7c235a5bf35049934978dc6bde5f0dc
$ echo $?   → 1
```

Drift against the `.github/workflows/ci.yml` pins:

| Nested repo | Pin | Commits ahead |
|---|---|---|
| dracon-sync | `2759b8c8` | **754** |
| dracon-system | `93b0edd9` | 31 |
| dracon-warden | `2388bee4` | 47 |

**Impact.** CI builds and tests code that is 754 commits behind the deployed `dracon-sync`
0.113.95 — including the entire bucket-runner bridge module that A5-01/A5-02 live in. The
`check-nested-pins.py --check-local` gate exists precisely to catch this (its own comment:
*"which is what makes this a real gate"*) and is currently red, so the drift is detected and
not acted on.

**Fix.** Re-pin to the current nested HEADs (`scripts/repin-nested-sources.py` exists for this),
after A5-03's formatting is landed, so CI exercises the code that actually ships.

### A5-14 — HIGH — the `repos` view reports "0 ahead / CLEAN" for a repo that has never been pushed

**Evidence.** The report's ahead/behind comes from the porcelain-v2 parser
(`src/report.rs:4029` → `src/git/inspection.rs:parse_status`), which reads the
`# branch.ab +N -M` line. Git emits that line **only when the remote-tracking ref exists**.
For a repo whose upstream is configured but never fetched, git emits
`# branch.upstream origin/main` and **no** `branch.ab`:

```
$ git status --porcelain=v2 --branch -z --untracked-files=all
# branch.oid 067139798d0a33c449ee9f38128ca4ab921048a4
# branch.head main
# branch.upstream origin/main
$ git status --porcelain=v2 --branch | grep -c branch.ab
0
```

`parse_status` therefore never sees an ahead/behind value, and `status.ahead` / `status.behind`
stay at their default **0**.

The daemon compensates — but the report does not. `count_all_head_commits` and
`upstream_tracking_ref_missing` (`src/git/status.rs:311-330`, `:360-393`) are referenced
**only** from `src/daemon.rs:8528`; `src/report.rs` uses neither. The daemon's own comment at
`src/daemon.rs:8520-8530` names this exact defect:

> "…then libgit2 computed ahead=0 (nothing to compare against), and
> `has_local_or_pending_work` evaluated false — the repo was skipped forever and **the report
> showed a false 'synced'**. When the tracking ref is missing, every commit on HEAD is
> unpushed; `count_all_head_commits` is the fallback after the mirror-based counts."

That fallback was added to the daemon in v0.112.30/0.112.31 and never to the report.

**Live confirmation** — `mcode-plugins`:

| Measurement | Value |
|---|---|
| `git rev-list --count HEAD` | **25** commits |
| `git ls-remote --heads origin` | **0** branches (empty remote) |
| `git for-each-ref refs/remotes/` | *(no output — no tracking ref)* |
| `dracon-sync repos` AHEAD / BEHIND | **0 / 0** |
| `dracon-sync repos` STATUS | **✅ CLEAN** |

Twenty-five commits are unpublished and the operator's table says zero.

**Impact.** The ahead column is the operator's measure of outstanding work. Reading "0 ahead,
✅ CLEAN" for a repo with 25 unpushed commits and a broken remote is a false all-clear, and it
is the same false "synced" the daemon's comment says was fixed. It also silently disarms the
report-side push-stuck classification (A5-17).

**Fix.** Apply the daemon's fallback in the report: when `upstream_tracking_ref_missing` is
true, set `ahead = count_all_head_commits(repo)` (and `behind = 0`). The helper already exists
and is unit-tested (`src/git/status.rs:656-800`).

### A5-16 — HIGH — a plaintext `password=` credential sits in mcode-plugins history, in the pending push range, guarded only by a local hook

**Evidence.** `mcode-plugins` is push-stuck with an exhausted budget:

```
Oct 19 14:51:50 🛑 push stuck (5 consecutive failures ≥ budget 5) — auto-push paused
Oct 19 14:51:50 🔔 sync alert: Push Stuck (budget exhausted): 5 consecutive push failures —
   local pre-push hook refused the push (repo policy: asset-guard / size budget / secret scan
   / history guard — fix the policy, not the network); auto-push paused
```

Replaying the repo's own pre-push guard reproduces the refusal exactly:

```
$ printf 'refs/heads/main %s refs/heads/main 0000…0000\n' "$(git rev-parse HEAD)" \
    | .git/hooks/pre-push origin git@github.com:DraconDev/mcode-plugins.git
⚠️  Possible plaintext secrets detected in added file
   mcode-chrome/chunk-5ADYFF2U.js.orig.bak (binary-safe scan).
   The warden filter may have been bypassed.
   Run: dracon-warden once /home/dracon/Dev/mcode-plugins
HOOK_EXIT=1
```

The offending file is a **6.4 MB JavaScript bundle backup** (blob `f5b32e814ed9de69a4f6b74f…`):

- added by `b9d4365` — *"auto: initial commit (18 files)"*
- deleted by `e867003` (the worktree copy is now covered by `*.orig.bak` in the warden's
  managed `.gitignore` block, line 128 — but the blob is still reachable)
- `git rev-list --objects HEAD --not --remotes | grep -c chunk-5ADYFF2U.js.orig.bak` → **1**
  — i.e. it is **inside the range that a push to the empty remote would publish**

Scanning the blob against the guard's own `SECRET_RE` categories (reporting only which
category matched, never the value) identifies it as a **`password = "…"` assignment** — a
plaintext credential, not an exotic false positive. The private-key, GitHub PAT, GitLab PAT,
AWS, Stripe, Slack, OpenAI, Google, SendGrid, npm, HuggingFace, Square and OVH alternatives
all did not match.

**Impact.** A plaintext password is committed to git history in a repo whose remote is empty,
so **the local pre-push hook is the only thing standing between it and a public GitHub push**.
The AGENTS.md policy forbids agent loops from rewriting history, and the sanctioned remedy
(`docs/design/sanctioned-slimming-2026-09-26.md`) is operator-only, so this cannot be
self-healed. Meanwhile the daemon keeps committing — the journal shows
`📝 committed 1 file(s)` every ~20-60 s from 14:43 to 14:52 — because "commit-only mode"
continues while pushes are paused. The local pile grows without bound, and each new commit
is another candidate for the next push attempt.

**Fix (operator).** Treat the credential as compromised and rotate it. Then either rewrite the
history to drop the blob (sanctioned-slimming procedure, operator-approved) or push a
sanitised `main` to a fresh remote and re-point `origin`. The 6.4 MB blob is also a
GitHub-push size risk in its own right.

## MEDIUM

### A5-05 — MEDIUM — the daemon has no reboot persistence; the prepared fix is not applied

**Evidence.**

- The live unit is `/run/systemd/system/dracon-sync-operator.service`; `/run` is **tmpfs**
  (`findmnt -no FSTYPE /run` → `tmpfs tmpfs`), so the unit does not survive a reboot.
- The user scheduler is deliberately **masked** (persistent):
  `~/.config/systemd/user/dracon-sync.service -> /dev/null` (root-owned, Oct 19 01:25).
- The remedy is prepared but **not applied**:
  `dracon-sync/scripts/dracon-sync-operator.nix` header — *"Prepared (NOT applied) NixOS module
  … After a reboot no scheduler would start … Operator activation (requires sudo, NOT done by
  the audit)"*.
- The host config does not import it: `~/.dracon/nixos/configuration.nix:4-10` imports only
  `hardware-configuration.nix`, `desktop.nix`, `user.nix`, `custom-tools.nix`, `obs-hotkey.nix`;
  `~/.dracon/nixos/modules/` holds only `common-server.nix` and `gcc-libs.nix`.

**Impact.** A reboot of this host leaves the entire fleet unsynced with nothing scheduled. This
is the *same* failure class AGENTS.md's quiesce policy exists to prevent ("a forgotten restart
leaves the fleet unsynced silently"), and it is currently open.

**Fix.** Import the prepared module into the host config and `nixos-rebuild switch`, or ship
an equivalent persistent (non-`/run`) system unit.

### A5-06 — MEDIUM — `doctor.sh` is blind to the live daemon and its warnings point at a forbidden action

**Evidence.** `doctor.sh` reports:

```
⚠️  dracon-sync.service (installed but not running)
⚠️  dracon-sync-watchdog.timer (installed but not enabled/active — its backstop will never fire)
Results: 25 passed, 2 warnings, 0 failures
```

Both warnings are **false** for the live deployment, and the daemon's real backstop *is* firing:

```
$ systemctl list-timers --all | grep dracon
dracon-sync-operator-watchdog.timer  dracon-sync-operator-watchdog.service
  LAST 14:40:37  NEXT 14:42:37
$ systemctl is-enabled dracon-sync-operator.service  → enabled-runtime
$ systemctl is-active  dracon-sync-operator.service  → active
$ systemctl --user is-enabled dracon-sync-watchdog.timer → disabled   (deliberate)
```

`doctor.sh:118-121` checks a hardcoded list of **user**-manager units and contains **zero**
references to the operator service (`grep -c operator doctor.sh` → `0`); it never queries the
system manager at all.

**Impact.** `doctor.sh` is the operator's first diagnostic. It tells them the backstop "will
never fire" when it fires every 2 minutes, and that the daemon "is not running" when it is.
The natural remediation — enable the user watchdog timer, unmask the user service — is
**explicitly forbidden** by the migration record
(`dracon-sync/evidence/bucket-live-qual-2026-10-18/migration-preparation.md`): *"the old
restart watchdog is disabled and old scheduler is masked during handover … Never run both
schedulers."* Following doctor.sh's advice risks two schedulers writing the same repos.

**Fix.** Teach `doctor.sh` about the system-manager operator service and its watchdog: check
`dracon-sync-operator.service` (system) as the authoritative daemon, check
`dracon-sync-operator-watchdog.timer` as its backstop, and stop reporting the deliberately
masked/disabled user units as warnings.

### A5-07 — MEDIUM — `install.sh` provisions the superseded supervision topology

**Evidence.** `install.sh:552-561` copies the **user** units
`dracon-sync.service`, `dracon-sync-watchdog.{service,timer}`,
`dracon-freeze-watchdog.{service,timer}`, `dracon-system-guard-watchdog.{service,timer}` and
the three watchdog scripts; `:573` then runs

```
systemctl --user enable --now dracon-sync-watchdog.timer dracon-freeze-watchdog.timer \
                                  dracon-system-guard-watchdog.timer
```

It ships **no** operator service and **no** operator watchdog. The shipped
`dracon-sync-watchdog.sh` only knows the user unit: it calls
`systemctl --user is-active --quiet dracon-sync.service` and
`systemctl --user start dracon-sync.service` — a unit the live deployment deliberately masks.

**Impact.** A fresh install on a host with the current topology resurrects the user restart
watchdog, whose only action is to start a masked (or a second) scheduler. On the current host
the timer is enabled and fires, `systemctl --user start` on the masked unit fails, and the
operator's `~/.config/systemd/user` ends up with a permanently failing timer and a misleading
unit.

**Fix.** Make the install path topology-aware: install the operator service + watchdog (or
detect the existing operator deployment and skip the user restart chain), and point the
shipped watchdog script at whichever unit is authoritative.

### A5-08 — MEDIUM — the bridge discards the verdict's `reason`, so refusals are unactionable

**Evidence.** `bucket_runner_bridge.rs:573-581`:

```rust
if matches!(status.as_str(), STATUS_REFUSED | STATUS_PLANNABLE | STATUS_UNSUPPORTED) {
    bail!("bucket-runner status refused: {status}");
}
```

Only `status` is used. The runner emits a `reason` on every refusal —
`scripts/bucket-runner/runner.mjs:27`
`const refused = reason => ({ schemaVersion: 1, status: 'refused', reason, commitEligible: false })`
— with distinct causes (`'publication-refused'` `runner.mjs:157`, `'consumer-changed'`
`runner.mjs:159`, `'request-or-authority-refused'` `runner.mjs:201`, and the publisher's
`error.message` at `publisher.mjs:309`, e.g. a storage/credential failure).

The operator-visible result is the string `bucket-runner status refused: refused` — the word
"refused" twice and the actual cause nowhere. The fleet has been logging this every ~5
minutes for 48 h.

**Impact.** The exact gap the push path was already repaired for: the
`sync-blockers-2026-10-19` workstream made push stdout *"captured (bounded, redacted) so a
future refusal explains itself"* (`evidence/sync-blockers-2026-10-19/browser-extensions-push-cause.md`),
but the bucket path was not given the same treatment. A credential failure and a
consumer-changed verdict are indistinguishable from the log.

**Fix.** Surface `verdict["reason"]` in the bail message (bounded and redacted, as the push
path does), and include it in the incident ledger.

### A5-13 — MEDIUM — the `repos` fleet view silently omits repos whose status inspection exceeds 8s

**Evidence.** The report path uses a hardcoded 8-second budget
(`src/report.rs:4029` `inspect_status(&repo, Duration::from_secs(8))`, and the same value as
`PORCELAIN_BUDGET` at `src/report.rs:1356`), while the daemon's own scheduler allows 60s
(`src/daemon.rs:269` `const STATUS_TASK_TIMEOUT_SECS: u64 = 60;`, used at `daemon.rs:8383`).
The report's budget is therefore **7.5× stricter than the daemon's**.

On timeout the repo is dropped from the table entirely —
`src/report.rs:4029-4035` emits `status_failed` and `return None`. The failure *is* counted
(`failures` in `--json`) and printed on stderr, but the repo contributes **no row to the
table**, so the operator scanning the table sees a fleet that is one repo short with nothing
marking which one is missing. Observed four consecutive times during this audit:

```
❌ /home/dracon/Dev/dracon-platform | status_failed: status inspection timed out;
   owned Git process group cancelled
❌ /home/dracon/Dev/dracon-strategy/ai-auto-writer | status_failed: status inspection timed
   out; owned Git process group cancelled
```

The affected repo **moved between runs** (dracon-platform in runs 1–3, ai-auto-writer in
run 4), so it is load-dependent rather than a property of one repo. The JSON confirms the
omission exactly: the daemon reports **36 discovered repos** (`dracon-sync health`), while
`dracon-sync repos --json` reports `repos: 35` with `ok: 26, active: 9, warn: 0, concern: 0`
(the counters themselves sum correctly — 26+9+0+0 = 35 — so the R3-L09 contract holds) and
`failures: 1`, which is the timed-out repo that has no row.

**Impact.** The `repos` table is the operator's primary fleet view — the very command in the
audit request. A large repo under load vanishes from it. That is the same "nearly invisible"
failure class the project treats as dangerous elsewhere (AGENTS.md, canonical-checkouts
section: *"their loss is nearly invisible … stayed gone for two days"*). The daemon may well
be syncing that repo perfectly; the *view* simply cannot see it.

**Fix.** Raise the report budget to the daemon's `STATUS_TASK_TIMEOUT_SECS`, make it a policy
knob, and represent a timed-out repo as an explicit in-table row (e.g. `❔ UNKNOWN` +
`status inspection timed out`) rather than dropping it, so the count in the header always
equals the number of discovered repos.

### A5-17 — MEDIUM — the "N repos requiring attention" footer cannot see a push-stuck repo

**Evidence.** The footer is built by `rich_hint_link_out` (`src/report.rs:6871-6897`), which
keeps only rows whose `current_health::classify(row)` is one of
`Blocked | Stalled | Retrying | Paused | Unknown | Queued | Excluded`. `classify`
(`src/current_health.rs:39-95`) starts with:

```rust
let pending = row.modified + row.staged + row.untracked > 0 || row.ahead > 0 || row.behind > 0;
…
if !pending {
    return if row.excluded_dirty > 0 { Current::Excluded } else { Current::Converged };
}
```

and only later:

```rust
if row.ahead > 0 && row.push_status == "PUSH_STUCK" {
    return Current::Stalled;
}
```

For `mcode-plugins` the report's `ahead` is **0** (A5-14) and the worktree is clean, so
`pending` is false → `Current::Converged` → the row is **excluded from the footer** — even
though the row's own hint reads
`🛑 push-stuck (5 failures): git push returned non-zero (remotes: github, gitlab, origin…)`
and the daemon had fired a `Push Stuck (budget exhausted)` alert for it.

**Impact.** The operator's summary line under-reports. In the fleet table supplied with this
audit request, `mcode-plugins` (row 18) showed `❌ FAIL` in the PUSH column while the footer
read *"💡 1 repo requiring attention"*, naming only `bridge-fixture`. A repo that is
permanently un-pushable, blocked on a credential, and committing every minute was not in that
count.

**Fix.** Make the footer's inclusion test depend on the *evidence* rather than only on
`pending`: a row whose `push_status` is `PUSH_STUCK` / `BROKEN` / `BLOCKED`, or whose hint
carries a push-stuck marker, must reach the footer regardless of the ahead count. Fixing
A5-14 also repairs this for the never-pushed case, but the independence should be fixed too —
`PUSH_STUCK` is defined by *exhausted-budget evidence*, and the report already carries it.

## LOW

### A5-09 — LOW — AGENTS.md is stale after the 2026-10-19 operator-service migration

AGENTS.md still documents *"**`dracon-sync-watchdog.timer`** (2m) restarts the service if it
is inactive"* and lists the three user watchdog timers as the backstop stack. It does not
mention `dracon-sync-operator.service`, the system-manager watchdog, the user-unit mask, or
the `/run` ephemerality. An agent following AGENTS.md today would target the wrong unit and
the wrong backstop — the same confusion doctor.sh exhibits (A5-06).

### A5-10 — LOW — the prepared Nix module omits two live-unit properties

`dracon-sync/scripts/dracon-sync-operator.nix` does not carry
`PassEnvironment=SSH_AUTH_SOCK` or `EnvironmentFile=-/run/systemd/system/dracon-sync-operator.environment`,
both present in the live unit. Everything else was verified to match the **effective** live
configuration (the live base unit's `CPUQuota=100%`/`MemoryHigh=768M` are overridden by
`dracon-sync-operator.service.d/10-resource-headroom.conf` to `200%`/`1536M`, and the module
encodes exactly those values; the `20-bridge-fixture.conf` drop-in's extra
`ReadWritePaths` entry is likewise present in the module).

Impact is currently **nil**: the daemon's own environment has no `SSH_AUTH_SOCK`
(`/proc/2039261/environ` → absent) and no ssh-agent is running, so `~/.ssh/config`'s
`IdentityFile` + `IdentitiesOnly yes` path is what actually authenticates. It is a latent
divergence: if an agent is ever started, the module-built unit would silently lose it.

### A5-11 — LOW — `bridge-fixture` alert storm is unbounded

The daemon emits `Sync Blocked (>30 min)`, `Dispatch Starved (>10 min)` and
`Changes Piling Up` alerts for `bridge-fixture` roughly every 5 minutes, continuously, for
48+ hours. The refusal is the deliberate qualification state
(`git log`: *"Stage live retry-qualification fixture (local transport failure injected)"*), so
the repo will stay blocked until the injection is removed. A permanently-blocked, intentionally
blocked repo should not generate a repeating alert every 5 minutes; it dilutes the signal for
genuine blocks.

### A5-12 — LOW — the fleet table's "requires attention" hint names the wrong cause

The `repos` table's attention hint for `bridge-fixture` pointed at the *pack check* being
unknown, which resolved to "clear" on a deep run. The actual blocker (the bucket-runner
bridge refusal) was not surfaced in the row at all — the row showed `❔ UNKNOWN` / `🟡 QUEUED`
activity with no indication that the repo is blocked-by-guard and has been for 24 h. The
hint should prefer affirmative block evidence over an unmeasured pack check.

## Gate results (2026-10-19, local HEADs)

| Gate | Command | Result |
|---|---|---|
| Release build | `cargo build --release --locked` | ✅ exit 0 |
| Workspace tests | `cargo test --workspace --locked` | ✅ exit 0 — all suites green (warden 220, integration 18, storage_stream 2, doc-tests 0, plus the sync/system suites) |
| Clippy | `cargo clippy --workspace --locked --all-targets -- -D warnings` | ✅ exit 0 |
| Format | `cargo fmt --all --check` | ❌ **exit 1 — 138 diffs / 20 files** |
| Dependencies | `cargo deny check` | ✅ `advisories ok, bans ok, licenses ok, sources ok` |
| Doctor | `./doctor.sh` | ⚠️ 25 passed, 2 warnings, 0 failures (both stale — A5-06) |
| Daemon health | `dracon-sync health` | ✅ healthy, running, freeze off, policy valid, 36 repos / 3 roots |
| Nested pins | `python3 scripts/check-nested-pins.py --check-local` | ❌ **exit 1** (A5-04) |

## Policy conformance (AGENTS.md) — all confirmed

| Claim | Observed | Verdict |
|---|---|---|
| `untracked_exclude_patterns = []` (commit-all) | `[]` | ✅ |
| `max_stage_file_bytes = 104857600` | `104857600` | ✅ |
| `push_op_timeout_secs = 900` (operator bump) | `900` | ✅ |
| `auto_repair_concerns = true` | `true` | ✅ |
| `auto_prune_stale_backup_branches = true` | `true` | ✅ |
| Git identity `DraconDev <dracsharp@gmail.com>` | matches | ✅ |
| `mailmap.file` configured | `/home/dracon/.config/git/dracon-identities.mailmap` | ✅ |
| warden no-rewrite hooks present | `pre-commit`, `pre-push`, `pre-rebase` under `core.hooksPath` | ✅ |
| destructive reap opt-ins default OFF | `reap_stale_dev_servers` / `reap_orphans_on_pressure` absent from the system config | ✅ |
| `auto_renice_on_memory` / `bias_oom_on_pressure` | `true` / `true` | ✅ |

## Recommended order

1. **A5-01** — move the profile location under operator/global authority; refuse a
   repo-supplied `profilePath`. *This is the one finding that can be triggered by a routine
   commit to a watched repo.*
2. **A5-16** — operator action, today: rotate the `password` credential exposed in
   `mcode-plugins` history, then either run the sanctioned-slimming rewrite or push a
   sanitised `main` to a fresh remote. Until then a single local hook is the only thing
   preventing a plaintext credential from reaching GitHub, and the daemon keeps committing
   every minute.
3. **A5-02** — enforce `profile.runner.sha256` before spawn.
4. **A5-03 + A5-04** — `cargo fmt --all` in the three nested repos, commit, re-pin, turn CI
   green. The drift is 754 commits on `dracon-sync`; CI currently proves nothing about it.
5. **A5-14 + A5-17** — apply the daemon's never-pushed ahead fallback in the report and make
   the attention footer depend on push evidence rather than on the ahead count. These two make
   the fleet view tell the truth about `mcode-plugins` and every other never-pushed repo.
6. **A5-05** — apply the prepared operator module (or an equivalent persistent unit) so the
   daemon survives a reboot.
7. **A5-06 + A5-07** — make `doctor.sh` and `install.sh` topology-aware.
8. **A5-08** — surface the verdict `reason`.
9. **A5-13** — align the `repos` status budget with the daemon's 60s and render a timed-out
   repo as an explicit row rather than dropping it.
10. **A5-09 … A5-12** — documentation and alert-noise hygiene.

## Out of scope / not verified here

- The `dracon-platform` reset incident of 2026-10-19 00:19:10 (operator-reported) is recorded
  in `web/jobs/note.md` and was **not** re-investigated; reflog does not identify the caller.
- The bucket-runner component's own JS test suites (`planner`, `publisher`, `storage`,
  `bindings`, `restore`, `consumer`, `e2e`, `adapters`, `routing`, `capabilities`) were not
  re-run in this round; only the Rust workspace gates were.
- The 754-commit `dracon-sync` delta since the CI pin was **not** line-by-line reviewed. A5-01
  and A5-02 were found by targeted review of the bridge; a full component audit of that delta
  remains outstanding.
- `dracon-warden` 0.113.16 and `dracon-system` 0.112.45 release deltas were covered by the
  mechanical gates only, not by source review.
- **No credential value was printed, stored, or transmitted.** The category identification in
  A5-16 was done by matching the guard's own `SECRET_RE` alternatives against the blob and
  reporting only which category matched. The temporary working copy of the blob was moved to
  recoverable trash (`/tmp/blob.bin` → trash) rather than deleted outright, because the
  permission gate blocks irrecoverable destruction — **note that this places a copy of a
  plaintext credential in the trash store**, which is exactly what the 2026-08-10 credential
  discipline in AGENTS.md warns against. The operator should purge that trash entry.
- **No history rewrite, no force-push, no service change, no freeze and no push was performed
  by this audit.** The `mcode-plugins` credential and the `dracon-platform` reset incident both
  need operator action; neither was touched.
