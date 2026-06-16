# GitHub / GitLab / Codeberg Feature Façade Repositories

Dracon utility façade repositories are intentionally small presentation
surfaces. They make `dracon-sync`, `dracon-system`, and `dracon-warden` easier
to feature on GitHub, GitLab, and Codeberg without splitting the
implementation out of the `DraconDev/dracon-utilities` monorepo.

## Why descriptive names matter on Codeberg

On GitHub and GitLab, a repo's name barely affects search ranking — those
platforms are too large and too saturated to surface a "descriptive" name over
established projects. On Codeberg (Forgejo) the math is different: descriptive
repo names get upvotes and free attention because readers immediately know
what the project does. A repo called
`dracon-sync-watch-debounce-commit-push-mirror` reads as a one-line feature
list, while `dracon-sync` reads as a vague label that competes with thousands
of other generic-named sync tools.

The trade-off is name length. Codeberg's name search and topic search both
treat the full name as a string, so descriptive names gain discoverability
there at the cost of typing. This is acceptable because the façade repos are
not the ones people clone day-to-day — the monorepo is.

## Name composition rules

Names are deliberately constrained to **brutal descriptiveness**:

- All lowercase, hyphens only, must start with `dracon-`
- **No "ai"** claim (the operator has explicitly excluded AI marketing words
  from project names)
- **No filler**: no `the`, `for`, `in`, `and`, `with`, `of`; no audience/UX
  claims like `workspace`, `infrastructure`, `tool`, `utility`, `framework`;
  no domain labels like `development`, `coding`, `platform`, `software`, `app`
- **Every word must be a concrete feature** the project actually does — a
  verb, a mechanism, a library name, a command name
- 30–60 chars total (long enough to be descriptive, short enough to type)
- The primary keyword for the utility must appear in the name

These rules are enforced in
`scripts/scaffold_feature_repos.py:_validate_name` and the
`--validate-name` / `--self-test` flags.

## Supported façades (descriptive names)

| Utility | Façade repo | Canonical monorepo path |
|---------|-------------|-------------------------|
| `dracon-sync` | `DraconDev/dracon-sync-background-auto-commit-multi-remote` | `dracon-sync/` |
| `dracon-system` | `DraconDev/dracon-system-di[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBFS0ZmYkhKM0dDdnczeHF4SXY3aWVXSkhrUGV4KzRSRUZWajRRODRxalNjCkRTMHdMclVYc1lLYkNJQmhHS0x4K01LbFJUUTNwbWN5ZUFiMkNQV3NROWcKLT4gWDI1NTE5IHJWbXg0RDM1d1FUL0VXWU1CTmlBWU4wVURUYnpWVzlGZ1pDSTJsV1FqbWcKYWhKUi9aWHNOTEMrUnB0Z2ZWWDdYckJYRmpQMnNxOExjRnAvaktXYW5LTQotPiBYMjU1MTkgT2pubFB2NVZSMTAvT3hWMGZ6Q1dhT3ZiWjFyMUw2N0pHaEZPNVhzWklBawpJbnVwSFlDZnE3Nzk5K3FXOHltUkViVFVvNTlTUEZPTlMyaFVWRVdjMkxrCi0+IFgyNTUxOSBwbUxUMHZTZ2JYaXJKdHlSSE42SE5kdTArZHU1S204N3YwOWE5di9OMjBvCmtBQ3FUOWpScExZWkFWaU5DTktUV1BLYVJhMmJTQ0NTb2cyQ0k1K0NHMWsKLT4gWDI1NTE5IEIrTmNNVEtWeU1MYk9OM21udEhRZG1YU3ZxT0lYV0FyMlpqa1JmSS9uUjQKYVZXWURleGVOcmhUYU5YOHJYNlFZNDRNUW9nSjJ5RWEyMVJoTnpqNE1JawotPiAsVGNPW0QkLWdyZWFzZQpYZTZxc3VVdmZXTlZxWThBTDFrRXg5MmViZzJIRHcvQTIyQ29nYUVZZmxJVE1SbHo1bTBFSzV2VlVXdwotLS0gUzIzdFB0WDI1bmpmakFVQWJhYjhMUlJLOHdlSDQrb1dudG1rUHA4L0ZnZwoH6No3y+OTxWIHUPXGm7//IQNR3DXhMrHElqJVzVagGWeCQPQG9/N74Ro/RBkzQUatgeltLb6N]` | `dracon-system/` |
| `dracon-warden` | `DraconDev/dracon-warden-secret-encrypt-age-git-filter` | `dracon-warden/` |

Mirrors exist on:

- **GitLab**: `DraconDev/<descriptive-name>`
- **Codeberg**: `dracondev/<descriptive-name>` (the primary discoverability target)

The first set of names deployed in 2026-06-16 (Set A — man-page-style, e.g.
`dracon-sync-watch-debounce-commit-push-mirror`) was replaced the same day with
Set B (this section's table) because the operator wanted names that read as
sentences, not feature lists. See the [CHANGELOG] for the rename entry.

## Invariants (v0.112.7 — facade repos are now mains, not shells)

**This is a deliberate inversion of the original v0.112.5 invariant.** The
operator (goal `6a105c59` / 2026-06-16) pushed back on the shell-only
architecture: "are they mains? we are not pushing to them they are still
shells". The new architecture:

1. **Each façade repo is a canonical "main"** — it contains the actual
   source code, `Cargo.toml`, tests, examples, and the per-utility README
   from the monorepo. It is **independently buildable**: `git clone
   <repo>; cargo build --release` works (with sibling `dracon-libs` +
   sibling `dracon-utilities` for `dracon-warden`).
2. **The monorepo is the dev workspace** — it is where the operator
   develops and where coordinated changes across all 3 utilities are made.
   The monorepo's per-utility subdirs (`dracon-sync/`, `dracon-system/`,
   `dracon-warden/`) are the **source of truth** that the auto-sync
   mechanism (`scripts/regenerate_facade_repos.py` + `post-commit` hook)
   pushes to the 3 façade repos.
3. **The 3 façade repos are 4-remote aligned** (github, gitlab, codeberg,
   + monorepo path) and auto-pushed by the `dracon-sync` daemon.
4. **Sibling layout**: each façade repo's `Cargo.toml` uses path deps
   pointing to `../dracon-libs/...` for the internal `dracon-git` and
   `dracon-system-lib` crates. The `dracon-warden` façade also depends on
   `../dracon-utilities/dracon-warden/src/security` for the `dracon-security`
   kit. The README in each façade repo documents the required sibling
   layout.
5. **Regenerate façade repos** with `scripts/regenerate_facade_repos.py
   --all` so the per-utility source content stays consistent with the
   monorepo.

## Why this is not a hack

GitHub, GitLab, and Codeberg cannot natively present a subdirectory as a
first-class repository with separate issues, projects, topics, and README
without duplicating or moving files. A façade repo avoids both bad options:

- Moving code entirely to the façade repos would lose the monorepo's
  coordinated-build advantage (one `cargo test --workspace` for all 3).
- Copying code by hand would create drift and duplicate maintenance.

The new architecture uses an **auto-sync mechanism** to bridge the two:
the monorepo's per-utility subdirs are the source of truth, but the façade
repos are full mirrors of that content (with a `Cargo.toml` adjusted for
the sibling layout). The `regenerate_facade_repos.py` script + the
monorepo's `post-commit` hook keep them in sync automatically.

The façade repo is therefore a scripted, one-way mirror with a sibling-repo
build layout. It owns the **publishable surface** of each utility, while
`dracon-utilities` owns the **development workflow**.

## Maintenance

### One-off regenerate + push (per utility)

```bash
# Generate + commit locally (no remote writes)
./scripts/scaffold_feature_repos.py --apply --init-git \
    --target-root /tmp/fa-test --repo dracon-sync

# Replace a single utility's README + commit info in an existing clone
cd /path/to/<descriptive-name>
# copy the new README.md in, then:
git -c user.email=dracsharp@gmail.com -c user.name=DraconDev commit -am "..."
git push github main && git push gitlab main && git push codeberg main
```

### Validate the constraint set on the current names

```bash
./scripts/scaffold_feature_repos.py --validate-name
./scripts/scaffold_feature_repos.py --self-test
```

### End-to-end scaffold + push (new utility)

```bash
./scripts/scaffold_feature_repos.py --apply --init-git --push-all-remotes \
    --target-root /tmp/fa --repo dracon-sync
```

The script writes only these files in each façade repo:

- `README.md`
- `LICENSE`
- `SECURITY.md`
- `.gitignore`
- `.github/ISSUE_TEMPLATE/feature-or-problem.md`
- `.github/CODEOWNERS`
- `docs/SOURCE_OF_TRUTH.md`

With `--init-git`, the script also runs `git init -b main`, makes an initial
commit (`--no-verify`, because the warden pre-commit hook rejects repos that
do not yet have the warden filter configured), and the three remote targets
are added by `--push-all-remotes` and pushed sequentially to
github → gitlab → codeberg (sequential push is deliberate, to avoid the same
race condition fixed in `dracon-sync`'s `multi_remote` module).

## Relation to `dracon-sync repos`

The recurring `WARN` rows in `dracon-sync repos` are not solved by hiding rows
or changing the table labels. They are a signal that tracked files changed and
the daemon has not yet produced a pushed commit for that snapshot. The façade
repos are not a workaround for sync state — they are a presentation surface.

## Release history

- **v0.112.5 (2026-06-16)** — Set B names deployed + auto-sync infrastructure
  added. The 3 façade repos were renamed in place from Set A (man-page-style,
  e.g. `dracon-sync-watch-debounce-commit-push-mirror`) to Set B
  (sentence-style, e.g. `dracon-sync-background-auto-commit-multi-remote`).
  See the [`CHANGELOG`](../../CHANGELOG.md) entry for `0.112.5` and
  `release-notes-v0.112.5.md` for the full story. The
  `scripts/regenerate_facade_repos.py` script + a `post-commit` hook
  automatically regenerate the 3 façade repos when a utility's source
  files change in this monorepo. The 3 façade repo clones live at
  `/home/dracon/Dev/facade-repos/`, a path the daemon already watches.

## Repository architecture

This is a 4-repo system. Each repo has one job:

| Repo | Role | Contains | Updated by |
|------|------|----------|------------|
| `DraconDev/dracon-utilities` | **Dev workspace / build source** | All 3 utilities' source code + monorepo build + `install.sh` + tests + docs | Operator (manual commits) + `dracon-sync` daemon (auto-commits to all 4 remotes) |
| `DraconDev/dracon-sync-background-auto-commit-multi-remote` | **Façade main** for `dracon-sync` | README + LICENSE + SECURITY + .gitignore + .github/ + docs/SOURCE_OF_TRUTH.md | `post-commit` hook → `regenerate_facade_repos.py` → `dracon-sync` daemon |
| `DraconDev/dracon-system-di[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBQM3VocjE0S1FtRVZSN2lkTXBUYllieFp3dHVNaHc3K09TdU8zNEFXSlhZCnlVS0pxSlNkMzRLZC9DUjc1UHowZUZuVmMzS2FsRVRjZUZiM0lKUXlULzAKLT4gWDI1NTE5IFd5aThxZlZrNEVBY2tlcUlQZ1hGbVU5T1VQOTJkYUo1dnIzRm1GR0ZHV28KamFvWk13RzZxZkRGUVZnSzl1RVpqYk9PVGVaZHlDWDlPOVRXc2lLaUlldwotPiBYMjU1MTkgSzMrWU1hbTkzTDlicHlLMTE1Qk5DZHpKb2JZWkF6TzFPR1ZoaThpZlBqawoxVVVQMGtvbUhZdVlnaDJkZXZjM1k1bnNqNndTN2NZYjJZUXkyeFNGZVA4Ci0+IFgyNTUxOSBLOERxcmJLZFFJNDh3Q1R6QldPeTQvT1dDbWpFWm9ycTBiWXM2Qm5aNHlFCjVpYzd6M2oxVTNPbnJOWVMvVTNPd2VKcmdKc0hOK25TbjBtSjFtZUhGNlkKLT4gWDI1NTE5IHBHSWMyTmdPQ1ZHUGUvNEJIWDE2eXUxNk9BNlpvMlQxY2JPamJ3QWJuUUkKdTB2RitQNTdBaUFoQ0lraTNhc1VIa0ZCdVlzYjBOeVgwT1k5NFcreWhjTQotPiBHS2BJXi1ncmVhc2UgX01HQD1MOyA6V3cgRkpyI0chUEwgWUdICisxaGNzMDFUNTdBU0lBCi0tLSA0cHpTZTdhOG8xVVhja0t6czc3RmR5b0piZjc5WHJidXlWTlB0YXk2Y1VnCqOmJHDm0t8nYbayBC6wGeETVaxYn4lcHXHirZ5r5GNQFMUMGzChlTXuHOnAtDUoMMTKDtQRxwc=]` | **Façade main** for `dracon-system` | Same 7 files as above | Same |
| `DraconDev/dracon-warden-secret-encrypt-age-git-filter` | **Façade main** for `dracon-warden` | Same 7 files as above | Same |

**The 3 façade repos are the canonical "mains" for users** (presentation +
discoverability). The monorepo is the canonical "main" for builds
(`./install.sh` clones the monorepo, not the façade repos — the façade repos
contain only presentation content, not source code).

**The flow is one-way**: operator edits code in the monorepo → commits trigger
the `post-commit` hook → the hook runs `regenerate_facade_repos.py` for the
affected utility → the script writes the new README + metadata to the 3 façade
repo clones at `/home/dracon/Dev/facade-repos/` → the daemon (`dracon-sync`)
sees the local change and auto-pushes to GitHub + GitLab + Codeberg.

**Why the monorepo is the install target, not the façade repos**: the façade
repos are deliberately tiny (7 files) so they render fast on Codeberg / Forgejo
and don't expose the implementation. Cloning a façade repo would not give you
a buildable source tree. The source lives in the monorepo + `dracon-libs`.

**Why the façade repos exist at all**: search discoverability. On Codeberg /
Forgejo, descriptive names get upvotes and free attention. The brutal-descriptive
name is what makes someone click. Once they click, the README points them to
the monorepo for the source and to `install.sh` for installation.

## Cross-references

- [`CHANGELOG.md`](../../CHANGELOG.md) — release notes
- [`release-notes-v0.112.5.md`](../../release-notes-v0.112.5.md) — full v0.112.5 release notes
- [`scripts/scaffold_feature_repos.py`](../../scripts/scaffold_feature_repos.py) — generates the façade content
- [`scripts/regenerate_facade_repos.py`](../../scripts/regenerate_facade_repos.py) — auto-sync glue
