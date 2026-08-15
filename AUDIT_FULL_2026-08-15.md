# Full audit follow-up — 2026-08-15

This follow-up continues [the 2026-08-14 full audit](AUDIT_FULL_2026-08-14.md)
and closes the residual correctness, deployment, and repository-layout issues
found during the final review. The parent repository remains meta-only; the
three utility repositories are standalone nested git repos.

## Fixes completed

### Runtime safety and correctness

- `dracon-system` now checks for root or `CAP_SYS_NICE` before applying
  reversible priority changes. An unprivileged user service skips the
  renice/recovery path instead of lowering a process and repeatedly failing
  to restore it. The capability check is warning-once and unit-tested.
- Progress-aware Git execution now has a hard wall-clock ceiling of four times
  the configured idle timeout. Continuous remote output can extend the idle
  deadline, but cannot keep a child alive indefinitely.
- Remote branch cleanup in `dracon-sync` treats an already-absent branch as an
  idempotent success while still surfacing other deletion failures.
- Warden global hook installation is now a transaction: all three temporary
  hooks are staged before replacement, same-named foreign hooks are preserved
  beside the managed hooks, and the managed wrappers chain them. Failures
  attempt rollback. Atomic replacement and foreign-hook chaining are tested.

### Repository and automation consistency

- `install.sh` no longer requires the deleted `../dracon-libs` checkout,
  validates the three nested utility manifests, and honors `--no-restart`
  without stopping or killing running services. `--binaries-only` no longer
  requires `systemctl`.
- The parent doctor, spec verifier, release dispatcher, CI workflow, and Nix
  flake understand the meta-only layout. CI and Nix now pin the same nested
  source revisions, with explicit `/main` flake inputs and
  `scripts/check-nested-pins.py` enforcing agreement with local checkouts and
  `Cargo.lock`.
- `cargo deny` now treats unapproved duplicate dependency versions as errors.
  The currently unavoidable transitive duplicates have explicit skip reasons;
  `cargo deny check` is clean.
- `dracon-system-lib` is pinned to `=94.2.7`, resolving `sysinfo 0.32.1`, so
  the Nix package remains compatible with the pinned Rust 1.94.1 toolchain.
  Warden integration tests use `git` from PATH (or the existing override)
  instead of assuming `/run/current-system/sw/bin/git`; sandbox-only test
  assumptions for Git template hooks and `/home` were removed.
- The Nix wrapper validates the flake while explicitly filtering only the
  documented generic-checker warning for the conventional
  `homeManagerModules` output.
- The obsolete CLA workflow, which referenced a deleted `CLA.md`, was removed.
  Active README, contributor, architecture, standalone utility, and
  source-of-truth docs no longer instruct users to clone `dracon-libs` or
  invoke removed façade-generation scripts. The former façade design is
  explicitly marked historical.
- The `doomtap` path-owned identity was corrected to
  `doomtap@dracon.local` / `doomtap-dev`, with the historical scaffold
  identity retained in the explicit trust lists. No history rewrite was used.

## Deployment and live verification

The built binaries were installed to `~/.local/bin` and the two daemon
services were restarted through systemd. The installed versions are:

- `dracon-sync 0.113.50`
- `dracon-system 0.112.37`
- `dracon-warden 0.113.4`

Both `dracon-sync.service` and `dracon-system-guard.service` are active.
`dracon-sync health` reports a healthy daemon, valid policy, freeze off, and
31 discovered repositories. The fleet summary reports 0 warnings and 0
concerns; the remaining active rows are normal settling activity. The guard
journal has no post-restart `mem-unrenice ... Permission denied` entries; the
only current message is the intended capability-gate warning that renice
mitigation is disabled without `CAP_SYS_NICE`.

The installer removed only stale utility binaries and installer backup
artifacts from `~/.cargo/bin`/`~/.local/bin`; no operator data was deleted.

## Verification

All final checks pass:

- `cargo test --workspace --locked -- --test-threads=1`: **1,391 passed,
  9 ignored, 0 failed** (149 security, 979 sync, 136 system, 127 warden).
- `cargo build --release --locked`
- `cargo deny check` — advisories, bans, licenses, and sources passed.
- `cargo clippy --workspace --locked -- -D warnings`
- `cargo fmt --all -- --check`
- `./scripts/verify-spec.sh`
- `./scripts/test_release.sh` — 8 dispatcher checks passed
- `./install.sh --dry-run --no-restart --binaries-only`
- `./scripts/check-flake.sh`
- `nix build .#dracon-sync .#dracon-system .#dracon-warden --no-link`
- `python3 scripts/check-nested-pins.py --check-local`
- YAML parsing of `.github/workflows/ci.yml`
- `bash -n` for all root shell scripts
- `git diff --check` in the parent and all nested repositories

No service was manually stopped and left down for remediation, no history was
rewritten, and no operator data was deleted during this follow-up.

## Intentional residuals

1. `/home/dracon/.config/git/hooks/pre-commit.bak` is an older Warden hook
   artifact. It remains untouched because its provenance is ambiguous; the
   active hooks are the current transactional Warden wrappers.
2. `cargo deny` still contains the documented transitive duplicate versions
   that cannot be consolidated locally. New unlisted duplicates now fail the
   gate.
3. The Nix `homeManagerModules` output is intentionally retained for Home
   Manager consumers; its generic `nix flake check` warning is handled by
   `scripts/check-flake.sh` and is not a functional failure.
4. Nested utility pin updates remain a deliberate release step, now guarded
   by the CI/Nix/Cargo consistency check.
