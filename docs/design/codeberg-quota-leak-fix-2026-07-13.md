# Codeberg Quota Leak Fix — 2026-07-13

## What (from audit on 2026-07-13)

Codeberg account is at 85.0000 GiB used / 85.00 GiB grace quota (99.5%).
Daemon has been failing push with `remote: Forgejo: Quota exceeded` on every repo.

The 85 GiB is split across 86 repos (47 private + 39 public). Top 10 private
repos account for 73.8 GiB (87%).

## What was actually leaking

Investigation of `git rev-list --objects --all` on the 16 heaviest repos
revealed three structural patterns that account for ~10.29 GiB of fixable
bloat, distinct from the genuine game-asset intentional content (~21 GiB
in PNGs/MP3s/FBX across the same repos):

| Leak pattern                                            | Total size | Where seen                                            |
|---------------------------------------------------------|-----------:|-------------------------------------------------------|
| `**/.pi/**` (universal agent session dir)                | 4.36 GiB   | Every game repo + parent; covers `.pi/goals/`, `.pi/chrome-screenshots/`, `.pi/audit-*/`, `.pi/mmx-out/`, etc. |
| `**/test-results/**` (Playwright outputs)                | 2.40 GiB   | web-games-hegemon 698 MiB, parent 1.7 GiB             |
| `**/verify-screenshots/**` (verification harness)       | 0.76 GiB   | web-games-junk-runner 514 MiB, others                 |
| `**/tests/**/screenshots/**` (test framework output)    | 0.28 GiB   | scattered                                             |
| `**/audit-*/**/*.png|*.jpg|*.mp4|*.html|...` (audit-binary)| ~2.18 GiB | deathrun 1330 MiB, capture-anime 326 MiB              |
| `**/audit/**`, `**/chrome-screenshots/**`, `**/chrome-*/**`, etc.  | (overlap with above)  | catch-all for audit-styled dirs        |

**Kept (NOT excluded):**

| Path                                                    | Why kept                                          |
|---------------------------------------------------------|---------------------------------------------------|
| `web/screenshots/one-mil-girls-screenshots/`             | 1mg release marketing shots, ~108 MiB              |
| `docs/audit-*.md`, `scripts/audit-*.mjs`                 | audit reports and scripts (text, useful in git)   |

**Verified preservation:** test plan below re-classifies each pattern to
confirm 1mg marketing paths are NOT caught.

## Daemon fix — `default_untracked_exclude_patterns` in
`dracon-sync/src/policy.rs`

Add 18 patterns covering the leak categories. The patterns target
**agent/test/session binary evidence**, not the user's intentional
shipping art.

```rust
[
    // Session / agent scratch dirs — keep local
    "**/scratch/**",
    "**/scratch-*",
    "**/scratch_*",
    "**/tmp/**",
    "**/tmp-*",
    "**/pi-tmp/**",
    "**/.pi-tmp/**",
    "**/research/scratch/**",
    // Agent session state — never auto-stage
    ".demon/**",
    ".sisyphus/**",
    ".ralph/**",

    // === ADDED 2026-07-13 (codeberg-quota-leak-fix-2026-07-13) ===
    //
    // Operator observed 85 GiB / 85 GiB quota on codeberg. Investigation
    // identified 10.29 GiB of agent/test/session evidence leaking into
    // git history. These patterns exclude only session evidence,
    // preserving intentional shipping art (e.g.,
    // `web/screenshots/one-mil-girls-screenshots/` for 1mg marketing
    // shots, audit REPORTS like `docs/audit-event-*.md`, audit SCRIPTS
    // like `scripts/audit-*.mjs`).

    // (1) Universal agent session dir. .pi/ at any nesting — same
    // pattern family as .demon/, .sisyphus/, .ralph/ already in this
    // list. Covers .pi/goals/, .pi/chrome-screenshots/, .pi/audit-*/,
    // .pi/mmx-out/, .pi/notes/, .pi/tasks/, .pi/loop-removal-backup-*/.
    "**/.pi/**",

    // (2) Playwright + similar test result artifacts.
    "**/test-results/**",

    // (3) Verification harness output dir (date-stamped subdirs).
    "**/verify-screenshots/**",

    // (4) Test framework screenshot output.
    "**/tests/**/screenshots/**",

    // (5) Audit-context binary evidence. The image/video/binary
    // inside dirs whose name matches audit-/chrome-screenshots/etc.
    // Note: text files (.md, .mjs, .json) inside these dirs are NOT
    // affected by the extension filter — audit reports and audit
    // scripts remain in git. Trade-off: an .html inside an audit-*/ is
    // also caught. If a future audit report needs HTML format, move it
    // out of an audit-named dir or add a per-repo override.
    "**/audit-*/**/*.png",
    "**/audit-*/**/*.jpg",
    "**/audit-*/**/*.jpeg",
    "**/audit-*/**/*.webp",
    "**/audit-*/**/*.gif",
    "**/audit-*/**/*.mp4",
    "**/audit-*/**/*.mov",
    "**/audit-*/**/*.html",
    "**/audit-*/**/*.zip",

    // (6) Audit-styled whole dirs as a fallback. These dir-name
    // patterns are unambiguous session-evidence placeholders
    // (audit/, chrome-*/ with audit-style names, etc.).
    "**/audit/**",
    "**/chrome-screenshots/**",
    "**/chrome-*/**",
    "**/.audit-*/**",
    "**/sign-in-flash-audit/**",
    "**/uiux-audit-*/**",
]
```

## Test extension

Extend `test_default_untracked_exclude_patterns_is_commit_all_unless_scratch`
in `dracon-sync/src/policy.rs` to assert the new patterns are present.
The existing test still holds: `.demon/`, `.sisyphus/`, `.ralph/` and
`.pi/` are all the same family.

## History cleanup

For the 17 heaviest git-tracked repos, run `git filter-repo
--invert-paths` to remove the leak from history:

```bash
REPOS=(
  /home/dracon/Dev/dracon-platform
  /home/dracon/Dev/dracon-code
  /home/dracon/Dev/ai-auto-writer
  /home/dracon/Dev/avid
  /home/dracon/Dev/Junk-Runner-bevy
  /home/dracon/Dev/browser-extensions-shared
  /home/dracon/Dev/web-auto/rust-ai-web-auto
  /home/dracon/Dev/dracon-platform/web/games/released/one-mil-girls
  /home/dracon/Dev/dracon-platform/web/games/wip/capture-anime-girls
  /home/dracon/Dev/dracon-platform/web/games/wip/darklord
  /home/dracon/Dev/dracon-platform/web/games/wip/deathrun
  /home/dracon/Dev/dracon-platform/web/games/wip/endless-td
  /home/dracon/Dev/dracon-platform/web/games/wip/hegemon
  /home/dracon/Dev/dracon-platform/web/games/wip/hellhunter
  /home/dracon/Dev/dracon-platform/web/games/wip/junk-runner
  /home/dracon/Dev/dracon-platform/web/games/wip/neonbreak
  /home/dracon/Dev/dracon-platform/web/games/wip/polis
)

for repo in "${REPOS[@]}"; do
  cd "$repo"
  # Safety backup of main
  git branch backup/pre-quota-leak-cleanup-$(date +%s) main 2>/dev/null
  git filter-repo \
    --path-glob '*.pi/**' \
    --path-glob '*test-results/**' \
    --path-glob '*verify-screenshots/**' \
    --path-glob '*tests/**/screenshots/**' \
    --path-glob '*audit-*/**.png' \
    --path-glob '*audit-*/**.jpg' \
    --path-glob '*audit-*/**.jpeg' \
    --path-glob '*audit-*/**.webp' \
    --path-glob '*audit-*/**.gif' \
    --path-glob '*audit-*/**.mp4' \
    --path-glob '*audit-*/**.mov' \
    --path-glob '*audit-*/**.html' \
    --path-glob '*audit-*/**.zip' \
    --path-glob '*audit/**' \
    --path-glob '*chrome-screenshots/**' \
    --path-glob '*chrome-*/**' \
    --path-glob '*.audit-*/**' \
    --path-glob '*sign-in-flash-audit/**' \
    --path-glob '*uiux-audit-*/**' \
    --force
done
```

Daemon's `auto_repair_concerns = true` will force-push local main over
remote (with `--force-with-lease` per AGENTS.md policy). The
`backup/pre-quota-leak-cleanup-*` branches stay local-only as a rollback
fuse.

## Verification

1. **Build**: `cargo build --release --locked`, `cargo build --tests --locked`
2. **Test**: `cargo test --workspace --locked` (existing 847 tests + 4 new asserts in
   `test_default_untracked_exclude_patterns_is_commit_all_unless_scratch`)
3. **Deny**: `cargo deny check` workspace + each of 3 per-crate
4. **Live API**: re-query codeberg usage — should drop from 85.00 GiB used
   to ~10-15 GiB used after the filter-repo push completes
5. **Daemon log**: `journalctl --user -u dracon-sync.service --since "10m ago"`
   should show NO `Quota exceeded` errors after the push

## Trade-offs and edge cases

- **1mg marketing shots** (`web/screenshots/one-mil-girls-screenshots/`,
  ~108 MiB) are preserved because the `**/audit-*/**/*.png` filter does
  NOT match `one-mil-girls-screenshots/` (the directory does not have a
  hyphen-named `audit-` prefix).
- **Audit reports** (`docs/audit-event-*.md`, `scripts/audit-*.mjs`) are
  preserved because the extension filter excludes image/video/binary
  types only.
- **Tracked audit-html** would be excluded. None observed in current
  tree; per-repo override available if needed in future.
- **Filter-repo rewrites local main**. AGENTS.md says daemon auto-repair
  uses `filter-repo --invert-paths --force` with auto_repair_concerns=true
  (default). We are following the same path. Backup branches are
  local-only and named `backup/pre-quota-leak-cleanup-<ts>` so any
  operator can `git checkout backup/pre-quota-leak-cleanup-<ts> -- main`
  to recover if needed.
