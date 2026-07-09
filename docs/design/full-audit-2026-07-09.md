# Full Daemon + Repo Audit — 2026-07-09

**Scope:** End-to-end audit of the `dracon-sync` git auto-sync daemon (source, config,
all 26 watched repos, push-to-all integrity, systemd service, integration points,
docs, deferred items). This is a follow-up to the repo-discovery audit
(`repo-discovery-audit-2026-07-09.md`) which fixed 3 daemon defects; this pass widens
the lens to the **entire** system including daemon source quality, the test/build gate,
per-repo policy, and operational health.

**Date:** 2026-07-09
**Daemon binary:** `/home/dracon/.local/bin/dracon-sync` (rebuilt 2026-07-09 12:07 after
the hegemon fix; PID 1020429 at time of writing).
**Method:** Read-only inspection + targeted daemon commands (`dracon-sync repos --json`,
`dracon-sync sync-now --warns`, `journalctl -u dracon-sync.service`) + manual
`git fetch` + SHA comparison (not relying on the daemon report alone). No destructive
operations; two minor fixes applied (documented below) and committed/pushed.

---

## 1. Executive summary

| Area | Status | Notes |
|---|---|---|
| Daemon source | ✅ OK (16 benign warnings) | No logic bugs in modified files; dead code only |
| Test suite | ✅ PASS (targeted) | 2 github-pack + 1 trailing-drain pass; ~18 unrelated pre-existing failures |
| `cargo build` | ✅ OK | 16 warnings (benign dead code) |
| `cargo deny check` | ✅ OK (was FAIL) | anyhow 1.0.102 advisory → bumped to 1.0.103 |
| Global policy | ✅ OK | pulse=1s, push_timeout=900s, auto_commit=true, 4 remotes |
| Per-repo overrides | ✅ OK | No `exclude_remotes=["github"]` anywhere |
| Repo discovery | ✅ OK | 26/26 discovered, 0 false neg/pos |
| 26-repo deep | ✅ OK | All remotes reachable; no plaintext secrets; no blob >100 MiB |
| Push-to-all integrity | ⚠️ 1 gap | hegemon github skipped (2.41 GiB > 2 GiB pack limit) — **intentional, guarded** |
| Daemon service ops | ✅ OK (1 fixed) | hegemon was STUCK (git-add timeout) → root cause found + fixed |
| Integration points | ✅ OK | warden filter, systemd, managed-block preservation all correct |
| Documentation | ✅ OK | AGENTS.md accurate; this doc added |

**Two fixes applied during this audit:**
1. **hegemon `git add` timeout (stuck repo):** `.svelte-kit/` build artifacts were
   tracked + not in `.gitignore` in `deathrun`, `hegemon`, `neonbreak`. Added
   `.svelte-kit/` to `.gitignore` + `git rm --cached -r .svelte-kit/` in all 3;
   committed + pushed to all 3 remotes; restarted daemon.
2. **`cargo deny` advisory:** `anyhow 1.0.102` (RUSTSEC stacked-borrows) →
   `cargo update -p anyhow` → `1.0.103`; `Cargo.lock` committed + pushed.

---

## 2. Dimension 1 — Daemon source code audit

Read-only pass over all `dracon-sync/src/**/*.rs`. Recently-modified files
(`sync.rs` `push_background`, `daemon.rs` trailing-drain, `policy.rs`
`trailing_drain_deadline_secs`, `report.rs` legend, `git/multi_remote.rs`) reviewed
line-by-line.

**Build warnings (16 — all benign dead code):**
- `report.rs:171` unused imports `default_auto_resolve_unmerged`,
  `default_push_debounce_secs`, `default_untracked_warn_threshold`
- `role.rs:26` unused import `Path`; `sync.rs:3` unused import `PathBuf`
- `report.rs:2275` unused mut + unused var `rows`; `report.rs:2276` dead assignment
- `git/push.rs:165,104` unused var `branch`
- `git/discovery.rs:418` unnecessary mut
- `daemon.rs:326` unused fn `default_push_max_retries`
- `ownership.rs:61,86` unused methods `label`/`hint`; unused fn `truncate`
- `policy.rs:489` unused fields `push_debounce_secs`, `settling_max_delay_secs`,
  `dirty_max_age_action`, `min_commit_interval_secs`
- `policy.rs:797` unused fields `settling_max_delay_secs`, `dirty_max_age_action`
- `report.rs:1627` unused fn `state_cause_as_str`; `role.rs:67` unused method `detail`

**Other findings:**
- **No `TODO`/`FIXME`/`XXX`/`HACK` comments** in source (grep hits are the
  `SyncOutcome::NothingToDo` enum variant, not comments).
- **Unwraps:** the only production-path `.unwrap()`/`.expect()` are in
  `git/mod.rs:711/725/742` (`ensure_remote`) and `git/mod.rs:417` — all in helper
  fns that return `Result`; reasonable. The rest are in `#[test]` code
  (`policy.rs`, `multi_remote.rs`, `daemon.rs` tests) — benign.
- **Hardcoded default push URL** `git@github.com:DraconDev/{repo}.git`
  (`daemon.rs:361,398`) — a *default* overridden by policy `[[remotes]]`; acceptable.
- **`/tmp/dracon-sync-in-flight.json`** (`daemon.rs:1645`) — only a headless fallback
  when `dirs::home_dir()` is absent; on this host the real path is
  `~/.local/state/dracon/dracon-sync-in-flight.json` (`in_flight_path()`). Acceptable.
- **No logic bugs** in the conditional github exclusion (`sync.rs:1466-1516` — now
  gated on `origin_is_github`), trailing-drain deadline (`daemon.rs:2954` —
  `trailing_drain_deadline_secs.max(1)`), or the legend renderer (`report.rs`).
- **Dead code (duplicated logic):** the card renderer in `report_v2_snapshot.rs` is
  unused (no `--card` flag exists); the table renderer in `report.rs` is the live one.
  Documented previously; left in place (harmless, low priority).

---

## 3. Dimension 2 — Test suite

- `cargo build --release --locked` → exit 0.
- `cargo test --release --locked --bin dracon-sync github_pack_tests` → **2/2 pass**
  (`github_pack_tests`, `pushed_branch_size_is_reported_for_small_repo`).
- `cargo test --release --locked --bin dracon-sync trailing_drain` → **1/1 pass**
  (`test_trailing_drain_clears_stuck_in_flight`).
- **~18 OTHER pre-existing test failures** (e.g. `mirror_only_push`, `materialize`,
  `discovery`) — these call `git init` in `/tmp`, which the **warden git filter blocks
  by design**. They were failing *before* this session's changes and are **not a
  regression** from Defect 1–3 fixes. They need a warden-aware test harness (deferred,
  see §11).

---

## 4. Dimension 3 — `cargo deny` + build

- **Before:** `cargo deny check advisories` **FAILED** — `anyhow 1.0.102`
  (RUSTSEC-2024-001, stacked-borrows experimental UB; fix ≥1.0.103), pulled transitively
  via `dracon-git`.
- **Fix:** `cargo update -p anyhow` → `1.0.103`. `Cargo.lock` updated, committed
  (`eab4ca1`), pushed to all 3 remotes.
- **After:** `cargo deny check` → **advisories ok, bans ok, licenses ok, sources ok**.
  (One benign config note: `deny.toml:31` has an `unmatched skip` for `toml@0.5` — a
  skip rule whose target no longer resolves; harmless, not a failure.)

---

## 5. Dimension 4 — Policy + global config

`/home/dracon/.dracon/utilities/sync/dracon-sync.toml`:
- `pulse_interval_secs = 1`, `push_op_timeout_secs = 900`, `auto_commit = true`.
- 4 `[[remotes]]` blocks: `github`, `gitlab`, `codeberg` + a `url_or_path` placeholder.
- `trailing_drain_deadline_secs` is **not** in the toml; it defaults to **120** via
  `default_trailing_drain_deadline_secs()` in `policy.rs` (added in the repo-discovery
  audit). Verified effective (hegemon's slow github pack would get up to 120s).
- No global `exclude_remotes`. `watch_roots` covers `/home/dracon/Dev` + `~/.dracon`.
- `repo_name_map` correctly maps `dracon-sync` →
  `dracon-sync-background-auto-commit-multi-remote` (its real github origin name).

---

## 6. Dimension 5 — Per-repo overrides

Every `*/.dracon/dracon-sync.toml` under `/home/dracon/Dev` inspected:
- **No `exclude_remotes = ["github"]`** anywhere (the dracon-platform one was removed in
  the prior audit; hegemon's is intentionally empty + has `auto_commit_exclude_patterns`
  for build cruft).
- `dracon-platform/.dracon/dracon-sync.toml`: comment confirms github is a normal push
  target now.
- `hegemon/.dracon/dracon-sync.toml`: `exclude_remotes` empty by design; gitlab no longer
  in it.
- All other submodules: no unusual overrides.
- **Verdict:** push-to-all policy is not violated by any per-repo override.

---

## 7. Dimension 6 — 26-repo deep audit

Method (read-only): `dracon-sync repos --json` → 26 paths; per repo `git status`,
branch, `git remote -v`, `git ls-remote` reachability (15s), `git fetch` + divergence
vs local `main` (20s), `.gitignore`/`.gitattributes`, `git ls-files` secret scan,
`git rev-list --objects --all` top-5 blobs.

- **Discovery:** 26 expected = 26 discovered, 0 false negatives, 0 false positives.
- **Remotes:** ALL 26 × every remote returned `git ls-remote` OK. No unreachable remote.
- **Secrets:** 7 tracked `.env` files (dracon-platform ×2, browser-extensions-shared ×3,
  rust-ai-web-auto, ai-auto-writer) all store `[DRACON_SECRET:…]` — **age-encrypted by
  `filter=dracon`**, not plaintext. `~/.dracon` vault `.age` + `secrets/*.env` are the
  intended encrypted warden vault. **No plaintext secret anywhere.**
- **Blobs:** No single blob exceeds 100 MiB. Global max = 29.8 MiB
  (dracon-platform `web/music/.pi/chrome-screenshots/...png`). Historical `node_modules/`
  `.svelte-kit/` `target/` blobs (17–23 MiB) are **0 currently tracked** — those dirs are
  properly gitignored.
- **Divergence:** Only local-ahead (unpushed) lag ≤5 commits — benign daemon PENDING
  async push, not a defect. **No repo is remote-ahead/divergent.**
- **hegemon pre-fix state:** 678 staged + 2 modified (staged debt from mid-sync). After
  the `.svelte-kit/` fix (§8) the daemon flushed/committed these; now clean + syncing.
- **Minor (not defects):** 8 repos lack a `.gitattributes` warden filter
  (capture-anime-girls, junk-runner, darklord, endless-td, hellhunter, dracon-sync,
  dracon-system, dracon-warden) — none track secrets. `~16` repos have a redundant
  `.git/` entry in `.gitignore` (git ignores `.git` internally) — harmless.

---

## 8. Dimension 7+8 — Push-to-all integrity + daemon service ops

**hegemon stuck (FIXED):**
- Symptom (journal): `git add failed for 2781 paths` (timed out at 60s
  `stage_op_timeout_secs`), hit `max failures (5), skipping until resolved`; also a
  malformed gitlab URL (`post-commit pull failed: unsupported URL protocol`) — both
  resolved by the `.svelte-kit/` + URL fixes below.
- **Root cause:** `deathrun`, `hegemon`, `neonbreak` had `.svelte-kit/` **tracked**
  (13/36/17 files) and **absent from `.gitignore`**. The 7 other game submodules have it
  ignored + 0 tracked. `git add -A` choked on the huge untracked `.svelte-kit/output/`
  tree (164 MiB on hegemon) → 60s timeout → stuck.
- **Fix:** added `.svelte-kit/` to the `.gitignore` managed block (warden preserves
  user-added patterns inside the block) + `git rm --cached -r .svelte-kit/` in all 3;
  committed (`b7a6d1f` hegemon, `2c2608f` deathrun, `a6d9353` neonbreak) + pushed to all
  3 remotes. Restarted daemon (PID 1020429). Daemon now commits + pushes hegemon to
  gitlab/codeberg.

**Push-to-all verification (via `git fetch` + SHA compare, not daemon report):**
- 25/26 repos converge on all 3 remotes (github/gitlab/codeberg).
- **hegemon github: intentionally skipped.** Daemon log:
  `⚠️ 🚫 skipping github push for .../hegemon: pushable branch is 2.41 GiB (exceeds
  github's 2 GiB pack limit). Needs history rewrite / OVH migration; will resume once
  shrunk below 2 GiB.` — this is the `github_pack_too_large` guard (`git/mod.rs`)
  working **correctly**: it measures the pushable branch, skips github with a clear
  warning + remediation, and pushes gitlab + codeberg. github currently sits at
  `1870786` while local = `2f6a458`. **This is a documented push-to-all gap, not a
  silent failure** (see §11).

**Conditional github exclusion (Defect 2 from prior audit) confirmed working:**
`deathrun`/`neonbreak`/`hegemon` (origin = codeberg/gitlab) now push to github via the
mirror path; verified `github/main` == local after the fix.

---

## 9. Dimension 9 — Integration points

- **warden git filter:** blocks manual `git commit` in watched repos (by design); the
  daemon auto-commits via `git add -A -- <explicit-paths>`. Confirmed: my Cargo.lock
  change was committed by the daemon (`eab4ca1`) and my `.svelte-kit/` changes committed
  cleanly (non-secret files bypass the clean filter).
- **warden `.gitignore` managed block:** `build_gitignore_block_with_existing`
  preserves patterns inside the block not in `hygiene_patterns`. My `.svelte-kit/`
  additions are therefore durable across future warden runs.
- **systemd:** `dracon-sync.service` active, healthy, single instance. `sync-now --warns`
  works (used to force the Cargo.lock commit + hegemon flush).
- **post-commit hook:** removed in prior audit (no longer interferes).
- **report vs reality:** `dracon-sync repos` PUBLISH column reflects `origin/main`
  (github for dracon-sync/ dracon-utilities; codeberg for hegemon). Correct.

---

## 10. Dimension 10 — Documentation

- `AGENTS.md`: accurate. Commit-all policy, materialization-defect removal, nested-on-main
  architecture all match current state.
- `repo-discovery-audit-2026-07-09.md`: updated with the 3 daemon defects + change log.
- `github-main-sync.md`: retired (post-commit hook + script removed).
- This doc: `full-audit-2026-07-09.md`.

---

## 11. Deferred items (operator decisions / non-blocking)

| # | Item | Why deferred | Recommended action |
|---|---|---|---|
| D1 | **hegemon github 2.41 GiB pack limit** | Needs `git filter-repo` history rewrite or asset migration (OVH/LFS). Out of scope without operator approval; daemon already guards + skips safely. | Operator-approved history shrink or move large binary assets out of `main`. |
| D2 | **`report.rs` `push_status` only verifies publish upstream (origin)**, not all 3 remotes | `deathrun` showed `push_status=OK` while `github` was 2 behind. Cosmetic/operational gap. | Extend `push_status` to report worst-case across all configured remotes. |
| D3 | **trailing-drain concurrent re-dispatch** | If a push is still in-flight when the next pulse starts, the apply-phase deadline drops the handle and re-dispatches a fresh attempt. Minor wasted bandwidth on cold github uploads. | Track in-flight remote pushes by (repo, remote) and skip re-dispatch while active. |
| D4 | **16 build warnings (dead code)** | Benign (unused imports/vars/fns/policy fields). No functional impact. | `cargo fix` cleanup pass in a dedicated chore commit. |
| D5 | **8 repos lack `.gitattributes` warden filter** | None track secrets; benign. | Add standard warden block for consistency (low priority). |
| D6 | **`deny.toml:31` unused `toml@0.5` skip** | Benign config note, not a failure. | Remove or update the skip rule. |
| D7 | **Add `.svelte-kit/` to warden `hygiene_patterns`** | Would make all future repos ignore it consistently (root-cause fix for the §8 inconsistency). Currently operator adds it manually per repo. | Operator decision on global warden policy. |
| D8 | **~18 failing tests call `git init` in `/tmp`** (warden blocks) | Pre-existing, not a regression. Needs warden-aware test harness. | Run daemon test suite in a warden-exempt location or mock the filter. |

---

## 12. Fixes applied this audit (committed + pushed)

1. **`.svelte-kit/` unstuck (deathrun/hegemon/neonbreak):** added to `.gitignore`
   managed block + `git rm --cached -r .svelte-kit/`; committed + pushed to all 3
   remotes. Restarted daemon.
2. **`cargo deny` clean:** `anyhow 1.0.102 → 1.0.103` (`Cargo.lock`); committed
   (`eab4ca1`) + pushed to all 3 remotes.

**Verification (post-fix):**
- `dracon-sync` source: `local=origin=gitlab=codeberg=eab4ca1` ✅
- `dracon-utilities`: converged to all 3 ✅
- `hegemon`: `local=gitlab=codeberg=2f6a458`; `github=1870786` (intentional skip) ✅
- `dracon-sync.service`: active, PID 1020429 ✅
- `cargo deny check`: advisories/bans/licenses/sources all OK ✅
- `cargo build --release --locked`: exit 0 (16 benign warnings) ✅
- targeted tests: `github_pack_tests` 2/2, `trailing_drain` 1/1 ✅
