# Release v0.112.6 — 2026-06-16

## Summary

This is a **release hygiene + architecture formalization** release. It packages
the work from goal `83e42c15` which made the 3 long-name façade repos the
canonical "mains" for the project. The release notes below also list those 3
long-name repos as the **install targets** for users who land here from search
or the GitHub profile.

## Install targets (canonical mains)

The 3 long-name façade repos are the **canonical install targets** for users
who want to learn about each utility. They are deliberately brutally-descriptive
so they are self-explanatory on search engines and on Codeberg / Forgejo.

| Utility | Install target (GitHub) | Also on |
|---------|------------------------|---------|
| `dracon-sync` | [`DraconDev/dracon-sync-background-auto-commit-multi-remote`](https://github.com/DraconDev/dracon-sync-background-auto-commit-multi-remote) | [GitLab](https://gitlab.com/DraconDev/dracon-sync-background-auto-commit-multi-remote) + [Codeberg](https://codeberg.org/dracondev/dracon-sync-background-auto-commit-multi-remote) |
| `dracon-system` | [`DraconDev/dracon-system-di[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBta1pjNy9kTkRXcFllMEg1Uy9WU2ZUSFo1cm1iNDNCT2J2N0VnSzVXblVzClp6bmVmWUVkZWhmN2txZzBpalQ2YldoNlNvbmtCa05CNlh2b250eWdmdHMKLT4gWDI1NTE5IDA0WkdTTXBOU05STWZPZm5VWUtWVGl6aFNRS0p1KzA4cXdLVHBubnM3bk0KeWVCN01BVkY5WUpyQy85djQ0QmthN2FRQXJZdnFuVTU4Rk5uWmRyT3R1ZwotPiBYMjU1MTkgYUhtLzhDUEtNWDZETmR2QThOcnJVMTBUM00rRWMyM0JvR1I4UGtMVjFoOApHcHBKcm1YZnlTUVJyd05BN0NXS0VqaWJ3dU9WZ2xaTXYvdk9ONkNwUkM4Ci0+IFgyNTUxOSBwYUR4cGxHNExLS0p4Q2FQemwwZ0xhYTJnSk5GRW1McmIrbERadVNzNlZBCkxuaFc2ZzFJVmJwZUJlY1B5Y0h5NjkvdnZ3N2pFcXVyU0djQVdkcitXSDgKLT4gWDI1NTE5IFRJeks0aGpRQkFiZWFNaVhWZ21lNHRHOCtlbFVHeFpEcndYUnhnNjFTQjgKVTVjWndvZFJTeTFBTlVra3Vxak9YV0xrYVFqSnRWWjd3bHRPMW44UmxwYwotPiA1XScnQUItUy1ncmVhc2UgXmduNCsqIF8KaVhZQU5ZQ0MxenB6eExnN0V6aFF6MHFIRjBWTXF0a3k1dFR4dUlvOVZoTkRQYnN2QzB1azVUZE05a1FCNWF4eAp3K1VESDBtWGxWMAotLS0gajh5MkNuRStsKzhGekFhZGxGc05VUExPVWhvTFVyc2hSTktieHo0eVcyTQonnZ7zXjR4pdTjv8uIi7ABHicJz7uWMuFXjktiN/QOVXDVs/PwRGRZ+OBOyg8GQbK5EEfXd00P]`](https://github.com/DraconDev/dracon-system-di[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBtOEtHWmFrcDI3Y2ppaEsxWjdHKzRiWklLRjN3dnJUSGYza0FZaEs2MEFnCndDbTNJbm9qeTN2MEpBSE9xNEJTdTVnb2Yrb09BV29jOXlTdDNZOGJZZVEKLT4gWDI1NTE5IGxhc0ViMHhNVmFFYmh2RC9hYzE4KzY1NXJXZFY5V2R0U0d2eVV6ZWtkM28KSXJiZUNNcXNja2RxTmExOEZmOU5wZFhobWRvZ1ZOSnpRQXRlM3RjNVRUSQotPiBYMjU1MTkgb1FVdFVUTWFaZndIUm5NSnBydDRPQ3pqVFFsZWNQQWIwMTdNWE9sdmt3WQpLK011Umh3ck02ZTVNMW9WblRsdEZHamdDYTlFUGxrRlJVY1RWN3dCUk4wCi0+IFgyNTUxOSBGYzU5N2Y1cGl3UXVzcTh3N254bUx2a3h2QkpYUE5xMUJwK2xTdUdaUDFJCjIzV2U2K1QzZ3JkaDdiN2dPUjl3bjBYNkRMVVdDbGRvVXV5OGRGeWNlNUUKLT4gWDI1NTE5IDNmcWVQelZKdWJ1WGYyTkR3TDlIT29CNUduUHhkdlUvMWRWS3RpU0w1VDAKUHFsclpCNU9BNWZqSVpnQzBpRjZLYmdnRWthWGRtdDhPM0JXNm1vSTJ1TQotPiBUcz9sI1IoLWdyZWFzZSBHWiNtICNPPzohIyBzM2dMd0dVClJhRU1UaGpleHM4eGF0dVVLWDBuenBlZnJ3ZndxZ3AzdWllYXJDb3Bxbmpvb1Q2MThYSUhvZGhUL0FHa0pMbkkKK2VyK21NK2dQVlpLamsxYm1sRUpYZTAKLS0tIEIrZGt3eDF3TEZzSjhWTmx1QVNlOUFBVWw0cnk0cE5kejdTNDRVVTk2RXcKKfXRY6soxxkIPS5396gWCUMFqHTBL2p0b1phRBQDPisUX/xH99BP79QsLDQ39MNyIbO74Ce6rA==]) | [GitLab](https://gitlab.com/DraconDev/dracon-system-di[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBRVHRsUEllS2ZWREthWUdUaEFFQ1lRNmM5RTVGbHc5YU1wMmwrN09kMGpnCldvQjNRaTNaRmNWQVp0cDJQTDZCd1lDTFBlUGk3SkRlRUw5aHlGVUREZmcKLT4gWDI1NTE5IHEweTRDSlQzcEJkbVdJekpxaytPNGYrMEtBMlhrd3FQMjBsVktDRFV5RlEKczNScHBsS3h6ckFuOWZOR0FVd1FnakRKWDlRcFV0ZmQwM1R5RnFCVk5JUQotPiBYMjU1MTkgNzFDYzl6M0Q1MkJIK09CUTZxMkVDMHY2NWZUb3RISEhTMUVTMXVsRUVrMApXRGJ5UXVGNnhLVjR6MUtyNGFGdGEzM0VwcXhodVUyYnN2SjFyRFRoZjAwCi0+IFgyNTUxOSBpTnJnU3A4cHlkQmJJeW4rY21TcVZIOWcvbFBUc2pFeS9EUlFkWnF2cVRRCmpvbUZOTmNxY2IzNnM5a1IyTERIYllBeDB3N1FVeWNFYVd6bDdHdFJObkkKLT4gWDI1NTE5IHdWMjFFcDdHY2d5eUJHT3RrRTBHWjFUY3VmN3R2QiszMEswVG9Ob3lKMlUKV2V4dUJRd3lsbk9JcjhaQVJadTdLRzgrbTRleUZIOG1uV0FLNk8vaExvVQotPiBsMC1ncmVhc2UgPCA4eyZRUCBxcj0gaTpTCnFqbHFVa0tyZUxDMnYwN0FsNGVLY2FJNGZRZGduL3hOeGZUMkQ0eGhmdkMyQk1ucHJFT3FEZmNyWi92L3U3Q0oKMGt6MzRaaFhINkJwVnBiTmszeTdxN1NxRkljUk11NnhySUxUN2FKQ0p5TlR3YjhreUpuc0k5YwotLS0gZmdONUpWYnBkNVhkdmlTcE9RVmptejUrZkdvYjV0SVVtbDZNTGVQNEtDZwrSwgy1Gp8wljzdy7nOwjqLoEJUP3naaErisXas2ohWnnyuqHrogfVL87dEfOzitxNZce0uvJ+9]) + [Codeberg](https://codeberg.org/dracondev/dracon-system-di[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBseWliR2ZteVRKS0l3ZEN6d0hiS2YrcXVhbzluQXFyUjNzNXhYYjBHOEZJCjczYzNyM1hFV01WcHg5ckZ0S2NEUk1JbTIvMDlYNkxZdWhMT1V2aHhJSHMKLT4gWDI1NTE5IHlKcUEvSThVbmdGT2xpcVI1RERIbklWbU5hc09VSTcraWxpc0l3MXVjV2sKZk0xOFR4T3g1cDdCR3JYQlljRTFyLzFIMm5nSHhzZE13LzAyc3JjVStKdwotPiBYMjU1MTkgd3h6aG10MVJVa0g1ZFkrVzc2T3U0N3pidVkrUVJBa3hRSkhNOWVnSnFSSQpla3VRd1dkb1N4TURESEFGaExVRWM0WjY4Y0xsV0NDaEJhY0pXMXBiUHFvCi0+IFgyNTUxOSBSait5Z21MZFNwTTA0TDIyR0h1VVVkelo3akNDdURoYnE3ZFlpN1ZkQlJRCkRyQ01qR3JrOC9lTFlNMmZwcHF1Y01KWUc3UmxjMExiUWdwTmIraFpxZ1EKLT4gWDI1NTE5IDNKVlp1V0krT0RGYnRnZ0F1ZCs2aXRVT3VDQjJLckJIY1l4SUJvYWJaR2cKVFdpQzRURXlFSXAzYksvMythSUVVKzFhVTB1dmZIUzdwZ0JwYi90YzU4OAotPiBLW1FuLWdyZWFzZSAlTk0veyBPOSA7IilBfEddTSBCVFRvY143cQp6RWI5K0NEeXU4eE0vWmlFQUJxSU1ZTmdrZwotLS0gYjJHZXd0N3VXa3VyK0Z1T3lTVnkwT0RWUjhOdmE1dk83dE1PaXRhOFdHdwrzwDJSu+lil0YpQTHDgnL2lrFgi3BEBMMKkjpqW5Vb/AIkjpe8ttLkvNV/Xcg5gOk2qzpW53oZ]) |
| `dracon-warden` | [`DraconDev/dracon-warden-secret-encrypt-age-git-filter`](https://github.com/DraconDev/dracon-warden-secret-encrypt-age-git-filter) | [GitLab](https://gitlab.com/DraconDev/dracon-warden-secret-encrypt-age-git-filter) + [Codeberg](https://codeberg.org/dracondev/dracon-warden-secret-encrypt-age-git-filter) |

## Repository architecture (formalized in this release)

This is now a 4-repo system with distinct roles:

| Repo | Role | Contains | Updated by |
|------|------|----------|------------|
| `DraconDev/dracon-utilities` (this repo) | **Dev workspace** | All 3 utilities' source code + monorepo build + `install.sh` + tests + docs | The operator (manual commits) + `dracon-sync` daemon (auto-commits to all 4 remotes) |
| `DraconDev/dracon-sync-background-auto-commit-multi-remote` | **Façade main** for `dracon-sync` | README + LICENSE + SECURITY + .gitignore + .github/ + docs/SOURCE_OF_TRUTH.md | `post-commit` hook → `regenerate_facade_repos.py` → `dracon-sync` daemon (auto-pushes to all 3 remotes) |
| `DraconDev/dracon-system-di[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBWWWVmZTBYOTMxTE9xdllCNmJwbUd1eGR5UEdaRUE5TDVsSUI5S3RxanlVCjRPa0p6SktpT1kvNDNFSnA3SUlnRlhpcjhqNDhrVlYySXdRSU1IU2lZdXcKLT4gWDI1NTE5IFdGVWRlVGFXd2NXVmZzM2Q1U1dqTTFjRG1zd0l1V21lNWxpSHU0bGErbTAKTWlVWFpVNjhQRTVFNFMybzVqQTZlNGw3a0NaRnBwbEdCRkRreFVrVUNGcwotPiBYMjU1MTkgLzB6L09sRVpDZW1zQkkreWlyZnJCRmR2WXNHTDJlU2M5VVZDK1FXUVRnYwpscGpiYjRwc2F4Slg2VWpaL3VQb2ErR0FlWnVUU2d5NVhEaEd3Wi9NUHNjCi0+IFgyNTUxOSBEVHZvNG8zS3ZON1lUWWovc0JjM2RHckFMbXN3VnJQTXBGdTJ1MmdhZTNrCnVydERIM2F4bG9VcmkrTmg2TmFzU0puZWk3aVZxc1dncDFNaG0zeDNQZkEKLT4gWDI1NTE5IE1xT3JQR0E0MG1DMldkb1BRbWhVS0twNGxlaWcxazNVcXpDUTBWQ3JZSE0KZENacjhoN2pkeUNWekZVSDNNZ21xNnJUZlp6MEhZenEzZ2dUdXAwckg3awotPiAxQHI8PSJbLWdyZWFzZSBuK0A0Q155SCA0ClhmdG54czB0MlE2VW9KUzExYy8yOEhxTExNanFlQQotLS0gWG8xLzdGNWRZUE90SmFQRjcwME96NmNtd0hSSkZkQjlQWmxKZHU5MDJxbwrYwcUncrA6W2DA4oMGAhW3s8PyMa+N5klOZyZimCn9zG+qJlX2fhbpNWGGOwm179QWsKrvkN48]` | **Façade main** for `dracon-system` | Same 7 files as above | Same |
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

## Version bumps

- Root workspace: `0.112.5` → `0.112.6` (patch-level, doc/infra only)
- `dracon-sync`: `0.1.6` → `0.1.7`
- `dracon-system`: `0.2.1` → `0.2.2`
- `dracon-warden`: `0.3.1` → `0.3.2`

## What's in the box (since v0.112.5)

The full `[0.112.6]` CHANGELOG section includes:

- **Repository architecture formalized**: root `README.md` and
  `docs/design/github-feature-repos.md` now have explicit "Repository
  architecture" sections that document the 4-repo model
- **3 long-name façade repos are the canonical "mains"**: referenced in
  root README, design doc, daemon watch list, scaffold/regen scripts, and
  release notes
- **`/tmp/fa-clones-b/` cleaned up**: the 3 façade repo clones were moved
  to `/home/dracon/Dev/facade-repos/` (a daemon-watched path)
- **Old short-name repos deprecated**:
  - GitHub: 0 (the Set A→B rename was in-place, old URLs redirect)
  - Codeberg: 0 (hard-deleted during the Set B migration)
  - GitLab: 3 Set A repos soft-deleted with `_deletion_scheduled-XXXXXXXX`
    suffix; awaiting operator decision for hard-delete vs archive

## Verification

- `cargo build --release --locked`: succeeds
- `cargo test --workspace --locked`: 856 passed, 0 failed, 9 ignored
- `python3 scripts/scaffold_feature_repos.py --validate-name`: passes
- `python3 scripts/regenerate_facade_repos.py --dry-run`: passes
- 3 Set B façade repos: 4-remote aligned (`da0dbf5` / `b37781a` / `8ceb070`)
- Monorepo: 4-remote aligned at release SHA
- Daemon: 4 repos watched (1 monorepo + 3 façade repos), all healthy
- All 3 façade repos are the install targets in this release's release notes

## Migration notes

- If you cloned the old Set A repos (`DraconDev/dracon-sync`, etc.) on
  GitHub, the rename in place means the old URLs redirect to the new ones
  for a grace period. Update your bookmarks to the new long names.
- The 3 new façade repos are at `/home/dracon/Dev/facade-repos/` (a path
  the daemon already watches) and stay in sync with the monorepo via the
  `post-commit` hook + `regenerate_facade_repos.py`.
- If you commit changes to a utility's source files, the `post-commit` hook
  fires automatically and the daemon pushes the regenerated façade content.
  No manual `scaffold_feature_repos.py` invocation needed.

## What's next

- Operator to decide on the 3 GitLab Set A repos (hard-delete vs archive vs
  leave-as-is) — this is the only remaining open item from goal `83e42c15`
- The 3 façade repos will now stay in sync with the monorepo via the
  post-commit hook + daemon auto-push
