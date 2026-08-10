# Disk-full incident, credential verification, and cleanup discipline (2026-08-10)

## 1. What happened

Root filesystem (`/dev/nvme0n1p2`, 907 GB) hit **98% full — 8.5 GB free**. Consequence:

- **Google Chrome 148 crashed**: crashpad failed with
  `writev: No space left on device (28)`, then
  `Failed to write the temporary index file`, then SIGTRAP core dump
  (`systemd: Main process exited, code=dumped, status=5/TRAP`).
  Service: `app-google\x2dchrome@79336ad94ad140a58eaf0fe689614179.service`,
  Mem peak 8.1 G + 10.7 G swap.
- Node processes OOM'd in the same window (V8 `FatalProcessOutOfMemory`).
- Operator concern raised: **"credentials missing after the Chrome crash."**

**Verified outcome: no credentials were lost.** The crash was ENOSPC, not
corruption. Detail in section 3.

## 2. What was freed (operator-approved scope: "regenerable artifacts")

| Item | Size | Command |
|---|---|---|
| 6 inactive Rust `target/` trees | ~204 GB | `rm -rf` per tree |
| npm cache `~/.npm/_cacache` | ~54 GB | `npm cache clean --force` (a prior `npm cache verify` alone GC'd 19.4 GB) |
| act cache `~/.cache/act` | ~10 GB | `rm -rf` |

Result: **8.5 GB → 290 GB available (67% used)**.

Explicitly NOT touched:

- Chrome profile + cache (`~/.config/google-chrome/**`, `~/.cache/google-chrome/**` — 5.3 GB; only clean with Chrome fully closed)
- Trash `~/.local/share/Trash` — **56 GB, contains credential-related deleted files (665 pattern matches), do not empty blindly**
- `~/dracon/backups` (38 GB)
- Session stores (`~/.local/share/opencode`, `mimocode`, `containers`, `baloo`, `pnpm`), journals (3.9 GB)
- `~/.dracon/**` secrets, git/gh credential plumbing

## 3. Credential verification protocol (read-only, rerunnable)

### 3.1 Chrome profile (active profile: `Default` / "Dracon")

```bash
# DBs are WAL-mode and locked by the live browser; copy main+wal+shm to /tmp first.
CK=/tmp/credcheck; mkdir -p "$CK"
for db in 'Login Data' 'Cookies' 'Web Data' 'Login Data For Account'; do
  cp -p "$HOME/.config/google-chrome/Default/$db" "$CK/$(echo "$db" | tr ' ' '_')"
  for ext in -wal -shm; do [ -f "$HOME/.config/google-chrome/Default/$db$ext" ] && cp -p "$HOME/.config/google-chrome/Default/$db$ext" "$CK/$(echo "$db" | tr ' ' '_')$ext"; done
done
# Integrity (expect: ok on all four):
for f in "$CK"/*; do sqlite3 -readonly "$f" 'PRAGMA integrity_check;'; done
# Row counts (expect: logins 252, cookies 3473 — values NEVER selected):
sqlite3 -readonly "$CK/Login_Data" 'SELECT COUNT(*) FROM logins;'
sqlite3 -readonly "$CK/Cookies"     'SELECT COUNT(*) FROM cookies;'
# Encryption prefix histogram (prefix only, never the payload):
sqlite3 -readonly "$CK/Login_Data" "SELECT substr(password_value,1,3), COUNT(*) FROM logins GROUP BY 1;"
```

2026-08-10 results: 4/4 `integrity_check = ok`; 252 logins (240 `v11`
encrypted, 12 empty); 3473 cookies; all 7 profiles retain `gaia_id` +
`user_name` in `Local State`; `signin.active_accounts` present.

**Chrome 148 key-storage note (do not false-alarm on this)**:
`Local State` has `os_crypt` = `{portal: {prev_desktop, prev_init_success}}`
and **no `encrypted_key`** — normal for this generation on Linux. The
encryption key lives in KWallet (`kwalletd6` running,
`~/.local/share/kwalletd/`). `portal.prev_init_success: true` confirms the
keyring initialized. Absence of the legacy `encrypted_key` field is NOT
credential loss; absence of `portal` + no keyring + fresh Local State WOULD be.

A live-browser `sqlite3 -readonly` returning `database is locked (5)` is
normal contention, not corruption — copy the files first (as above).

### 3.2 `~/.dracon` secrets (values never printed)

```bash
# Presence/perms/mtime: all files must be pre-incident mtimes, mode 600.
find ~/.dracon/secrets -maxdepth 2 -printf '%M %s %TY-%Tm-%Td %TH:%TM %p\n'
# Variable NAMES only — proves each file has its expected key:
for f in ~/.dracon/secrets/pat/*.env ~/.dracon/secrets/ai/*.env; do
  printf '%s: ' "$(basename "$f")"; grep -oE '^[A-Za-z_][A-Za-z0-9_]*=' "$f" | tr -d '=' | tr '\n' ' '; echo
done
# Expected: codeberg=CODEBERG_TOKEN, cratesio=CARGO_REGISTRY_TOKEN, github=GH_TOKEN,
#           gitlab=GITLAB_TOKEN, npm=NPM_TOKEN, minimax=MINIMAX_API_KEY, cloudflare=CLOUDFLARE_API_KEY
# SSH key parse (output to /dev/null):
for f in ~/.dracon/secrets/ssh/*.key ~/.dracon/secrets/ssh/id_ed25519 ~/.dracon/secrets/ssh/codeberg_dracon_sync; do
  ssh-keygen -y -f "$f" >/dev/null && echo "OK  $(basename "$f")" || echo "BAD $(basename "$f")"
done
```

2026-08-10 results: all env files carry their expected variable names; all
SSH private keys parse. `google_compute_engine` reports BAD to
`ssh-keygen -y` but is actually an **OpenSSH public key blob** (`AAAAE2VjZHNh
...` = `ecdsa-sha2-nistp256`) — a false alarm, not a credential issue.

### 3.3 Git / GitHub plumbing

```bash
git config --global credential.helper   # = store (see 4.3 risk note)
[ -f ~/.git-credentials ] && echo present
[ -f ~/.config/gh/hosts.yml ] && echo present
```

## 4. Credential-bearing paths — NEVER cleanup candidates

Unless the operator explicitly approves a specific path, cleanup work must
preserve **all** of these:

| Path | Why |
|---|---|
| `~/.config/google-chrome/**` (and `~/.cache/google-chrome/**` while Chrome runs) | Login Data\*, Cookies, Web Data, Preferences, Local State, Network |
| `~/.config/chromium/**`, `~/.config/microsoft-edge/**` (if present) | same |
| `~/.dracon/**` | secrets/, utilities/*/secrets/, pats, ssh, keys.archive (age) |
| `~/.config/git/**` | hooks, credential config |
| `~/.ssh/**`, `~/.config/age/**`, `~/.age/**` | keys |
| `~/.git-credentials`, `~/.netrc`, `~/.npmrc` | plaintext tokens |
| `~/.config/gh/hosts.yml`, `~/.config/glab-cli/**` | auth tokens |
| `~/.local/share/kwalletd/`, `~/.local/share/keyrings/` | browser/OS keyrings |
| `*.env`, `*.pem`, `*.key`, `*.age` **anywhere in `~/Dev`** | warden's protected set |

## 5. Filename patterns that signal credentials

Scan **before** any bulk delete or Trash empty. This exact scan found **665
matches** inside the 56 GB Trash (including a deleted `CREDENTIALS.md` and
`facade-repos/dracon-sync-*/src/secrets.rs`):

```bash
find <DIR> -xdev \( -iname '*chrome*' -o -iname '*chromium*' \
  -o -iname '*credential*' -o -iname '*password*' -o -iname '*secret*' \
  -o -iname '*token*' -o -iname '*login data*' -o -iname '*.env' \
  -o -iname '*.pem' -o -iname '*.key' -o -iname '*.age' \
  -o -iname '.git-credentials' -o -iname '.npmrc' -o -iname 'hosts.yml' \) -print
```

Non-zero matches → review the list (names only) before deleting; matches that
are test fixtures/module caches (e.g. `go/pkg/mod/.../token.go`) are benign,
matches under repo trees are not.

## 6. Cleanup rules learned

1. **Guard against live builds before `rm -rf target/`**: check
   `ps -eo args=` for `cargo (build|test|check|run)`, `rustc`, `npm install`,
   and **`vite build`** (a dev-server-only guard would have deleted the tree
   under an active build; poll the build PID to completion first). Long-running
   `vite dev` servers are safe to ignore for Rust targets.
2. **Root-owned files block `rm -rf`** (from `/tmp`-dir builds done as root):
   no passwordless sudo → remove user-owned content, leave the small root
   remnant (here: 358 files, 1.5 MB in `dracon-utilities/target`) for
   `cargo clean` or operator `sudo rm`.
3. **npm**: `npm cache verify` GC's old entries (19 GB here); `npm cache
   clean --force` nukes the whole cache (54 GB) — both safe, clean costs a
   re-download.
4. **Trash**: never empty blindly; run the section 5 scan first.
5. **Chrome caches**: only with Chrome fully closed; never while a profile is
   in use (WAL + lock contention).
6. **Verify credentials after any cleanup**, not before: the protocol in
   section 3 is read-only and takes under a minute.

## 7. Open risk (not acted on)

`credential.helper = store` with plaintext `~/.git-credentials`. Works, but a
compromise of the home dir leaks all git credentials. Consider migrating to
`credential.helper = libsecret` or `gh auth` before the next full-disk
recovery forces broader cleanup decisions.

## 8. Follow-up (same day): why the incidents went unmonitored + the fix

Post-incident inspection found the root gap: **`dracon-system-guard.service`
was disabled and inactive** — its journal has no entries since 2026-08-07 and
`systemctl --user is-enabled` reported `disabled`. The 2026-08-09 swap-thrash
and the 2026-08-10 ENOSPC crash both happened with NO guard daemon watching.

Remediation (dracon-system **v0.112.35**, 2026-08-10):

- **Memory/swap pressure guard** (`monitor_memory`): `/proc/meminfo` + PSI
  (`/proc/pressure/memory`) every pass; alerts on low free memory, high swap
  usage, or swap thrashing (PSI `full avg10`), with the top-5 RSS offenders
  in the notification. Never kills anything.
- **Rapid disk-fill alert** (`disk_rapid_fill_gbph`, default 20 GiB/h):
  byte-precise df history catches "disk filling fast" before the percent
  thresholds.
- **Stuck-candidate escalation** (`process_stuck_after_secs`, default 600):
  sustained-heavy processes are flagged "POSSIBLY STUCK".
- **Zombie detail**: per-pid zombie reports with parent/age (diagnostic).
- **Trash credential guard** (`trash_credential_guard`): the section 5 scan
  now runs inside `empty_trash` before any deletion; matches abort it.
- **Service re-enabled**: `systemctl --user enable --now
  dracon-system-guard.service` (0.112.35 binary installed to
  `~/.local/bin/dracon-system`).

See `dracon-system/CHANGELOG.md` and
`dracon-system/release-notes-v0.112.35.md` for details.
