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

## Residual 2 — Newline-in-filename edge — CLOSED 2026-10-09 (audit D15)

**Status: CLOSED.** The operator chose to close it (audit D15, side (a))
after this doc's own future-work note named the change and the hook had
since gained `xargs -0` pathspec passing. Implementation: every
`git diff-tree -z` list is captured and validated by
`reject_newline_paths` (main.rs PRE_PUSH_HOOK) BEFORE the
`tr '\0' '\n'` flattening, at all three scan sites (the hatch filter,
the added-file blob loop, and the modified-binary loop). A path
containing a newline now REFUSES the push with an actionable message.

Why refuse rather than iterate: in `-z` mode git emits no newlines of
its own, so any newline byte in the stream is inside a path — detection
is exact and needs no reconstruction. True NUL-aware iteration was
evaluated and rejected for this hook: the only portable construct is
`xargs -0`, which runs a child shell in which `exit 1` cannot abort the
push (a fail-OPEN trap) and in which the blob-novelty helper's lazily
cached remote object list could not be shared without duplicating that
helper or making it eager on every push; `while IFS= read -r -d ''` is
bash-only and this hook ships with `#!/bin/sh` behind crates.io. The
refusal is fail-closed: the pathological file cannot be verified, so it
does not go up unnoticed.

Cost accepted: a repo whose paths legitimately contain newlines must
rename them (or bypass with `git push --no-verify`, this hook's already
documented general bypass).

Regression coverage:
`test_prepush_newline_named_file_refuses_instead_of_skipping`
(dracon-warden/tests/integration_test.rs) drives the real installed
hook against a real repo holding `evil\nsecrets.env` containing a live
AWS secret shape and asserts the push is refused and names the cause.
Verified failing against the pre-fix hook (the push succeeded with zero
output) and passing after. The paired
`test_prepush_space_named_file_still_pushes` pins the F4.6 guarantee
that space-containing paths keep pushing clean.

Original text, for the record: the hook iterated `git diff-tree -z`
output via `tr '\0' '\n'` + `IFS= read -r`, so a filename containing
a literal newline split into two fragments; neither fragment resolved
to the real path, the diff scan's pathspecs matched nothing and the
added-loop `git cat-file blob $sha:<fragment>` failed silent — a secret
in such a file pushed clean (allow-shaped miss, both loops). Why it had
been accepted: POSIX allows newline in filenames but no tool in the
fleet creates them; git itself quotes such paths in most non-`-z`
outputs, and true NUL-delimited iteration in `/bin/sh` (no `read -d`)
would have required restructuring the loop around a helper.

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
