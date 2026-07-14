# scan-bloat surface review — 2026-07-15

Run: `dracon-sync scan-bloat --min-size-mib 0 --min-repo-count 1` (v0.112.15)
Scanned 28 repos. Total untracked bloat surfaced: **5.75 GiB** across 10 buckets.

## Bucket-by-bucket decision

| Bucket | Size | Repos | Verdict | Action |
|---|---|---|---|---|
| `dracon-sync/` | 5.36 GiB | 1 (`dracon-utilities`) | **Exclude** — nested standalone repo per AGENTS.md; `target/` build artifacts dominate | Add `dracon-sync/` to parent `.gitignore` |
| `dracon-system/` | (part of 5.36) | 1 | Same as above | Add to parent `.gitignore` |
| `dracon-warden/` | (part of 5.36) | 1 | Same as above | Add to parent `.gitignore` |
| `assets/` | 176.50 MiB | 1 (`hegemon`) | **KEEP** — intentional game art (`static/assets` + `build/assets`) | — |
| `test-books/` | 72.33 MiB | 1 (`ai-auto-writer`) | **KEEP** — intentional content drafts (batches of prose) | — |
| `~/` | 55.85 MiB | 1 (`browser-extensions-shared`) | **Already caught** by v0.112.15 `**/~/**` | — |
| `.state-recon/` | 48.86 MiB | 1 (`deathrun`) | **Already caught** by v0.112.15 `**/.state-recon/**` | — |
| `artifacts/` | 34.82 MiB | 1 (`.dracon`) | **System scope** — `/home/dracon/.dracon/artifacts/` not a repo concern | — |
| `web/` | 13.24 MiB | 2 (`dracon-platform`, `junk-runner`) | **KEEP** — submodule working tree content | — |
| `tasks/` | 703 KiB | 7 | **Already caught** — these are `.pi/tasks/` (leaf `tasks`), matched by v0.112.15 `**/.pi/**` | — |
| `plugins/` | 50 KiB | 1 (`opencode-plugins`) | **KEEP** — untracked plugin tools | — |
| `observed/` | 0 B | 1 (`pully-fully-pull`) | **KEEP** — empty dir | — |

## Applied change

Parent `/home/dracon/Dev/dracon-utilities/.gitignore` now ignores the 3 nested
standalone repos (`dracon-sync/`, `dracon-system/`, `dracon-warden/`). This is
outside warden's managed block, so warden won't clobber it. Because these are
standalone git repos (not submodules), the parent treats them as untracked by
design (see AGENTS.md "Repository architecture"); the `target/` build artifacts
(5.36 GiB of the 5.75 GiB total) are the only reason they surfaced.

## Verification

After change: `git ls-files --others --exclude-standard --directory` at the
parent returns 0 dirs (or only `.pi/tasks/`, which `**/.pi/**` covers).

## Deferred

`assets/`, `test-books/`, `web/` are intentional content. They will NOT be
excluded. If they grow, the operator can revisit but for now they are correct.
