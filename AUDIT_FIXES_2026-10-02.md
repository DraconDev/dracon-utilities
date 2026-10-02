# Audit remediation — 2026-10-02

All seven findings from [the audit](AUDIT_2026-10-02.md) are addressed in source.
Final spec-contract verification is in progress; the other checks below passed.

## Changes and regression evidence

| Finding | Result | Regression evidence |
|---|---|---|
| A1 | Warden enumerates newly published commits and scans introduced content, including merge-parent diffs and binary modifications. | Secret add/delete, modify/revert and merge-resolution ranges reject; inherited binary matches, plaintext-sibling exceptions, tags and hook chaining pass. |
| A2 | Interactive storage apply rechecks active Rust and Node/package-manager processes and recent artifact modification at the deletion boundary. Inspection failures retain candidates. | Live named process fixtures preserve Rust targets, build output and node_modules. Real release CLI reproductions retain artifacts with live Rust/Node processes. Failed/malformed/empty process listings fail closed. |
| A3 | Spec verification uses Cargo's exit status, reports failure output and consistently uses `--locked`. | Compilation exit 101, ordinary test exit 1 and success exit 0 produce the correct script outcomes. |
| A4 | All 21 CI checkouts and the three Nix inputs pin the fixed sources below. The checker reads package metadata from those revisions. | A newer live manifest cannot conceal stale pinned metadata. Exact GitHub-downloaded sources pass `cargo check --workspace --locked --offline` with the parent lockfile. Standalone lockfiles were refreshed from tested parent versions and verified in isolated checkouts. |
| A5 | Quarantine rejects an existing reserved `.quarantine.json` pathname before moving. New metadata is published atomically without replacement or symlink following. Existing entry layout stays restorable. | Regular files, symlinks and dangling symlinks preserve source and outside bytes; cross-device refusal and successful move/restore are covered. Real release CLI reproductions pass. |
| A6 | Normal, mirror, HTTPS fallback and maintenance pushes honor pre-push hooks. Approved rewrite maintenance uses the narrow history exception while retaining content/operator hooks. | Actual daemon push helpers against a disposable bare remote honor rejecting and accepting hooks; maintenance alone passes `DRACON_ALLOW_REWRITE=1`. Existing success fixtures now use an allowed synthetic identity. |
| A7 | Nix service verification anchors to the Git root and uses a Git-aware flake reference. Ignored local files remain outside the source import. | The actual expression excludes an ignored build artifact and a synthetic private environment file; generated-service assertions and flake evaluation pass. |

## Fixed source pins

| Utility | CI/Nix revision |
|---|---|
| dracon-sync | `5441b0f595113a4a3ea3fd040e16f80b07ad66e9` |
| dracon-system | `e3f148db014b960ec753631d42d24a94a099b621` |
| dracon-warden | `dd4159a6276eebd6ffab827529e89dda322c4412` |

Nix hashes were obtained by fetching these exact published GitHub revisions.
Each nested HEAD equals its source pin. Workspace versions remain sync
`0.113.92`, system `0.112.42`, warden `0.113.14`, security `0.4.0`.
Canonical nested checkouts and published history were preserved.

## Validation

| Check | Result |
|---|---|
| Locked workspace tests | PASS: 2,127 tests, 25 existing ignored; two additional Warden preservation regressions also pass. Final spec reruns the complete latest suite. |
| Warden pre-push regression group | PASS: 22 tests |
| Complete system suite | PASS: 420 tests |
| Parent audit and convergence regressions | PASS: 34 tests |
| `cargo build --release --locked` | PASS |
| Stable and Rust 1.89 all-target/all-feature Clippy, warnings denied | PASS |
| `cargo deny check` | PASS: advisories, bans, licenses and sources |
| Formatting and complete script ShellCheck | PASS |
| `python3 scripts/check-nested-pins.py --check-local` | PASS |
| Exact downloaded pinned sources, locked offline workspace check | PASS |
| `./scripts/check-flake.sh` | PASS: flake evaluation and generated-service policy |
| `./scripts/verify-spec.sh` | Final run in progress |

Audit-gate regressions are wired into the spec verifier and CI. The Nix CI job
runs its source-isolation regression with Nix installed; the scripts job may
skip that one test when Nix is unavailable. Nix package realization remains
the separate existing CI build step.

This is source remediation. Running fleet utilities and installed global hook
scripts still use their existing installations. The confidentiality fixture
uses only synthetic content; it does not establish what the interrupted
pre-fix store import encountered (see A7 in the original audit).
