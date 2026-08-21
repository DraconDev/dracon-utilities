# Full audit — 2026-08-21 (post-fix re-audit)

Scope: fresh pass over the three utility sources and the meta repo after
today's remediation session (v0.113.52 deploy, guard active-build fix,
storage-apply rule alignment, checkout restoration). Covers the three LOW
carryovers from the morning audit. Evidence gathered by three independent
read-only survey passes plus live state checks. Verdict: **no HIGH
findings**; 4 MEDIUM (deferred with rationale below); LOWs recorded.

## Findings

### MEDIUM

**M1 — sync: scp-style remotes with IPv6 hosts or non-`git` usernames fall out of canonical matching.**
`src/git/urls.rs:66-70` splits scp-style input at the first `:` (inside a
bracketed IPv6 literal: `git@[2001:db8::1]:org/repo.git` garbage-splits);
`urls.rs:65` recognizes only the literal prefix `git@`, so
`deploy@github.com:org/repo.git` yields `None`. Consequence at
`src/sync.rs:1801-1822`: such an origin is never classified GitHub, so an
identical named mirror gets pushed twice (harmless no-op) and the 2 GiB
pack guard stops firing for that repo (silent behavioral downgrade).
Deferred rationale: requires rare remote configs; failure mode is
redundant-push or skipped pack-guard, not data loss. Fix: bracket-aware
scp split + generalize the username prefix.

**M2 — system: storage cleanup can target `.git` when explicitly asked.**
The apply path now uses guard rules (`validate_storage_cleanup_path`,
main.rs:5134) whose safety rests on candidates being artifact dirs.
Candidates come from `analyze_workspace_storage`, whose kinds include
`git-db` (.git); `dracon-system storage --cleanup --kinds git-db --apply`
would `remove_dir_all` a project's git history under /home — `allow_tracked`
does not protect `.git` (git never tracks it, main.rs:4733-4759).
Mitigation present: requires two explicit non-default flags; default kinds
exclude git-db. Deferred rationale: opt-in footgun, not reachable by
default. Fix: filter `git-db` out of selectable cleanup kinds.

**M3 — system: node_modules cleanup remains ungated and age-only.**
Confirmed carryover (was LOW, reassessed): `run_auto_cleanup`
(main.rs:3776+) gates every other kind on a feature flag; node_modules
(main.rs:3844-3868) runs whenever `auto_cleanup_apply` is on, depth ≤5,
mtime > 30 days, under `~/Dev`, with **no active-build protection**
(unlike rust targets). Resuming a stale project loses deps mid-session
(recoverable via reinstall; now partially mitigated on this host because
`node_modules_search_roots` behavior was observed before the flag flip).
Fix: add `clean_node_modules: bool` (default true) for symmetry.

**M4 — meta: CI/Nix pins lag all three nested HEADs.**
`python3 scripts/check-nested-pins.py --check-local` fails: flake/ci pin
`dracon-sync-src` at `1c76635d` while local main is `c885cf6c` (13 commits,
including the v0.113.52 release commit `beaebe1`); system pinned `7366c7b3`
vs local `620636ca`; warden pinned `1832c442` vs local `9c6a4728`. CI would
build stale source until pins are bumped. Deferred rationale: mechanical
release-step drift, exactly what the disappearance doc's Prevention 2
(daily local pin-check timer, not yet installed) is meant to surface.

### Deployment-state check (live)

- `dracon-sync` 0.113.52 installed == source ✓ (deployed today).
- `dracon-system` installed 0.112.37 **predates today's two fixes**
  (committed f0cf666/7b592a5/620636c, no version bump yet): the running
  guard does not yet recognize rust-analyzer/cargo-watch. Recorded as
  follow-up: bump version, `cargo build --release --locked -p
  dracon-system`, install, restart `dracon-system-guard.service`.
- `dracon-warden` 0.113.5 RC installed == Cargo.toml ✓; no v0.113.5 tag
  exists locally or remotely — consistent with the documented
  "publication/tags remain operator-approved steps" policy, not drift.

### LOW

- **sync**: repeated `.git` suffix strip can collide a real `name.git`
  repo with `name` (urls.rs:100-103) — single-strip would be safer.
- **sync** (carryover, accepted): symlink TOCTOU window in staging
  (check-then-add, sync.rs:815-843 → 1160-1175) self-heals next cycle;
  git rejects bad pathspecs so worst case is one delayed commit.
  `Component::ParentDir` fail-open unreachable from real libgit2 status
  output; skip-on-unrecognized would be more conservative.
- **sync**: pushes longer than ~5 s render `🟡 waiting` instead of
  `🔄 now` when the in-flight marker goes stale mid-push
  (report.rs:931 `IN_FLIGHT_MAX_AGE_SECS`) — errs conservative; cosmetic
  nit: "waiting Xm" duration is time-since-last-commit, not queued-time.
- **system**: log-truncate path (main.rs:4392) keeps the strict classifier
  — same `/home` asymmetry class fixed today, but fail-safe direction
  (truncation skipped, nothing deleted). Coherence switch optional.
- **system**: regression-test fixture writes into the real home dir;
  pid+nanos naming prevents collisions; stray empty dir possible only on
  assert failure (cosmetic).
- **meta**: disappearance doc cites warden last sighting 02:07; the final
  journal line is 02:08:14 (the doc's timeline elsewhere includes both).
- **meta**: AGENTS.md protection line for canonical checkouts
  (disappearance doc Prevention 3) not yet adopted.
- **meta**: aggregated CHANGELOG still files warden 0.113.5 under
  "[Unreleased]" though binary+manifest both say 0.113.5.

### Verified clean

- urls.rs credentials-in-password, percent-encoding, default-port trim,
  case-insensitive schemes, query/fragment stripping, GitLab `/-/`
  preservation, degenerate inputs → None (unit-tested).
- Direct-symlink staging preserved correctly alongside descendant skips
  (commit 1b466b0 goal met; regression test sync.rs:9169).
- report.rs queued-vs-active classification internally consistent; legend
  updated; REM width derivation correct (+2 padding, 8-col floor).
- Both of today's dracon-system fixes verified coherent: all other
  `check_safe_to_delete_guard` callers are artifact-constrained by
  construction; `is_rust_build_process` exact names all < 15-char comm
  truncation; suite 138 passed / clippy clean.
- Meta: Cargo.lock ↔ dracon-sync version agreement (0.113.52);
  disappearance doc fact-checked against journal (all verifiable claims
  hold); AGENTS.md contains no claims made false by today's events;
  warden main/RC-docs/binary/CHANGELOG coherent.
- Live fleet: `dracon-sync health` healthy (27 repos), both daemon
  services active.

## Carryover disposition

| Morning finding | Disposition |
|---|---|
| canonical_repository_url host variants | Confirmed LOW, documented above (plus upgraded details in M1) |
| symlink TOCTOU window | Confirmed LOW/self-healing, accepted with rationale |
| ungated node_modules cleanup | Reassessed **M3** (age-only brake, no active-build guard) |

## Follow-ups (recorded, non-blocking)

1. Fix M1 (urls.rs scp parsing), M2 (exclude git-db kind), M3
   (clean_node_modules flag), M4 (pin-bump commit or accept until next
   release).
2. Build+install+restart dracon-system to ship today's fixes.
3. Adopt AGENTS.md canonical-checkout protection line; install daily
   local pin-check timer; implement watched-repo-vanished daemon concern
   (per docs/design/utilities-checkout-disappearance-2026-08-21.md).
