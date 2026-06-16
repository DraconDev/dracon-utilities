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
| `dracon-system` | `DraconDev/dracon-system-di[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBGaHdNYjYxZUpibSthT08xRHU2RmVNbEtwY3QyODFqKzFCMjVmeVZTWEJJCm1qS3o1cEI1eDVSQ00vblpMc25xS282ekczVUFnTUdUa2xLajNJUlp6RFkKLT4gWDI1NTE5IE92UVBYU1BUa2RFUkZHOW1hOGpIa1EvOXNpZHZrY3Q5SnlwdEJNYmIxbnMKZm8vaU5kMGp5TWl5MWdvRWV3WC9KQ1d6YmVnbDREV0MvRUNVVFF4LzdwUQotPiBYMjU1MTkgd0VTaTBoZHlNR3R5a0dCeUV3Nk1RemFZTEVUWjRYSzVZTkFxcUMyTzloYwprYWh3akIrcFZMcHQ3ME5kKzRlYkh3Mk5kOWxBc3ZybFpWRDBaSjNPd2dnCi0+IFgyNTUxOSBZOEJma2JRTVBteWRpVTZCbTlUdm5ISnlBM1FSY3BqaWZxMGplWGkrUWdnCkp6bWs3L3dFazZNMGRqb2FzQUlDYnR2dWUvNWRkWDV4dmZQdjZFZWhMNzQKLT4gWDI1NTE5IHQvVWQrTXVkaHJmVHl1U011b1F0K0xHYmR4MUZXUGtUWUNPU21QUnV1eWcKNnFiTmI2djJJTWJpYzVQejhkaHZiTGcxZzJ2QVZXWTdsWERvNjc3NW00bwotPiBPRyJtLWdyZWFzZQpEcm9ZY0EKLS0tIElyTzFMNno0b1FqNm1wbng5UUVhcytFUGdMTWd3Z2NveTkwdzcvam5PVWMKLXymqUqfXZQ4FLrWqlXI5qLhA/CmT++jdgL6Uu96EXNXVgfDffC/AUgGqTjiNLmpPpp/gS7JjQ==]` | `dracon-system/` |
| `dracon-warden` | `DraconDev/dracon-warden-secret-encrypt-age-git-filter` | `dracon-warden/` |

Mirrors exist on:

- **GitLab**: `DraconDev/<descriptive-name>`
- **Codeberg**: `dracondev/<descriptive-name>` (the primary discoverability target)

The first set of names deployed in 2026-06-16 (Set A — man-page-style, e.g.
`dracon-sync-watch-debounce-commit-push-mirror`) was replaced the same day with
Set B (this section's table) because the operator wanted names that read as
sentences, not feature lists. See the [CHANGELOG] for the rename entry.

## Invariants

1. The monorepo is the only source of truth for implementation code, tests,
   release packaging, and changelog entries.
2. Façade repos contain only navigation, issue/project metadata, licenses, and
   links back to the monorepo paths.
3. Do not copy implementation files into façade repos. If code needs a public
   home, create a real separate crate/binary repo and update the monorepo
   architecture docs first.
4. Regenerate façade repos with `scripts/scaffold_feature_repos.py --apply`
   so the presentation layer stays consistent.

## Why this is not a hack

GitHub, GitLab, and Codeberg cannot natively present a subdirectory as a
first-class repository with separate issues, projects, topics, and README
without duplicating or moving files. A façade repo avoids both bad options:

- Moving code would split the implementation and break the current release
  pipeline.
- Copying code would create drift and duplicate maintenance.

The façade repo is therefore a documented, scripted boundary: it owns
platform-specific feature metadata only, while `dracon-utilities` owns code
and releases.

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

## Cross-references

- [`CHANGELOG.md`](../../CHANGELOG.md) — release notes
- [`release-notes-v0.112.5.md`](../../release-notes-v0.112.5.md) — full v0.112.5 release notes
- [`scripts/scaffold_feature_repos.py`](../../scripts/scaffold_feature_repos.py) — generates the façade content
- [`scripts/regenerate_facade_repos.py`](../../scripts/regenerate_facade_repos.py) — auto-sync glue
