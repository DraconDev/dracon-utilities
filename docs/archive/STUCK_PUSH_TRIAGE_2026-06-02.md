# STUCK_PUSH Triage Report — 2026-06-02

## Triage Results

Triage was performed using `git rev-list --objects --all | git cat-file --batch-check='%(objectsize) %(rest)' | sort -rn | head -20` for each of the 12 affected repos (6 CONCERN + 6 WARN with AHD>0).

## Repos Needing `git filter-repo` (7 repos)

These repos have previously-tracked binary files >50MB in their git history:

| # | Repo | Largest Object | Size | Path Pattern |
|---|------|----------------|------|--------------|
| 1 | avid | target/debug/avid | 330MB | `target/` |
| 2 | ai-auto-writer | target/debug/deps/ai_auto_writer-* | 283MB | `target/` |
| 3 | dracon-code | target/debug/deps/custom_agent_platform-* | 162MB | `target/` |
| 4 | rust-ai-web-auto | target/release/deps/libchromiumoxide_cdp.rlib | 74MB | `target/` |
| 5 | dracon-ai-lib | target/debug/examples/basic_chat-* | 78MB | `target/` |
| 6 | dracon-voice-notifications | assets/models/kitten_tts_nano_v0_8.onnx | 56MB | `assets/models/*.onnx` |
| 7 | browser-extensions-shared | scrap/web-automator/benchmark/models/gte-small.onnx | 34MB | `*.onnx`, `node_modules/` |

## Repos NOT Needing filter-repo (5 repos)

These repos have either no large binaries in history, or all large files are <50MB (the push threshold):

| # | Repo | Status | Reason |
|---|------|--------|--------|
| 1 | dracon-platform | OK | Max 3.8MB (test_namespace, repro) — under threshold |
| 2 | cli-file-manager | OK | Max 12MB (micro binary) — under threshold |
| 3 | Junk-Runner-bevy | OK | Max 5.7MB (assets/crew_trade.png) — under threshold |
| 4 | dracon-terminal-engine | OK | Max 6.8MB (rust_out) — under threshold |
| 5 | dracon-utilities | OK | Max 4.3MB (rust_out, tarpaulin-reports) — under threshold |

**Note:** For these 5 repos, sync should be able to handle them normally — the modified files just need to be committed and pushed. They may have been marked WARN/CONCERN due to other issues (uncommitted changes, stuck state from earlier failed pushes).

## Backup Branch

Backup branch name: `backup/pre-filter-2026-06-02`

Will be created for each of the 7 affected repos and pushed to all 4 remotes (origin, github, gitlab, codeberg) before any filter-repo operation.

## Filter Strategy

For each affected repo, the filter will be:

- **avid, ai-auto-writer, dracon-code, dracon-ai-lib, rust-ai-web-auto**: `--invert-paths --path target/`
- **dracon-voice-notifications**: `--invert-paths --path-glob '*.onnx' --path assets/models/`
- **browser-extensions-shared**: `--invert-paths --path-glob '*.onnx' --path-glob 'node_modules/'`

After filter-repo, the repos will be force-pushed to all 4 remotes.

## Risk Assessment

- **High risk**: Force-push to all 4 remotes rewrites history — any existing clones will need to be re-cloned or `git reset --hard origin/main`'d
- **Medium risk**: If filter-repo misses any large files, the push will still fail
- **Low risk**: Backup branches provide a safety net — if filter-repo is wrong, can reset to backup

## Next Steps

1. ✅ Triage complete
2. ✅ git-filter-repo installed
3. ⏳ Create backup branches for all 7 affected repos
4. ⏳ Push backup branches to all 4 remotes for each repo
5. ⏳ Run filter-repo on each repo
6. ⏳ Force-push to all 4 remotes
7. ⏳ Clear STUCK_PUSH state via `dracon-sync repair stuck-unstuck`
8. ⏳ Final verification
