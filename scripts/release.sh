#!/usr/bin/env bash
# Dispatch release work to one of the three standalone utility repositories.
#
# The parent repository is meta-only. It intentionally has no coordinated
# package version, changelog, or publish transaction, so a root-level release
# script must never try to bump or publish all utilities together.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(git -C "$SCRIPT_DIR" rev-parse --show-toplevel)"

usage() {
    cat <<'EOF'
Usage: scripts/release.sh <utility> <version> [options]

Dispatch to the standalone release pipeline for one utility:

  scripts/release.sh dracon-sync   <version> [options]
  scripts/release.sh dracon-system <version> [options]
  scripts/release.sh dracon-warden <version> [options]

Run the selected repository's scripts/release.sh --help for its options.
The parent meta workspace has no coordinated release transaction.
EOF
}

if [[ $# -eq 0 || "$1" == "--help" || "$1" == "-h" ]]; then
    usage
    exit 0
fi

utility="$1"
shift

case "$utility" in
    dracon-sync|dracon-system|dracon-warden)
        release_script="$REPO_ROOT/$utility/scripts/release.sh"
        if [[ ! -x "$release_script" ]]; then
            echo "error: missing executable release pipeline: $release_script" >&2
            exit 1
        fi
        exec "$release_script" "$@"
        ;;
    *)
        echo "error: '$utility' is not a standalone utility repository" >&2
        echo "choose dracon-sync, dracon-system, or dracon-warden" >&2
        echo "run scripts/release.sh --help for usage" >&2
        exit 2
        ;;
esac
