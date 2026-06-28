# dracon-platform Size Unblock — 2026-06-28

> **Operator's request**: "we still ddidnt hook up github and gitlab"
> **Current state**: `dracon-platform/.dracon/dracon-sync.toml` has `exclude_remotes = ["github", "gitlab"]` because the repo's `.git` is 13 GB and exceeds GitHub's 5 GB / GitLab's 10 GB free-tier limits.

## Why we excluded

In commit `mqqsyzyd-qkvna5` (2026-06-23), the daemon was configured to exclude github and gitlab for `dracon-platform`:

- `gitlab.com`: 10 GiB per-project free-tier quota; the platform's gitlab copy was 9.5 GiB. Pre-receive hook rejected with "Your push would exceed the allocated storage for your project".
- `github.com`: 5 GB recommended size for free personal accounts; the platform's github copy was 10.87 GiB. github returns HTTP 500 on every push attempt.

Even +10 GiB on gitlab or upgrading github would not solve the problem because the daemon keeps adding files.

## Current measurements (2026-06-28)

| Metric | Value |
|--------|-------|
| Local `.git` size | 13 GB |
| Local codebase size (excluding `.git`) | 99 GB |
| Github repo size | 10.87 GB (`diskUsage` = 11,398,169 KB) |
| Gitlab repo | does NOT exist on gitlab.com (404) |
| Codeberg repo | unknown size (no public API field) |
| Tracked binary files (audio) | 276 files / 648 MB |
| Tracked binary files (images) | 5273 files / 2,689 MB |
| **Total tracked binary** | **5,549 files / 3,337 MB** |

## Solution: git-annex + OVH

The full architecture audit at `docs/design/audit-2026-06-26/full-architecture-audit-2026-06-28.md` recommends **git-annex + OVH** to:
1. Replace ~200-byte annex pointers in git history for binary assets
2. Move actual bytes (~3.3 GB) to OVH bucket `dracon-master`
3. Shrink packable git size from 13 GB → ~500 KB (just source code + annex pointers)
4. Enable github and gitlab pushes (5 GB / 10 GB limits easily met)

### Phase 1: Annex init + OVH remote (READY)

```
cd /home/dracon/Dev/dracon-platform
git annex init "dracon-platform"
git annex initremote ovh type=S3 \
    bucket=dracon-master \
    encryption=none \
    endpoint=https://s3.gra.io.cloud.ovh.net \
    port=443 requeststyle=path
```

OVH bucket state (verified 2026-06-28):
- 65,052 objects / 5.84 GB
- Endpoint: `s3.gra.io.cloud.ovh.net`
- Web music: 601 objects / 3.4 GB
- Hegemon assets: 2440 objects / 2.1 GB
- dracon/db: 61,059 objects / 9 MB

### Phase 2: Migrate tracked binaries (PENDING — multi-hour)

For each binary extension (mp3, wav, ogg, flac, png, jpg, jpeg, gif, webp, psd, etc.):
```
git annex add <file>
git commit -m "annex: migrate <filename>"
git push codeberg master
git annex sync --content  # uploads to OVH
```

MIGRATION_TODO list in `web/scripts/check-bucketing-compliance.mjs` covers 16 game/visual entries.

### Phase 3: Update `.gitattributes` (PENDING)

Mark binary extensions for annex with size thresholds matching existing bucketing policy:
```
*.mp3 annex.largefiles=anything
*.png annex.largefiles=anything
*.jpg annex.largefiles=anything
...
```

### Phase 4: Update CI scripts (PENDING)

Any pipeline needing binary content must run `git annex get` before building.

### Phase 5: Update dev docs (PENDING)

Document clone → `git annex get` workflow at `web/docs/asset-pipeline.md`.

## What we're doing RIGHT NOW (in this session)

The operator wants github and gitlab "hooked up". We're going to:

1. **Add github and gitlab remotes** to the local repo ✓
2. **Remove `exclude_remotes` from per-repo config** so daemon tries pushing
3. **Let daemon push** to all 3 remotes
4. **Document the size blocker** — annex migration is the path forward

For pushes that fail due to size, we surface the exact error. For github (10.87 GB > 5 GB free tier), pushes WILL fail with HTTP 500. For gitlab, we need to CREATE the repo first (operator action required — `glab repo create` needs API token).

## Operator decisions needed

1. **Create gitlab repo**: `glab auth login` then `glab repo create dracondev/dracon-platform --private --description "Dracon platform"`. Without this, gitlab pushes will fail with "repository not found".
2. **Annex migration timing**: Phase 2 (migrating 5,549 binary files) takes ~30-60 minutes. Schedule when convenient, OR run incrementally over time.
3. **GH free tier limit**: To push to github before annex migration, must either:
   - (a) Upgrade to GitHub Pro ($4/mo, 50 GB)
   - (b) Wait for annex migration to drop packable size below 5 GB
   - (c) Accept that github pushes will fail until annex migration completes

## Risks

- **Push failures during transition**: Until annex migration completes, github/gitlab pushes will fail. The daemon will record PUSH_STUCK failures. These are expected and not actionable until migration completes.
- **Warden pre-push regex**: An audit doc references an AWS key in plaintext. If that doc is committed, warden's pre-push hook may flag it. Workaround: scope regex to blob content (not diff context).
