# Mechanical commit index: general improvements (2026-09-03)

Status: **proposed** (no code changed).
Supersedes `commit-message-harness-trails-2026-09-03.md` (same day):
per operator direction, harness-specific parsing (GLLA or otherwise) is
OUT — this design covers only general, staged-diff-derived signals.

## 1. Standing philosophy (unchanged)

`docs/ARCHITECTURE.md` already states the contract this design extends:

- "Deterministic commit messages (no AI) — extractable facts from diffs
  for `git log --grep=` queries"
- Queryability row: `git log --grep="JWT"` finds commits touching that task.

And `compute_blast_radius` (`dracon-sync/src/sync.rs:~2751`) documents:
"AI-generated commit messages are bad for AI workflows … The commit
message is an INDEX, not a description."

Rules that follow (all verified below, none new):

1. **Index, not prose.** Stable token order, fixed vocabulary, bounded
   lists. The next AI reads the diff; the message only has to point.
2. **Staged diff is the only input** (`git diff --cached`: numstat,
   name-status, unified-0) plus staged paths. No network, no LLM, no
   reads outside the repo, no intent inference. Safe inside the
   3-second debounce loop.
3. **Graceful absence.** Any token may be missing; the commit proceeds
   identically to today.

## 2. Verified facts (all file-mediated, 2026-09-03)

Method note: large piped tool outputs can be line-folded by the capture
layer (observed phantom "exactly 120 chars" clusters). Every number
below was re-verified by writing git output to a file first and
analyzing the file. Future forensics on this codebase must do the same.

### 2a. Top-dir scope is useless in deep repos, fine in shallow ones

`N file(s) in <top-3-first-path-components>` per 100–200 recent
subjects (regex `(\d+) file(s) in (dirs)( \[| DELTA)`):

| repo | shape | dominant scope | verdict |
|---|---|---|---|
| dracon-platform | deep monorepo | `web` 174/200 | useless |
| browser-extensions-shared | deep-ish | `extensions` 79/100 | useless |
| pi-plugins | mixed | `extensions` 24/88, real subdirs beside | partial |
| polis (nested game) | shallow | `src` 49/100 | weak |
| dracon-strategy | component dirs | `ai-auto-music` 76/100 | useful |
| dracon-utilities | crates | `dracon-sync`, `dracon-warden`, … | useful |

The mechanism is structural, not harness-related: one generic top
component (`web/`, `extensions/`) swallows every scope. Shallow repos
are unaffected either way.

### 2b. Subjects are uncapped and reach 1.4 KB

dracon-platform, n=2000: min=26, p50=102, p90=220, p99=515, max=1421;
283 >200 chars, 21 >500. Other repos: medians 77–157, maxes 268–547.
No truncation exists in the write path (single `svc.commit` call site,
`sync.rs:3500`; no cap in source, dracon-git, or global hooks —
an earlier "120-char cap" reading was the capture-fold artifact above).
Implication: length is currently unbounded; anything past ~one
terminal line degrades `git log --oneline`, the codeberg UI, and the
daemon's own `repos` report (which logs the first line).

### 2c. Common states have no token

dracon-platform, last 200 commits (name-only, one git call):
doc-only 15, lockfile-only 0, all-test-path 4. Top extensions:
svelte 96, ts 93, md 30, json 18 (`<noext>` 161 is dominated by
submodule-gitlink pointer paths + extensionless files — must be
excluded from any histogram via the existing `is_gitlink`).
`TESTONLY:` exists; `DOCSONLY:` does not. Lockfile-only and
extension-mix signals do not exist.

### 2d. Renames are invisible

`--name-status` is invoked WITHOUT `-M` at all four call sites
(`sync.rs:2578,3433,3775,3822`), and `extract_new_deleted_files`
ignores `R` statuses. A move-heavy refactor currently reads as
mass NEW+DEL. (Staging call sites must keep their exact parsing —
see §3C.)

### 2e. Harness boundary: sessions live outside the repo

| harness | session/state store | in-repo trace usable in diff |
|---|---|---|
| codex CLI | `~/.codex/sessions/-home-dracon-Dev-<slug>/` | none (`AGENTS.md` is docs) |
| claude | `~/.claude/projects/-home-dracon-Dev-<slug>/` | none observed |
| opencode | `~/.local/share/opencode/`, `~/.config/opencode/` | `.opencode/goals/state.json` observed EMPTY |
| antigravity | `~/.gemini/antigravity*`, install dirs | none observed (maxdepth-2 find) |
| pi / GLLA | `~/.pi/sessions/` + `.pi-glla/` in-repo | in-repo files exist but are harness-specific → OUT per operator |

Conclusion: the commit path MUST NOT cross the watch root for message
material. Whatever a harness keeps outside the repo is invisible by
construction — and that is the correct boundary for all harnesses,
not a GLLA-specific limitation.

### 2f. Consumers of the format

- AI + operator `git log --grep=` (ARCHITECTURE.md contract).
- Design docs quote subjects verbatim in investigations (format
  stability matters; token order must not churn).
- `repos` report first-line logging (`sync_commit` incident).

## 3. Proposed changes (no new config)

No new `SyncPolicy` field (avoids the `RepoPolicyOverride` tripwire).
All parsing stays inside `compute_blast_radius` + helpers, under the
existing caps (`take(3)` files/dirs, `take(10)` tasks/new/del,
`take(5)` deps, 60-char task compaction).

### 3A. Deepest-common-prefix scope (fixes §2a)

Replace `in <top-3-first-components>` with the longest common
directory prefix of staged paths, capped at 4 components:

- single file `web/games/wip/polis/src/main.ts` → `in
  web/games/wip/polis/src` (parent dir —today: `in web` + redundant
  full path in the file list).
- files under one subtree → that subtree.
- files spanning top dirs (multi-crate) → common prefix is root →
  fall back to today's top-3 dir list, byte-identical behavior
  (dracon-utilities, dracon-strategy unaffected by construction).
- root-level files → today's empty-scope rendering (unchanged).
- when the scope already equals the whole file list, drop the
  redundant `[top-files]` bracket (it adds zero information).

Pure path math on the already-fetched staged list. Deterministic,
bounded, harness-blind.

### 3B. File-signal tokens (fixes §2c)

- `EXT:ts,SV…` — extension histogram, top 3, lowercase, excluding
  gitlink paths (`is_gitlink`) and extensionless entries. Tells the
  next agent which toolchain/tests matter before opening the diff.
  Example: `EXT:svelte,ts,json`.
- `DOCSONLY:<n>` — mirror of `TESTONLY:` (same abbreviation style)
  when every staged file is `.md`/docs. 7.5% of platform commits.
- `LOCK` — lockfile-only change (basename in
  {`Cargo.lock`,`package-lock.json`,`bun.lock`,`pnpm-lock.yaml`,
  `poetry.lock`,`go.sum`,`Gemfile.lock`} or endswith `.lock`).
  Dependency bumps with no manifest edit are currently silent.
- Order: fixed, after `DELTA:` with the other metrics:
  `… DELTA:+a/-r | EXT:… TEST:… BIN:… DOCSONLY…/TESTONLY… LOCK …`.

### 3C. Rename awareness (fixes §2d)

- Add `-M` ONLY to the metadata path (`extract_new_deleted_files`,
  `sync.rs:2578`): parse `R100 <old> <new>` → `REN:n` token, and
  exclude renamed paths from the NEW:/DEL: lists (a move is neither).
- Keep `-M` OUT of the three staging call sites (3433, 3775, 3822):
  3-column status lines would disturb `git_name_status_entries`
  parsing and staging semantics. Metadata-only change, zero staging
  risk.

### 3D. Length budget with priority truncation (fixes §2b)

Budget: 240 chars (covers today’s ~p90=220; only the unbounded tail
is touched). Enforcement order — cut at token boundaries, never
mid-token, never touching, in order:

1. intent prefix (`CLOSED:/WIP:/MERGE:/REVERT:`) — never dropped.
2. scope — never dropped.
3. `DELTA:+a/-r` — never dropped.
4. safety metrics (`BIN:`, `ENV:`, `TESTONLY:`, `DOCSONLY:`, `LOCK`,
   `REN:`) — never dropped (blob/size/secrets-adjacent signals).
5. count metrics (`TEST:`, `TOKENS:`, `EVIDENCE:`, …) — trim right.
6. `[top-files]` bracket — first to go (fully recoverable via
   `git show --name-only`; the count in `N file(s)` already survives).

A unit test asserts `compute_blast_radius(..).len() <= 240` on a
pathological fixture. Grepability is preserved: every surviving token
is whole; dropped tokens are the recoverable ones.

### 3E. Deferred: daemon `auto:` marker

Precedent exists (`auto: initial commit`, bootstrap path). Benefit:
one-grep separation of daemon vs human (`fix(…):`, `release:`)
commits for AI and operator alike. Cost: shifts EVERY existing
`^CLOSED:`/`^[0-9]+ file` grep pattern and the quoted-subject
history in design docs. Recommendation: defer until §3A–3D prove
out; decide explicitly with the operator, not bundled.

## 4. Non-goals (operator-set or reasoned)

- Count-based batching stays (`max_stage_batch_files`). Coherence
  batching (split by dir/type) changes staging semantics and push
  volume → needs an opt-in knob + tripwire work. Explicitly out.
- No bodies, no prose, no LLM anywhere near the path.
- No harness-specific parsing of any kind (this supersedes the
  GLLA-trails doc; GLLA/`.pi` files keep flowing through the generic
  checkbox scanner as any other markdown does).
- No reads outside the watch root (per §2e table).
- No ledger/staging/push behavior changes. Worst case per token is
  "absent" and the subject equals today's.

## 5. Tests (per AGENTS.md test discipline)

- LCP scope: single file, shared subtree, spanning crates (fallback
  identical to today), root files, 4-component cap, scope==filelist
  bracket drop.
- Tokens: EXT top-3 + gitlink/noext exclusion, DOCSONLY incl.
  mixed `.md`/case variants, LOCK set incl. negative
  (`changelog.md` must NOT match), REN via `-M` fixture with
  NEW/DEL exclusion.
- Budget: pathological fixture (10 long tasks + long paths + all
  metrics) asserts len ≤ 240 with intent/scope/DELTA/safety intact.
- Back-compat: golden subjects for the existing test fixtures
  unchanged where the new code paths don't fire.
- Gates: `cargo test --workspace --locked`, `cargo build --release
  --locked`, `cargo deny check`,
  `cargo clippy --workspace --locked -- -D warnings` (monorepo root).

## 6. Evidence appendix (repro, file-mediated)

- Lengths: `git log --format="%H %ci %s" -2000 > f; python3` offset
  parse (`h=f[:40], s=f[67:]`) → p50=102 p90=220 p99=515 max=1421.
- Scopes: `git log --format="%s" -200 > f` + regex
  `(\d+) file\(s\) in (dirs)( \[| DELTA)` → platform `web` 174/200.
- Composition: `git log --format="COMMIT %h" --name-only -200 > f` →
  docsonly 15/200, extension counts.
- Harness stores: `ls ~/.codex/sessions ~/.claude/projects
  ~/.local/share/opencode ~/.pi/sessions`; in-repo
  `find <repo> -maxdepth 2 -iname '.codex|.agent|.gemini|*.plan.md'`
  (empty); `.opencode/goals/state.json` = empty goals.
