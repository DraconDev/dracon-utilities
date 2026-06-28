# Annex Migration Implementation Plan — dracon-platform

## Phase 1 (today): Resolve PUSH_STUCK + minimal migration

### Pre-conditions verified (before this plan)

1. ✅ OVH bucket `dracon-master` reachable via SDK (`bun` + `@aws-sdk/client-s3`, 65,052 objects, 5.84 GB).
2. ✅ git-annex 10.20251114 installed at `/home/dracon/.nix-profile/bin/git-annex`.
3. ✅ Scratch annex test (5 MB random file uploaded to OVH in <1s at 53 MB/s; cloned, fetched back).
4. ✅ OVH credentials available: `web/music/.env.ovh` (gitignored plaintext, has real `MUSIC_OVH_ACCESS_KEY_ID` + `MUSIC_OVH_SECRET_ACCESS_KEY`).
5. ✅ dracon-warden pre-commit hook active; filter patterns in `.gitattributes` block 1-53.
6. ✅ The 1 divergent codeberg commit `6a7cf693240…` is small (758 bytes commit, 693 bytes tree) — rebase likely trivial.

### Real bucket inventory (verified, not audit doc's claim)

| Prefix | Objects | Size |
|---|---|---|
| music/audio/ | 601 | 3.4 GB |
| music/covers/ | 61 | 18 MB |
| games/hegemon/assets/ | 2440 | 2.1 GB |
| games/junk-runner/ | 155 | 311 MB |
| games/one-mil-girls/ | 101 | 54 MB |
| games/polis/ | 217 | 20 MB |
| dracon/ (db metadata) | 61059 | 9 MB |
| **Total** | **65,052** | **5.84 GB** |

Note: prior audit doc said "1.3 GB in bucket" — actual is **5.84 GB**.

### Step-by-step

1. **Reconcile divergent commit (operator authorization required)**:
   - The codeberg commit `6a7cf693240…` is small, changes 17 binary files.
   - Operator options:
     - **(A) Rebase local onto codeberg**: replay local 3038 commits on top of `6a7cf69`. Push with `--force-with-lease`. Risk: binary-file conflicts on the 17 files codeberg changed.
     - **(B) Force-push local 3038 onto codeberg**: discard the codeberg commit. Risk: loses those 17 file changes.
     - **(C) Manual merge**: commit-by-commit resolution.
   - AGENTS.md forbids force-push on >5-commits-ahead repos without explicit operator override. **MUST surface this to operator before executing.**

2. **Initialize annex on local dracon-platform**:
   ```bash
   cd /home/dracon/Dev/dracon-platform
   git annex init "main"
   ```

3. **Add OVH as annex special remote**:
   ```bash
   git annex initremote ovh type=S3 \
     bucket=dracon-master \
     host=s3.uk.io.cloud.ovh.net \
     port=443 \
     region=uk \
     requeststyle=path \
     encryption=none
   ```
   Credentials picked up from `AWS_ACCESS_KEY_ID` / `AWS_SECRET_ACCESS_KEY` env vars (set from `web/music/.env.ovh`).

4. **Update `.gitattributes`** (add AFTER warden block, lines 54+):
   ```
   # Annex pointer markers for binary asset migration
   *.mp3 annex.largefiles=anything
   *.wav annex.largefiles=anything
   *.ogg annex.largefiles=anything
   *.flac annex.largefiles=anything
   *.m4a annex.largefiles=anything
   *.aac annex.largefiles=anything
   *.png annex.largefiles=anything
   *.jpg annex.largefiles=anything
   *.jpeg annex.largefiles=anything
   *.gif annex.largefiles=anything
   *.webp annex.largefiles=anything
   *.psd annex.largefiles=anything
   *.mp4 annex.largefiles=anything
   *.webm annex.largefiles=anything
   *.mov annex.largefiles=anything
   *.mkv annex.largefiles=anything
   *.avi annex.largefiles=anything
   ```
   Note: `*.png -filter` (disable warden) and `*.png annex.largefiles=anything` are compatible. `-filter` is a negative attribute that overrides any positive `filter=...` from earlier lines. Both can coexist.

5. **Migrate the largest tracked binary files first** (Phase 1 subset):
   - Tracked audio: 276 files / 648 MB
   - Tracked images: 5,273 files / 2.7 GB
   - **Phase 1 scope**: the 11 MP3s > 5 MB in wip/* (audio files, ~80 MB total). Small enough to validate end-to-end without committing hours.
   - **Phase 2 scope**: full audio (276 files) + full images (5273 files) = 3.3 GB.

6. **Commit + push**:
   ```bash
   git annex add <files>
   git commit -m "annex: migrate audio assets to OVH bucket"
   git push codeberg main-temp
   ```
   The push is now tiny (200 bytes per annex pointer) and won't fail on size cap.

7. **Sync content to OVH**:
   ```bash
   git annex sync --content
   ```
   Uploads actual bytes to OVH bucket.

8. **Verify bucket received content**:
   - Use the bun/boto3 listing script to confirm new objects under annex's content-addressable path (`annex/objects/XX/YY/<sha>`).

### Risks specific to this plan

| Risk | Likelihood | Mitigation |
|---|---|---|
| Rebase of codeberg commit fails on the 17 changed binary files | Medium | Surface to operator with conflict list; offer manual resolution |
| `git annex init` triggers warden hook with no `.gitattributes` annex lines | Low | Add `.gitattributes` lines FIRST, then init |
| OVH upload is slow for 3.3 GB | Low (5 MB tested at 53 MB/s → ~60s for 3.3 GB) | Run in background, monitor |
| daemon sees annex pointer commits and panics | Very low | Annex pointers are normal git blobs; daemon handles them as any other commit |
| Existing OVH bucket content (junk-runner build artifacts, etc.) gets overwritten | Very low | annex uses content-addressable paths (`annex/objects/...`); doesn't touch `games/junk-runner/`, `music/audio/` prefixes |

## Phase 2 (this week): Full migration

1. Migrate all remaining tracked audio (276 files total).
2. Migrate all tracked images (5,273 files).
3. Run `bun run web/scripts/check-bucketing-compliance.mjs --strict` — should report 0 violations for migrated categories.
4. Run `git annex sync --content` (full 3.3 GB upload to OVH).
5. Push to codeberg (fast, small).

## Phase 3 (this month): CI integration + docs

1. Update `.dracon/utilities/ci/*.sh` and any CI scripts to do `git annex get <path>` for needed assets before build.
2. Write `web/docs/annex-workflow.md` covering: clone, get, add, commit, push, sync, unlock/edit/add/commit.
3. Update `web/CANONICAL-ASSET-HOSTING.md` §2 to reference annex + OVH as the canonical binary storage strategy.
4. Update `web/scripts/check-bucketing-compliance.mjs` — empty out MIGRATION_TODO since annex migration supersedes the per-game migration TODO.
