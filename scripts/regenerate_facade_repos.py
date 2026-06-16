#!/usr/bin/env python3
"""Regenerate façade repos when the monorepo's source files change.

This is the auto-sync glue between `DraconDev/dracon-utilities` (the monorepo,
source of truth) and the 3 façade repos:

  - dracon-dev/dracon-sync-background-auto-commit-multi-remote
  - dracon-dev/dracon-system-di[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSByRXBPTHRhbjBxeHVkdXNlYjhWN3Nlbm1uQ1IwNndDcDFZQXJkb0lJSUFFCmdkemM2RU51V1l4MjkyR2prNnJtSnI0emFXcFlobDJKVUdGeWt4RENpZFkKLT4gWDI1NTE5IHo1cmpOSjk2eDVtcGRIb1IrNXJReHVFY3ZSWlhqdm5LbXpnRWFlQU5NUkUKSGVncnlWM2RGbm1vczQ5SkpDWE5JcXJLR1luQnlOaTNQWi9OVEx6Z0pYZwotPiBYMjU1MTkgandWMVU1c0U2WkNyNWN4WXpTOXVnMDBsZjlVY1d3UU5WcC91WmFRczVCSQoyYmZQb3k1bklZTWFyeVpmcGpHZ3Y4YUpPcWZPRTNCZUpBbUR3SzhDanlZCi0+IFgyNTUxOSB4OTZiNTRObStVY3VKMHB5SkI5dmdXVUpqN2NjZlB6VHJHbm1vbWdDaEJrCnY5Ykg3aXkzMEZnWE5YdnIvS2d3NFBya3VQVEtHZi9uQXRFeXJnTVptWkUKLT4gWDI1NTE5IHdpdFN5QUdGeHl1emRMVjdPREhIaHdNVW14WGQ4SjlPdE4yU2RQRWNkU2sKN0ZnTThhMnpMelBUaExOVzJMK3JXUnFOdGdzUTZHS2hwQWFudTlrQWI4MAotPiB0aistZ3JlYXNlIEMgN19eUFQgKj8KNEZTbkN0bXlzZwotLS0gaDRHTWtaN05DbGdHN2krbTRCMGZEdmZZQnhxVzJUd1YzMDRGbVYxQzF2MArGgsaqR4TznOIwJzJdcZ7IO88kk4KHyZg4/K58ZQgMkCYE+EePs4xfAttR4vr6ajwZGP4HWgXB]
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
    # Resolve the canonical long name from the scaffold script's UTILITIES
    import importlib.util
    spec_path = monorepo_root / "scripts" / "scaffold_feature_repos.py"
    spec = importlib.util.spec_from_file_location("scaffold_feature_repos", spec_path)
    if spec is None or spec.loader is None:
        print(f"  [{utility}] cannot import scaffold_feature_repos.py", file=sys.stderr)
        return False
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    long_name = mod.UTILITIES[utility]["name"]
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
        # Resolve the long names for the dry-run output
        import importlib.util
        spec_path = monorepo_root / "scripts" / "scaffold_feature_repos.py"
        spec = importlib.util.spec_from_file_location("scaffold_feature_repos", spec_path)
        if spec is not None and spec.loader is not None:
            mod = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(mod)
            for u in affected:
                long_name = mod.UTILITIES[u]["name"]
                print(f"  [dry-run] would regenerate {u} → {target_root / long_name}")
        else:
            for u in affected:
                print(f"  [dry-run] would regenerate {u} → {target_root / u}")
        return 0

    success = True
    for utility in affected:
        if not _regenerate_one(monorepo_root, target_root, utility):
            success = False
    return 0 if success else 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
