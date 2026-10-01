# Git and object storage: implementation plan

Status: proposed implementation plan. Writing this document does not enable
uploads, change file placement, alter retention, or authorize history rewrites.
Date: 2026-10-01. Operator: DraconDev.

## Objective

Keep platform and game development simple while preserving valuable work
without repeatedly filling Git history with media, render intermediates, and
large generated datasets. Keep the current project boundaries: games remain
submodules; the utilities remain nested standalone repositories.

Success means a repository version can be recovered with its exact assets,
Warden protection remains effective, and a failed bucket operation cannot
silently put external payloads back into Git or stall unrelated repositories.

## Baseline and evidence

- dracon-sync 0.113.92 fixes classification cooldowns and resolves TOUCHED
  aliases. It does not implement a fleet-wide automatic storage policy.
- GitHub's 2 GiB limit applies to one push, not total repository size.
  Local disk usage, reachable Git history, projected push bytes, and provider
  account quotas must be reported separately.
- The platform's current 5.5 GiB bucket-guard budget is an operator growth
  budget. It is not a provider guarantee; this plan does not raise it.
- The September catalog audit found 15.9 GB of historical blob content from
  repeated catalog revisions. The recent Strategy stall showed a separate
  64 MiB Warden filter bound. File-size policy alone misses both churn and
  whole-file processing costs.
- The shared bucket planner and reviewed publisher exist; the reviewed
  publisher is currently restricted to music. The shared resolver supports
  local caches and immutable asset references.
- Doomtap has a separate automatic clean/smudge-filter pilot. Its clean
  filter uploads during Git operations, changes the manifest separately,
  overrides Warden for selected extensions, and falls back to raw Git blobs
  when credentials/uploads fail. Do not extend these behaviors fleet-wide.
  Audit and consolidate the pilot before changing its live configuration.

References:
- `docs/design/parent-bloat-quarantine-analysis-2026-09-26.md`
- `docs/design/sanctioned-slimming-2026-09-26.md`
- `dracon-platform/web/docs/bucket-strategy.md`
- `dracon-platform/web/scripts/bucket-promote.mjs`
- `dracon-platform/web/games/wip/doomtap/scripts/assets/filter-clean.py`
- GitHub: https://docs.github.com/en/get-started/using-git/troubleshooting-the-2-gb-push-limit

The July storage/LFS documents contain historical assumptions, pricing, and
conflicting recommendations. In particular, deleting/untracking a file does
not shrink existing reachable history, and generation prompts alone do not
guarantee reproduction of the exact original media. This plan supersedes
those assumptions for new implementation; sanctioned maintenance remains a
separate procedure.

## Proposed policy

Placement is deterministic per path and purpose. A repository crossing a
size threshold does not silently change placement or migrate history.

| Content | Default placement |
| --- | --- |
| Source, tests, authored docs, configs, generator inputs | Git, protected by Warden where required |
| Small final media changed infrequently | Git |
| Declared final media over 20 MiB | Object storage with a versioned reference |
| Declared render intermediates or frequently regenerated media | Object storage regardless of the 20 MiB threshold |
| Large generated catalogs | First assess canonical inputs and sharding; do not route arbitrary JSON by extension |
| Logs, sessions, database snapshots | Separate archive policy; preserve current behavior until that policy is implemented |

20 MiB is a proposed initial media threshold, not a provider limit. Repos can
set documented path-specific exceptions. Keep existing Git media as legacy
content by default. Initial opt-in applies to declared new paths; converting
an existing tracked path requires a reviewed migration that handles hooks,
consumers, and recovery. Existing versions remain available.

Private repository membership does not make an asset public, and a media
extension does not make its contents non-sensitive. Publication is explicit.
Unknown classification stays visible for review; never silently upload it.

Initially retain every recorded external version. No automatic expiry,
bucket deletion, cache eviction, raw-file deletion, or Git history rewrite
is included in this rollout. Later retention policies require an explicit
operator decision. Keeping versions forever moves growth to object storage;
it does not eliminate storage costs.

## Responsibilities and durability contract

Warden owns classification, encryption, and sensitive metadata protection.
Sync owns scheduling, preservation status, and committing the exact version.
A shared object-storage adapter owns bounded upload/download and verification.
Git owns source history and immutable references. Object storage owns payloads.
The adapter is invoked by Sync; do not add an independently racing daemon.

References must record schema version, stable repository ID, logical path,
backend/object identity, payload byte count and digest, encryption format,
and the restore contract. Public digests may address public media. Private
references must avoid exposing plaintext digests or sensitive paths; protect
metadata with Warden where needed. Do not use a repo's mutable HEAD as its
storage namespace. Namespace prefixes are not access-control boundaries.

A completed preservation operation means:
1. Capture a stable source snapshot; detect edits during capture/upload.
2. Warden prepares its approved representation, encrypting if required.
3. Upload immutable bytes and verify length, digest, and intended access.
4. Record recoverable references and manifests in the same Git index snapshot.
5. Commit/push through existing hooks, then report the actual durability state.

An upload failure retains local data and a retry record. Never fall back to
raw Git storage for a path declared external. Never commit a reference to an
unverified object. A source edit during upload schedules a fresh snapshot;
manifest, pointer, and payload must agree on the committed version.

Uploads/downloads must not run inside Git clean/status/diff operations.
Filters may serialize an already-prepared local reference or refuse an
unprepared managed path. Resolve how the existing required Warden filter
composes with references before rollout: Git selects one effective filter,
so extension overrides cannot silently bypass Warden. Manual `git add` and
commits must obey the same rules as automatic staging.

Track pending-upload, verified-pending-commit, committed, and restore-failed
states durably. Retry across restarts with bounded concurrency/timeouts and
streaming I/O. Hold the Git index lock only for local staging/commit work.
A pending media file must not block unrelated source files or repositories;
Git commits and asset preservation each get truthful status.

## Phases and completion gates

### 1. Inventory and policy simulation

Produce a read-only inventory for platform, music, one game, and video output:
current files, tracked/ignored state, history growth by path/class, existing
manifests, filters, publishers, and consumers. Measure old raw blobs separately
from actual stored history. Review existing producer services for stale writer
code and large-file rewrite patterns.

Add a policy simulator that explains placement and estimates future Git versus
external growth without uploads or index changes. Identify the initial 20 MiB
exceptions and intermediate-output paths. Document proposed settings and their
per-repository overrides; apply the policy coverage tripwire to new Sync knobs.

Gate: every pilot path has an explicit reason, privacy class, retention rule,
and restore requirement. No existing file silently changes storage.

### 2. Storage and security prototype

Implement the shared reference schema and Warden integration against a local
fake object backend. Reuse reviewed publisher/resolver components where their
contracts fit. Keep public publishing distinct from private preservation.
Specify and test filter ordering and behavior for both manual and daemon Git.
Decide whether existing manifests are sufficient or a pointer format is needed;
avoid building a second incompatible format without this comparison.

Gate: same bytes/digest round-trip; encrypted restoration with authorized keys;
no plaintext or metadata leak; Warden is never bypassed; no network in Git
classification; retry never changes an existing immutable object.

### 3. Sync integration and recovery

Add the durable per-file state machine, selective staging, upload verification,
asset-aware status, and an explicit hydration/verification command. Ordinary
Git paths must retain current behavior. Cache hits permit offline development;
a cache miss reports missing assets clearly, without treating pointer text as
valid image/video data.

Gate: tests cover upload failure, lost credentials, source edits during upload,
process death at each phase, concurrent/manual commits, duplicate uploads,
corrupt/missing objects, files over 100 MiB, bounded memory, offline cache
hits/misses, and encrypted restore. A bucket outage cannot block unrelated
source commits. A cold checkout restores both the latest and an older version.

### 4. Controlled live pilot

Opt in a small declared set of music assets first. Check live bucket access
and billing terms without exposing credentials. Require one verified
independent recovery copy in addition to the primary object store before
calling irreplaceable asset preservation complete. Existing Git mirrors alone
do not back up external payloads. Report a missing recovery copy as degraded.

Run upload, consumer resolution, and cold restore checks; observe retries and
Git growth. Next pilot one game's new assets and temporary/final video outputs.
Consolidate Doomtap's existing filter path only after preserving compatibility
and verifying every existing pointer can still be restored.

Gate: preserved current/older versions, healthy source syncing during failure,
no unexpected public objects, bounded Git growth for external paths, and
explicitly accepted storage/recovery cost. No automatic old-data deletion.

### 5. Rollout and separate legacy cleanup

Roll out declared paths per repo. Record exceptions and teach generators the
same policy. Revisit catalog structure separately. Keep current repo/submodule
boundaries and daemon coverage. Update AGENTS, examples, operator docs, release
notes, packaged-install fixtures, and all required workspace checks.

For old history, prepare a separate measured slimming proposal with exact
paths, verified backups, preserved shipped content, and rollback evidence.
Execute only under explicit operator authorization and the sanctioned procedure.

Gate: all opted-in repositories pass restore drills and report both Git and
asset preservation health. Release the feature only after these gates pass.

## Monitoring and rollback

Report local Git bytes, reachable stored history, projected push bytes,
configured growth budgets, external retained bytes, pending bytes, recovery-copy
health, and backend/account quota where available. Begin with warnings at 70%
and 85% of an explicit repository growth budget; these are planning signals,
not automatic migration triggers. Evaluate the real push cap separately.

Rollback stops new external enrollment and preserves the adapter needed to
read existing references. Retain all payloads, manifests, caches, and retry
records. Never switch declared external paths back to raw Git as an outage
fallback. Immutable writes left orphaned by a crash are recorded for later
review rather than automatically deleted.

## Decisions before implementation

- Confirm the simulated media policy and exceptions; the proposed 20 MiB
  default is adjustable before any paths are enrolled.
- Confirm the independent recovery destination and acceptable recurring costs.
- Approve the first live pilot's exact paths and public/private classification.
- Leave retention at preserve-all initially; choose expiry separately if wanted.

No production storage changes or live bucket writes were performed while
preparing this plan.
