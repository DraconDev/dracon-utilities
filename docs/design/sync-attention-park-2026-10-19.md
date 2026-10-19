# Sync attention sweep 2026-10-19 — 3 cleared, 1 parked

Scope: the 4 repos flagged by `dracon-sync repos` on 2026-10-19.
Repairs used only sanctioned paths (maintenance wrapper,
`repair stuck-unstuck`, `sync-now`, fast-forward pushes, the warden
`.plaintext` hatch). No history rewrite, no force-push, no
`--no-verify`, no daemon/warden source changes.

## Cleared

- **dracon-utilities** (was STALLED, 19 ahead, push-stuck): the warden
  pre-push diff-line scan tripped on one prose line in
  `AUDIT_FULL_2026-10-19-ROUND5.md` describing, in words, the
  credential-assignment category that matched another repo's finding
  (introduced in `bcc9afaa`; full scan of the 19-commit range found
  exactly this one match). Added the approved `.plaintext` sibling
  for that audit doc, unstuck, `sync-now`. Result: CLEAN, 0/0,
  PUSH OK.
- **dracon-strategy/ai-auto-video** (was STALLED, 15 ahead,
  push-stuck): the pre-push added-blob scan tripped on the 84 MB
  `pi-session-2026-10-17T23-03-51-484Z_01a14c1b-79bb-705e-8118-1d9276991587.html`
  transcript (added in `32d47d49`). Full scan found exactly 2 matches,
  both Google-key-shaped 39-char substrings embedded inside base64
  streams — chance false positives, no other secret shapes. Added the
  `.plaintext` sibling, unstuck, `sync-now`. Result: CLEAN, 0/0,
  PUSH OK.
- **dracon-strategy/ai-auto-writer** (was BLOCKED, history probe
  failed): history is healthy — `rev-list --objects HEAD` lists
  677,582 objects and `cat-file --batch-check` reports 0 missing.
  The daemon probe (`probe_history` in dracon-sync `report.rs`)
  bounds each step at 10 s; the batch-check needed ~15 s over the
  old 4-pack/11k-loose-object layout. Sanctioned `git gc` via
  `dracon-sync maintenance` consolidated the repo to 1 pack / 0 loose
  (rev-list 1.8 s, batch-check 8.3 s), after which
  `dracon-sync repos --deep ai-auto-writer` reports CLEAN, healthy,
  0/0. Nothing needed pushing.

## Parked: dracon-strategy (RETRYING)

- Repo: `/home/dracon/Dev/dracon-strategy`, branch `main`,
  publish `origin/main`, push-to github+gitlab (codeberg excluded).
  State at park time: 23 modified + 3 untracked, 0 ahead / 0 behind,
  last push OK — the push side is fine; the commit side is wedged.
- Exact failure: `dracon-sync sync-now /home/dracon/Dev/dracon-strategy`
  fails with `git diff HEAD exited with exit status: 128`, caused by
  the warden clean filter refusing the file
  `ai-auto-music/catalog/cookbook.json`:

  > dracon-warden: refusing to clean 67108865 bytes (limit 67108864
  > bytes): the file would be committed UNENCRYPTED. Encrypt it
  > out-of-band (dracon-warden encrypt-file) or .gitignore it. Binary
  > media and archives are exempt via binary_filter_exempt_patterns.

  (The quoted byte count is the limit+1 sentinel; the actual file is
  69,279,277 bytes as of 2026-10-19 21:01, up from the 66,298,233-byte
  blob at HEAD.)
- Why sanctioned repairs cannot clear it: the catalog is a generated,
  already-plaintext music catalog (5,600 cooks; a full secret-shape
  scan of the worktree file finds 0 matches) and it already has a
  tracked `.plaintext` sibling. But the size guard
  (`filter_clean_refusal_with_limit`, dracon-warden `main.rs:3529`)
  runs before the hatch is consulted and exempts only
  `binary_filter_exempt_patterns` paths; a `.json` catalog is not
  exempt, so every commit attempt in this repo fails deterministically
  while the file exceeds 64 MiB. Exempting it changes warden filter
  policy — a security-boundary decision, not a sync repair.
- The one operator action that clears it: approve ONE of
  (a) a repo-local `.gitattributes` `-filter` exemption for
  `ai-auto-music/catalog/cookbook.json` in dracon-strategy,
  (b) a warden policy `binary_filter_exempt_patterns` entry for that
  path, or (c) the ai-auto-music owner splitting/shrinking the
  catalog below 64 MiB —
  then run `dracon-sync sync-now /home/dracon/Dev/dracon-strategy`
  (the pending 23 modified + 3 untracked files will commit and push;
  nothing is ahead, so no push recovery is needed).
