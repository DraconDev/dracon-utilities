# Quarantine treadmill (2026-10-04)

## Symptom

`/mnt/data` (440G second drive) hit 91% with 42G free, filling ~60G/day.
Blame first fell on cold relocate; the actual filler was quarantine:
192G across 49 entries, all moved Oct 1–4, i.e. since quarantine-first
routing went live (2026-10-01 namespace fix). Swap was healthy (14G of
124G used), so this did not explain the earlier OOM kills — but a full
`/mnt/data` plus `/` at the 92% critical tier was the live fire.

## Root cause: the treadmill

Agent loops rebuild `target/` within hours of a quarantine move, and the
guard (action tier, `clean_quarantine_first = true`) quarantines the
rebuild as a brand-new generation that sits for the full 30-day TTL:

- `terhub/target`: 9 generations in 3 days
- `eve/target`: 3 generations, 39G
- `ai-auto-writer/target`: 4 generations, 25G

Nothing deduped; nothing expired for weeks. Emergency dedup to
newest-per-origin freed 74G (91% → 74%).

## Fix: replace-on-re-quarantine

An auto move now records `auto: true` in its manifest and, after the new
copy is secured, deletes older AUTO generations of the same origin:
quarantine holds at most one auto generation per origin. Deliberate
asymmetries:

- Manual `quarantine move` records `auto: false` and never replaces —
  deliberate snapshots accumulate and age out at TTL.
- Pre-flag manifests (missing key) parse as manual: unknown provenance
  fails closed, so the fix never deletes an entry it cannot prove is
  auto-created. (The pre-existing duplicates were hand-deduped once.)
- Unreadable manifests keep the expiry fail-safe: pinned, never replaced.
- Replacement runs after manifest publish and is best-effort (loud
  failure, TTL backstop): it can only cost disk, never data.

8 new tests; mutation-verified (flipping the provenance flag fails 3).

## Residual: 21 unmanifested hand-copies (~78G)

Oct 3, 13:30–19:34, twenty-one `target.<nanos>` dirs appeared with no
manifest and no guard log line (the guard logs every move it makes, and
its copy path removes the entry dir on any verification failure — a
guard move cannot produce a manifest-less entry silently). Dep-info
fingerprints tie 14 of them to treadmill origins (terhub ×6, eve ×5,
ai-auto-writer ×2, folder-auto-banner ×1), several with identical build
hashes — duplicate copies of duplicates. No shell history identifies the
actor; most likely an agent cleanup session copying (not moving) target
dirs into quarantine as "safe" staging.

They are pinned forever by the fail-safe (no origin record, never
expire, never replaced). Purge is an operator call after inspecting the
fingerprints above — `quarantine purge <name> --apply` per entry, the
documented escape hatch. The treadmill fix does not touch them.
