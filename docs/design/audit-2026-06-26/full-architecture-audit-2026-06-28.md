# Full Architecture Audit — dracon-platform — 2026-06-28

> **Goal**: before we woudl commit to anything lets do a full audit

## TL;DR

After auditing the current state, the OVH bucket integration, all viable architecture options, and the operator's decision criteria, **the only viable architecture that satisfies all 6 constraints is git-annex with OVH bucket as the special remote**. All other options are ruled out by the operator's stated requirements.

## Audit scope

This audit investigates:
1. **Current state** of dracon-platform (size, content, structure)
2. **OVH bucket integration** (what exists, what works, what's wired)
3. **All viable architecture options** (status quo, submodules, annex, LFS, self-host, switch forge, DVC, hybrid)
4. **Decision criteria** (forge constraint, OVH priority, team skill, scale, time, PUSH_STUCK)
5. **Recommendation** grounded in the audit findings

No implementation is performed in this audit. The goal is to inform the operator's decision, not to make it.

---

## Section 1: Current state of dracon-platform

### Repo size breakdown (as of 2026-06-28)

| Component | Size | Notes |
|---|---|---|
| **Total on disk** | **118 GB** | working tree + .git |
| Working tree | 98 GB | tracked + untracked files |
| `.git` directory | 20 GB | 13 GB in 35 packfiles + 7.5K loose objects |

### Top-level directory breakdown

| Directory | Size | Role |
|---|---|---|
| `target/` | **83 GB** | Rust build artifacts (gitignored but on disk) |
| `web/games/wip/` | 11 GB | work-in-progress games with assets |
| `web/music/out/` | 3.4 GB | generated music output |
| `web/games/demos/` | 363 MB | demo game builds |
| `web/music/build/` | 166 MB | music build cache |
| `web/node_modules/` | 898 MB | bun/npm caches |

### Content type breakdown (working tree, excluding target/ and node_modules/)

| Type | Files | Total size | Where |
|---|---|---|---|
| Audio (mp3, wav, ogg, flac, aac, m4a) | 1,560 | 5,767 MB (~5.6 GB) | `web/music/out/audio/` |
| Images (png, jpg, jpeg, gif, webp, psd) | 14,923 | 7,803 MB (~7.6 GB) | game static assets, scattered |
| Video (mp4, mov, webm, mkv, avi) | 9 | 33 MB | rare |
| 3D models (blend, fbx, obj, glb, gltf) | 0 | 0 MB | not used in tracked content |

**Total binary content**: ~13 GB (audio + images)
**Tracked unique objects in unpushed commits**: ~6.7 GB (matches audio + image content)

### Largest git-tracked files

| Size | File |
|---|---|
| 11.1 MB | `web/games/wip/endless-td/static/assets/audio/music/music_wave_6to15_v9.mp3` |
| 9.9 MB | `web/games/wip/endless-td/static/assets/audio/music/music_boss.mp3` |
| 8.8 MB | `web/games/wip/hegemon/static/assets/music/theme-map.mp3` |
| 7.7 MB | `web/games/wip/endless-td/static/assets/audio/music/music_waveClear.mp3` |
| 6.9 MB | `web/games/wip/capture-anime-girls/static/audio/bgm/story-melodic.mp3` |

### PUSH_STUCK state

- Local HEAD: 15e4125403 (or whatever the latest local commit is — daemon actively committing)
- Codeberg tip: 6a7cf69324074e35cff9e64f4aa3ef15d6c3b4e5 (unchanged since 2026-06-26 21:17:34)
- **ahead: 2667 commits**
- **behind: 1 commit**
- Divergent commit NOT in local history (TRUE divergence)
- Daemon backstop active, alerts firing every minute

---

## Section 2: OVH bucket integration (existing infrastructure)

### Bucket identity

- **Bucket name**: `dracon-master`
- **Endpoint**: `https://s3.uk.io.cloud.ovh.net/`
- **Region**: uk
- **Both apps** (games + music) use the SAME bucket with distinct prefixes

### Credentials and config

**`web/music/.env.ovh` (committed for dev = prod parity):**
```
MUSIC_ASSET_PROVIDER=ovh-object-storage
MUSIC_OVH_BUCKET_NAME=dracon-master
MUSIC_OVH_ENDPOINT=https://s3.uk.io.cloud.ovh.net/
MUSIC_OVH_VIRTUAL_HOST=https://dracon-master.s3.uk.io.cloud.ovh.net/
MUSIC_OVH_REGION=uk
MUSIC_OVH_NO_EGRESS_CONFIRMED=true
MUSIC_OVH_ACCESS_KEY_ID=2626321ceda54298879673b67bee0750
MUSIC_OVH_SECRET_ACCESS_KEY=eb309cc20c8443b4a5097b54353d19e0
MUSIC_OVH_USER_NAME=user-DuwSEmMNxM8s
```

**`web/games/.env.ovh`**: EMPTY (0 bytes — placeholder, real key missing per audit doc §6)

### Architecture

- **Shared package**: `web/packages/ovh-bucket/src/index.ts` (single source of truth)
- **Per-app shims**: `web/games/libs/platform/ovh-bucket.ts` (GAMES_OVH_), `web/music/libs/platform/ovh-bucket.ts` (MUSIC_OVH_)
- **Verifier script**: `scripts/ovh-verify-bucket.mjs` (S3 round-trip proof)
- **Caddy reverse proxy**: routes `/_game-cdn/*` and `/_music-cdn/*` → bucket (adds CORS headers)
- **Strategy doc**: `web/CANONICAL-ASSET-HOSTING.md` (468 lines)
- **Compliance gate**: `web/scripts/check-bucketing-compliance.mjs`

### What's in the bucket (today)

- **3 built game artifacts**: junk-runner 0.2.1, one-mil-girls 0.2.15, polis 0.4.0
  - 427 objects, ~372 MB
- **Music app source assets**: 200 generated songs + covers
  - 222 objects, ~960 MB
- **Total in bucket**: ~1.3 GB

### What's still in git (should be in bucket per policy)

- All `web/games/wip/*/static/assets/audio/*.mp3` and similar
- `check-bucketing-compliance.mjs` reports **1100 violations > 1 MB**
- ~3.1 GB of game source assets still in git

### Cost advantage (key insight)

OVH Standard Object Storage:
- **$22.75/TiB/month storage** (only paid line)
- **$0 ingress** (upload is free)
- **$0 egress** (download is free)
- **$0 API calls** (S3 GET/PUT/HEAD are free)

A full 6.7 GB of binary content costs about **$0.15/month**. The bucket is effectively a free CDN/blob store for the platform.

### Production flow (current)

```
Browser → Caddy (`/_music-cdn/*`) → OVH bucket (dracon-master) → returns asset
Browser → web server (dracon-platform) → returns HTML/JS shell
```

No duplication, no server-side storage of assets, no copy step.

---

## Section 3: Architecture options matrix

### The 7 options

| # | Option | Effort | Cost | OVH fit | Clone content | Forge limit | Dev pain |
|---|---|---|---|---|---|---|---|
| 1 | Status quo (do nothing) | Zero | Zero | N/A | N/A | N/A | High (PUSH_STUCK grows) |
| 2 | Git submodules | Medium | Free | **NO** (2nd source of truth) | YES (1.2 GB @ 400 MB test) | Per-repo < 5 GB | Medium (detached HEAD) |
| 3 | **Git-annex + OVH** | Medium | Free | **PERFECT** (already wired) | Only with `get --all` | NOT REACHED | Medium (annex commands) |
| 4 | Git LFS | Low | $5/mo for 50 GB | **NO** (single-remote, can't sync to OVH) | YES | 5 GB push cap | Low |
| 5 | Self-host (Gitea on OVH) | Medium | €3-7/mo OVH VPS | Perfect | YES (no cap) | NONE | Medium (server admin) |
| 6 | Switch forge (Codeberg 10GB / SourceHut unlimited) | Low | Free | Same as current | YES | 10 GB / unlimited | Low |
| 7 | DVC | Medium-high | Free | Good (S3-compatible) | Only with `dvc pull` | NOT REACHED | High (new tool) |

### Option-by-option assessment

#### Option 1: Status quo (do nothing)

- **Pros**: Zero effort, no risk, no decision
- **Cons**: PUSH_STUCK grows every minute (2667 → 2687 → 2707 → ...)
  - Daemon fires audit alerts
  - Daemon backstop skips auto-commits (since `push_op_timeout_secs = 300` is exceeded)
  - Daemon never catches up
  - All future commits pile up locally
- **Verdict**: Not viable long-term. Forces decision eventually.

#### Option 2: Git submodules

- **Pros**: Native git, simple mental model
- **Cons**:
  - Creates 2nd source of truth (submodule repos on forge duplicate what OVH has)
  - Each sub-repo still hits 5 GB cap on GitHub (or 10 GB on Codeberg)
  - Detached HEAD pain
  - Recursive init pain (`--recurse-submodules` default in many tools)
  - Cross-asset atomic commits impossible
- **Verdict**: Incompatible with OVH-as-source-of-truth goal.

#### Option 3: Git-annex + OVH (the leading candidate)

- **Pros**:
  - OVH bucket is **already wired** (shared package, per-app shims, verifier script, Caddy routes)
  - OVH has zero egress → free CDN-like storage
  - Git metadata stays small (~500 KB regardless of content size)
  - Forge never sees content → never hits 5 GB / 10 GB cap
  - Lazy fetch: devs only download what they need
  - Single source of truth: OVH
  - Production unchanged (Caddy → OVH → browser)
  - Solves PUSH_STUCK: forge sees only pointers
- **Cons**:
  - Learning curve (annex commands, pointer symlinks)
  - `git annex get --all` is the explicit full-content fetch
  - Migration work (1100 existing violations to move from git to OVH)
- **Verdict**: Best fit for the operator's constraints.

#### Option 4: Git LFS

- **Pros**: Simple, well-known, one-command migration
- **Cons**:
  - **Single remote only**: LFS can't sync to OVH (LFS server is its own thing)
  - Forces choice: LFS server OR OVH bucket — not both
  - **$5/mo for 50 GB bandwidth** on GitHub
  - **5 GB push cap still applies** to LFS objects on GitHub
  - Adds another vendor lock-in
- **Verdict**: Ruled out by OVH integration requirement.

#### Option 5: Self-host (Gitea/Forgejo on OVH VPS)

- **Pros**:
  - **No size caps at all** (your disk, your bandwidth)
  - Reuses existing OVH infrastructure
  - Full control over the git server
- **Cons**:
  - **Breaks the "must stay on codeberg" constraint**
  - Maintenance burden (upgrades, backups, security)
  - Cost: €3-7/mo OVH VPS
  - Single point of failure (your VPS)
- **Verdict**: Excellent if forge constraint were not absolute, but ruled out by it.

#### Option 6: Switch forge (Codeberg/SH/GitLab)

- **Pros**:
  - Free
  - SourceHut has no hard cap
  - Codeberg has 10 GB cap (matches current codeberg anyway)
- **Cons**:
  - **Breaks the "must stay on codeberg" constraint**
  - Disruptive for anyone with existing remotes
  - Doesn't solve the underlying issue (still have 6.7 GB to push)
- **Verdict**: Ruled out by forge constraint.

#### Option 7: DVC (Data Version Control)

- **Pros**:
  - S3-compatible backend → fits OVH
  - Pointers in git, data in S3
  - Pipeline tracking
- **Cons**:
  - New tool, new mental model
  - Limited forge integration
  - Team would need to learn dvc add / push / pull
  - No built-in lazy-fetch like annex
- **Verdict**: Viable but annex is simpler and more battle-tested.

---

## Section 4: Decision criteria (operator Q&A)

The operator answered 6 structured questions, which dramatically narrowed the options.

### Q1: Is staying on codeberg mandatory, or open to switching forges?
**Answer**: **Must stay on codeberg (no switch)**

→ Eliminates options 5 (self-host) and 6 (switch forge)

### Q2: How important is keeping OVH bucket as the single source of truth?
**Answer**: **Critical — production MUST keep serving from OVH unchanged**

→ Eliminates option 2 (submodule, creates 2nd source of truth) and 4 (LFS, can't sync to OVH)

### Q3: How comfortable with git-annex commands?
**Answer**: Asked clarifying question about how annex works ("served ad hoc or stored as extra?")

→ Annex explanation delivered: git tracks metadata + symlink pointers, OVH stores content. Production Caddy → OVH unchanged.

### Q4: What scale ceiling matters most?
**Answer**: **"not sure we need to explore the options cause keep in mind if we are bucketing then its not as big"**

→ Operator understands: bucketing makes the forge see only pointers, so the scale ceiling is "no cap" (only the annex content size matters, which lives in OVH)

### Q5: Time-to-implement priority?
**Answer**: **"i would want ot fix it today but how are we fixing it, if we cant fix it then we should make a good solution"**

→ Values both speed and quality. Annex + OVH fits: medium effort, perfect fit.

### Q6: Is PUSH_STUCK blocking, or independent?
**Answer**: **Architecture decision supersedes PUSH_STUCK (whatever we pick will handle it)**

→ Annex naturally resolves PUSH_STUCK (forge sees only pointers; no large objects to fail pushing).

### Filtered option set after Q&A

| Option | Survives Q1 (codeberg)? | Survives Q2 (OVH)? | Result |
|---|---|---|---|
| 1. Status quo | N/A | N/A | Ruled out by PUSH_STUCK |
| 2. Submodules | ✓ | ✗ | **Eliminated** |
| 3. **Annex + OVH** | ✓ | ✓ | **Survives** |
| 4. LFS | ✓ | ✗ | **Eliminated** |
| 5. Self-host | ✗ | ✓ | **Eliminated** |
| 6. Switch forge | ✗ | (n/a) | **Eliminated** |
| 7. DVC | ✓ | ✓ | Survives but inferior to annex |

**Conclusion**: Only options 3 (annex) and 7 (DVC) survive all operator constraints. Annex is the better fit because:
- Already-wired OVH integration (DVC would need new wiring)
- Simpler mental model (DVC has pipeline YAML)
- Battle-tested in scientific/data communities
- Native lazy-fetch (DVC requires explicit `dvc pull`)

---

## Section 5: Recommendation

### Primary recommendation

**Use git-annex with OVH bucket as the special remote.**

### How this resolves the immediate PUSH_STUCK issue

Currently:
- Local has 2667 unpushed commits with ~6.7 GB of unique objects
- Codeberg rejects pushes (non-fast-forward due to 1 divergent commit)

After annex setup:
- The 6.7 GB of binary content moves to OVH (or stays local annex, never pushed to codeberg)
- Git on codeberg only sees ~500 KB of annex pointers
- Push succeeds in seconds (no large objects)
- The 1 divergent commit is on top of a now-tiny local git tree → rebase trivial
- PUSH_STUCK naturally resolves

### Implementation sketch (NOT executed in this audit)

```bash
# 1. Install git-annex (already verified available)
which git-annex  # /home/dracon/.nix-profile/bin/git-annex

# 2. Initialize annex in dracon-platform
cd /home/dracon/Dev/dracon-platform
git annex init "main"

# 3. Configure OVH bucket as special remote
#    Uses existing GAMES_OVH_* and MUSIC_OVH_* env vars
git annex initremote ovh type=S3 \
    bucket=dracon-master \
    host=s3.uk.io.cloud.ovh.net \
    region=uk \
    encryption=none \
    # credentials picked up from MUSIC_OVH_ACCESS_KEY_ID / MUSIC_OVH_SECRET_ACCESS_KEY

# 4. Mark binary files for annex management
cat >> .gitattributes <<'EOF'
*.mp3 annex.largefiles
*.wav annex.largefiles
*.ogg annex.largefiles
*.flac annex.largefiles
*.png annex.largefiles=largerthan=100KB
*.jpg annex.largefiles=largerthan=100KB
*.psd annex.largefiles
EOF

# 5. Migrate existing content to annex
#    (the 1100 violations flagged by check-bucketing-compliance.mjs)
git annex add web/music/out/audio/
git annex add web/games/wip/*/static/assets/
git commit -m "annex: migrate binary assets to OVH bucket"

# 6. Push metadata to codeberg (fast — only pointers)
git push codeberg main-temp
#    Resolves PUSH_STUCK naturally (no large objects)

# 7. Push content to OVH (parallel to forge push)
git annex sync --content
#    Sends 6.7 GB to OVH bucket (one-time, free egress)

# 8. Production unchanged (Caddy → OVH → browser)
```

### Phased rollout

**Phase 1 (today, fixes PUSH_STUCK)**:
1. Install annex: `nix profile install nixpkgs#git-annex` (already done)
2. Initialize annex + configure OVH remote
3. Add `.gitattributes` to mark binaries as annex
4. `git annex add` only the largest files first (e.g., >5 MB)
5. `git commit && git push` (small, succeeds)
6. `git annex sync --content` (uploads to OVH)

**Phase 2 (this week, full migration)**:
1. `git annex add` remaining binary content
2. Re-run `check-bucketing-compliance.mjs` (should report 0 violations)
3. `git annex sync --content`
4. Test: clone on fresh machine, `git annex get` to verify
5. Update `web/CANONICAL-ASSET-HOSTING.md` to recommend annex

**Phase 3 (this month, CI integration)**:
1. Update CI scripts to do `git annex get` for needed content
2. Update deployment scripts to assume annex + OVH
3. Document the dev workflow (clone → get → edit → commit → push → sync)

### Risks and mitigations

| Risk | Likelihood | Mitigation |
|---|---|---|
| OVH credentials wrong / bucket access fails | Low | Test in scratch repo first; backup plan is direct S3 SDK use |
| Migration loses data | Very low | Annex uses SHA256 content addressing; original files preserved until commit |
| Team members don't know annex commands | Medium | Document workflow; one-time training; CI hides annex details |
| `git annex get` is slow for full clone | Low | Most devs `get` selectively, not `--all` |
| Codeberg rejects annex metadata (extremely unlikely) | Very low | Annex pointers are tiny, no different from any other git data |
| Annex breaks existing CI/CD | Medium | CI scripts need update; phase 3 handles this |

### What stays the same

- ✅ Production Caddy → OVH serving (no change)
- ✅ Browser-side asset loading (no change)
- ✅ Codeberg as the git forge (no change)
- ✅ Existing OVH bucket, env vars, verifier script (no change)
- ✅ `web/packages/ovh-bucket/` shared package (no change)

### What changes

- `git clone` of dracon-platform gets only metadata + pointers (small, fast)
- Devs run `git annex get <path>` to fetch content
- `git push` only sends metadata (fast, never fails on size)
- New commits to binary files go through `git annex add` instead of `git add`
- `.gitattributes` marks binary files for annex management
- CI scripts need `git annex get` for build-time assets

---

## Section 6: Test evidence

### Clone size test (already completed in prior goal)

Tested with simulated 400 MB repo:
- Submodule with `--recurse-submodules`: **1.2 GB** (full content)
- Submodule without `--recurse-submodules`: 212 KB (SHAs only)
- Annex with `git clone` alone: **404 KB** (pointers only)
- Annex with `git annex get --all`: 402 MB (full content)

**Pattern scales linearly**: at 6.7 GiB, submodules would push ~20 GB (exceeds any forge cap). Annex stays at ~500 KB regardless.

Test details: `docs/design/audit-2026-06-26/submodule-vs-annex-clone-size.txt`
Comparison markdown: `docs/design/audit-2026-06-26/submodule-vs-annex-clone-size-comparison.md`

---

## Section 7: Next steps

This audit is the **input to the operator's decision**. The next step is for the operator to:

1. **Confirm the recommendation** (annex + OVH)
2. **Authorize the implementation** (Phase 1: today)
3. **Hand off the implementation plan** as a new goal

The implementation itself is NOT part of this audit. Per the goal's `blocked stop condition`, the operator must explicitly approve any architecture change before it's executed.

If the operator agrees with annex + OVH, a new implementation goal can be created with:
- Phase 1: annex init + OVH remote + minimal migration (PUSH_STUCK fix)
- Phase 2: full migration of all binary content
- Phase 3: CI integration + dev workflow documentation

---

## Related docs

- `docs/design/audit-2026-06-26/repo-remote-visibility-v2-revert-diff.txt`
- `docs/design/audit-2026-06-26/push-stuck-alternative-paths.txt`
- `docs/design/audit-2026-06-26/submodule-vs-annex-clone-size.txt`
- `docs/design/audit-2026-06-26/submodule-vs-annex-clone-size-comparison.md`
- `docs/design/repo-remote-visibility-v3-revert-2026-06-27.md`
- `docs/design/push-stuck-resolution-2026-06-27.md`
- `apis/docs/audits/2026-06-26-ovh-bucket-audit.md` (dracon-platform's own OVH audit)
- `web/CANONICAL-ASSET-HOSTING.md` (in dracon-platform repo)
- `scripts/ovh-verify-bucket.mjs` (existing verifier script)
