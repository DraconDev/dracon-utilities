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
# parity with their shipped units (audit H1/H2, 2026-10-02) — every
# security-relevant property below is asserted so Nix/HM drift fails CI.
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
        config._module.args.pkgs = pkgs;
        config.services.dracon.system.enable = true;
        config.services.dracon.sync.enable = true;
      }
    ];
  };
  guard = evaluated.config.systemd.user.services.dracon-system-guard.Service;
  sync = evaluated.config.systemd.user.services.dracon-sync.Service;
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
    (needGuard "ExecReload" "/bin/sh -c ''kill -HUP $MAINPID''")
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
  ];
in
  builtins.deepSeq checks "PASS: generated services match shipped-unit parity (guard cleanup/restart + H1/H2 sandbox/quota/restart)"
' 2>&1)" || {
    printf '%s\n' "$service_check"
    exit 1
}
printf '%s\n' "$service_check"
