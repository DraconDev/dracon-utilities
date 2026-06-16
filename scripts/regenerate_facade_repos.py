#!/usr/bin/env python3
"""Regenerate façade repos when the monorepo's source files change.

This is the auto-sync glue between `DraconDev/dracon-utilities` (the monorepo,
source of truth) and the 3 façade repos:

  - dracon-dev/dracon-sync-background-auto-commit-multi-remote
  - dracon-dev/dracon-system-di[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSA3bmJFcjZyY2VUWHc2WFZVd002RXJXSFhQZWhhQWgvWDNRTURFVlB3V1FFCkJ6VDk3V0YrQzVDTWYyTU8yY0piK3VaUmMwcEhaMDNhMmJuZjF1MVd1NTAKLT4gWDI1NTE5IDdvWGMzR1dkbTdCcHZDU2dmb2JSaVZTeDJpbC9aUnltRHRXNCtzMkFOazgKMklUNnJ2aEhJOWd0RHlNU1ZGOEprenhqS1lrVUxDWFpzR0EwTTJlQjlLNAotPiBYMjU1MTkgM3dWTmVBWnpiL0VzaVRZUldWMVBkV0Ywb0VoS3VxSnNIYVNhRGtyTGxIYwpIVHovRWlLRXgvVjluYU1VajhnWkxDbC9SQjZ0dTFlVGg3RTFNMzdOTFFNCi0+IFgyNTUxOSBwSEY0eHdQNEdFQUFrdCs0NVRsT0lBWi82T0NPdEc1Y21hVkFzQjZHeGtVCjUyRTg4a1pUWDZjSEdxR1hyVldIcVQ4OFZJdFNUUVBCYU1JMWFOYVl6R0UKLT4gWDI1NTE5IG1HV1c3S25obXlmZmdTSDdNd0hzU3hoaFhJNnl4aElRcGM4eGRnU0h6aTgKRHFSUXJzOU1wNTc4UUhJWk1HNTdPWC9CTEtiSUF0Z1FSa1A2QllzajRhbwotPiBcLU4/flBmLWdyZWFzZSB9elJPTzYgP2EsSFokbgpRMUlIekZKRGEvVHJHN1VlOXd5czAzY0NjRzhMeUlWUHV5ZHJrVDY1NEliUnhsa2toVmJOQUw4V3lqTDJ4anBiCmVEWUpxb29kTjJYUUxzZFhNaENHaHM4VzVxK25HN0JwbTR4dVloNnFSaWdTOGsrTgotLS0gbDJZOUZkREdPQ08yMVk0ZStTQk5TNzFvQ29uREhZT0F3OWdEbkFVQkdUQQrwNM4IK0dS2jxNMSxA62UyCBz5yVxFrT8sPVRqJsHWGzGgedj/JTAgUM+7znQnH73hvQMl5fIk]
  - dracon-dev/dracon-warden-secret-encrypt-age-git-filter

Each façade repo lives at `<target_root>/<name>` and is watched by
`dracon-sync` (the daemon). When the monorepo commits a change to a utility's
source files (e.g. `dracon-sync/README.md`), this script regenerates the
corresponding façade's `README.md` (and other scaffold files) by re-running
`scripts/scaffold_feature_repos.py` for that utility. The daemon then picks up
the local change in the façade repo's working tree and auto-commits + auto-pushes
it to the 3 remotes (github, gitlab, codeberg) per the standard sync policy.

Designed to be called from a monorepo `post-commit` hook:

    #!/bin/sh
    # .git/hooks/post-commit in dracon-utilities
    exec python3 "$(git rev-parse --show-toplevel)/scripts/regenerate_facade_repos.py"

The script is a no-op if the commit did not touch any utility's source files.
This is a fast check: it diffs the last commit's changed files against the
known utility subdirectory prefixes.

See `docs/design/github-feature-repos.md` for the full design.
"""

from __future__ import annotations

import argparse
import os
import shutil
import subprocess
import sys
from pathlib import Path

#: Utility short name → monorepo subdirectory that triggers regeneration.
UTILITY_SOURCE_DIRS: dict[str, str] = {
    "dracon-sync": "dracon-sync",
    "dracon-system": "dracon-system",
    "dracon-warden": "dracon-warden",
}

#: Canonical long name per utility (mirrors UTILITIES in scaffold_feature_repos.py).
#: Kept in sync manually; the scaffold script is the source of truth at runtime
#: (this script invokes the scaffold for the actual regeneration, and the
#: directory name is whatever the scaffold uses for the target).
UTILITY_LONG_NAMES: dict[str, str] = {
    "dracon-sync": "dracon-sync-background-auto-commit-multi-remote",
    "dracon-system": "dracon-system-di[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBqU1JVckd2U2I5WC9NaTlzcUFwRTR1ZHYvOFVCUlVRdGN1cDB5aDRic0dVCjhRclRXTnJJdVVNU0dxcHZHamVsOUhzNE1kZUFBN25QOFp2UEZ2R010ek0KLT4gWDI1NTE5IFZWUWk4aGFYc2cwRjFqTUFqY1dqbHlUR3Z4dWNEdmc2R3ZHaTdzSEhaMm8KZFZWbXZZQ1pzRUcySUVHYzBSWnovKzQ3K3BJQ2NrcXY5TTg3V25WVEJ1awotPiBYMjU1MTkgaVZsdnY1eE1mMEdmdmE5dnQzTU9CYkkwNy90YWEzSUpGZnhCdm50emtuSQo0dFhwcWdrTUZPQkNESlNNeTJOZzRlYzhXT1U0a2JLN0VVbEhpUEtrK0g0Ci0+IFgyNTUxOSBMM2JZRmJodWErNGpHdXZXeGl4UEVJcEswMU14MzcwMWM5a2tMd1o0aWc0CkhVN0pHQmc5TTlBcjF3dnlpdVp2eDVhSFhHSkFIWlNvSmN0S2pEakxUNFUKLT4gWDI1NTE5IENuQUFYSXpCNUErV3g4eHFMUE1OcndtMkZNRzJtamxGaEI5MXcvY1NtRkkKYWxoRjQ2VW1rRXVwS2VVV09MWGNhc3hPOVlEdHp1MmtPTSs0K3FrNWh3bwotPiBocEQtZ3JlYXNlIDpFQ0o+QXtqIHdbIGFeU2kgZQpPb1lNL0JrTGpDQUpCSnVOaXcKLS0tIFNrUjlkLzNob0FXNkhGU2c2ZUdrdzZhY3pPZGJIb2Z6VWw0NVhuVXpLeWMK3UOn+qWXlEKTANdvU8h4Zpz7IAh+BdnPaigvKuETk9JhNQEGOImf6u6JZ0vSiQqEB2u65lD/Xw==]",
    "dracon-warden": "dracon-warden-secret-encrypt-age-git-filter",
}

#: Default target root for the façade repo clones. The daemon's `watch_roots`
#: includes `/home/dracon/Dev`, so this path is auto-synced.
DEFAULT_TARGET_ROOT = "/home/dracon/Dev/facade-repos"


def _changed_files_in_last_commit(monorepo_root: Path) -> list[str]:
    """Return the list of files changed in HEAD (vs HEAD~1)."""
    result = subprocess.run(
        ["git", "diff-tree", "--no-commit-id", "--name-only", "-r", "HEAD"],
        cwd=str(monorepo_root),
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode != 0:
        # HEAD~1 might not exist (initial commit); fall back to show --name-only
        result = subprocess.run(
            ["git", "show", "--name-only", "--format=", "HEAD"],
            cwd=str(monorepo_root),
            capture_output=True,
            text=True,
            check=False,
        )
    return [line.strip() for line in result.stdout.splitlines() if line.strip()]


def _affected_utilities(changed_files: list[str]) -> list[str]:
    """Return the list of utility short names whose source files changed."""
    affected: set[str] = set()
    for path in changed_files:
        # Normalize: strip leading `./` if present
        normalized = path.lstrip("./")
        first_segment = normalized.split("/", 1)[0]
        for short, subdir in UTILITY_SOURCE_DIRS.items():
            if first_segment == subdir:
                affected.add(short)
    return sorted(affected)


def _regenerate_one(
    monorepo_root: Path,
    target_root: Path,
    utility: str,
) -> bool:
    """Regenerate one façade repo. Returns True if a change was committed."""
    long_name = UTILITY_LONG_NAMES[utility]
    facade_dir = target_root / long_name
    scaffold = monorepo_root / "scripts" / "scaffold_feature_repos.py"

    if not facade_dir.is_dir():
        print(
            f"  [{utility}] facade clone not found at {facade_dir}; "
            f"run 'scaffold_feature_repos.py --init-git --target-root {target_root} --repo {utility}' first",
            file=sys.stderr,
        )
        return False
    if not (facade_dir / ".git").is_dir():
        print(
            f"  [{utility}] {facade_dir} is not a git repo; "
            f"skip (use --init-git to initialize)",
            file=sys.stderr,
        )
        return False

    # Run the scaffold script with --apply (writes files into the existing
    # clone). --init-git is NOT passed because the clone is already a repo.
    result = subprocess.run(
        [
            sys.executable,
            str(scaffold),
            "--apply",
            "--monorepo-root",
            str(monorepo_root),
            "--target-root",
            str(target_root),
            "--repo",
            utility,
        ],
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode != 0:
        print(
            f"  [{utility}] scaffold script failed: {result.stderr}",
            file=sys.stderr,
        )
        return False

    # Check if there are any changes to commit
    status = subprocess.run(
        ["git", "status", "--porcelain"],
        cwd=str(facade_dir),
        capture_output=True,
        text=True,
        check=False,
    )
    if not status.stdout.strip():
        print(f"  [{utility}] no changes (already up to date)")
        return False

    # Commit (do not push; the daemon handles push per policy)
    commit_msg = subprocess.run(
        ["git", "log", "-1", "--format=%s", "HEAD"],
        cwd=str(monorepo_root),
        capture_output=True,
        text=True,
        check=False,
    ).stdout.strip() or "refresh façade from monorepo"

    subprocess.run(
        ["git", "add", "-A"],
        cwd=str(facade_dir),
        check=True,
    )
    subprocess.run(
        [
            "git",
            "-c",
            "user.email=dracsharp@gmail.com",
            "-c",
            "user.name=DraconDev",
            "commit",
            "--no-verify",
            "-m",
            f"docs: refresh façade from monorepo\n\nSync from dracon-utilities: {commit_msg}",
        ],
        cwd=str(facade_dir),
        check=True,
    )
    print(f"  [{utility}] committed refreshed content (daemon will push)")
    return True


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(
        description=(
            "Regenerate façade repos for the utilities whose source files "
            "changed in the last monorepo commit. The daemon handles the push."
        )
    )
    parser.add_argument(
        "--monorepo-root",
        type=Path,
        default=Path(os.environ.get("DRACON_MONOREPO_ROOT", "/home/dracon/Dev/dracon-utilities")),
        help="Path to the dracon-utilities monorepo root.",
    )
    parser.add_argument(
        "--target-root",
        type=Path,
        default=Path(os.environ.get("DRACON_FACADE_ROOT", DEFAULT_TARGET_ROOT)),
        help=(
            "Directory containing the 3 façade repo clones. "
            "Must be under a daemon watch_root for auto-push to work."
        ),
    )
    parser.add_argument(
        "--all",
        action="store_true",
        help="Regenerate all 3 utilities regardless of what changed in the last commit.",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Print what would be regenerated without writing files or committing.",
    )
    args = parser.parse_args(argv)

    monorepo_root = args.monorepo_root.resolve()
    target_root = args.target_root.resolve()

    if not (monorepo_root / "scripts" / "scaffold_feature_repos.py").is_file():
        print(
            f"scaffold_feature_repos.py not found at {monorepo_root}/scripts/",
            file=sys.stderr,
        )
        return 1

    if args.all:
        affected = list(UTILITY_SOURCE_DIRS.keys())
        print(f"[regenerate_facade_repos] --all: regenerating all {len(affected)} utilities")
    else:
        changed = _changed_files_in_last_commit(monorepo_root)
        affected = _affected_utilities(changed)
        if not affected:
            print(
                f"[regenerate_facade_repos] no utility source files changed in HEAD; nothing to do"
            )
            return 0
        print(
            f"[regenerate_facade_repos] changed files in HEAD touch {len(affected)} utility/ies: "
            f"{', '.join(affected)}"
        )

    if args.dry_run:
        for u in affected:
            long_name = UTILITY_LONG_NAMES[u]
            print(f"  [dry-run] would regenerate {u} → {target_root / long_name}")
        return 0

    success = True
    for utility in affected:
        if not _regenerate_one(monorepo_root, target_root, utility):
            success = False
    return 0 if success else 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
