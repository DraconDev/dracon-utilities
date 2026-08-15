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
    printf '%s\n' "$output" | grep '^warning:' | grep -v "unknown flake output 'homeManagerModules'" || true
} | sed '/^warning: The check omitted these incompatible systems:/d' | sed "/^Use '--all-systems' to check all\./d")"

if [[ -n "$unexpected" ]]; then
    printf '%s\n' "$output"
    printf 'FAIL: unexpected Nix flake warning(s):\n%s\n' "$unexpected" >&2
    exit 1
fi

printf '%s\n' "$output" \
    | sed "/^warning: unknown flake output 'homeManagerModules'$/d" \
    | sed '/^warning: The check omitted these incompatible systems:$/d' \
    | sed "/^Use '--all-systems' to check all\.$/d"
echo "PASS: Nix flake checks passed (Home Manager output warning is intentional and documented)."
