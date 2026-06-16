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
| `dracon-system` | `DraconDev/dracon-system-di[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBsdUFQMVpqUGJBMEd3MjNIY09EaG1yZXdzdDA3SU0xM2xHamc3VmdBRlR3CmVUR0cxR0N0U3Y4Y3hiUUdIekdEMldPZG1Bd3FaMmJBRFZsV3YwS3NJNjgKLT4gWDI1NTE5IHJpQmJUUDZEd2NwS0loNjExZks1RFRoS3NiNVVsbGFQSU1yMTN3SmxLbFkKQVI4OWM1SUVYb01QR3NGOFZPR05qVWs5SkgvT2R5TDBLT3gremZYOUdndwotPiBYMjU1MTkgQWFrbXdRRWpLbDdIQWkwYkVDQ1Zsd05BM0dZVDJUcytNT29ZTHdpRTgzTQpTdUE0cW84T2NURVJ3S1NER0NiTThEYUdOajB3NERMZzdPbVBnd0NSZXVVCi0+IFgyNTUxOSBnSFk4VmE5VVV4dVRhVWR6WU8wVXNQWmdsYzJDK0txN0pWNmplaXBHMmhNCmV0UVBYZzdsZldVOUJRUUEvdVR1ZTMvV2tFRWt0RFIwQS9FMjd2emcwVUUKLT4gWDI1NTE5IGRTVWcvVmZBMkkxYkwxc0YrYmhFbmFuS2ljYVlNWUsrRGV3Zk9majI5bG8KcFptcTJPZ29sYnpzWlQ4M21MZXR2VGc1UDFsdFRSQzVTeW9rcHhYV0ZCUQotPiB3LWdyZWFzZSB9UUUgeiB8Q3AlCklmR2QwUk1EWlZwNkNTQU9FVThPTlBsQ3gxbG9qSUg5Ci0tLSBwK21nWlY5YXZLRWJMbVpkNW55WE5Ha1hldW5MZms2bHZmczZONGFnYXJzCp0V/OSWhYk1006L2YxUv9YvHw4i6dNXI9d1t/u7ZDe5dTddsRSxVgGRnIjU9qX7CtOOI6zCQ0w=]` | `dracon-system/` |
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

## Repository architecture

This is a 4-repo system. Each repo has one job:

| Repo | Role | Contains | Updated by |
|------|------|----------|------------|
| `DraconDev/dracon-utilities` | **Dev workspace / build source** | All 3 utilities' source code + monorepo build + `install.sh` + tests + docs | Operator (manual commits) + `dracon-sync` daemon (auto-commits to all 4 remotes) |
| `DraconDev/dracon-sync-background-auto-commit-multi-remote` | **Façade main** for `dracon-sync` | README + LICENSE + SECURITY + .gitignore + .github/ + docs/SOURCE_OF_TRUTH.md | `post-commit` hook → `regenerate_facade_repos.py` → `dracon-sync` daemon |
| `DraconDev/dracon-system-di[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBZcmgxYWEyZ0VMQ0I3cys5dnJyR1dRQllOeXZ4RnBPbzQvanFkd2tsSGxjCmxRRUlia0VBSUNVQ2cvNmFzeFVtaHM2L2VDNENJK1hPc0ltc2NlVFFVUmsKLT4gWDI1NTE5IHdxdkVaUTdXVitNSWRRK0FlTXM1a0NLaFo3VkZPbTQ3R0JWbEFHeVNMaTAKc1R1YkxTZUIrR2FWU3dNdE5rSHBMN2dJNUVRV1BrZXRlYXZwTndWN2VQSQotPiBYMjU1MTkgTlN6a3lKMUhXT0xBTW5oamthMmhLM2V6T1lwRklVVVhjeVErSmJjeHdSNApkc2lCQ2xSUVI4THdzZmx4SWxsb0VUSDg2MXM1Y2pRNjR2NW9heVp1dDhZCi0+IFgyNTUxOSA4MURYVTltZlJWVUJSRlYvMjFLa1NiRFlWVXpUdjFZc3hzVDhUelFvWkN3CjlZa3JRbTR0Mmw0SUJXZC9LaVl5WmM0eC9qSkFsYUFKbVIyQ2Npa0dINVUKLT4gWDI1NTE5IHppWEJMMUdtQmtwV251WWZMZmVYR2g0NTVJRDRkN1RMVzZobW1pNjg1VmMKMFNYVkhlZzdKZDZqQ055WlZrUStwNkFzTzZnbjd3YWFqK1BpNG5kQWl6RQotPiBsXV8tZ3JlYXNlICdrQm1mN10mIE1yJDwgIQpHT2R0TmlFbzJLbVE1cUphZjhoZktITWhESVZvVVBCb3F0VGJaRi8xMjVUeHc0ZXUyVFB0M1NQaVB6MDEyRkoxCkVORnM1elN5c3B2NmluTGVrUFp5V2Z1NC9kVHV0bHRUclZnRFFIMFlpSWhRRnB5dFpPZlVrM3JPdXcKLS0tIEk2TGVCNGNFNHVWeVV6TmNFZVVVRWFDR0RTZEkzMEFybXU4ellRNDJKOG8KEB0ozfuLTnKWqi5Mkim2Vs+QjzHmGHsP3lhPkxFfZgIjoMmj0supKzSZ7n+GwezdYIz3JZWQkw==]` | **Façade main** for `dracon-system` | Same 7 files as above | Same |
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
