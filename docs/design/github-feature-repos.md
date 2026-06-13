# GitHub Feature Façade Repositories

Dracon utility façade repositories are intentionally small GitHub presentation
surfaces. They make `dracon-sync`, `dracon-system`, and `dracon-warden` easier to
feature on GitHub without splitting the implementation out of the
`DraconDev/dracon-utilities` monorepo.

## Invariants

1. The monorepo is the only source of truth for implementation code, tests,
   release packaging, and changelog entries.
2. Façade repos contain only navigation, issue/project metadata, licenses, and
   links back to the monorepo paths.
3. Do not copy implementation files into façade repos. If code needs a real
   separate home, create a genuine crate/binary repository and update the
   monorepo architecture docs first.
4. Regenerate façade repos with `scripts/scaffold_feature_repos.py --apply` so
   the presentation layer stays consistent.

## Why this is not a hack

GitHub cannot natively present a subdirectory as a first-class repository with
separate issues, projects, topics, and README without duplicating or moving
files. A façade repo avoids both bad options:

- Moving code would split the implementation and break the current release
  pipeline.
- Copying code would create drift and duplicate maintenance.

The façade repo is therefore a documented, scripted boundary: it owns GitHub
feature metadata only, while `dracon-utilities` owns code and releases.

## Supported façades

| Utility | Façade repo | Canonical monorepo path |
|---------|-------------|-------------------------|
| `dracon-sync` | `DraconDev/dracon-sync` | `dracon-sync/` |
| `dracon-system` | `DraconDev/dracon-system` | `dracon-system/` |
| `dracon-warden` | `DraconDev/dracon-warden` | `dracon-warden/` |

## Maintenance

```bash
cd /path/to/dracon-utilities
./scripts/scaffold_feature_repos.py --apply
```

The scaffold writes only these files in each façade repo:

- `README.md`
- `LICENSE`
- `SECURITY.md`
- `.gitignore`
- `.github/ISSUE_TEMPLATE/feature-or-problem.md`
- `.github/CODEOWNERS`
- `docs/SOURCE_OF_TRUTH.md`

Push the generated façade repo normally after reviewing the diff. Do not push
implementation code into the façade.

## Relation to `dracon-sync repos`

The recurring `WARN` rows in `dracon-sync repos` are not solved by hiding rows or
changing the table labels. They are a signal that tracked files changed and the
daemon has not yet produced a pushed commit for that snapshot. For large or
actively edited repos, the long-term fix is to keep the daemon's operation
timeouts per git operation and to use the façade repos only for GitHub feature
presentation, not as a workaround for sync state.
