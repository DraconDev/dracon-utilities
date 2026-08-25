# Memory performance + /tmp hygiene — 2026-08-25

Incident: disk 93% (797G/907G) with swap thrash (34G/39G swap in use,
PSI memory some avg10=63%, load ~40). Manual full sweep reclaimed
~320 GiB (/tmp 207G, Docker build cache 83G, Trash 58G, stale build
dirs ~11G) and the VM tuning below removed the thrash pattern.

## Root causes

1. **No /tmp cleanup existed anywhere.** Guard cleaned `~/Dev` rust
   targets but never looked at `/tmp`, where puppeteer/playwright
   profiles (~17G), pi-bash scratch logs (6.8G+), and stale audit
   clones accumulated for weeks.
2. **Trash/nix/docker cleanup knobs were off** (`clean_trash=false`,
   `clean_nix_garbage=false`, `docker_prune=false` in the live
   `dracon-system.toml`) — the guard reported candidates every cycle
   without acting.
3. **VM settings tuned for disk swap, not zram**: `vm.swappiness=10`
   forced file-cache eviction + direct-reclaim stalls instead of cheap
   zstd zram eviction (measured 3.9x ratio); default
   `page-cluster=3` adds 32KiB read bursts that are pointless for
   in-RAM swap.

## Changes

### dracon-system v0.112.39 (code)

- `clean_tmp` / `tmp_search_paths` / `tmp_min_age_hours` knobs +
  age-based top-level `/tmp` cleanup with open-fd protection
  (`collect_open_paths_under` scans `/proc/*/fd`; held entries are
  skipped even when old).
- `trash_min_age_days` (default 7): aged trash purge keeps a recovery
  window; `0` = old empty-everything behavior. `.trashinfo` files are
  purged alongside their entries.
- Proactive tier (>= `proactive_cleanup_percent`, i.e. 80%) now also
  runs tmp/trash/nix/docker via `run_auto_cleanup(..., include_rust=false)`;
  the heavy rust scan stays on its own cadence.
- Tests: aged-trash, zero-age empty, tmp age/dry-run/open-fd, defaults.
  Workspace 1419 passed; clippy `-D warnings` clean.

### Config (live box)

- `~/.dracon/utilities/system/dracon-system.toml`: flipped
  `clean_trash` / `clean_nix_garbage` / `docker_prune` to true
  (volumes still false); `mem_available_warn_percent` 10 → 15.
- `~/.config/systemd/user/dracon-system-guard.service`: granted
  `AmbientCapabilities=CAP_SYS_NICE` (+ bounding set) so
  `auto_renice_on_memory` actually runs instead of emitting its
  "lacks CAP_SYS_NICE" diagnostic.
- `~/.dracon/nixos/configuration.nix`:
  - `zramSwap.memoryPercent` 100 → 150 (45G device at ~3.9x ratio).
  - sysctl: `swappiness` 10 → 150, `page-cluster` → 0,
    `watermark_scale_factor` → 125, `watermark_boost_factor` → 0;
    `vfs_cache_pressure=75` kept.
  - Apply: `sudo nixos-rebuild switch --flake ~/.dracon/nixos#nixos`
    (dry-build validated 2026-08-25).

## Results after sweep + reboot

Disk 93% → 56%; PSI memory some avg10 63% → 1.3%; swap 34G → 7.8G.

## Known follow-ups

- `micro1/2/3` flake targets import a missing `server-tools.nix`;
  `#nixos` unaffected. Restore the file or drop the imports.
- Journal vacuum (2.9G reclaimable) needs operator sudo.
