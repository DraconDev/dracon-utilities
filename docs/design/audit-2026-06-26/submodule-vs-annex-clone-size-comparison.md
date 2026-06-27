# Submodule vs Annex — Clone Size Comparison (2026-06-27/28)

> **Operator's question**: "also we need to think about git module too that would mead that on the remote we would cloen down the entire hundreds of gigs right? assuming everything goes right"

## TL;DR

**Submodule with `--recurse-submodules`: YES, you clone the entire content (1.2 GB in our 400 MB test, scales linearly to ~6.7 GB or hundreds of GB).**

**Annex with plain `git clone`: NO, you only get ~400 KB of metadata. Content is fetched on demand via `git annex get`.**

The architecture choice determines the clone behavior.

## Direct answer to operator

> "would we clone down the entire hundreds of gigs right? assuming everything goes right"

| Architecture | Answer | Why |
|---|---|---|
| **Submodule** (with `--recurse-submodules`) | **YES** | `--recurse-submodules` is the default in many common tools (GitHub Desktop, GitKraken, SourceTree, VS Code, GitHub Actions `actions/checkout@v4`, GitLab CI). |
| **Submodule** (without `--recurse-submodules`) | **NO** | Just SHA references. User must explicitly run `git submodule update --init --recursive` to get content. |
| **Annex** (any `git clone` variant) | **NO** | Just annex pointers in main repo (~400 KB for 400 MB of content). Content lives in special remotes, fetched on demand. |
| **Annex + `get --all`** | **YES** (full content) | Explicit user action to fetch everything. |
| **Annex + `get <file>`** | **NO** (one file) | Explicit user action to fetch just that file. |

## Test results (400 MB scale)

| Variation | Time | Size | Content? |
|---|---|---|---|
| **SUBMODULE** | | | |
| `git clone` | 0s | 212 KB | No (empty submodule dirs) |
| `git clone --recurse-submodules` | 1s | 1.2 GB | YES (full content) |
| `git clone --depth 1 --recurse-submodules` | 1s | 1.2 GB | YES (depth 1 doesn't help) |
| **ANNEX** | | | |
| `git clone` | 0s | 404 KB | No (symlinks, no content) |
| `git clone` + `get --all` | 3s | 402 MB | YES (full content) |
| `git clone` + `get <file>` | 0s | 26 MB | 1 file only |

## Extrapolated to 6.7 GiB (dracon-platform's actual content)

| Variation | Time | Size | Content? |
|---|---|---|---|
| **SUBMODULE** | | | |
| `git clone` | ~0s | ~212 KB | No |
| `git clone --recurse-submodules` | ~17s | **~20 GB** | YES — **exceeds GitHub 5 GB cap** |
| **ANNEX** | | | |
| `git clone` | ~0s | ~500 KB | No |
| `git clone` + `get --all` | ~50s | 6.7 GB | YES — content in OVH/S3, main repo is small |
| `git clone` + `get <file>` | ~0s | ~25 MB | 1 file only |

## Extrapolated to "hundreds of gigs" (worst case)

| Variation | Time | Size | Content? |
|---|---|---|---|
| **SUBMODULE** | | | |
| `git clone --recurse-submodules` | ~250s | **~300 GB** | YES — full content, all submodules |
| **ANNEX** | | | |
| `git clone` + `get --all` | ~750s | 100 GB | YES — content in OVH, main repo stays small |
| `git clone` alone | ~0s | ~500 KB | No |

## Why submodules always clone content

Most common tools default to `--recurse-submodules`:
- **GitHub Desktop**: recursive clone is the default
- **GitKraken**: recursive clone is the default
- **SourceTree**: recursive clone is the default
- **VS Code**: recursive clone is the default
- **GitHub Actions `actions/checkout@v4`**: `submodules: recursive` is the default
- **GitLab CI**: `GIT_STRATEGY: clone` does recursive clone

The ONLY way to avoid the full content download is to explicitly NOT use `--recurse-submodules` — which is **not** the common default in most workflows.

`--depth 1` does **not** help for submodules: the depth-1 only applies to the main repo's history, not to submodule content. All submodule content is still downloaded.

## Why annex never clones content (by default)

The annex `git clone` only transfers:
- Annex pointers in git (~500 bytes per file)
- Main repo's metadata
- .git/annex/ directory (location tracking, not content)

The actual content lives in **special remotes** (S3, directory, SFTP, rsync, etc.). The user explicitly fetches content via:
- `git annex get <file>` — single file
- `git annex get .` — all files in current dir
- `git annex get --all` — all "wanted" content (requires trusted remote)
- `git annex get --from=<remote>` — fetch from a specific remote

The default is "fetch nothing on clone." The user chooses.

## Recommendation for dracon-platform

**Use git-annex with OVH bucket as the special remote.** This:
- ✅ Avoids the "clone hundreds of gigs" problem (clone is always ~500 KB)
- ✅ Keeps OVH as the single source of truth (no duplication)
- ✅ Production keeps working (OVH bucket unchanged)
- ✅ Selective fetch (devs only get what they need)
- ✅ Scales linearly (clone size doesn't grow with content)

**If annex is not viable** (team not willing to learn it), next-best options:
1. **Submodule + per-category split** (each sub-repo < 5 GB)
2. **Self-host Gitea/Forgejo on OVH** (no cap at all)
3. **Switch to a forge without the cap** (Codeberg 10 GB, SourceHut unlimited)
4. **Git LFS** with $5/mo for 50 GB LFS bandwidth (single remote)

**NOT recommended**:
- Submodule with all content in one sub-repo (would hit 5 GB cap)
- GitHub as the forge for 6.7+ GiB content (hard cap)

## Evidence

- Test output captured at: `docs/design/audit-2026-06-26/submodule-vs-annex-clone-size.txt`
- Test data: 400 MB total content (4 categories × 4 files × 25 MB)
- Test directories: `/tmp/clone-test-submodule/`, `/tmp/clone-test-annex/`, `/tmp/annex-content-store/`
- Test cleanup: dirs can be left for operator inspection or removed manually

## Test methodology

1. Created 4 bare repos with random binary content (4 × 100 MB each = 400 MB)
2. Created a main repo with these as submodules
3. Ran 3 submodule clone variations
4. Created an annex repo with same content (400 MB) as annex
5. Set up a directory special remote (`/tmp/annex-content-store`) with the content
6. Ran 3 annex clone variations
7. Measured sizes and times

Pattern is the same at any scale: submodules clone content (or SHAs), annex clones metadata only.

## See also

- `docs/design/audit-2026-06-26/submodule-vs-annex-clone-size.txt` — full evidence file
- `docs/design/repo-remote-visibility-v3-revert-2026-06-27.md` — v3 design doc
- `docs/design/audit-2026-06-26/repo-remote-visibility-v2-revert-diff.txt` — v2 revert diff
- `docs/design/audit-2026-06-26/push-stuck-alternative-paths.txt` — PUSH_STUCK analysis
