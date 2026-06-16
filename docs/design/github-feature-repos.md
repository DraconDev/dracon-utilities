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
| `dracon-sync` | `DraconDev/dracon-sync-watch-debounce-commit-push-mirror` | `dracon-sync/` |
| `dracon-system` | `DraconDev/dracon-system-di[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBiMmxXbUxhZG80d0VncUovYThicFRBd3dSN1VnS09wY0FvMy9uMk9pZkJrClJnZWtYUCtGenpUTUhncnJrWmpVVVZra1VITnhuN3FJQXFqdFpiWGd5ZW8KLT4gWDI1NTE5IEZsRlh3UkVxL1ZscEVXK3MzNWpkamFiaHRFbGFKdmkzam9IaWI1Wk01aGsKbDZlZXA1Uk9NTWZ6dFpHUHdkVWxmQkZuVzNyUTM1ZDY4OVdaSkwzZDRtYwotPiBYMjU1MTkgdys4dTA1T0JIZFlDcEJ4c3FjanhGbkZmdWkzZFd2UmxnRmNEMGxOclFucwpRbDZoTkhMQThLNlcvQVBxVzhoOVhpWktURVpwR1J5TTRuTzM2VTZZQWJzCi0+IFgyNTUxOSB2cUxBV1lycUpBelY3Y0R4WGxKVHN3Q0hOdHFaWHZxTXMvdjdTTUFKcmpFCmtlTktieUlUWU04YTlZTHhsNVVtM1F1RVhPZ1o1UTN1NWFMejlwWDVjUUUKLT4gWDI1NTE5IG94NXVOanREY3JJVWIvcjhpMGZYb3JnUllqWWRxRUJHcEk0YStTQkxhRWsKM2V5TUoxdmQwVXk0b0xkZEZ4R2YvSTlzYitCR2xMQnpSWW9FcmlQdEJtRQotPiAjLzlSLlEtZ3JlYXNlClB1VnYvOWtJNHFQbUVhckV0YmZiU0dqcko3L3FkSy9WQUR6cGxGSzJLeHR0YXZJZ0prdFNlMnMKLS0tIGtUaDY0QXErNDlzMFJBQ1NwNTdNNTVocitta2R6bzFHQ3AxL2xVZXVncGMKneTOfbIJP4fFvx4C3oIXv5z5ODBObpnwQAjotgzLW0ukXCfz2j+7oyda+ybF37SqmeFRiq7Y90lOJg+2Lg==]` | `dracon-system/` |
| `dracon-warden` | `DraconDev/dracon-warden-age-git-filter-secret-encrypt` | `dracon-warden/` |

Mirrors exist on:

- **GitLab**: `DraconDev/<descriptive-name>`
- **Codeberg**: `dracondev/<descriptive-name>` (the primary discoverability target)

The old short names (`DraconDev/dracon-sync`, etc.) on GitHub were renamed in
place to the descriptive names — the rename preserves the git history and
GitHub redirects from the old URL for a grace period.

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
