# Full audit follow-up — 2026-08-15

This follow-up continues [the 2026-08-14 full audit](AUDIT_FULL_2026-08-14.md)
and closes the residual correctness and repository-layout issues found during
the final review. The parent repository remains meta-only; the three utility
repositories are standalone nested git repos.

## Fixes completed

### Runtime safety and correctness

- `dracon-system` now checks for root or `CAP_SYS_NICE` before applying
  reversible priority changes. An unprivileged user service skips the
  renice/recovery path instead of lowering a process and repeatedly failing to
  restore it. The capability check is warning-once and unit-tested.
- Progress-aware Git execution now has a hard wall-clock ceiling of four times
  the configured idle timeout. Continuous remote output can extend the idle
  deadline, but cannot keep a child alive indefinitely.
- Warden global hook replacement now writes through a same-directory temporary
  file, flushes and syncs it, applies executable permissions, and atomically
  renames it into place. The replacement path is regression-tested.

### Repository and automation consistency

- `install.sh` no longer requires the deleted `../dracon-libs` checkout,
  validates the three nested utility manifests, and honors `--no-restart`
  without stopping or killing running services. `--binaries-only` no longer
  requires `systemctl`.
- The parent doctor, spec verifier, release dispatcher, CI workflow, and Nix
  flake now understand the meta-only layout. CI restores the three standalone
  repositories at the paths expected by Cargo and pins their current commits
  so the parent lockfile is reproducible.
- The obsolete CLA workflow, which referenced a deleted `CLA.md`, was removed.
- Active README, contributor, architecture, standalone utility, and source-of-
  truth docs no longer instruct users to clone `dracon-libs` or invoke removed
  façade-generation scripts. The former façade design is explicitly marked
  historical.
- Script executable bits were restored for `doctor.sh`, `uninstall.sh`, and
  the parent release test/dispatcher.

## Verification

All checks completed successfully:

- `cargo test --workspace --locked -- --test-threads=1`: **1,389 passed,
  9 ignored, 0 failed** (149 security, 978 sync, 136 system, 126 warden).
- `cargo build --release --locked`
- `cargo deny check` — advisories, bans, licenses, and sources passed; the
  existing duplicate-version findings remain warnings only.
- `cargo clippy --workspace --locked -- -D warnings`
- `cargo fmt --all -- --check`
- `./scripts/verify-spec.sh`
- `bash -n` for all root shell scripts
- `./scripts/test_release.sh` — 8 dispatcher checks passed
- `./install.sh --dry-run --no-restart --binaries-only`
- `nix flake check --no-build`
- `nix build .#dracon-sync --no-link` — the first package build exposed a
  read-only copied nested-checkout replacement; the flake now makes the
  copied tree writable before installing the pinned standalone sources, and
  the package build passes.
- YAML parse of `.github/workflows/ci.yml`
- `git diff --check` in the parent and all nested repositories

No service was stopped, no history was rewritten, and no operator data was
deleted during this follow-up.

## Live read-only state

At verification time, both daemon services and both watchdog timers were
active. The installed `dracon-sync health` command reported a healthy daemon,
valid policy, freeze off, and 31 repositories. The report showed 23 clean,
7 active, 1 warning, and 0 concerns; the warning was the known path-owned
identity/origin warning for `doomtap`.

The running service binaries were not replaced: their mtimes predate the
newly built `target/release` binaries. The guard journal still contains 1,792
historical `mem-unrenice ... Permission denied` lines in the last 24 hours,
but none in the latest 30-minute window. The new capability gate is therefore
verified by source tests, not yet by a restarted live service.

## Deliberate residuals

1. Installing the new binaries and restarting services is an explicit release
   operation and remains pending operator direction.
2. Warden hook installation is atomic per hook; a multi-hook transaction and
   foreign-hook backup/migration policy still need a separate ownership
   decision.
3. `cargo deny` reports known duplicate transitive versions (`sha2`, `syn`,
   `openssl-probe`, and related crypto/HTTP dependencies). They are not
   vulnerabilities or gate failures; consolidating them would require
   upstream dependency changes.
4. CI and the Nix lock pin nested repository commits. When a nested utility
   advances, update the parent pin and `Cargo.lock` together as part of the
   normal daemon/release workflow.
