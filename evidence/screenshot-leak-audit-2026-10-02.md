# Screenshot-leak audit 2026-10-02 (agent-captured media in public repos)

Trigger: 2026 reports of AI coding agents leaking ~13,000 internal
screenshots to public GitHub repos. Our fleet combines agent loops that
capture screenshots with a daemon that commits everything and pushes to
public GitHub within seconds — the exact high-risk shape — so a
one-shot audit of already-committed media was warranted.

## Method

1. Enumerated all 35 daemon-watched repos (`dracon-sync repos --json`).
2. Queried GitHub visibility via `gh api` (30 repos) and GitLab
   visibility via `glab api` (all 35). Codeberg has no configured CLI
   here and was NOT checked.
3. Listed tracked media (`png/jpg/gif/webp/avif/bmp/tiff/ico/heic/psd/
   mp4/mov/mkv/webm/avi/mp3/wav/flac/ogg/pdf`) in every public repo.
4. Visually inspected all 20 files found (12 unique views; benchmark
   samples share one family).

## Result: clean, no exposure

- Public set is identical on GitHub and GitLab (11 repos): pi-plugins,
  pi-goal-list-loop-audit, dracon-warden, dracon-system, dracon-sync,
  dracon-utilities, folder-auto-banner, pi-agnes-tools,
  pi-use-last-selected-thinking-level, pi-codebuddy-sdk, DraconDev.
  No cross-forge visibility mismatch found.
- 20 tracked media files across the 11 public repos, all benign:
  - Terminal-UI renders of the operator's own open-source plugins
    (goal widget, lifecycle states, decision card, `--help` output,
    repo listings). Worst case visible: username/hostname
    (`dracon@nixos`, already public via the GitHub account).
  - Test/benchmark fixtures (synthetic gibberish OCR table, TUI
    smoke render, 68-byte `tiny.png`).
  - Generic art (banners, thumbnails, logos).
- No internal dashboards, customer data, credentials on screen,
  authenticated sessions, or PII in any inspected image.

## Follow-ups (from the 2026-10-02 discussion)

- DONE: `media_protected_patterns` opt-in Warden key (off by default)
  for whole-file encryption of selected binary media.
- OPEN, low priority: producer path discipline — agree on capture
  directories per agent loop so the new option has stable patterns
  to match. The live-browser-audit loop (drives signed-in Chrome) is
  the one worth pinning down first.
- NOT PURSUED: public-push guard for binaries. The audit found zero
  near-misses, so the backstop is deferred until real evidence of
  need appears. Revisit if a sensitive capture ever lands in a
  public repo.
- CAVEAT: Codeberg visibility was not verified (no CLI available).
  GitHub and GitLab agree everywhere, which suggests the daemon
  keeps visibility consistent, but it is unverified.
