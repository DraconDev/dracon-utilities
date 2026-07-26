# dracon-warden audit — 2026-07-26 (part 4, v0.113.0)

Scope: `/home/dracon/Dev/dracon-utilities/dracon-warden/` — `src/main.rs`,
`src/tests.rs`, `src/security/src/lib.rs`, `src/security/src/modules/*.rs`.
Focus: v0.113.0 hook-enforcement stack (pre-push ff-guard, pre-rebase guard,
setup-hooks) + regression checks (H9 binary-smudge, M3 oversized-clean, H8
managed-block wipe, swallowed errors, test git-config mutation).

Verified-good regression checks (no finding):
- **M3** oversized clean input now fails closed (`filter_clean_refusal_reason`,
  main.rs:2171-2208 + run_filter). ✓
- **H8** `harden_repo` now uses `replace_managed_block` for BOTH .gitignore and
  .gitattributes (main.rs:1084-1163); operator content outside the block is
  preserved. ✓ (one residual edge: finding L-4)
- Tests only mutate git config inside temp repos (src/tests.rs:97-99, 371-381);
  the F0.1 live-repo poisoning class is guarded by the pre-push identity check. ✓
- Hook details: new-branch push (remote_sha all-zeros) is exempted BEFORE the
  merge-base check (main.rs:2362) ✓; branch deletion blocked without
  DRACON_ALLOW_REWRITE (intended) ✓; new tags pass, moved tags blocked (correct);
  `git diff`/`git log` on annotated-tag ranges peel correctly (verified
  empirically) ✓; missing-remote-object `merge-base --is-ancestor` exits
  non-zero → hook blocks = fail-closed (correct for a guard; see L-3 for the
  shallow-clone consequence) ✓; `git pull --rebase` of UNPUBLISHED commits
  passes the pre-rebase guard ✓; blocking interactive rebase of published
  commits matches the stated policy ✓.

---

## HIGH

### H-1. H9 regression CONFIRMED: production filter-smudge still corrupts whole-file-encrypted binary secrets
- **Evidence**: `src/security/src/lib.rs:1115-1123` (`DraconWarden::smudge` — the
  path `main.rs:run_filter` actually calls for `filter-smudge`) goes straight to
  `String::from_utf8_lossy(bytes)` → `smart_smudge`, whose decrypt arm does
  `result.push_str(&String::from_utf8_lossy(&plaintext))`
  (`src/security/src/modules/filter.rs:361`). The v0.112.32 H9 fix
  (`decrypt_whole_file_tag`, lib.rs:804) is only wired into `seal_smudge`
  (filter.rs:585) and `decrypt_file` — and **neither `seal_smudge` nor
  `seal_clean`/`decrypt_path` has any caller in the binary** (grep: only
  `decrypt_file`←`decrypt_path`, itself uncalled). The H9 unit test exercises
  `decrypt_whole_file_tag` directly, so it passes while the production path
  stays broken.
- **Mechanism**: binary secret in a sensitive path → clean produces whole-file
  `[DRACON_SECRET:<b64>]` tag (ASCII, passes the `is_binary_content` NUL check)
  → checkout smudge decrypts inline → every invalid-UTF-8 byte becomes U+FFFD →
  corrupted worktree file → next clean re-encrypts the corrupted bytes →
  original bytes lost from history. Exactly the 2026-07-21 H9 corruption loop,
  still live.
- **Fix**: call `decrypt_whole_file_tag` FIRST in `DraconWarden::smudge` (and
  `Warden::smudge`) before the lossy inline path; add a byte-identical
  round-trip test that goes through `DraconWarden::smudge`, not the helper.

### H-2. Global pre-commit hook hard-blocks commits in EVERY non-hardened repo on the machine
- **Evidence**: `PRE_COMMIT_HOOK` (main.rs:2288-2317) exits 1 unless the repo's
  `.gitattributes` contains `filter=dracon`; `run_setup_hooks` sets global
  `core.hooksPath` (main.rs:2497-2510). **Empirically reproduced during this
  audit**: a throwaway repo in /tmp could not `git commit` ("Warden filter
  missing from .gitattributes"). Same mechanism also silently DISABLES all
  per-repo hooks fleet-wide (husky, pre-commit framework, cargo-husky) because a
  global `core.hooksPath` overrides `.git/hooks` for every repo.
- **Mechanism**: any third-party clone, scratch repo, or new repo outside
  warden's discovery roots cannot commit at all until hardened (or
  `--no-verify`). Operator tooling that commits in ad-hoc repos breaks with an
  unrelated-looking error.
- **Fix**: make the pre-commit hook a no-op for repos that are not
  warden-managed (e.g. only enforce when `.dracon/` or a warden marker exists),
  and chain/preserve repo-local hooks instead of shadowing them.

### H-3. pre-rebase `head -100` checks the NEWEST 100 commits — published commits deeper in the range silently escape
- **Evidence**: main.rs:2446 `for c in $(git rev-list "$upstream"..HEAD | head -100)`.
  `git rev-list` outputs newest-first; the cap drops the OLDEST commits in the
  range — precisely the ones most likely to be already published.
- **Mechanism**: `git rebase <upstream>` with >100 commits where commits 101+
  are on a remote → guard passes → published history rewritten → divergent
  fleet mirrors (the exact incident class v0.113.0 ships to prevent).
- **Fix**: the per-commit loop is unnecessary. Remote containment is
  ancestor-closed: if the OLDEST commit of the range is not contained in any
  remote-tracking branch, no newer one can be. Check only the boundary commit
  (`git rev-list "$upstream"..HEAD | tail -1`), or drop the cap entirely. One
  `git branch -r --contains` call instead of 100 subprocesses.

---

## MEDIUM

### M-1. `run_setup_hooks` overwrites foreign global hooks unconditionally (only-if-missing semantics NOT preserved)
- **Evidence**: main.rs:2467-2475 — `fs::write(&pre_commit_path, ...)`, same for
  pre-push/pre-rebase, with no existence check (contrast
  `install_hooks_for_repo`, main.rs:2563+, which refuses to overwrite).
  Non-atomic: direct `fs::write` (no temp+rename) and `chmod 755` only after
  all three writes — a crash mid-install leaves a truncated hook (syntax error
  on every commit/push machine-wide) or non-executable hooks (silently
  disabled). It also overwrites a pre-existing global `core.hooksPath` value,
  orphaning the operator's previous hooks directory.
- **Mechanism**: an operator's own `~/.config/git/hooks/pre-push` (or a
  hooksPath managed by another tool) is destroyed without backup or warning.
- **Fix**: refuse/backup when a non-warden hook exists (detect the warden
  header comment before overwriting); write temp+rename; chmod before rename.

### M-2. pre-push secret-scan regex: `\x27` is literal in GNU grep ERE — single-quoted secrets escape the scan
- **Evidence**: main.rs:2406 pattern `password\s*=\s*["\x27][^"\x27]+` (also
  `secret=`, `api_key=`). Verified against GNU grep 3.12: `grep: warning: stray
  \ before x`; `\x27` matches the literal string `x27`, so the class is
  `["x27]` and `password = 'def456'` does NOT match; the negated class
  `[^"\x27]+` additionally refuses values containing `x`, `2`, or `7`.
- **Mechanism**: the defense-in-depth scan (the ` catches --no-verify` layer)
  misses single-quoted credential assignments — a common .env/config style.
- **Fix**: replace `\x27` with a literal `'` (pattern is already in a
  double-quoted-ish context inside the hook; use `["\047]`-free plain quoting),
  or switch to `grep -P` with a fallback.

### M-3. pre-rebase bypass via the two-argument form `git rebase <upstream> <branch>`
- **Evidence**: main.rs:2442-2446 checks `git rev-list "$upstream"..HEAD`
  (`$1`..HEAD) and ignores `$2`. Git rebases `$2` (or HEAD if unset).
- **Mechanism**: `git rebase main feature` run while on `main` yields an empty
  range → guard passes while published `feature` commits are rewritten.
- **Fix**: use `${2:-HEAD}` as the range tip.

### M-4. `harden_repo` silently skips repos whose `.git` is a gitfile (linked worktrees, nested submodules)
- **Evidence**: `is_repo_checked_out` (main.rs:1039-1062) does
  `fs::read_to_string(repo.join(".git/HEAD"))`; for a worktree/submodule
  checkout `.git` is a FILE containing `gitdir: ...`, so the read fails →
  returns false → `harden_repo` returns `(false,false,false)` with no log at
  default verbosity (main.rs:1104-1114). `IndexLock::acquire` has the same
  gitfile blindness (ENOTDIR → skip).
- **Mechanism**: any discovered repo using a gitfile layout never gets managed
  .gitignore/.gitattributes blocks, filter config, or pubkey publishing — with
  no warning. Discovery (`discover_git_repos`) accepts gitfile repos
  (`.git.exists()`), so they are silently half-processed.
- **Fix**: resolve gitdir via `git rev-parse --git-dir` (handles gitfile) and
  log skips at verbosity 0.

### M-5. `salvage_invalid_json_markers` panics on non-ASCII content (and would mojibake if it didn't)
- **Evidence**: main.rs:1648-1656 — the loop indexes `content[i..]` (via
  `marker_prefix_at`) at every BYTE offset; slicing a `&str` at a non-char
  boundary panics. `out.push(bytes[i] as char)` (main.rs:1652) also corrupts
  multi-byte UTF-8 into Latin-1 mojibake.
- **Mechanism**: `scrub-markers --apply` on an invalid-JSON file that contains
  a marker AND any non-ASCII text (e.g. "José") crashes the whole warden pass
  mid-run (hardening for subsequent repos never happens).
- **Fix**: iterate `content.char_indices()` and push `char`, or work on
  `&[u8]`/byte-string throughout.

### M-6. test-identity push guard has no escape hatch and scans FULL history on new-branch pushes
- **Evidence**: main.rs:2419-2428 — the `BAD_AUTHORS` check is OUTSIDE the
  `DRACON_ALLOW_REWRITE` guard, and on a new-branch push RANGE is
  `empty-tree..local_sha` (main.rs:2376-2382) = entire history.
- **Mechanism**: a repo with ANY historical commit authored `test@test` /
  `test@test.com` / `test@example.com` (e.g. residue of the original F0.1
  incident) can never push a new branch without `--no-verify` — the guard
  becomes a permanent, undocumentable-in-band block. The identity patterns
  themselves are fine for fixtures (commit identity, not file content).
- **Fix**: honor `DRACON_ALLOW_REWRITE` (or a dedicated env) for this check,
  and/or only scan commits not already on a remote (`--not --remotes`).

---

## LOW

### L-1. Daemon interaction: dracon-sync ALWAYS pushes `--no-verify` — warden's pre-push layer never gates the fleet's primary writer
- **Evidence**: dracon-sync `src/git/push.rs:22,41,65,113` — every push path
  (SSH, HTTPS fallbacks) uses `push --no-verify`. Therefore: (a) the ff-guard
  cannot break daemon pushes (good), (b) the secret scan and ff-guard provide
  ZERO protection for daemon pushes (the main writer in this fleet), (c) the
  daemon's auto-repair filter-repo force-push path cannot be blocked by the
  hook, and (d) `DRACON_ALLOW_REWRITE` being unset in systemd is moot.
- **Fix**: decide deliberately whether the daemon should run the hooks (drop
  `--no-verify` and set `DRACON_ALLOW_REWRITE=1` only for the auto-repair
  path), or document that the guard is interactive-only.

### L-2. `backfill_env_headers` never adds a header (unreachable success branch)
- **Evidence**: main.rs:2087-2096 — for a plaintext .env (no markers),
  `DraconWarden::smudge` returns the input unchanged → `out == bytes` → no
  write. The "✅ header added" branch only fires if inline tags decrypt. The
  header is only ever added on the CLEAN path (`make_env_version_header`).
- **Fix**: build the header directly (`make_env_version_header` + content)
  instead of routing through smudge.

### L-3. Missing remote object → fail-closed block with misleading message (shallow clones)
- **Evidence**: main.rs:2363 — `git merge-base --is-ancestor` exits non-zero
  (1 or 128, verified) when `remote_sha` is not present locally; `2>/dev/null`
  hides the cause and the hook prints "refusing non-fast-forward push".
- **Mechanism**: correct fail-closed posture, but in shallow/partial clones
  legitimate fast-forward pushes are blocked with a wrong diagnosis.
- **Fix**: distinguish exit 1 (not ancestor) from >1 (object missing) and emit
  "cannot verify ancestry (shallow?); fetch --unshallow or set
  DRACON_ALLOW_REWRITE=1".

### L-4. `replace_managed_block` malformed-block branch deletes file tail
- **Evidence**: main.rs:529-532 — BEGIN marker without END consumes `rest` to
  end-of-file; everything after an orphan BEGIN (truncated write, operator
  doc-comment) is replaced by the managed block.
- **Fix**: on missing END, treat the BEGIN as literal text (or refuse with an
  error) rather than deleting the tail.

### L-5. Swallowed errors in enforcement-adjacent paths
- **Evidence**: main.rs:1161 `let _ = install_hooks_for_repo(repo);`;
  main.rs:2479-2483 `let _ = fs::remove_file(&stale);`;
  `Command::Resmudge` discards the result (`let _ = resmudge_repos(...)`).
- **Mechanism**: hook-install failure (the v0.113.0 enforcement layer!) is
  invisible; a repo can run unprotected indefinitely.
- **Fix**: log at warn level / surface in the harden summary.

### L-6. Residual notes on the hook stack
- `install_hooks_for_repo` writes into `.git/hooks`, which are DEAD once the
  global `core.hooksPath` is set — harmless but confusing; the two installers
  disagree about overwrite policy (M-1).
- No template-dir (`~/.git-templates`) sync exists (covered by global
  hooksPath, so informational only).
- Per-push-ref `SCAN_FILES_NUL` is truncated per ref (`: > "$SCAN_FILES_NUL"`)
  — fine; newline-in-filename edge is documented.
- pre-rebase runs `git branch -r --contains` once per commit (up to 100
  subprocesses) — subsumed by the H-3 fix.
