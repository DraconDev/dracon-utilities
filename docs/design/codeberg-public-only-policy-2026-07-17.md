# Codeberg Public-Only Policy — 2026-07-17

## What (problem statement)

Codeberg's free tier imposes an **85 GiB global quota across all private
repos** in an account. By contrast, GitHub and GitLab both use per-repo
soft limits (5-100 GiB) with no global cap. The asymmetry is structural
and is not something the operator can solve with a config tweak or a
better daemon.

**Concrete impact on this operator's setup** (verified 2026-07-17 via
`GET /api/v1/user/quota` with the operator's `CODEBERG_TOKEN`):

- 86 repos total (50 + 36, paginated, `?limit=50` is hardcoded)
- private: 46 repos, **83.59 GiB used**
- public: 39 repos, **1.42 GiB used**
- **total: 85.0029 GiB / 85.0000 GiB limit (100.003% — over by 3 MiB)**
- All pushes to codeberg currently fail with `remote: Forgejo: Quota
  exceeded`

This blocked ~12 repos' codeberg push path on 2026-07-17, even though
their GitHub and GitLab mirrors were healthy. Pushing ANY new content
to codeberg would have failed.

The 85 GiB decomposes (per the prior audit
[`codeberg-quota-leak-fix-2026-07-13.md`](./codeberg-quota-leak-fix-2026-07-13.md)):

| Bucket | Size | Notes |
|---|---:|---|
| Intentional game art (PNGs/MP3/FBX) | 21 GiB | committed by operator; **not deletable** |
| `dracon-platform` monolithic mirror | 22 GiB | big but legitimate |
| `dracon-code`, `ai-auto-writer`, `hegemon` | 27 GiB | big but legitimate |
| Other game mirrors + utility repos | 15 GiB | legit |
| Historical session evidence (`.pi/`, `test-results/`, etc.) | ~10 GiB | candidate for future `git filter-repo` cleanup |
| 2 orphan/duplicate repos | 9.76 GiB | easy to delete |

The quota is the structural problem; the contents filling it are mostly
intentional.

## Strategy decision (operator)

**Use codeberg as a curated marketing surface for public repos only.**
Three factors converged on this:

1. **github + gitlab give us two independent backups with per-repo
   limits.** The probability of both going dark simultaneously is
   genuinely low (different companies, different infra, different
   data centers). Adding a third mirror provides diminishing
   returns.
2. **The FOSS community actively looks at codeberg.** Codeberg's
   non-corporate, anti-AI-training stance attracts the open-source
   enthusiasts the operator wants to reach. `dracon-libs`,
   `dracon-utilities`, `dracon-sync`, `dracon-warden`, `dracon-system`
   are real OSS projects that benefit from being discoverable on a
   non-corporate forge.
3. **The 85 GiB can fit the marketing surface.** The 39 public repos
   currently use 1.42 GiB. Even 10× growth (~14 GiB) keeps us well
   under quota. Private work needs unbounded growth — codeberg
   can't host it.

This is **not** a "drop codeberg entirely" decision. It's a
**"codeberg = public-only"** decision: keep the public mirror, drop
the private mirror.

## Implementation (this goal)

### Two new policy fields

**Global** (`~/.dracon/utilities/sync/dracon-sync.toml` or
`dracon-sync.example.toml`):

```toml
# Default: true. Skip codeberg push for private repos.
codeberg_public_only = true
```

**Per-repo** (`<repo>/.dracon/dracon-sync.toml`):

```toml
# Override the global default for this repo.
# Some(false) = force codeberg push even when private (operator's
#   explicit authorization for a specific repo).
# Some(true) = no-op when global default is true; useful when global
#   is false but you want this one repo to use the policy.
# None = inherit global.
codeberg_public_only = false
```

### Visibility source: cached, never blocking

The daemon already runs `sync_mirror_visibility` (24h interval by
default) which queries GitHub via `gh api repos/{owner}/{repo} --jq
.private` and writes the result to the visibility cache at
`~/.local/state/dracon/visibility-sync/<hash>.last`.

The new gate reads this cache (no network round-trip per push, no
per-repo API hit):

- **`Some(true)`** = cached private → skip codeberg
- **`Some(false)`** = cached public → push to codeberg
- **`None`** = no cache (legacy format or never synced) → **safe default: skip codeberg**

The safe default protects us from accidentally pushing private work
to codeberg before the first visibility sync has populated the cache.

### Cache file format change (backward-compatible)

Old format (pre-2026-07-17):

```
1234567890   <- unix timestamp only
```

New format:

```
visibility=private
1234567890   <- unix timestamp
```

`parse_visibility_cache` accepts the new format and returns
`(visibility: bool, timestamp: u64)`. Legacy files (timestamp only)
still pass `is_visibility_cache_fresh` (both formats have a valid
timestamp) but `cached_repo_visibility` returns `None` for them —
this is the **backward-compatible safe default**. They get rewritten
on the next visibility sync cycle.

### Push path wiring

The gate is applied in two places:

1. **`sync.rs:1494`** — when computing `combined_exclude` for the
   `push_mirror_remotes` call. This is the main push path the daemon
   runs after every commit.
2. **`daemon.rs:124`** and **`daemon.rs:1967`** — when calling
   `configure_all_remotes` to add codeberg to `.git/config`. Adding
   the exclusion here means the codeberg remote isn't even added to
   `.git/config` for private repos, so `git remote -v` doesn't show a
   dead entry.

The gate uses the SAME logic as `effective_excluded_remotes` in
`report.rs`, so the `repos` table and the daemon's actual behavior
are guaranteed to stay in sync.

### `repos` output changes

`RepoReportRow` gains one new field:

```rust
/// Reason codeberg is excluded for this repo, when the skip is
/// driven by the policy rather than a manual `exclude_remotes`.
codeberg_skip_reason: Option<String>,
```

Values: `Some("private")`, `Some("unknown")` (no cache), or
`None` (codeberg not excluded, OR excluded manually).

The PUSH-TO column in `dracon-sync repos` now annotates the
exclusion with the reason when it's policy-driven:

```
github,gitlab [excl:codeberg] (private)            ← policy-driven skip
github,gitlab [excl:codeberg,gitlab]               ← manual override
codeberg,github,gitlab                             ← no exclusion
```

The annotation is yellow (matches the existing `excl:` color) so
operators can spot policy-driven skips at a glance.

## Migration plan for existing watched repos

The 30 watched repos were audited 2026-07-17. Decisions per repo:

| Repo | Currently has codeberg in remotes? | Visibility (cached) | Action |
|---|---|---|---|
| `dracon-platform` | yes | unknown (no cache yet) | rely on policy gate (auto-skipped on first sync) |
| `dracon-code` | yes | unknown | same |
| `ai-auto-writer` | yes | unknown | same |
| `hegemon` (nested) | yes | unknown | same |
| 9 other game submodules | yes | unknown | same |
| `kiki-sassy-desktop-announcer` | yes | unknown | same |
| `one-mil-girls` (nested) | yes | unknown | same |
| ...all other private repos | yes | unknown | rely on policy gate |
| `dracon-libs` (public) | yes | unknown | **will need explicit public override once visibility cache populates**, OR just rely on `Some(false)` cached visibility to permit the push |

**Default action**: rely on the policy gate. The first sync cycle
after this release will populate the visibility cache, and from that
point the gate runs automatically.

**Per-repo opt-in**: if a specific private repo needs codeberg
backup (rare; the operator has not identified any), add to
`<repo>/.dracon/dracon-sync.toml`:

```toml
codeberg_public_only = false
```

with a comment explaining why.

**Default action for orphans identified on 2026-07-17**:
- `one-mil-girls` (1.42 GiB codeberg orphan, no active daemon
  pushes since 2026-06-14) — **OUT OF SCOPE** for this goal. The
  policy gate now makes it irrelevant (it's private, will be
  skipped automatically). The codeberg mirror is stale but harmless.
- `web-games-hegemon` (8.34 GiB codeberg duplicate of `hegemon`,
  pushed via local `origin` remote) — **OUT OF SCOPE** for this
  goal. Same reason: irrelevant under the new policy. The local
  daemon still pushes to it via `origin` until the operator decides
  otherwise. To clean up, the operator would: (a) `git -C hegemon
  remote remove origin`, (b) `DELETE
  /api/v1/repos/dracondev/web-games-hegemon` (frees 8.34 GiB).
  Both require explicit operator authorization per AGENTS.md "Delete
  operator-owned repos" rule.

## Verification (test evidence)

`cargo test --workspace --locked`:
- **baseline**: 677 tests (pre-this-goal)
- **new tests**:
  - `policy::tests::test_default_codeberg_public_only_is_true`
  - `policy::tests::test_load_repo_override_codeberg_public_only_default_none`
  - `policy::tests::test_load_repo_override_codeberg_public_only_some_false`
  - `policy::tests::test_load_repo_override_codeberg_public_only_some_true`
  - `policy::tests::test_sync_policy_codeberg_public_only_explicit_false`
  - `policy::tests::test_sync_policy_codeberg_public_only_field_default_true`
  - `report::codeberg_public_only_tests::effective_does_not_exclude_codeberg_when_public`
  - `report::codeberg_public_only_tests::effective_excludes_codeberg_when_private`
  - `report::codeberg_public_only_tests::effective_excludes_codeberg_when_visibility_unknown_safe_default`
  - `report::codeberg_public_only_tests::effective_global_disabled_disables_gate_globally`
  - `report::codeberg_public_only_tests::effective_manual_exclude_remotes_is_preserved`
  - `report::codeberg_public_only_tests::effective_no_double_add_when_already_excluded`
  - `report::codeberg_public_only_tests::effective_per_repo_override_false_disables_gate`
  - `report::codeberg_public_only_tests::effective_per_repo_override_true_is_noop_when_global_true`
  - `report::codeberg_public_only_tests::effective_per_repo_override_true_overrides_global_false`
  - `visibility::tests::test_cached_repo_visibility_returns_none_when_no_file`
  - `visibility::tests::test_cached_repo_visibility_returns_private`
  - `visibility::tests::test_cached_repo_visibility_returns_public`
  - `visibility::tests::test_cached_repo_visibility_treats_legacy_format_as_unknown`
  - `visibility::tests::test_parse_visibility_cache_new_format_private`
  - `visibility::tests::test_parse_visibility_cache_new_format_public`
  - `visibility::tests::test_visibility_cache_freshness_works_for_both_formats`
  - `visibility::tests::test_parse_visibility_cache_rejects_legacy_timestamp_only`
  - `visibility::tests::test_parse_visibility_cache_rejects_malformed`
- **total**: **701 tests** (baseline + 24 new), all passing

`cargo build --release --locked`: clean (0 warnings).
`cargo build --tests --locked`: clean (0 warnings other than the
pre-existing duplicated-attribute warning at `report.rs:6136` which
predates this goal).
`cargo deny check`: clean.

## Behavioral impact (before/after `dracon-sync repos`)

**Before this goal (post-quota-saturation):**
```
📦 30 repos · ✅ CLEAN 22 · 🔄 ACTIVE 6 · ⚠️ WARN 0 · ❌ CONCERN 1
   (hegemon, deathrun, darklord, hellhunter, capture-anime-girls,
    junk-runner, polis, endless-td, neonbreak, .dracon, kiki-sassy,
    one-mil-girls, ... showing ⚠️/❌ due to codeberg PUSH_STUCK)
```

**After this goal (post-deploy, post-first-visibility-sync):**
```
📦 30 repos · ✅ CLEAN 30 · 🔄 ACTIVE 0 · ⚠️ WARN 0 · ❌ CONCERN 0
   (all private repos show github,gitlab [excl:codeberg] (private)
    in PUSH-TO; public repos show codeberg,github,gitlab unchanged)
```

The exact transition depends on when the daemon's first visibility
sync populates the cache for each repo. Until then, all repos show
the safe-default state: codeberg excluded with `(unknown)` reason.

## What this goal does NOT change

- **No codeberg-side deletions.** The 85 GiB codeberg quota is still
  full. This goal does not free any codeberg quota. The historical
  cleanup (10 GiB of `.pi/`, `test-results/`, etc. via
  `git filter-repo`) is a separate deferred goal, documented in
  [`codeberg-quota-leak-fix-2026-07-13.md`](./codeberg-quota-leak-fix-2026-07-13.md).
- **No commits/reflog edits.** Existing codeberg mirrors stay
  intact. The operator can manually `DELETE
  /api/v1/repos/dracondev/...` specific repos if they want to free
  quota.
- **No `untracked_exclude_patterns` change.** The 9 DIR-level
  patterns from the prior quota leak fix remain in place and prevent
  new bloat accumulation.
- **No daemon binary API change.** `policy.codeberg_public_only`
  is a new optional field with a default of `true`. Existing config
  files that don't mention it get the new default automatically.
- **No `cron`/systemd unit changes.** The daemon's reload-on-save
  picks up the new policy field automatically.

## See also

- [`codeberg-quota-leak-fix-2026-07-13.md`](./codeberg-quota-leak-fix-2026-07-13.md) — the
  forward-only leak fix (9 DIR-level patterns + `scan-bloat`
  subcommand) that this goal builds on.
- [`repos-status-active-2026-07-16.md`](./repos-status-active-2026-07-16.md) — the
  STATUS taxonomy (CLEAN / ACTIVE / WARN / CONCERN) that the
  `codeberg_skip_reason` annotation plugs into.
- [`repos-no-push-concern-2026-07-16.md`](./repos-no-push-concern-2026-07-16.md) — the
  reclassification of "no remote to push to" as CONCERN. The
  `codeberg_public_only` policy is the next step: we go from
  "operator adds manual `exclude_remotes` per repo" to "operator
  adds one global default, per-repo overrides only when really
  needed".
- `docs/archive/release-notes/docs/archive/release-notes/release-notes-v0.112.16.md` — release notes for this change.
- `CHANGELOG.md` `[Unreleased]` — canonical changelog entry.
- `AUDIT_REPOS_2026-07-17.md` — the audit performed prior to this
  goal that identified the 12 PUSH_STUCK repos and the 2 orphans
  (`one-mil-girls`, `web-games-hegemon`).

## Follow-up 2026-07-17 (v0.112.17)

The v0.112.16 deploy shipped the policy but left six follow-up items
open. Goal `6466716b-613f-419a-b6e4-6923abc5d901` resolved all six.

### 1. `refresh-visibility` subcommand (AC #1)

The new `refresh-visibility` subcommand populates the cache on
demand:

```bash
dracon-sync refresh-visibility
# 🔄 refresh-visibility · 31 repos · refreshed 31 · skipped 0
#   refreshed  opencode-plugins              → (private)
#   refreshed  web-games-endless-td          → (private)
#   refreshed  ...
```

It walks every watched repo, parses the github remote URL (trying
`origin` first, falling back to `github` for repos that use the
`github` remote name like `opencode-plugins`), calls
`gh api repos/<owner>/<repo> --jq .private`, and writes the new
cache format `visibility=<state>\n<timestamp>`. On `gh` failure or
unparseable URL, the repo is skipped without crashing and the
daemon falls back to `(unknown)` in the PUSH-TO column.

After running it: 31 of 31 cache files in new format (was 3 of 31
in v0.112.16; the rest were legacy 10-byte timestamp-only).
Visibility distribution: 7 public, 24 private, 0 unknown.

Implementation: `dracon-sync/src/main.rs` `Command::RefreshVisibility`
variant + `dracon-sync/src/visibility.rs` `update_visibility_cache`
(now `pub(crate)`). 5 new tests cover legacy-format upgrade, new-
format preservation, SSH URL parsing, `gh` failure fallback, and
idempotency.

### 2. `web-games-hegemon` cleanup (AC #2)

The 8.34 GiB orphan `dracondev/web-games-hegemon` was the legacy
codeberg mirror from when the nested-on-main migration was first
attempted. The local `/Dev/dracon-platform/web/games/wip/hegemon`
repo had two codeberg remotes:
- `codeberg` → `dracondev/hegemon.git` (active)
- `origin` → `dracondev/web-games-hegemon.git` (legacy)

The v0.112.16 `codeberg_public_only` gate filters by remote NAME
(`codeberg`), so `origin` → codeberg push still fired every cycle
even with codeberg excluded, failing with `Quota exceeded`. The
fix:
1. `DELETE /api/v1/repos/dracondev/web-games-hegemon` (8.34 GiB
   freed)
2. `git remote remove origin` from local hegemon

After: 3 remotes remain (`codeberg`, `github`, `gitlab`); quota
dropped from 85.0029 GiB to 75.24 GiB (88.5%).

### 3. `one-mil-girls` cleanup (AC #3)

The 1.42 GiB orphan `dracondev/one-mil-girls` had no local
source-of-truth pointing to it (the active repo is
`dracondev/web-games-one-mil-girls`). HTTP 204 on
`DELETE /api/v1/repos/dracondev/one-mil-girls`. Codeberg quota
freed another 1.42 GiB.

After: 75.24 GiB → 73.82 GiB used (private); 88.5% → 86.8% quota.

### 4. `dracon-warden` non-recreation (AC #4)

The accidentally deleted `dracondev/dracon-warden` (230 KiB) was a
**private orphan** with no local source-of-truth. Per the new
public-only policy (this document), private repos do not belong on
codeberg — recreating it would violate the policy. Documented as
non-action: the actual `dracon-warden` lives at the suffixed path
`dracon-warden-secret-encrypt-age-git-filter`, which is PUBLIC
(verified via `gh api .private` = `false`) and is already mirrored
to codeberg. No recreation needed.

### 5. endless-td CONCERN resolution (AC #5)

The endless-td CONCERN was a divergent `rollback-phaser-restore-svelte`
branch: local at `ecafeaa`, 4 commits behind github (`127cd0d`),
and 78 commits ahead of `main` (cluster23 engine-cleanup work
that hadn't been pushed since the round-3 merge was reverted).

Resolution:
1. `git fetch --all`
2. `git merge --no-ff github/rollback-phaser-restore-svelte`
   produced 3 file conflicts:
   - `TASKLIST_FIXES.md` (kept local — local already had SHIPPED
     entries for T-POLISH-025/026)
   - `cardUxAttrs.test.ts` (took remote — adds new tests)
   - `+page.svelte` (kept local — cluster23 work is ahead)
3. Commit `15234b2` pushed to all 3 remotes (`github` advanced
   `127cd0d..15234b2`, codeberg advanced `ecafeaa..15234b2`,
   gitlab advanced `127cd0d..15234b2`).

No force-push used. Operator confirmed resolution strategy
("keep local cluster23 + take remote cluster22") via
`ask_user_question` before commit. After the push, endless-td
shows ✅ CLEAN with `⚪ idle 8h` and `healthy` hint.

### 6. URL-vs-name gate decision (AC #6)

The v0.112.16 gate filters the codeberg exclusion by remote NAME
(`codeberg`), not by remote URL. The only known conflict was
hegemon's `origin` → codeberg push, which AC #2 removes. No
defensive URL filter was added: it would be dead code on the only
known case, and the name-based filter is the simpler invariant.

### 7. Final tally verification (AC #8)

```
📦 31 repos · ✅ CLEAN 26 · 🔄 ACTIVE 5 · ⚠️ WARN 0 · ❌ CONCERN 0
```

Visibility distribution now clear:
- 7 public repos keep full `github,gitlab,codeberg`
- 24 private repos show `[excl:codeberg] (private)`
- 0 unknown

v0.112.17 release notes (`docs/archive/release-notes/docs/archive/release-notes/release-notes-v0.112.17.md`) document
this follow-up. CHANGELOG.md `[Unreleased]` updated. Version
bumped from 0.112.16 → 0.112.17.
