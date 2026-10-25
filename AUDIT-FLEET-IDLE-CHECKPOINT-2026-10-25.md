# Fleet idle repair checkpoint — active, not accepted

Active goal: `.pi-glla/goals/20261025090441-ya0tp4.md`.
Baseline task1 is complete; implementation2/2.1 and necessary gate repairs3/3.1
remain in progress. No new qualified deployment or whole-goal completion claim.
The strict all37 clean+idle requirement is unchanged.

## Current independently assessed work

- Full37 inventory, remote/object queries, staged raw identities and local-path
  lists are in `dracon-sync/evidence/fleet-idle-convergence-2026-10-25/`.
  No repository/ref-query failures; all intended destinations resolve. Browser
  origin+GitLab were behind observed HEAD, not hidden or labelled converged.
- Source-only coverage-aware inspection implementation passed11 development
  tests. It only suppresses recursive child dirt when every indexed gitlink is
  independently in the observation's watched inventory; child HEAD/staged-link
  changes remain visible. The initial unused import warning was fixed afterward;
  final coherent strict qualification still required. No deployment yet.
- Warden rendered-hook correction accepts exact root declarations under
  docs/screenshots, never broad PNG/type globs, generic caches or stronger
  scratch/Chrome/audit-cache categories. Security still refuses a deliberately
  broken required filter. Four targeted hygiene tests pass, retained red first.
  Original serial component suite passed232unit+18integration+2stream.
  A parent experiment overriding configured serial tests with threads2 caused
  existing shared-PATH interference (231/1); retained truthfully, no assertion
  weakened. See Warden docs/design/scoped-audit-deliverables-2026-10-24.md and
  fleet-idle-warden-* receipts/logs. No new installed hook.
- Doomtap declares74 exact comparison-contract names (14surfaces*3,10states*3,
  capture manifest and baseline README). Its two new Node contract tests pass,
  with effective root-ignore-rule and meaningful negative controls. No media,
  baseline, tolerance or UI assertion changed. Its ORIGINAL tooling-layer gate
  REFUSED exit2 due natural host thrashing (load86, swap126/162MiB/s): nothing
  ran, no verdict. No preflight skip or stopped producer. Original product gate
  still needs a suitable host. See Doomtap docs/design/visual-input-publication-2026-10-25.md.

## Staged intent is NOT repaired yet

Worker75a67370 added capture building block and inode-owned generic lock cleanup:
`src/git/{staged_intent.rs,status.rs,mod.rs}`. Five new and two old lock tests
passed, exact source hashes independently matched. All successful/failed logs
were archived under checks/staged-intent-building-block. Nine production dead
code warnings remain until legitimate integration; not suppressed.

Independent review d93d7087 BLOCKED this building block on two P1 issues:
non-Unix IndexLock would retain every successful lock; synchronous gitdir/lock
acquisition cannot be bounded by Tokio timeout. Also cancellation test did not
observe actual PID/descendant termination and flags/sparse/split tests were thin.
Do not interpret initial passing counts as reviewer approval.

Follow-up worker **b7b7b1bc-73d7-4ac8-8d08-341811cffcfe** has finished.
Parent inspected source and raw seven-test passing output and archived report,
logs and exact source hashes in checks/staged-intent-review-fixes. The two old
lock regressions also passed per retained report. Non-Unix regression exists but
was not executed on Linux. Independent re-review is running as
**abe9811e-b1ac-4c11-b2c0-f3b59e229d8f**, read-only; assess before integration.
No staging/commit integration or final qualification yet; dead-code warnings
remain unsuppressed. Implemented design change: capture is a read-only observation through
one owned cancellable `git ls-files --stage --debug --sparse -z` process, binding
entries and flags from the same index read, with HEAD before/after. No synchronous
real-index lock in capture, no spawn_blocking timeout fiction. A later atomic
commit boundary MUST revalidate exact identities; this observation is not
commit authorization. Generic unsupported-platform lock acquisition must refuse
BEFORE creating a file. Additional flag/sparse/split/cancellation tests requested.
One bounded targeted test attempt finished after4m04compilation and26.42s tests;
seven passed with no failures, including actual PID and shared-runner descendant
termination. Do not confuse those observations with final atomic commit safety.
Assess the independent re-review before integration.

The source-proven net-HEAD classification pruning seam remains distinct from
Doomtap's actual latest precommit screenshot refusal. Prepared enrollment was
ruled out for current Doomtap. Do not union cached pathnames, re-add partial
staging from worktree or waive extras/private guards to obtain green.

Native Git2.51.2 source plus ten installed fixtures rule out a commit-msg hook
wrapper as an atomic final-tree solution: plain commit releases index lock and
selects tree before message hooks. Reports archived as native-commit-seam-scout
and staging-design-followup. Need genuine immutable-tree/expected-parent
transaction and newly staged provenance, preserving hooks and uncertain index.

## Latest browser observation and tooling caveats

`dracon-sync repos browser-extensions-shared --json` exited0 but emitted HUMAN
text, saved verbatim as browser-current-report.json (not valid JSON). Detail mode
ignores the requested machine format; do not parse it as a machine receipt or
count it as full fleet observation. It showed zero local dirt,163ahead, publishing
with push FAIL15m and upstream age2h. Actual autonomous publication remains open;
active publication state alone is not acceptance. Diagnose its actual latest gate
failure/progress next, without manual recovery pushes or stopping editors.

Parent lock briefly lagged concurrent dracon-system manifest0.11250 at0.11249.
Another actor reconciled it before parent's exact edit could apply; that edit made
NO change. Both then0.11250 and locked cargo metadata passed. Worker was explicitly
allowed one bounded retry, not removing --locked or editing dependency guards.

Final qualification must identify one coherent assembly, preserve all original
failed/refused outcomes, pass full workspace/release/all-target Clippy/deny and
original affected product gates, verify installed execution, then capture all37
clean+idle plus every intended advertised ref. None of that is waived by these
component results or the background task finishing.

## Latest continuation: actual integration is now the priority

The re-review of the two P1 fixes found no remaining issue in that observation
building block, explicitly not atomic commit safety. Native batched mktree tree
materialization was then implemented; worker's first8/10 run timed out in two
tests because ChildStdin shutdown did not deliver EOF. The explicit stdin drop
was fixed afterward. Parent independently reran FINAL source:10/10 passed;
checks/tree-materialization/parent-after-eof-fix.log. Independent current-source
review65853eb1 found no issue in the materializer or Warden runtime seam; no
committed-range or whole-assembly approval. Production integration still open.

Warden source now uses a real two-worker runtime constructor (same async dispatch,
blocking filter worker retained). One constructor test passed. An isolated OLD
installed filter-process with no request measured18threads. Service has TasksMax
512 and cumulative pids.events max104; a prior browser snapshot install failed
Go newosproc errno11 having4threads despite scripts-concurrency1 and existing
hook native-thread caps2. Amplification is proven; unique attribution at the
failure timestamp is NOT. No new Warden binary/hooks installed. Full previous
252 tests predate this runtime change; not a new full-suite pass. See Warden
docs/design/bounded-cli-runtime-2026-10-25.md and its raw receipts.

Fixed the demonstrated `repos <name> --json` text-output bug: selected JSON detail
now reaches the ordinary JSON envelope, retaining not-found/ambiguous semantics.
An actual-binary hermetic integration test passed; the improved test clears all
inherited Git injection variables in its child commands (no global mutation).
Logs checks/report-json-detail{,-hermetic}.log. This is not yet deployed.

Read-only fresh index-format audit across ALL37 BASELINE-COHORT repositories:
zero query/parse failures, zero unsupported flags/sparse entries/unmerged stages.
See parent AUDIT-FLEET-INDEX-FLAGS-2026-10-25.md. This is not final current discovery
or idle acceptance and cannot justify omitting later-discovered repositories.

Browser historical snapshot cb2117... failed auto-form-filler imports spelled
@drakon instead of its actual @dracon dependency; current canonical source no
longer has those imports, so parent did not duplicate or regress the other
editor's fix. The next captured push epoch8bf8... was running; later outcomes
still need inspection, not inference from its publishing label. Historical
Tabmon TEST failure remains to assess against current source. No product gate
assertion was weakened and no manual recovery push occurred.

**Active implementation worker6cb3b8f5-d12a-4569-a7b5-389bc4a58562** is sole
writer in dracon-sync. Its task is actual staged-epoch production integration
using docs/design/staged-epoch-integration-2026-10-25.md, not another unconsumed
helper. Preserve JSON detail/inspection changes. Parent approved a storage
verification-only immutable-tree entrypoint preserving real bindings, repo ID,
authenticated metadata/provenance/readback; execute potentially blocking
verification through a bounded qualified helper, not spawn_blocking timeout.
Old storage preparer cannot be called under the epoch lock because it acquires
its own CommitLock and rereads live index. No fabricated enrollment or unguarded
fallback is allowed. A read-only verifier cannot take/remove caller locks or
turn an asserted ownership bool into authority. Await the worker, assess exact
implementation/tests/gaps, then independently review BEFORE qualification.

Parent should not edit dracon-sync source while that writer is active. Warden
and browser/parent-meta work is separate. Helpers alone still do not satisfy
staged-intent reconciliation; task2.1 remains open, as do all deployment,
original product-gate and all-current-watched clean+idle acceptance gates.

Storage/executable follow-up approved to that worker: add immutable-tree
verification-only CLI/API preserving authenticated authority; never call the old
lock-taking/libgit2 commit preparer inside the new epoch lock or silently drop
storage/signing. Potentially blocking validation must run in an owned bounded
helper process. Unit-test current_exe is a Rust harness, installed CLI is old;
no fallback may treat either as the new verifier. A deterministic build.rs/source
closure digest plus protocol/tree handshake was approved (all relevant Rust
source, manifest/lock, target/features, relative sorted names), not merely a hash
of two files. Minimal build-dependency on already-used sha2 permitted without
weakening locked/deny checks. This is stale-helper detection, not malicious binary
attestation; actual executable receipts are still required. Production defaults
to its qualified executable; cfg(test)-only absolute helper override must use the
same real verifier, never a success stub. Parent will do ONE jobs2 locked helper
build after worker says source is frozen, then tests use that exact binary. No
source edits during matching build. Helper gets child-only Tokio worker cap2;
parent will not start competing builds while that source writer is active.

Latest worker decision: ordinary native add can overwrite foreign same-path
staging even before a fresh follow-up epoch, and existing hygiene/reset paths
can erase later actual-index intent. Parent approved the necessary alternate-
index staging rewrite, not permanent ordinary-path refusal or silent disabling
of build_artifact_cleanup. All add/rm/gitlink/hygiene mutations must use a private
captured-baseline index and conditionally install exact proven deltas under the
actual owned lock. On mismatch preserve actual staging and any otherwise-lost
private staged version with durable diagnostics; no stale restore/reset or
unresolved conflict labelled idle. Existing destructive opt-ins need explicit
content/ownership handling, not ignored fields. Bounded no-follow private message
IO is approved so another editor's COMMIT_EDITMSG is not overwritten. If the
bounded slice ends incomplete, report gaps honestly; no deployment/completion.
