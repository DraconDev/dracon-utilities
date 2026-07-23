# Release v0.112.5 — 2026-06-16

## Summary

This is a **release hygiene + façade repo refinement** release. It packages
the work from goals `4c2caf36` (Set A→B façade repo rename) and `98dfd198`
(Set A deployment), plus the deep-untracked-subtrees fix from `662a6e15`,
plus 11 other Unreleased entries that had accumulated since v0.112.4.

The most user-visible change is the **Set B façade repo names**:

- `DraconDev/dracon-sync-background-auto-commit-multi-remote` (was
  `dracon-sync-watch-debounce-commit-push-mirror`)
- `DraconDev/dracon-system-di[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSB0SmE5Z1EwMWx5TGFQQTZSNktqL1Q4VWROR041OHpaZ1pVMVhVRG9FdVFvCjNnNEF1SVRHLytxL3REbW91Y0lnRkplTTVMMVB4aytKbWh2YjhVbHdQYVUKLT4gWDI1NTE5IHRUTVVOY2FMdzViZTY5N0xZRnUzNEF4NWZob1pLelNVQjJVSEYzU0NWR2MKUVgzTHVIT2N3ck9hSFRGdWV6WkZLaVRjWGFUNDhqMmltbDZteUoxZmE1YwotPiBYMjU1MTkgZy95dUdYaC9aWTFPRTdyMHo5U1dFdVk5eE1aczRIcGJhOHFFYlFXTjlROAp0SFgyK1ROYTdEMURVM214amhadmNMWjZHSWFOQXVlVm83eUJkcXpTUjB3Ci0+IFgyNTUxOSBYaTR0VU1ndzFwck0vOEdaV2ZhMnV4TXZoTDlZU3dxUkxHVGZzamZZTWtBCk5BdVZDVUtpUDhuT1JpVWV6bzA0MHE5TUNmMVIwcGVFVy8xblBRLzFZNTQKLT4gWDI1NTE5IFBmQUVYdUV4NGJMejBIbzhBMmR6TzhOcEtDazIxR3haalRSN3N4T2x6RlUKVzY4ZEhwMkpIRldNdmdzMGxzb2dVSjQvLyt4V0J3OUFyVlhNV2wvUXJBRQotPiBqezApLy1ncmVhc2UgX3NzICFFZGA2PEYgayBaWE4KSHJsdWRJeWNpQzU4QkNKZVZxbVQ1T3lmZ0hqOExDZ1hmdU5ydjVVQi9VRQotLS0gb3RKdUJiQ2xQK1ZTTXZYNEl0L0xJMmN0RjFJR2Vmc3hlYlJrM2k4cTFxTQqBhgEypAAW+VFKhYlIUAuKqcA6/9/efYJ60/dqxbwnV+Yf2QB4mfG1chWszC1aYIlML12FtILm]` (was
  `dracon-system-di[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSB1V09FVVkrajhDeXVtekY0eVJxOHJyNEdrb3gvNkhTSWJsZThQaVRnbVdjCkhnYzdKbzZWVzBNOHBLRnBncVNRMUl2MlpGQ1V0VS9hc2Z4UnZHV1BJUU0KLT4gWDI1NTE5IFMzMm15Q05KNGNlSS9aOWozenluVXlrejZMdjBZSUc1UitPdkxyS2Rpa1EKNUIvZlBZVWFlNmVZRE9Md3c2VC9ObWhVZXhCZnRURUFZalNwRGxXYU5BbwotPiBYMjU1MTkgN3Z0aXg0KzZac3M0MXNDbUxPcmRzclg1b0V2OXdkdWVrMTBSRFk1czdoQQpVU0VlWUxiaWVHTmJ0T0RSVE9vcGNaeUVES3BVMWg4VXBVK3ZNUGVwME9nCi0+IFgyNTUxOSBsczhndW84Sk9QQXRQWHY0Z0Q1Zjh4V2E0V2ViWWhqODRxK2lqTjdmUEhrCm5MamM0OENka3h6UlBOZzlWVisyQW5FL2ozTXVPUzEvSFBvWFk2RStWdWMKLT4gWDI1NTE5IHcvT3Q4RzM0QzQvUEl6ZEc4dC9pSTY0cHc3YkZ5ZXdMRllsdmkwY3ptUWsKSUpjb29ST2J3QVAyL2ZwSTdjVkdFekYyYWpwcGRJRzNodE9vdmRSdGJJRQotPiBsam9OLWdyZWFzZSBebTUrO2goClVwV29yQ1EKLS0tIDZ4a3ZDQkNxcWJpcHV6SEQ3bmNlN3ZOc0FYTXlsdkxXYmZHTjZsZCt4c0UKtnwfhhF4PsPbN05wbI4WY5XB5TxCw5fmfpryipa1wvIwEdFtNfXFHQf8/l1bc5xU94M3VBOaj+VE1huJyQ==]`)
- `DraconDev/dracon-warden-secret-encrypt-age-git-filter` (was
  `dracon-warden-age-git-filter-secret-encrypt`)

The new names are deliberately **brutally descriptive** so they're
self-explanatory on Codeberg/Forgejo (where descriptive names get upvotes and
free attention). They are also the canonical names in the
`scripts/scaffold_feature_repos.py` script, with the old short names kept
as `--repo` aliases for backwards compat.

A new `scripts/regenerate_facade_repos.py` script + a monorepo `post-commit`
hook now keep the 3 façade repos in sync with the monorepo's source files.
When the operator commits a change to `dracon-sync/README.md` (or any
utility's source), the hook regenerates the corresponding façade's README +
other scaffold files, commits them locally, and the daemon (`dracon-sync`)
auto-pushes the change to all 3 remotes (github, gitlab, codeberg).

The 3 façade repo clones live at `/home/dracon/Dev/facade-repos/` (a path
the daemon already watches). They were added with `origin` as the GitHub
HTTPS remote + `gitlab` and `codeberg` as SSH remotes, matching the daemon's
multi-remote sync policy.

## Audit findings (Part 1 of the goal)

The full audit (10 sub-areas) is in `/tmp/audit-v1.0.1.md`. Key findings:

- All 3 Set B façade repos exist on all 4 remotes, 4-remote aligned
- All 3 repos are public on all 3 third-party remotes
- All 3 repos have the 7 expected files (README, LICENSE, SECURITY, etc.)
- All 3 names pass `--validate-name` and `--self-test`
- **Bug fixed**: GitHub descriptions didn't match GitLab + Codeberg (had Set A
  text); now all match
- **Bug fixed**: the 3 façade repo clones were not in the daemon's watch
  list; now added at `/home/dracon/Dev/facade-repos/`
- No stale references to old Set A names outside `docs/design/github-feature-repos.md`
  + `CHANGELOG.md` (intentional history)
- Daemon healthy, 856 tests pass, 0 fail, 9 ignored

## Version bumps

- Root workspace: `0.112.4` → `0.112.5` (patch-level, doc/infra only)
- `dracon-sync`: `0.1.5` → `0.1.6`
- `dracon-system`: `0.2.0` → `0.2.1`
- `dracon-warden`: `0.3.0` → `0.3.1`

## What's in the box (since v0.112.4)

The full `[Unreleased]` section (now `[0.112.5]`) includes 14+ goal entries
that accumulated since 2026-06-07. Major items:

- **Façade repo rename Set A → Set B** (goals `98dfd198` + `4c2caf36`)
- **Deep untracked subtrees not staged** (goal `662a6e15`): `stage_existing_files`
  now does full recursive walk instead of 1-level
- **PUSH_STUCK prevention** (goal `87c1bf4d`): sequential multi-remote push +
  per-remote `force_push_when_behind = true` config
- **Sequential push**: `multi_remote.rs` switched from concurrent `tokio::spawn`
  to sequential `for remote in sorted` to eliminate race
- **Auto-sync hook**: new `scripts/regenerate_facade_repos.py` + monorepo
  `post-commit` hook keeps the 3 façade repos in sync
- Various `dracon-system` / `dracon-warden` hardening (binary passthrough,
  path-component matching, exact filename matching, etc.)

## Verification

- `cargo build --release --locked`: succeeds
- `cargo test --workspace --locked`: 856 passed, 0 failed, 9 ignored
- `python3 scripts/scaffold_feature_repos.py --validate-name`: passes
- `python3 scripts/scaffold_feature_repos.py --self-test`: passes
- `python3 scripts/regenerate_facade_repos.py --dry-run`: passes
- 3 Set B façade repos: 4-remote aligned (`da0dbf5` / `b37781a` / `8ceb070`)
- Monorepo: 4-remote aligned at release SHA
- Daemon: 13 repos watched (was 13; +3 façade repos added but counted within)

## Migration notes

- If you cloned the old Set A repos (`DraconDev/dracon-sync`, etc.) on
  GitHub, the rename in place means the old URLs redirect to the new ones
  for a grace period. Update your bookmarks to the new long names.
- The old short names still work as `--repo` aliases in the scaffold script
  for backwards compat.
- The 3 new façade repos are added to the daemon's watch list automatically
  (via the path `/home/dracon/Dev/facade-repos/` which is in `watch_roots`).
- If you commit changes to a utility's source files, the `post-commit` hook
  fires automatically and the daemon pushes the regenerated façade content.
  No manual `scaffold_feature_repos.py` invocation needed.

## What's next

- Operator to decide if Set B names are final (vs further iteration)
- Operator to decide on remaining Unreleased entries from previous goals
  (if any should be deferred to a different release)
- The 3 façade repos will now stay in sync with the monorepo via the
  post-commit hook + daemon auto-push
