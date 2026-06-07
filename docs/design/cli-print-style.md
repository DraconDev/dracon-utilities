# CLI Print Style

**Status:** Approved · **Date:** 2026-06-07

## Purpose

Define a consistent visual language for human-facing CLI output across all
three dracon utilities: `dracon-sync`, `dracon-warden`, `dracon-system`.

This is a *cosmetic* contract. It does not change the meaning of any output, the
set of commands, or any machine-readable formats (`--json`, Prometheus metrics,
scripted output). It only makes the human-readable form clearer, more
informative, and easier to scan.

## Invariants (must always hold)

1. **Machine-readable formats are untouched.** `--json` output, Prometheus
   metrics, anything piped to another program: no cosmetic changes.
2. **Default install (no config) stays recognisable.** The icons, the row
   labels, the section ordering are all stable. Existing scripts that grep
   for `📜 Policy` or `🏛️ System repo` keep working.
3. **NO_COLOR is honoured.** When the env var is set (even to empty),
   all ANSI colour codes are suppressed.
4. **TTY detection.** When stdout is not a terminal, colour is suppressed
   (consistent with the conventions of `ls`, `grep`, etc.).
5. **DRACON_FORCE_COLOR escape hatch.** Setting this env var forces colour
   even when not a TTY (useful for `tee` and `script(1)` recordings).

## Icon set

Unicode emoji and box-drawing characters are used as visual markers. They are
chosen for broad terminal support (Linux, macOS, modern Windows Terminal):

| Icon | Meaning | Used in |
|------|---------|---------|
| 📜 | Policy / config file | `dracon-sync status`, `dracon-warden status` |
| 🔁 | Watch root / cyclic | `dracon-sync status` (Roots row) |
| 📦 | Repo count | `dracon-sync status` (Repos row) |
| ⏱️ | Time / pulse / timeout | `dracon-sync status` (Pulse, timeouts) |
| ⏳ | Inactivity delay | `dracon-sync status` (Inactivity row) |
| ⏸️ | Freeze / paused | `dracon-sync status` (Freeze row) |
| ⚙️ | Flag / setting | `dracon-sync status` (Flags row) |
| 📏 | Size limit | `dracon-sync status` (Max stage file) |
| 🧱 | Threshold | `dracon-sync status` (Push blob threshold) |
| 🚫 | Exclusion | `dracon-sync status` (Exclude rows) |
| 🔁 | Retry | `dracon-sync status` (Push retries) |
| 🧯 | Repair | `dracon-sync status` (Repair cooldown) |
| 📒 | Ledger / log | `dracon-sync status` (Incident ledger) |
| 🏛️ | System / state | `dracon-sync status` (System repo) |
| 🧰 | Backup | `dracon-sync status` (Backup) |
| 🌐 | Remotes | `dracon-sync status` (Remotes) |
| 🛡️ | Watch / security | `dracon-warden status` (Watch roots) |
| 🧭 | Discovery | `dracon-warden status` (Discovery roots) |
| 🔑 | Key / pubkey | `dracon-warden status` (Pubkey source) |
| 🏠 | System root | `dracon-system status` |
| 🐧 | NixOS | `dracon-system status` (NixOS root) |
| 📋 | Summary one-liner | All status tables (first row) |
| ✅ | Pass / ok | `dracon-system doctor`, status indicators |
| ❌ | Fail / missing | `dracon-system doctor`, status indicators |
| ⚠️ | Warning | `dracon-system doctor`, health check |
| 🏥 | Health | `dracon-sync health` |

## Human-readable formatters

Three helpers live in `dracon-sync/src/print.rs`,
`dracon-warden/src/print.rs`, and `dracon-system/src/print.rs`. They are
intentionally duplicated (no shared crate) to keep each binary self-contained.

### `format_bytes(n: u64) -> String`

Binary units (KiB, MiB, GiB), 2-decimal precision, never more than 3 sig figs.

| Input | Output |
|-------|--------|
| `0` | `0 B` |
| `512` | `512 B` |
| `1_572_864` | `1.50 MiB` |
| `52_428_800` | `50.0 MiB` |
| `1_073_741_824` | `1.00 GiB` |

The raw byte count is also shown in parentheses for transparency:
`50.0 MiB (52428800)`.

### `format_secs(secs: u64) -> String`

Compact, no-leading-zero, two-unit max.

| Input | Output |
|-------|--------|
| `0` | `0s` |
| `5` | `5s` |
| `60` | `1m` |
| `130` | `2m 10s` |
| `3600` | `1h` |
| `3900` | `1h 5m` |
| `86_400` | `1d` |
| `90_000` | `1d 1h` |

### `format_relative(unix_ts: Option<u64>, now: u64) -> String`

Relative time with a fixed reference point (no clock drift between formatter
calls). Returns `"never"` for `None`, `"just now"` for < 2s, otherwise
`"{format_secs(diff)} ago"`.

### `should_color() -> bool`

Honours `NO_COLOR` (any value, including empty) and `DRACON_FORCE_COLOR`
(forces colour even when not a TTY). Falls back to `std::io::stdout().is_terminal()`.

## Section grouping in `status`

The `status` table for each binary now uses a *summary one-liner* as the first
row, followed by grouped sections separated visually by section dividers.
This is the only structural change; row labels are stable.

Example: see the rendered output of `dracon-sync status` or
`dracon-warden status`. The grouping is:

1. Header (Policy, Summary)
2. Discovery (Roots, Repos)
3. Rhythm (Pulse, Inactivity, Freeze)
4. Flags
5. Limits (Max stage file, Push blob threshold, Exclude)
6. Timeouts & retries (split per row, not crammed)
7. Repair (cooldown, ledger)
8. System (System repo, Backup, Remotes)

## Doctor remediation

`dracon-system doctor` now emits per-check remediation hints. Format:

```
⚠️  Some checks failed. Remediation:
  ❌ <check label>: <hint>
  ❌ <check label>: <hint>

Run with --json for machine-readable details.
```

When all checks pass, a single success line is emitted:
`✅  All checks passed.`

## Out of scope (intentionally not changed)

- `--help` text is auto-generated by `clap` and not customised.
- Prometheus metrics output is unchanged.
- `repos` table is left as-is in this round (very wide, but every column has
  a clear purpose; redesigning it would require breaking the column order
  that some scripts may rely on).
- `events` table styling is unchanged.
- Daemon logs (which go to the incident ledger) are unchanged.
