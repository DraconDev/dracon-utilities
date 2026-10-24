# Disk-full recovery, 2026-10-20

**Status:** complete — 97 GiB reclaimed, guard hardened, dev stack restored.

## Incident

The root filesystem reached 99% (12 GiB free) on 2026-10-20. A
targeted `dracon-system storage --cleanup --apply` run over 8 selected
paths reclaimed ~97 GiB and dropped usage from 97% to 88%.

One of the selected paths was `/home/dracon/Dev/dracon-platform/target`.
That directory is not pure build output: the dracon-platform dev stack
keeps its **service runtime state** inside it:

```
target/dev-stack/sockets/*.sock   unix socket paths (auth-api, billing-api,
                                  email-api, music-api, studio-api, ai-hub,
                                  dashboard)
target/dev-stack/*.pid             pidfiles
target/dev-stack/*.identity        pid → kernel start-time binding
target/dev-stack/bin/*             pinned hardlinks of the service binaries
target/dev-stack/Caddyfile.dev.rendered
```

Deleting it unlinked the socket paths of five **live** API workers.

## Observed failure mode

The workers kept serving — Caddy held pooled connections, so
`GET /api/auth/health` still returned 200 — but no *new* connection
could be opened:

```
$ python3 -c "connect('/…/target/dev-stack/sockets/auth-api.sock')"
connect FAIL: [Errno 2] No such file or directory
```

`scripts/dev-status.sh` reported the accurate diagnosis:

> ⚠ SOCKET MISSING: …/sockets/auth-api.sock unlinked while the
> process still holds it — restart auth-api to recreate

`make down` would also have orphaned the processes (no pidfiles), and any
restart would have failed (no pinned binaries).

This is the class of bug where a health probe returning 200 is
**not** evidence of health: the success came from a pooled connection to
an inode whose *path* no longer existed.

## Root cause

`validate_storage_cleanup_activity`
(`dracon-system/src/main.rs`) guarded cleanup candidates against:

- an active Rust **build** (cargo/rustc whose cwd is the project), and
- a 60-second mtime backstop.

It had no notion of a running **service**. No cargo/rustc was running
against the platform tree at cleanup time, and the tree's mtime was
older than 60 seconds, so every check passed and the candidate was
deleted.

The same class was confirmed in a second repo: `db-gateway` runs
`.cargo/target/debug/dg serve` (a TCP listener, no socket, no pidfile in
that tree).

## Fix (durable)

Three live-state probes added to the apply-path validation. A candidate
is refused when any of them fires:

| Probe | Mechanism | Catches |
|---|---|---|
| `live_process_binary_under` | `/proc/<pid>/exe` for every PID, ` (deleted)` suffix stripped | a service running its binary out of the candidate |
| `live_unix_socket_under` | `/proc/net/unix`, listening stream sockets | the dev-stack socket paths |
| `live_pidfile_under` | `*.pid` under the candidate naming a live PID | services tracked only by pidfile |

`live_pidfile_under` deliberately treats a **stale** pidfile (PID gone)
as eligible — otherwise the guard could never reclaim a target dir whose
services have stopped. The scan skips cargo's own cache dirs
(`deps`, `build`, `incremental`, `debug`, `release`, `.fingerprint`) so it
stays cheap on a multi-gigabyte target dir.

### Verification

Six targeted tests in `dracon-system/src/tests.rs` (four new):

```
storage_cleanup_activity_refuses_live_service_socket
storage_cleanup_activity_refuses_live_pidfile
storage_cleanup_activity_refuses_live_process_binary
storage_cleanup_activity_allows_stale_pidfile
storage_cleanup_activity_refuses_live_rust_and_node_processes   (pre-existing)
storage_cleanup_activity_refuses_recent_artifacts               (pre-existing)
```

Live check with the rebuilt release binary:

```
$ dracon-system storage --cleanup --apply --kinds rust-build
🛡️ keeping /home/dracon/Dev/dracon-utilities/target: live process 1105810
   runs …/target/debug/deps/dracon_sync-8e52d8a68f9d56e1 from inside this path
🛡️ keeping /home/dracon/Dev/eve/target: live process 173833 runs …/eve/target/debug/eve …
🛡️ keeping /home/dracon/Dev/db-gateway/.cargo/target: live process 1183596 …
🛡️ keeping /home/dracon/Dev/dracon-platform/target: live process 3260069
   runs …/target/dev-stack/bin/auth-api from inside this path
🛡️ keeping /home/dracon/Dev/dracon-strategy/ai-auto-writer/target: active Rust build …
🗑️  Deleting /home/dracon/Dev/dracon-platform/crates/ai-api-sdk/target
```

Full workspace suite green; `cargo test --workspace --locked` passes.

## Dev stack restoration

The state dir was rebuilt using the project's own primitives, not
by hand:

- `dev_stack_write_pidfile` wrote pidfile + `.identity` for the five
  live workers and for caddy, so the tooling's identity binding
  (pid → kernel start time) is correct.
- `ln -f target/debug/<svc> target/dev-stack/bin/<svc>` re-pinned the
  rebuilt binaries.
- `make render-caddy` regenerated `Caddyfile.dev.rendered`.
- `bash scripts/dev-restart.sh <svc>` per Rust API re-bound each unix
  socket. The tooling's own reap/adopt logic handled the unlinked-socket
  workers; the Vite servers (port-bound) were adopted, not restarted.
- `bash scripts/dev-restart.sh ai-hub dashboard` respawned only the two
  socket bridges whose socket paths had been unlinked.

Result: 10/10 services up, no `SOCKET MISSING` warnings, all API health
endpoints 200 through Caddy on :18080.

**Lesson:** `dev-status.sh` showing `SOCKET MISSING` is the authoritative
signal; a 200 from a pooled connection is not.

## Additional reclamation

| Item | Reclaimed | Note |
|---|---|---|
| Codex `app-server-daemon` releases | 4.2 GiB | 10 superseded versions kept; only `current` (0.163.0) is live |
| Pi session retention stash | 11.0 GiB | 1017 policy-expired sessions (>30 d) |
| `crates/ai-api-sdk/target` | 553 MiB | genuinely disposable build output |

### Session retention was never running

`pi-session-retention` existed in `~/Dev/pi-plugins/extensions/` and its
purge timer was installed and firing, but the extension was **not** in
Pi's package list — so the 30-day policy never ran and the stash
stayed empty. The timer was purging nothing.

`pi install ~/Dev/pi-plugins/extensions/pi-session-retention` registers
it, so the policy now runs on each Pi startup. The purge timer alone was
never sufficient: it only expires runs that the extension has already
stashd.

Remaining 30-day-old sessions are **forks of retained parents** and are
protected by design (`protectedByParent: 100`).

## Deliberately NOT removed

| Path | Size | Why kept |
|---|---|---|
| `web/music/out/` (129 GiB audio) | 129 GiB | live publish tree; the music rebuild was actively writing it. `out/.stale-orphans` and `out/audio/.stash` are the music pipeline's own stash convention, not junk. |
| `dracon-platform/.git/modules` | 26 GiB | submodule gitdirs with real published history |
| `dracon-utilities/target` | 55 GiB | `protected_paths` in the policy; the repo that ships the guard |
| `~/dracon/backups/*.bundle` | ~14 GiB | pre-slim rollback bundles; the sanctioned-slimming procedure requires a verified bundle to exist |
| `/var/lib/systemd/coredump` | 2.4 GiB | needs root; not attempted |
| `~/.codex/sessions`, `~/.pi/agent/sessions` (active) | 31 GiB | within retention policy |

## Residual risk

The disk returned to 96% within ~30 minutes because the concurrent
workload (music rebuild, `cargo test` runs, Vite builds) writes faster
than cleanup reclaims. The guard's rust auto-cleanup is
`auto_cleanup_rust = false` (disabled 2026-10-04), so it will not
reclaim build output on its own, and it now correctly refuses the
target dirs that hold live services.

Reclaiming the remaining headroom requires either enabling
`auto_cleanup_rust` (with the new live-state guards now in place) or
stopping the concurrent build workload. Both are operator decisions, not
agent decisions.
