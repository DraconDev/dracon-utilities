# dracon-platform AWS Key Rotation — 2026-06-28

> **Goal**: 007296af-5469-4a34-989e-0012219e6732
> **Operator**: "we would want ot check on the platwhich is what i intedned ... also you are talking about this i makde a new aws key we can jsut rotate to this"
> **Status**: PARTIAL — platform state captured (Part A done). Part B (key rotation) BLOCKED on operator providing the new key values.

## 1. Executive summary

The operator identified two issues from the prior session:
1. **Platform state** needed a fresh check (it was the original alarm "we still didn't hook up github and gitlab" + the underlying concern that real AWS keys were tracked in `dracon-platform`).
2. **A new AWS key was made** to rotate to, replacing the leaked `<AKIA-OLD-KEY>` / `<AWS-OLD-SECRET>` pair from `apis/services/email-api/.env.{dev,prod}`.

This doc captures the platform state (Part A), documents the operator's product-direction statement (Part E), and provides the template for the rotation evidence (Part B) once the operator provides the new key values.

## 2. Operator product intent (Part E — captured for record)

> "I ship real products and I show them working. Instead of just writing about AI, I integrate it into live, usable platforms like browser games and extensions. As an ambassador, I will build MiniMax into actual shipped features on Dracon and demonstrate exactly how they perform in production."

This is a documentation-only capture of the operator's ambassador framing. **No actual MiniMax integration work is in scope for this goal.** The framing is recorded here so future goals that DO build MiniMax features on Dracon have the operator's product-direction statement on record.

## 3. Platform state — Part A (DONE)

Captured 2026-06-28 20:16 BST, immediately before the rotation work begins.

### 3.1 Branch and upstream

```
$ git -C /home/dracon/Dev/dracon-platform branch --show-current
main
$ git -C /home/dracon/Dev/dracon-platform rev-parse --abbrev-ref --symbolic-full-name '@{u}'
codeberg/master
```

- Branch: `main` ✅ (no longer detached)
- Upstream: `codeberg/master` ✅
- Working tree: clean (1 untracked dir, 0 modified tracked files)

### 3.2 Remotes

```
$ git -C /home/dracon/Dev/dracon-platform remote -v
codeberg  git@codeberg.org:dracondev/dracon-platform.git (fetch/push)
github    git@github.com:DraconDev/dracon-platform.git (fetch/push)
gitlab    git@gitlab.com:dracondev/dracon-platform.git (fetch/push)
```

All 3 remotes configured ✅. Note: `gitlab` remote is configured but the repo doesn't exist on gitlab.com (404) — this is a separate operator action item.

### 3.3 PUSH-TO configuration

The per-repo TOML at `.dracon/dracon-sync.toml` does NOT contain an `exclude_remotes` block (it was removed 2026-06-28 per goal `30c80eff-e628-4283-b1bb-c6f0dcf4ddba`). The daemon's view shows:

```
PUSH-TO: github,gitlab,codeberg
```

All 3 remotes are attempted ✅. (github fails with HTTP 500 due to 11 GB > 5 GB free tier; gitlab fails with 404 because the repo doesn't exist — both are unrelated to the key rotation.)

### 3.4 Ahead/behind

```
codeberg:  0 ahead, 0 behind
github:    6320 ahead, 0 behind   (annex migration not done; not in scope for this goal)
gitlab:    unknown (repo 404)     (not in scope)
```

Codeberg is in sync ✅. The 6,320-commit github lag is the size-unblock issue documented in `dracon-platform-size-unblock-2026-06-28.md` (deferred to a future goal).

### 3.5 .gitignore un-ignore gap (line 67)

```
$ grep -n '!.env' /home/dracon/Dev/dracon-platform/.gitignore
66:!.env
67:!.env.prod
68:!.env.dev
69:!.env.production
70:!.env.ovh
71:!.env.turso
```

The `!.env.dev` and `!.env.prod` un-ignore patterns at lines 67-68 are what cause the env files to be tracked. **This is a known security issue from the full-architecture-audit-2026-06-28.md** but the fix requires operator authorization because:
- The env files contain warden-encrypted secrets; removing them from tracking could break warden's encryption flow
- A git history rewrite would be required to fully un-track them (AGENTS.md prohibits without override)
- The 100% safe alternative is to use a secret manager (e.g. AWS Secrets Manager, Vault), not git + warden

**Documented; NOT fixed in this goal.**

### 3.6 Working-tree evidence (the keys that need rotation)

**`.env.dev`** (decrypted view via `dracon-warden smudge`):

```
EMAIL_PROVIDER=ses
EMAIL_API_KEY=dev-email-api-key
SES_ACCESS_KEY=<AKIA-OLD-KEY>              <-- LEAKED, needs rotation
SES_SECRET_KEY=<AWS-OLD-SECRET>  <-- LEAKED, needs rotation
SES_REGION=us-east-1
SES_FROM_ADDRESS=noreply@dracon.uk
EMAIL_REPLY_TO=support@dracon.uk
```

**`.env.prod`** (decrypted view):

```
EMAIL_PROVIDER=ses
SES_ACCESS_KEY=<AKIA-OLD-KEY>              <-- LEAKED, needs rotation
SES_SECRET_KEY=<AWS-OLD-SECRET>  <-- LEAKED, needs rotation
SES_REGION=us-east-1
SES_FROM_ADDRESS=noreply@dracon.uk
EMAIL_REPLY_TO=support@dracon.uk
EMAIL_RATE_LIMIT_PER_SECOND=14
```

Both files contain the leaked keys. The operator stated they made a new key — **awaiting the new values**.

## 4. Key rotation plan — Part B (PENDING operator input)

Once the operator provides the new AWS access key ID and secret, the steps are:

### 4.1 Replace values in env files (in-place)

```bash
# Decrypt env file to plaintext (warden smudge does this on checkout,
# so the working tree is already plaintext)
$EDITOR /home/dracon/Dev/dracon-platform/apis/services/email-api/.env.dev
$EDITOR /home/dracon/Dev/dracon-platform/apis/services/email-api/.env.prod

# Replace SES_ACCESS_KEY and SES_SECRET_KEY lines with the new values
```

### 4.2 Re-encrypt with warden

```bash
$ dracon-warden once /home/dracon/Dev/dracon-platform
```

This re-encrypts the env files in place (using warden's age/sops-style encryption with the operator's GPG/age key).

### 4.3 Read-back verify (decrypt)

```bash
$ cat /home/dracon/Dev/dracon-platform/apis/services/email-api/.env.dev
# Should now show the new key values
$ grep -c "<AKIA-OLD-KEY>" /home/dracon/Dev/dracon-platform/apis/services/email-api/.env.dev
# Should return 0
```

### 4.4 Operator action items after this goal completes

1. **Disable OLD key in AWS IAM** — go to https://console.aws.amazon.com/iam/home#/security_credentials and delete access key `<AKIA-OLD-KEY>`. The leak window closes once the key is disabled, even if it remains in git history.
2. **History-rewrite decision** — old key still in `dracon-platform` git history. Options:
   - `git filter-repo` history rewrite + force-push (destructive, requires explicit override)
   - Accept the leak (justified if you trust the key has not been used by an attacker)
   - Wait for AWS IAM to disable the key, then the leak becomes moot
3. **Create gitlab repo** — `glab auth login` then `glab repo create dracondev/dracon-platform --private --description "Dracon platform"`. Out of scope for this goal.
4. **Annex migration** — Phase 2-5 of `dracon-platform-size-unblock-2026-06-28.md` to unblock github. Out of scope for this goal.
5. **Fix .gitignore un-ignore** — change `!.env.dev` to `.env.dev` (with leading `!` removed). This would un-track the env files but the secrets are already in history, so it only helps going forward. Out of scope for this goal.

## 5. History-leak risk — Part C

The old key `<AKIA-OLD-KEY>` and secret are still in `dracon-platform` git history because the files were committed in plaintext. Even after the working-tree rotation, the keys remain in every commit that touched those files.

**The leak window closes** when:
1. The operator disables the key in AWS IAM (immediate, this goal)
2. The key was not used by an attacker between the leak (when files were committed) and the disable (now)

**To remove the keys from history** requires:
- `git filter-repo` to rewrite the history
- Force-push to all 3 remotes (3,178+ commits ahead on github, 0 on codeberg)
- AGENTS.md "no force-push on >5-commits-ahead repos without explicit operator override" applies

**Recommendation**: Disable the key in AWS IAM (operator action). Do NOT do a history rewrite unless the operator explicitly authorizes it.

## 6. Daemon view — Part D evidence

The `dracon-platform` row in `dracon-sync repos`:

```
│ 2  ┆ ✅ OK    ┆ dracon-platform                         ┆ main      ┆ codeberg/master ┆ 0      ┆ 0      ┆ 9     ┆ 0       ┆ 0        ┆ OK         ┆ github,gitlab,codeberg ┆ 55b7500589e… 2 file(s) in web [web/games/wip/endless-td/WAVE1_BALANCE.md, web/games/wip/… ┆ -         ┆ ⚪ idle 1h                  ┆ dracon    ┆ 0     ┆ 3178  ┆ 3178   ┆ ⚪ untracked-only ┆ 1h ago sync_commit  ┆ healthy                                                                                                 │
```

✅ dracon-platform row is OK with 0/0 ahead/behind codeberg. The `⚠️ WARN 1` count in the daemon summary is on `dracon-utilities` (this repo) due to github GH013 push-protection block — separate issue from the platform key rotation. Full state captured at `dracon-platform-state-2026-06-28.txt`.

## 7. Files

- This doc: `docs/design/audit-2026-06-26/dracon-platform-aws-rotation-2026-06-28.md`
- State capture: `docs/design/audit-2026-06-26/dracon-platform-state-2026-06-28.txt` (25 lines, fresh `dracon-sync repos` output)
- Earlier audit (referenced): `docs/design/audit-2026-06-26/full-architecture-audit-2026-06-28.md`
- Size unblock plan (referenced): `docs/design/audit-2026-06-26/dracon-platform-size-unblock-2026-06-28.md`

## 8. Hard acceptance criteria — current status

| # | Criterion | Status |
|---|-----------|--------|
| 1 | Fresh `dracon-sync repos` capture at `dracon-platform-state-2026-06-28.txt` | ✅ DONE |
| 2 | `git status -sb` shows `main` as current branch | ✅ DONE |
| 3 | `git remote -v` shows codeberg + github + gitlab | ✅ DONE |
| 4 | PUSH-TO includes github,gitlab,codeberg | ✅ DONE |
| 5 | `.gitignore` un-ignore pattern still present (line 67-68) | ✅ DOCUMENTED |
| 6 | `<AKIA-OLD-KEY>` absent from `.env.dev` | ⏳ PENDING new key |
| 7 | `<AKIA-OLD-KEY>` absent from `.env.prod` | ⏳ PENDING new key |
| 8 | NEW key present in both env files | ⏳ PENDING new key |
| 9 | `dracon-warden once` exits 0 | ⏳ PENDING rotation |
| 10 | Files still decrypt and contain new values | ⏳ PENDING rotation |
| 11 | This audit doc exists with all sections | ✅ DONE (skeleton) |
| 12 | Doc committed and pushed to codeberg + gitlab | ⏳ PENDING (will be auto-committed by daemon) |
| 13 | `dracon-sync repos` shows 16 OK, 0 WARN, 0 CONCERN | ⚠️ Currently 15 OK 1 WARN (dracon-utilities GH013) — not a platform issue |
| 14 | Working-tree scrub of old key confirmed | ⏳ PENDING new key |

**5 of 14 criteria met (Part A + skeleton). 9 criteria pending operator providing new key values.**
