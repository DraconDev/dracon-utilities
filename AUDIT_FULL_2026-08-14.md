# Full audit — 2026-08-14

## Scope and method

Audited the meta workspace and its three nested utility repositories:

- `dracon-sync` v0.113.50
- `dracon-system` v0.112.37
- `dracon-warden` v0.113.4, including the in-tree `dracon-security` v0.3.1 crate

The review covered the existing audit/design documents, Git/process boundary
handling, daemon/report/policy paths, system cleanup and pressure controls,
Warden managed-file and hook handling, and repository/workspace consistency.
The live checks were read-only; no service was stopped, no history was
rewritten, and no fleet remediation was applied.

## Result

The source tree is clean and all required quality gates pass. The audit found
and fixed correctness, safety, and operator-guidance issues rather than only
re-running the historical tests.

### Git and daemon fixes

- Git status parsing now consumes copy and unmerged records without losing
  path alignment, and staged/tracked queries fail on Git command errors.
- Large-blob detection preserves paths containing spaces and accepts both
  SHA-1 and SHA-256 object IDs.
- Bounded Git execution now uses file-backed stdin, cleans temporary files via
  RAII, waits after timeout termination, and caps captured stderr at 1 MiB.
- Askpass scripts are cleaned up by an RAII guard; branch/ref inputs are
  validated before refspec construction; SSH hardening quotes paths safely.
- Mirror tracking follows the checked-out branch with a legacy `main` fallback;
  remote diagnostics preserve remote names, classify `ls-remote` failures more
  narrowly, and cache successful auto-created repositories.
- The origin-gone ledger is atomically replaced, visibility caches reject
  future timestamps, and Cargo version extraction handles real inline-comment
  syntax.
- Active diagnostics now emit valid commands (`dracon-sync health` and
  `dracon-sync repair concerns --apply`) instead of removed command names.

### System and Warden fixes

- Inode checks honor the configured mount; lock files are truncated only after
  acquiring the lock; poisoned binary-resolution caches recover safely.
- Nix cleanup no longer invents reclaimed-byte totals, Unicode event-time
  formatting is panic-safe, nested `node_modules` trees are counted once, and
  zram output labels its ratio correctly.
- Malformed Warden managed blocks preserve the unparsed file tail; explicit
  resmudge errors and stale hook-artifact removal failures are surfaced.

## Verification

The final source passed:

- `cargo test --workspace --locked --quiet`: all suites passed — 1,386 tests
  passed and 9 existing tests ignored; no failures.
- `cargo build --release --locked`
- `cargo deny check` — advisories, bans, licenses, and sources passed.
- `cargo clippy --workspace --locked -- -D warnings`
- `cargo fmt --all -- --check`
- `git diff --check` in the parent and all three nested repositories.

`cargo deny` still reports duplicate-version warnings for transitive crates
such as `sha2`, `syn`, and `openssl-probe`; these are non-failing dependency
graph cleanup opportunities.

## Live read-only observations

At audit time:

- `dracon-sync.service` and its watchdog timer were active.
- `dracon-system-guard.service` and its watchdog timer were active.
- `dracon-sync health` reported healthy, policy valid, freeze off, and 31
  discovered repositories.
- `dracon-sync repos --summary` reported 24 clean, 6 active, 1 warning, and
  0 concerns. The warning was the documented path-owned identity/origin
  warning for `doomtap`; active/pushing rows were transient daemon work.

## Deferred risks and follow-ups

These were reviewed but not changed because they require an operational or
policy decision rather than a local correctness patch:

1. The installed user-service binaries were not replaced or restarted during
   this audit. The source changes are committed and pushed; deployment remains
   an explicit release/install action.
2. The guard journal contains repeated `mem-unrenice ... Permission denied`
   messages. The user service can lower priority but cannot always restore a
   higher priority without the required privilege; service capability policy
   should be reviewed before changing the limiter defaults.
3. Progress-aware Git operations use an idle timeout, so a hostile or broken
   remote that emits continuous progress can outlive the nominal timeout.
   Adding a separately configurable hard wall-clock ceiling is a sensible
   follow-up.
4. Warden's global hook installation intentionally owns its configured global
   hook directory, but the writes are not a transactional backup-and-rename
   operation. Improve that only with an explicit hook ownership/migration
   design.
5. Historical evidence and release-note files retain old command names and
   source line references as historical records; active source hints and
   operator guidance were corrected.
