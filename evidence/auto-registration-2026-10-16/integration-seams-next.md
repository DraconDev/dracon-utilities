# Integration continuation — components are not rollout

## Current ownership

The nested `dracon-sync` main line is single-writer. Production S3 transport
handoff is complete (four files through 5cdfd9d); independent read-only review
is pending. The sole remaining writer owns only consumer.mjs and its tests;
parent is read-only in that checkout until its handoff. No production profile,
credential, service or application build script has been changed.

## Settled parent checks

Binding component static review accepted the authority boundary, with a P2
failed-write temporary leak. Parent fixed the leak and injected write/fsync
failure regressions; retries succeed without leftover temporary state files.
Measured-budget routing and changed managed source renewal are implemented;
60 tests passed before the storage component. Storage worker reports 69 tests
including 9 new transport tests, not independently re-executed by parent yet.
The aggregate shell gate still explicitly enumerates 60 tests and needs a
single update after consumer integration, including storage/consumer suites.

## Consumer seam decision

Initial supported adapter is intentionally narrow: plain Vite, default public/
root, exact npm lifecycle build, no custom Vite/Svelte config, no conflicting
prebuild. Existing catalog, Polis and Hellhunter are NOT qualified by this
adapter; their existing prebuild/plugins/static roots stay untouched.
Reserved portable command: `dracon-sync bucket-assets restore`.
Fixed repository-root manifest: `bucket-assets.manifest.json` (tracked path,
not an ignored .dracon/ file). Consumer component inspects/plans and returns
snapshot evidence; parent owns any actual bootstrap mutation, manifest/binding
validation, installed CLI qualification and create-only restoration.
A byte-exact script is necessary but never sufficient execution proof.

## Parent implementation seams remaining

1. Add a dependency-closed bundled runner CLI using component modules and
   pinned SDK; no sibling-checkout imports or mutable import closure. A build
   tool is not currently installed globally. Node runtime is v24.18.0. Install
   a pinned bundler dependency as build tooling; package runtime closure into
   one tested artifact. Cargo-install packaging must include/embed that bundle.
2. Add Rust trusted profile loader/runner bridge: operator-owned fixed profile
   path (or explicit global policy authority), validate file ownership/private
   modes, symlink-free paths, full runner hash before invocation. Repository
   config must never select uploader executable/credentials. Preserve explicit
   disabled or malformed existing bucket-assets config semantics. No ambient
   AWS provider chain. Production profile is currently absent, not enabled.
3. Expose the restore CLI above; consumer role only. Fresh consumers validate
   portable manifest within separately authorized read capability; restore to
   scratch and publish create-only. Matching materializations are no-op;
   differing operator bytes are never replaced. Unsupported/missing local-only
   assets fail clearly, not silently fetched as verified remote references.
4. Preserve operator auto-commit exclusions, not only Git ignores. A safe
   bridge option is a trusted exact allowed-candidate set derived in Rust from
   tracked+untracked Git paths after effective per-repo exclusion matching.
   Pass it over bounded stdin to the runner and ignore later new paths until
   the next scan. Do not translate glob semantics approximately in JavaScript
   or let a new path evade a precomputed exclusion list.
5. Use accepted complete discovery snapshots (daemon.rs ~7342) and a small
   separate bounded asset-work pool. Admission before both clean-repo gates
   (~8499, ~8644), same existing in_flight mutation owner (~7529), ownership
   retained through cancellation/process teardown/join. Never hold ordinary
   sync capacity while waiting for bucket requests or scan all repos inline.
6. Prepare/stage rendezvous in bucket_assets::prepare (currently explicit-only)
   and sync.rs ~5405; preserve exact staged manifest verification and raw/cache
   refusal just before commit. Consumer bootstrap, portable manifest and newly
   routed path suppression must be coherent, with renewed current evidence.
   Existing unsupported edits must not be suppressed merely because old
   manifest metadata says bucketed. Never restore absent source during scans.
7. Feed scan results into ordinary diff/classification; persist explicit
   unsupported vs retryable outcomes with bounded backoff and rescan on
   profile/policy/runner/source changes. Refresh full fleet inventory.
8. Only then run final full gates, verify executable identity from current
   source, sanctioned atomic install/retained rollback, and actual autonomous
   real-storage enrollment/readback/restore/retry with unrelated fleet work.

No missing external capability blocks writing/testing these seams. Any real
credential/profile enrollment requiring new authority must be resolved before
that dependent rollout action, not inferred from a stub run or component tests.
