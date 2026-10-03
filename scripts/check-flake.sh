#!/usr/bin/env bash
# Validate the flake while allowing the standard Home Manager module output.
# Nix's generic flake checker does not know the Home Manager convention
# `homeManagerModules`, although Home Manager consumes it correctly.

set -euo pipefail

# Anchor both checks to this Git checkout, including when called elsewhere.
cd "$(git -C "$(dirname "${BASH_SOURCE[0]}")" rev-parse --show-toplevel)"

output="$(nix flake check --no-build 2>&1)" || {
    printf '%s\n' "$output"
    exit 1
}

unexpected="$({
    printf '%s\n' "$output" \
        | grep '^warning:' \
        | grep -v "unknown flake output 'homeManagerModules'" \
        | grep -Ev "^warning: Git tree .* is dirty$" || true
} | sed '/^warning: The check omitted these incompatible systems:/d' | sed "/^Use '--all-systems' to check all\./d")"

if [[ -n "$unexpected" ]]; then
    printf '%s\n' "$output"
    printf 'FAIL: unexpected Nix flake warning(s):\n%s\n' "$unexpected" >&2
    exit 1
fi

printf '%s\n' "$output" \
    | sed -E '/^warning: Git tree .* is dirty$/d' \
    | sed "/^warning: unknown flake output 'homeManagerModules'$/d" \
    | sed '/^warning: The check omitted these incompatible systems:/d' \
    | sed "/^Use '--all-systems' to check all\.$/d"
echo "PASS: Nix flake checks passed (Home Manager output warning is intentional and documented)."

# Evaluate the Home Manager module with systemd service options supplied by a
# minimal test harness.  This checks the generated services, not just the
# standalone units shipped beside the crates. Both services must stay in
# parity with their shipped units (audit H1/H2, 2026-10-02; extended to
# EVERY shipped-unit property by R3-L25, 2026-10-03 — the earlier subset
# let unasserted properties re-drift invisibly) so Nix/HM drift fails CI.
# The only deliberate exception is ExecStart (store path vs ~/.local/bin),
# which is asserted by suffix instead of equality.
service_check="$(nix eval --impure --raw --expr '
let
  # A plain path imports ignored build output and private local files.
  flake = builtins.getFlake ("git+file://" + toString ./.);
  pkgs = import flake.inputs.nixpkgs { system = builtins.currentSystem; };
  lib = pkgs.lib;
  evaluated = lib.evalModules {
    modules = [
      flake.homeManagerModules.dracon
      {
        options.systemd.user.services = lib.mkOption {
          type = lib.types.attrsOf lib.types.anything;
          default = {};
        };
        options.systemd.user.timers = lib.mkOption {
          type = lib.types.attrsOf lib.types.anything;
          default = {};
        };
        options.home.file = lib.mkOption {
          type = lib.types.attrsOf lib.types.anything;
          default = {};
        };
        config._module.args.pkgs = pkgs;
        config.services.dracon.system.enable = true;
        config.services.dracon.sync.enable = true;
      }
    ];
  };
  guard = evaluated.config.systemd.user.services.dracon-system-guard.Service;
  sync = evaluated.config.systemd.user.services.dracon-sync.Service;
  guardUnit = evaluated.config.systemd.user.services.dracon-system-guard.Unit;
  syncUnit = evaluated.config.systemd.user.services.dracon-sync.Unit;
  guardInstall = evaluated.config.systemd.user.services.dracon-system-guard.Install;
  syncInstall = evaluated.config.systemd.user.services.dracon-sync.Install;
  services = evaluated.config.systemd.user.services;
  timers = evaluated.config.systemd.user.timers;
  files = evaluated.config.home.file;
  guardPaths = guard.ReadWritePaths;
  need = path: if !(builtins.elem path guardPaths) then throw "dracon-system-guard ReadWritePaths must contain ${path} (shipped-unit parity)" else null;
  needSync = name: value: if sync.${name} != value then throw "dracon-sync ${name} must be ${builtins.toString value} (shipped-unit parity)" else null;
  needGuard = name: value: if guard.${name} != value then throw "dracon-system-guard ${name} must be ${builtins.toString value} (shipped-unit parity)" else null;
  checks = [
    (if guard.PrivateTmp != false then throw "dracon-system-guard must share the host temporary namespace" else null)
    (if !(builtins.elem "/tmp" guardPaths) then throw "dracon-system-guard must explicitly permit host /tmp" else null)
    (if guard.WorkingDirectory != "%h" then throw "dracon-system-guard must anchor relative paths in the user home" else null)
    (if guard.Restart != "on-failure" then throw "dracon-system-guard must not restart after clean policy disablement" else null)
    (if guard.RestartPreventExitStatus != "2 78" then throw "dracon-system-guard must prevent usage and EX_CONFIG restarts" else null)
    # H1 parity: optional roots must be "-" prefixed or systemd refuses
    # to start when they are absent; required roots + ExecReload +
    # capabilities + sandbox must match the shipped unit.
    (need "%h/.dracon")
    (need "%h/.local/state/dracon")
    (need "%h/.local/share")
    (need "-%h/Dev")
    (need "-%h/.local/share/Trash")
    (need "-%h/.cargo")
    (need "-%h/.cache")
    (need "-%h/.npm")
    (need "-%h/.local/state/nix")
    (need "-/mnt/data/quarantine")
    (need "-/mnt/data/cold")
    (if builtins.match ".*/bin/sh -c .*kill -HUP.*" guard.ExecReload == null then throw "dracon-system-guard ExecReload must be the SIGHUP reload (shipped-unit parity)" else null)
    (needGuard "MemoryDenyWriteExecute" true)
    (needGuard "AmbientCapabilities" "CAP_SYS_NICE")
    (needGuard "CapabilityBoundingSet" "CAP_SYS_NICE")
    (needGuard "SystemCallFilter" "@system-service")
    (needGuard "SystemCallErrorNumber" "EPERM")
    (needGuard "RestrictNamespaces" true)
    (needGuard "PrivateDevices" true)
    # H2 parity: quota/restart/sandbox must match the shipped unit;
    # MemoryDenyWriteExecute must stay ABSENT (daemon honors JIT-based
    # pre-push hooks since the Oct-2026 audit fix).
    (needSync "Restart" "always")
    (needSync "CPUQuota" "100%")
    (needSync "MemoryMax" "2G")
    (needSync "SystemCallFilter" "@system-service")
    (needSync "SystemCallErrorNumber" "EPERM")
    (needSync "RestrictNamespaces" true)
    (needSync "NoNewPrivileges" true)
    (needSync "PrivateDevices" true)
    (if (sync ? MemoryDenyWriteExecute) then throw "dracon-sync must not set MemoryDenyWriteExecute (breaks JIT hook helpers)" else null)
    # M8: watchdog timers + their scripts must ship for Nix installs.
    (if !(timers ? dracon-sync-watchdog) then throw "Nix module must ship dracon-sync-watchdog.timer" else null)
    (if !(timers ? dracon-freeze-watchdog) then throw "Nix module must ship dracon-freeze-watchdog.timer" else null)
    (if !(timers ? dracon-system-guard-watchdog) then throw "Nix module must ship dracon-system-guard-watchdog.timer" else null)
    (if timers.dracon-sync-watchdog.Timer.OnUnitActiveSec != "2min" then throw "sync watchdog must fire every 2min" else null)
    (if timers.dracon-freeze-watchdog.Timer.OnUnitActiveSec != "2min" then throw "freeze watchdog must fire every 2min" else null)
    (if timers.dracon-system-guard-watchdog.Timer.OnUnitActiveSec != "2min" then throw "guard watchdog must fire every 2min" else null)
    (if timers.dracon-sync-watchdog.Install.WantedBy != [ "timers.target" ] then throw "sync watchdog must be wanted by timers.target" else null)
    (if timers.dracon-freeze-watchdog.Install.WantedBy != [ "timers.target" ] then throw "freeze watchdog must be wanted by timers.target" else null)
    (if timers.dracon-system-guard-watchdog.Install.WantedBy != [ "timers.target" ] then throw "guard watchdog must be wanted by timers.target" else null)
    (if files.".dracon/sync-notify/dracon-sync-watchdog.sh".executable != true then throw "sync watchdog script must be provisioned executable" else null)
    (if files.".dracon/sync-notify/dracon-freeze-watchdog.sh".executable != true then throw "freeze watchdog script must be provisioned executable" else null)
    (if files.".dracon/system-notify/dracon-system-guard-watchdog.sh".executable != true then throw "guard watchdog script must be provisioned executable" else null)
    # R3-H1: `.executable` alone passed while `.source` pointed into
    # git-filtered `${self}` (no utility source) — assert the pinned
    # inputs actually carry the scripts.
    (if !(builtins.pathExists files.".dracon/sync-notify/dracon-sync-watchdog.sh".source) then throw "sync watchdog script source must exist in the pinned input (R3-H1)" else null)
    (if !(builtins.pathExists files.".dracon/sync-notify/dracon-freeze-watchdog.sh".source) then throw "freeze watchdog script source must exist in the pinned input (R3-H1)" else null)
    (if !(builtins.pathExists files.".dracon/system-notify/dracon-system-guard-watchdog.sh".source) then throw "guard watchdog script source must exist in the pinned input (R3-H1)" else null)
    # R3-L25: every REMAINING shipped-unit property (the H1/H2 subset
    # above stays; this closes the gap so no property can re-drift
    # invisibly). ExecStart is store-path by design — suffix only.
    (needGuard "Type" "simple")
    (needGuard "StandardOutput" "journal")
    (needGuard "StandardError" "journal")
    (if builtins.length guard.Environment != 1 then throw "dracon-system-guard Environment must carry exactly PATH (shipped-unit parity)" else null)
    (if !(builtins.elem "PATH=%h/.local/bin:/run/wrappers/bin:%h/.local/share/flatpak/exports/bin:/var/lib/flatpak/exports/bin:%h/.nix-profile/bin:/nix/profile/bin:%h/.local/state/nix/profile/bin:/etc/profiles/per-user/%u/bin:/nix/var/nix/profiles/default/bin:/run/current-system/sw/bin" guard.Environment) then throw "dracon-system-guard PATH must match the shipped unit" else null)
    (if builtins.match ".*/bin/dracon-system guard daemon" guard.ExecStart == null then throw "dracon-system-guard ExecStart must end in /bin/dracon-system guard daemon" else null)
    (needGuard "RestartSec" "10")
    (needGuard "MemoryMax" "250M")
    (needGuard "CPUQuota" "20%")
    (needGuard "TasksMax" "64")
    (needGuard "NoNewPrivileges" true)
    (needGuard "ProtectSystem" "strict")
    (needGuard "ProtectHome" "read-only")
    (needGuard "ProtectKernelTunables" true)
    (needGuard "ProtectKernelLogs" true)
    (needGuard "ProtectClock" true)
    (needGuard "ProtectHostname" true)
    (needGuard "ProtectControlGroups" true)
    (needGuard "LockPersonality" true)
    (needGuard "RestrictRealtime" true)
    (needGuard "RestrictSUIDSGID" true)
    (needGuard "RemoveIPC" true)
    (if guardUnit.Description != "Dracon System Guard - Proactive disk space monitoring and cleanup" then throw "guard Unit Description drift (shipped-unit parity)" else null)
    (if guardUnit.Documentation != "https://github.com/DraconDev/dracon-utilities" then throw "guard Unit Documentation drift (shipped-unit parity)" else null)
    (if guardUnit.After != [ "network.target" ] then throw "guard Unit After must be [ network.target ] (shipped-unit parity)" else null)
    (if guardInstall.WantedBy != [ "default.target" ] then throw "guard Install WantedBy must be [ default.target ] (shipped-unit parity)" else null)
    (needSync "Type" "simple")
    (needSync "StandardOutput" "journal")
    (needSync "StandardError" "journal")
    (if builtins.length sync.Environment != 3 then throw "dracon-sync Environment must carry exactly PATH+POLICY+PROMPT (shipped-unit parity)" else null)
    (if !(builtins.elem "PATH=%h/.local/bin:/run/wrappers/bin:%h/.local/share/flatpak/exports/bin:/var/lib/flatpak/exports/bin:%h/.nix-profile/bin:/nix/profile/bin:%h/.local/state/nix/profile/bin:/etc/profiles/per-user/%u/bin:/nix/var/nix/profiles/default/bin:/run/current-system/sw/bin" sync.Environment) then throw "dracon-sync PATH must match the shipped unit" else null)
    (if !(builtins.elem "DRACON_SYNC_POLICY=%h/.dracon/utilities/sync/dracon-sync.toml" sync.Environment) then throw "dracon-sync DRACON_SYNC_POLICY default must match the shipped unit" else null)
    (if !(builtins.elem "GIT_TERMINAL_PROMPT=0" sync.Environment) then throw "dracon-sync GIT_TERMINAL_PROMPT must match the shipped unit" else null)
    (if sync.PassEnvironment != [ "SSH_AUTH_SOCK" ] then throw "dracon-sync PassEnvironment must be [ SSH_AUTH_SOCK ] (shipped-unit parity)" else null)
    (needSync "ExecStartPre" "-/run/current-system/sw/bin/pkill -x -f \"dracon-git pulse\"")
    (if builtins.match ".*/bin/dracon-sync daemon" sync.ExecStart == null then throw "dracon-sync ExecStart must end in /bin/dracon-sync daemon" else null)
    (needSync "RestartSec" "5")
    (needSync "RestartPreventExitStatus" "2 78")
    (needSync "Nice" "10")
    (needSync "MemoryHigh" "768M")
    (needSync "TasksMax" "96")
    (needSync "ProtectSystem" "strict")
    (needSync "ProtectHome" "read-only")
    (if builtins.length sync.ReadWritePaths != 4 then throw "dracon-sync ReadWritePaths must list exactly 4 roots (shipped-unit parity)" else null)
    (if !(builtins.elem "%h/.dracon" sync.ReadWritePaths) then throw "dracon-sync ReadWritePaths must contain %h/.dracon (shipped-unit parity)" else null)
    (if !(builtins.elem "%h/Dev" sync.ReadWritePaths) then throw "dracon-sync ReadWritePaths must contain %h/Dev (shipped-unit parity)" else null)
    (if !(builtins.elem "%h/.local/state/dracon" sync.ReadWritePaths) then throw "dracon-sync ReadWritePaths must contain %h/.local/state/dracon (shipped-unit parity)" else null)
    (if !(builtins.elem "%h/.ssh" sync.ReadWritePaths) then throw "dracon-sync ReadWritePaths must contain %h/.ssh (shipped-unit parity)" else null)
    (needSync "PrivateTmp" true)
    (needSync "ProtectKernelTunables" true)
    (needSync "ProtectKernelLogs" true)
    (needSync "ProtectClock" true)
    (needSync "ProtectHostname" true)
    (needSync "ProtectControlGroups" true)
    (needSync "LockPersonality" true)
    (needSync "RestrictRealtime" true)
    (needSync "RestrictSUIDSGID" true)
    (needSync "RemoveIPC" true)
    (needSync "CapabilityBoundingSet" "")
    (if syncUnit.Description != "Dracon Sync (deterministic sync runtime)" then throw "sync Unit Description drift (shipped-unit parity)" else null)
    (if syncUnit.Documentation != "https://github.com/DraconDev/dracon-utilities" then throw "sync Unit Documentation drift (shipped-unit parity)" else null)
    (if syncUnit.After != [ "default.target" ] then throw "sync Unit After must be [ default.target ] (shipped-unit parity)" else null)
    (if syncInstall.WantedBy != [ "default.target" ] then throw "sync Install WantedBy must be [ default.target ] (shipped-unit parity)" else null)
    # R3-L25: watchdog oneshot services + timer details (previously only
    # existence, OnUnitActiveSec, and timer WantedBy were asserted).
    (if services.dracon-sync-watchdog.Service.Type != "oneshot" then throw "sync watchdog must be Type=oneshot (shipped-unit parity)" else null)
    (if services.dracon-sync-watchdog.Service.ExecStart != "%h/.dracon/sync-notify/dracon-sync-watchdog.sh" then throw "sync watchdog ExecStart drift (shipped-unit parity)" else null)
    (if services.dracon-sync-watchdog.Service.TimeoutStartSec != "15" then throw "sync watchdog TimeoutStartSec must be 15 (shipped-unit parity)" else null)
    (if services.dracon-freeze-watchdog.Service.Type != "oneshot" then throw "freeze watchdog must be Type=oneshot (shipped-unit parity)" else null)
    (if services.dracon-freeze-watchdog.Service.ExecStart != "%h/.dracon/sync-notify/dracon-freeze-watchdog.sh" then throw "freeze watchdog ExecStart drift (shipped-unit parity)" else null)
    (if services.dracon-freeze-watchdog.Service.TimeoutStartSec != "10" then throw "freeze watchdog TimeoutStartSec must be 10 (shipped-unit parity)" else null)
    (if services.dracon-system-guard-watchdog.Service.Type != "oneshot" then throw "guard watchdog must be Type=oneshot (shipped-unit parity)" else null)
    (if services.dracon-system-guard-watchdog.Service.ExecStart != "%h/.dracon/system-notify/dracon-system-guard-watchdog.sh" then throw "guard watchdog ExecStart drift (shipped-unit parity)" else null)
    (if services.dracon-system-guard-watchdog.Service.TimeoutStartSec != "15" then throw "guard watchdog TimeoutStartSec must be 15 (shipped-unit parity)" else null)
    (if timers.dracon-sync-watchdog.Timer.OnBootSec != "2min" then throw "sync watchdog OnBootSec must be 2min (shipped-unit parity)" else null)
    (if timers.dracon-sync-watchdog.Timer.RandomizedDelaySec != "30" then throw "sync watchdog RandomizedDelaySec must be 30 (shipped-unit parity)" else null)
    (if timers.dracon-sync-watchdog.Timer.AccuracySec != "1s" then throw "sync watchdog AccuracySec must be 1s (shipped-unit parity)" else null)
    (if timers.dracon-freeze-watchdog.Timer.OnBootSec != "2min" then throw "freeze watchdog OnBootSec must be 2min (shipped-unit parity)" else null)
    (if timers.dracon-freeze-watchdog.Timer.RandomizedDelaySec != "15" then throw "freeze watchdog RandomizedDelaySec must be 15 (shipped-unit parity)" else null)
    (if timers.dracon-freeze-watchdog.Timer.AccuracySec != "1s" then throw "freeze watchdog AccuracySec must be 1s (shipped-unit parity)" else null)
    (if timers.dracon-system-guard-watchdog.Timer.OnBootSec != "2min" then throw "guard watchdog OnBootSec must be 2min (shipped-unit parity)" else null)
    (if timers.dracon-system-guard-watchdog.Timer.RandomizedDelaySec != "30" then throw "guard watchdog RandomizedDelaySec must be 30 (shipped-unit parity)" else null)
    (if timers.dracon-system-guard-watchdog.Timer.AccuracySec != "1s" then throw "guard watchdog AccuracySec must be 1s (shipped-unit parity)" else null)
    # R3-L25: watchdog Unit metadata + output routing (the flake omitted
    # Documentation/After until this finding; harmless metadata, now pinned).
    (if services.dracon-sync-watchdog.Unit.Documentation != "https://github.com/DraconDev/dracon-utilities" then throw "sync watchdog Unit Documentation drift (shipped-unit parity)" else null)
    (if services.dracon-sync-watchdog.Unit.After != [ "timers.target" ] then throw "sync watchdog Unit After drift (shipped-unit parity)" else null)
    (if services.dracon-sync-watchdog.Service.StandardOutput != "journal" then throw "sync watchdog StandardOutput drift (shipped-unit parity)" else null)
    (if services.dracon-sync-watchdog.Service.StandardError != "journal" then throw "sync watchdog StandardError drift (shipped-unit parity)" else null)
    (if timers.dracon-sync-watchdog.Unit.Documentation != "https://github.com/DraconDev/dracon-utilities" then throw "sync watchdog timer Documentation drift (shipped-unit parity)" else null)
    (if services.dracon-freeze-watchdog.Unit.Documentation != "https://github.com/DraconDev/dracon-utilities" then throw "freeze watchdog Unit Documentation drift (shipped-unit parity)" else null)
    (if services.dracon-freeze-watchdog.Unit.After != [ "timers.target" ] then throw "freeze watchdog Unit After drift (shipped-unit parity)" else null)
    (if services.dracon-freeze-watchdog.Service.StandardOutput != "journal" then throw "freeze watchdog StandardOutput drift (shipped-unit parity)" else null)
    (if services.dracon-freeze-watchdog.Service.StandardError != "journal" then throw "freeze watchdog StandardError drift (shipped-unit parity)" else null)
    (if timers.dracon-freeze-watchdog.Unit.Documentation != "https://github.com/DraconDev/dracon-utilities" then throw "freeze watchdog timer Documentation drift (shipped-unit parity)" else null)
    (if services.dracon-system-guard-watchdog.Unit.Documentation != "https://github.com/DraconDev/dracon-utilities" then throw "guard watchdog Unit Documentation drift (shipped-unit parity)" else null)
    (if services.dracon-system-guard-watchdog.Unit.After != [ "timers.target" ] then throw "guard watchdog Unit After drift (shipped-unit parity)" else null)
    (if services.dracon-system-guard-watchdog.Service.StandardOutput != "journal" then throw "guard watchdog StandardOutput drift (shipped-unit parity)" else null)
    (if services.dracon-system-guard-watchdog.Service.StandardError != "journal" then throw "guard watchdog StandardError drift (shipped-unit parity)" else null)
    (if timers.dracon-system-guard-watchdog.Unit.Documentation != "https://github.com/DraconDev/dracon-utilities" then throw "guard watchdog timer Documentation drift (shipped-unit parity)" else null)
  ];
in
  builtins.deepSeq checks "PASS: generated services match shipped-unit parity (all properties + watchdogs + script sources)"
' 2>&1)" || {
    printf '%s\n' "$service_check"
    exit 1
}
printf '%s\n' "$service_check"
