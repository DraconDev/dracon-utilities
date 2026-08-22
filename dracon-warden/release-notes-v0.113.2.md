# dracon-warden v0.113.2 — pre-push hook `--not --remotes` scan (tag-push false-positive fix)

Released 2026-07-27. Smallest-possible follow-up to v0.113.1
(uncovered by the operator's first tag push for v0.113.4 in
`dracon-sync`). F0.1 follow-up — the scan range for the
BAD_AUTHORS defense was broadened to "everything reachable from
the pushed ref" which caused a tag-push false-positive.

## Fixed

### `BAD_AUTHORS` scan false-positive on tag pushes

For a **branch** push the hook's `$RANGE = remote_sha..local_sha`
already implicitly limits the scan to only the new commits being
added to the branch tip. But for a **tag** push `remote_sha = 0`
(brand new ref), so the hook computed the range as
`empty-tree..tag-sha` — the **entire repo history reachable from
the tag object**, not just the NEW commits. A test-identity commit
reachable only via a non-first-parent merge of a feature branch
(where the daemon's drop-test helper left a `test@test` author on
the side) then blocked the tag push even though main's
first-parent history is clean and the test identity was already
published to all mirrors by a prior commit push.

The published branch that every forge renders and every consumer
reads is exactly the first-parent chain — anything already on any
remote-tracking branch has already been accepted by a prior F0.1
scan, so re-scanning it on a later tag push is wasted and prone to
false positives.

The scan now distinguishes:

- **Existing-ref update** (branch push, `remote_sha != 0`):
  `git rev-list "$local_sha" --not "$remote_sha"` — only the new
  commits being added to the branch tip.

- **New-ref push** (tag or new branch, `remote_sha == 0`):
  `git rev-list "$local_sha" --not --remotes` — only commits
  reachable from the tag object that are NOT already on ANY
  remote-tracking branch.

Defense in depth is preserved for the new push itself.

### Regression test

`pre_push_hook_test_identity_on_non_first_parent_merge_passes`:

- Sets up a `--no-ff` merge of a feature branch where the feature
  commits are authored by `test@test`.
- Models the production scenario by creating a fake
  `refs/remotes/origin/main` pointing at the merge commit
  (representing the prior branch push that already published the
  poisoned commits).
- Asserts that a TAG push (`remote_sha = 0`) is ALLOWED — the new
  `--not --remotes` exclusion returns empty.
- **Counter-test**: deletes the fake remote ref and re-pushes;
  asserts the hook REJECTS the same commit-set with "test identity"
  in stderr. Defense-in-depth for the truly-new case is intact.

## Tests

- 92 → 93 in `dracon-warden` (one new test, one updated
  `pre_push_hook_rejects_test_identity_author`); integration suite
  unchanged at 10/10.
- All 103 pass; clippy clean.

## Upgrade notes

- Re-run `dracon-warden setup-hooks` after upgrade — the
  `pre-push` hook content changed.
- No operator action beyond the hook re-install.
- The prior `pre_push_hook_rejects_test_identity_author` test
  (branch-push of a poisoned commit) still passes — defense in
  depth for the typical case is intact.

## Audit cross-references

- F0.1 follow-up to `AUDIT_FULL_2026-07-26.md` (not a new finding
  — the scan was correct in principle; the range selector was
  overly broad for tag pushes).
- Triggered by the operator's `git push origin v0.113.4` attempt
  in `dracon-sync` for the v0.113.4 release tagging.