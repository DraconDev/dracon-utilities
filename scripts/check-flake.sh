#!/usr/bin/env bash
# Validate the flake while allowing the standard Home Manager module output.
# Nix's generic flake checker does not know the Home Manager convention
# `homeManagerModules`, although Home Manager consumes it correctly.

set -euo pipefail

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
# minimal test harness.  This checks the generated service, not just the
# standalone unit shipped beside dracon-system.
service_check="$(nix eval --impure --raw --expr '
let
  flake = builtins.getFlake (toString ./.);
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
        config.services.dracon.sync.enable = false;
      }
    ];
  };
  service = evaluated.config.systemd.user.services.dracon-system-guard.Service;
in
  if service.PrivateTmp != false then
    throw "dracon-system-guard must share the host temporary namespace"
  else if !(builtins.elem "/tmp" service.ReadWritePaths) then
    throw "dracon-system-guard must explicitly permit host /tmp"
  else if service.WorkingDirectory != "%h" then
    throw "dracon-system-guard must anchor relative paths in the user home"
  else if service.Restart != "on-failure" then
    throw "dracon-system-guard must not restart after clean policy disablement"
  else if service.RestartPreventExitStatus != "2 78" then
    throw "dracon-system-guard must prevent usage and EX_CONFIG restarts"
  else
    "PASS: generated dracon-system-guard exposes safe cleanup/restart policy"
' 2>&1)" || {
    printf '%s\n' "$service_check"
    exit 1
}
printf '%s\n' "$service_check"
