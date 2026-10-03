# Warden pre-push scan residuals (Tier-2 future work)

**Date:** 2026-10-03
**Status:** Accepted (audit R4-W-10: "none required; track Tier-2 hook coverage as future work")
**Scope:** `dracon-warden` pre-push hook only

## Context

The pre-push hook is defense-in-depth behind the clean filter: it scans
pushed commits for secret shapes in case the filter was bypassed. The
hook's `SECRET_RE` renders from `hook_token_shapes_ere` (scanner.rs),
a POSIX-ERE transliteration of the Tier-1 provider-token shapes plus
`password`/`secret`/`api_key` assignments. Three residuals are
conscious tradeoffs, recorded here so a future audit can tell
"accepted gap" from "regression".

## Residual 1 — No Tier-2 hook coverage

The filter's `tier2_patterns` (scanner.rs:366 — AWS secret access
keys, session tokens, generic keyword-anchored and high-entropy
shapes) has no hook transliteration. Concretely, all of the following
push clean when the filter is bypassed:

- unquoted `secret=` / `api_key=` assignments (the hook only has the
  quoted forms plus unquoted `password=`);
- AWS secret access keys (`(?i)aws(.{0,20})?['"][0-9a-zA-Z/+]{40}['"]`);
- generic high-entropy Tier-2 shapes.

Why accepted: Tier-2 shapes are low-floor by design (entropy +
keyword proximity, `(?i)` matching) — porting them to a line-oriented
`grep -E` scan would block legitimate pushes (fixtures, docs, prose
mentioning `secret =`). The filter remains the enforcement point for
Tier-2; the hook only needs the high-confidence shapes that are never
false positives.

Future work: a bounded Tier-2 hook subset (e.g. the quoted AWS-secret
shape, which has a rigid 40-char body) with a measured false-positive
run over fleet history before enabling. Do NOT transliterate the
`(?i)` / entropy shapes without that measurement.

## Residual 2 — Newline-in-filename edge

The hook iterates `git diff-tree -z` output via `tr '\0' '\n'` +
`IFS= read -r` (main.rs PRE_PUSH_HOOK, "residual newline-in-filename
edge is accepted as absurd"). A filename containing a literal newline
splits into two fragments; neither fragment resolves to the real
path, so the diff scan's pathspecs match nothing and the added-loop
`git cat-file blob $sha:<fragment>` fails silent — a secret in such
a file pushes clean (allow-shaped miss, both loops).

Why accepted: POSIX allows newline in filenames but no tool in the
fleet creates them; git itself quotes such paths in most non-`-z`
outputs. Handling true NUL-delimited iteration in `/bin/sh` (no
`read -d`) would require restructuring the loop around a helper.

Future work: if ever needed, replace the `tr` stage with an
NUL-aware iteration (e.g. a `while` over `git diff-tree -z` piped
through `xargs -0 -n1`).

## Residual 3 — Modified-binary parent-match allowance

The changed-binary loop (main.rs PRE_PUSH_HOOK, "Binary
modifications have no added text lines") blocks only match strings
NOT already present in a parent blob: a new binary blob whose secret
matches are all byte-identical to matches in a parent passes. A
grandfathered match string copied into a NEW binary file therefore
pushes clean.

Why accepted: without the allowance, every binary touch re-trips on
grandfathered content (observed class: re-encoded assets carrying old
placeholder strings). The added-file loop still judges
never-before-seen blobs strictly.

Future work: none planned; if the bypass matters, compare per-file
(parent-blob-of-SAME-path only — already the case) AND require the
matching parent blob to be on a remote (blob-novelty, as the added
loop does).

## Verification

- Residual 1: `SECRET_RE` membership is pinned by
  `test_hook_shapes_cover_tier1_names` (no Tier-1 entry without a
  hook shape); Tier-2 exclusion is by construction (no transliteration
  source exists).
- Residuals 2–3: documented in hook comments at the use sites; this
  doc is the durable record. No behavior change in R4-W-10.
