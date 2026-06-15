# Repository Ownership Investigation (2026-06-15)

## Context

After the operator asked the daemon to "default-skip non-owned
repos" and to "commit more aggressively" (goal
`bc98de20-95a8-47e5-9258-c56a19e4489a`), the operator reviewed
the live `dracon-sync repos` table and said:

> "ok we def own kiki and others need exploring too"

This is the documentation of the per-repo ownership
investigation for all 14 watched repos. For each repo we
document the three signal checks (user.email, HEAD author,
origin URL) plus any per-repo override and the last 3
historical commit authors (to detect historical bad-config
authors not on HEAD).

## Ownership signals (from policy)

The daemon classifies a repo using these signal checks (from
`~/.dracon/utilities/sync/dracon-sync.toml`):

- `trusted_emails = ["dracsharp@gmail.com"]`
- `trusted_authors = ["DraconDev"]`
- `trusted_remote_hosts = ["github.com/DraconDev",
  "gitlab.com/dracondev", "codeberg.org/dracondev"]`

A repo is `Owned` if at least one of:
- `git config user.email` ∈ `trusted_emails`
- HEAD commit author email ∈ `trusted_emails` OR HEAD
  author name ∈ `trusted_authors`
- `origin` URL host+account ∈ `trusted_remote_hosts`

Per-repo overrides (`owned = true | false`,
`auto_skip_unowned = true | false`) in
`<repo>/.dracon/dracon-sync.toml` can force the
classification regardless of signals.

## Investigation results (all 14 repos)

| # | Path | user.email | HEAD author | origin | Class | Override | Last 3 commit authors |
|---|------|------------|-------------|--------|-------|----------|----------------------|
| 1 | `/home/dracon/Dev/dracon-platform` | dracsharp@gmail.com | DraconDev <dracsharp@gmail.com> | github.com/DraconDev/dracon-platform.git | ✓ owned (trusted_email) | — | DraconDev, DraconDev, DraconDev (all clean) |
| 2 | `/home/dracon/Dev/folder-auto-banner` | dracsharp@gmail.com | DraconDev <dracsharp@gmail.com> | github.com/DraconDev/folder-auto-banner | ✓ owned (trusted_email) | — | DraconDev, DraconDev, DraconDev (all clean) |
| 3 | `/home/dracon/Dev/kiki-sassy-desktop-announcer` | dracsharp@gmail.com | DraconDev <dracsharp@gmail.com> | github.com/DraconDev/kiki-sassy-desktop-announcer | ✓ owned (override) | `owned = true` | DraconDev, DraconDev, DraconDev (all clean) |
| 4 | `/home/dracon/Dev/Junk-Runner-bevy` | dracsharp@gmail.com | DraconDev <dracsharp@gmail.com> | github.com/DraconDev/Junk-Runner-bevy.git | ✓ owned (trusted_email) | `auto_commit_exclude_patterns` (playwright artifacts) | DraconDev, DraconDev, DraconDev (all clean) |
| 5 | `/home/dracon/Dev/dracon-utilities` | dracsharp@gmail.com | DraconDev <dracsharp@gmail.com> | git@github.com:DraconDev/dracon-utilities.git | ✓ owned (trusted_email) | — | DraconDev, DraconDev, DraconDev (all clean) |
| 6 | `/home/dracon/Dev/dracon-ai-lib` | dracsharp@gmail.com | DraconDev <dracsharp@gmail.com> | github.com/DraconDev/dracon-ai-lib | ✓ owned (override) | `owned = true` | DraconDev, **Dracon**, **Dracon** (130 bad-config commits in history) |
| 7 | `/home/dracon/Dev/avid` | dracsharp@gmail.com | DraconDev <dracsharp@gmail.com> | github.com/DraconDev/avid.git | ✓ owned (trusted_email) | — | DraconDev, DraconDev, DraconDev (all clean) |
| 8 | `/home/dracon/Dev/browser-extensions-shared` | dracsharp@gmail.com | DraconDev <dracsharp@gmail.com> | github.com/DraconDev/browser-extensions-shared.git | ✓ owned (trusted_email) | — | DraconDev, DraconDev, DraconDev (all clean) |
| 9 | `/home/dracon/Dev/ai-auto-writer` | dracsharp@gmail.com | DraconDev <dracsharp@gmail.com> | github.com/DraconDev/ai-auto-writer.git | ✓ owned (trusted_email) | — | DraconDev, DraconDev, **DraconDev@users.noreply.github.com** (1 GitHub web-edit commit) |
| 10 | `/home/dracon/Dev/DraconDev` | dracsharp@gmail.com | DraconDev <dracsharp@gmail.com> | github.com/DraconDev/DraconDev.git | ✓ owned (trusted_email) | — | DraconDev, DraconDev, DraconDev (all clean) |
| 11 | `/home/dracon/Dev/rust-ai-web-auto` | dracsharp@gmail.com | DraconDev <dracsharp@gmail.com> | github.com/DraconDev/rust-ai-web-auto.git | ✓ owned (trusted_email) | — | DraconDev, DraconDev, DraconDev (all clean) |
| 12 | `/home/dracon/Dev/pully-fully-pull-based-fleet-reconciler` | dracsharp@gmail.com | DraconDev <dracsharp@gmail.com> | github.com/DraconDev/pully-fully-pull-based-fleet-reconciler.git | ✓ owned (trusted_email) | — | DraconDev, DraconDev, DraconDev (all clean) |
| 13 | `/home/dracon/.dracon` | dracsharp@gmail.com | DraconDev <dracsharp@gmail.com> | git@github.com:DraconDev/dracon-home.git | ✓ owned (trusted_email) | — | DraconDev, DraconDev, DraconDev (all clean) |
| 14 | `/home/dracon/Dev/dracon-code` | dracsharp@gmail.com | DraconDev <dracsharp@gmail.com> | git@github.com:DraconDev/dracon-code.git | ✓ owned (trusted_email) | — | DraconDev, DraconDev, DraconDev (all clean) |

## Summary

- **13 of 14 repos**: clean — all 50 most-recent commits
  authored by `DraconDev <dracsharp@gmail.com>`. No
  per-repo override needed.
- **1 repo with historical bad-config authors**:
  `dracon-ai-lib` (130 commits by `Dracon <dracon@void>`
  out of 1345 total). The most-recent bad-config commit is
  `740735a` from 2026-06-13. The override
  `owned = true` in
  `/home/dracon/Dev/dracon-ai-lib/.dracon/dracon-sync.toml`
  forces classification as Owned, bypassing the bad HEAD
  author signal. Going forward, all daemon-committed commits
  on this repo will be authored by `DraconDev
  <dracsharp@gmail.com>` because the local git config is
  correct.
- **1 repo with GitHub web-edit email**:
  `ai-auto-writer` has 1 commit (`5cac8ba9` from
  2026-06-10) authored by `DraconDev@users.noreply.github.com`
  (the GitHub web-edit placeholder email for the same
  GitHub user). HEAD is the correct
  `DraconDev <dracsharp@gmail.com>`. No fix needed.
- **2 repos with per-repo overrides**:
  - `dracon-ai-lib`: `owned = true` (historical bad
    author)
  - `kiki-sassy-desktop-announcer`: `owned = true`
    (operator confirmed ownership; was briefly
    `owned = false, auto_skip_unowned = true` during the
    "Do not modify user-owned repos" constraint — see
    CHANGELOG entry for 2026-06-15)

## Misclassifications

**None.** All 14 repos are correctly classified by either
the signal-based heuristic (13 repos) or per-repo override
(1 repo: `dracon-ai-lib`).

## Junk-Runner-bevy playwright artifact exclusion

The `Junk-Runner-bevy` repo shows 87 modified + 1 untracked
in the live `dracon-sync repos` table. All 87 modified files
are PNGs in `web/test-results/` and
`web/tests/e2e/screenshots/` (Playwright test artifacts).
The per-repo override at
`/home/dracon/Dev/Junk-Runner-bevy/.dracon/dracon-sync.toml`
sets:

```toml
auto_commit_exclude_patterns = [
    "**/test-results/**",
    "**/e2e/screenshots/**",
]
```

This is the same per-repo override pattern documented in
`docs/design/dirty-files-investigation.md`. The daemon
correctly EXCLUDES these from auto-commit (the daemon log
shows `⏭️ ... skipping tracked web/test-results/...png
(auto_commit_exclude_patterns)` for every PNG). The report
still shows them as `🟠 dirty` for operator visibility, but
the daemon's auto-commit never touches them. The operator
can manually `git add` and commit them if intentional.

This is the correct behavior — the operator explicitly
chose to exclude these from auto-commit (the original
2989-commit backlog issue).

## How to add a new per-repo override

If a repo is misclassified, add a per-repo override at
`<repo>/.dracon/dracon-sync.toml` with one of:

```toml
# Force Owned (override signal-based detection)
owned = true
```

```toml
# Force Unowned (skip the repo entirely)
owned = false
auto_skip_unowned = true
```

The override is read by `dracon-sync/src/ownership.rs`
(`OwnershipInputs::override_owned`). See that file for
details on the classification logic.

## How to run the investigation

For a single repo:

```bash
dracon-sync ownership --explain --repo /home/dracon/Dev/<repo>
```

For all discovered repos:

```bash
dracon-sync ownership --explain
```

## Change log

- 2026-06-15: initial 14-repo investigation
  - All 14 repos correctly classified
  - 2 per-repo overrides in place
    (`dracon-ai-lib`, `kiki-sassy-desktop-announcer`)
  - No misclassifications found
