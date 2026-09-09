# Host `/tmp` cleanup and the guard service sandbox — 2026-09-09

## Finding

`dracon-system` added age-based `clean_tmp` cleanup with a default
`tmp_search_paths = "/tmp"`. The shipped user service still had
`PrivateTmp=true`, so the daemon saw systemd's private temporary directory
instead of the host `/tmp` that the policy intended to reclaim. In addition,
`ProtectSystem=strict` makes the filesystem read-only unless a path is listed
in `ReadWritePaths`.

## Chosen architecture

The guard service deliberately shares the host temporary namespace:

```ini
ProtectSystem=strict
ReadWritePaths=... /tmp
PrivateTmp=false
```

`PrivateTmp=false` is explicit rather than relying on systemd's default. The
`/tmp` exception is narrow: it is added only because this daemon's purpose
includes deleting aged, top-level temporary entries. All existing cleanup
safety checks remain in force: age threshold, symlink exclusion, protected
paths, and open-file/open-ancestor protection. Other service hardening (such
as `ProtectHome=read-only`, `PrivateDevices`, and the syscall filter) remains
unchanged.

Keeping `PrivateTmp=true` would be safe from host `/tmp` mutation but would
also make the configured cleanup a no-op against the disk pressure it was
introduced to address. Delegating deletion to an unsandboxed helper would
weaken the service boundary more broadly and is not needed.

The F65 follow-up keeps the cleanup namespace explicit at the policy layer as
well: `tmp_search_paths` may name only canonical descendants of `/tmp` or
`/var/tmp`. Roots are validated before any scan, and each candidate is checked
against the validated root before an apply deletion, so a configuration such
as `tmp_search_paths = "~"` cannot turn this host namespace into recursive
home-directory cleanup.

## Verification

`dracon-system/src/tests.rs` includes the shipped
`dracon-system-guard.service` with `include_str!` and asserts that:

- the unit uses `PrivateTmp=false`;
- `/tmp` is an explicit `ReadWritePaths` entry; and
- the default guard policy enables `clean_tmp` and selects `/tmp`.

The same invariant is checked for the declarative deployment in
`scripts/check-flake.sh`. Its `nix eval` harness evaluates
`homeManagerModules.dracon` with `services.dracon.system.enable = true` and
fails unless the generated `dracon-system-guard.Service` has
`PrivateTmp = false` and `/tmp` in `ReadWritePaths`. This production
configuration check proves that both deployment paths expose the namespace
that the cleanup policy targets. After installing the updated unit,
operators should run `systemctl --user daemon-reload` and restart the guard
service before expecting the existing process to use the new namespace.
