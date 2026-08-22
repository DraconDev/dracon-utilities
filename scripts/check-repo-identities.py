#!/usr/bin/env python3
"""Verify every watched repository's git identity is intentional.

Prevention follow-up to the 2026-08-22 finding where ~/Dev/dracon-platform
had a LOCAL git config override (`dracon <dracon@local>` — the machine
bootstrap default) shadowing the global DraconDev identity, so ~1000/day
daemon auto-commits were attributed to an identity no forge links to the
operator. This check makes such drift surface within 24h instead of
silently rotting.

Acceptable effective identities per repo:
  1. canonical operator:  DraconDev <dracsharp@gmail.com>
  2. deliberate loop identity: <repo-basename>-dev <basename>@dracon.local
     (game loops, per AGENTS.md "deliberate identity" policy)

Anything else — especially the bootstrap defaults `dracon@local` /
`dracon@localhost` — is a failure. Exits non-zero listing offenders;
journalctl --user -u dracon-nested-pins-check.service shows details.
"""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

HOME = Path.home()
DEV = HOME / "Dev"
PLATFORM_GAMES = DEV / "dracon-platform" / "web" / "games"

CANONICAL = ("DraconDev", "dracsharp@gmail.com")
BOOTSTRAP_EMAILS = {"dracon@local", "dracon@localhost", ""}


def git(repo: Path, *args: str) -> str:
    return subprocess.run(
        ["git", "-C", str(repo), *args],
        capture_output=True,
        text=True,
        check=False,
    ).stdout.strip()


def discover_repos() -> list[Path]:
    repos: set[Path] = set()
    for d in sorted(DEV.iterdir()):
        if (d / ".git").exists():
            repos.add(d)
    if PLATFORM_GAMES.is_dir():
        for env in sorted(PLATFORM_GAMES.iterdir()):
            if not env.is_dir():
                continue            for game in sorted(env.iterdir()):
                if game.is_dir() and (game / ".git").exists():
                    repos.add(game)
    return sorted(repos)


def expected_loop_identity(repo: Path) -> tuple[str, str]:
    base = repo.name.removesuffix(".git")
    return (f"{base}-dev", f"{base}@dracon.local")


def main() -> int:
    failures: list[str] = []
    checked = 0
    for repo in discover_repos():
        checked += 1
        name = git(repo, "config", "user.name")
        email = git(repo, "config", "user.email")
        ok = (name, email) in {
            CANONICAL,
            expected_loop_identity(repo),
            # meta-repo convention: bare org dir uses the canonical pair too
        }
        if email in BOOTSTRAP_EMAILS or not ok:
            failures.append(f"{repo}: {name!r} <{email!r}>")

    # Global default must be the canonical operator identity.
    gname = subprocess.run(
        ["git", "config", "--global", "user.name"], capture_output=True, text=True, check=False
    ).stdout.strip()
    gemail = subprocess.run(
        ["git", "config", "--global", "user.email"], capture_output=True, text=True, check=False
    ).stdout.strip()
    if (gname, gemail) != CANONICAL:
        failures.append(f"GLOBAL: {gname!r} <{gemail!r}> != {CANONICAL}")

    print(f"identity check: {checked} repos scanned")
    if failures:
        print("IDENTITY DRIFT DETECTED:")
        for f in failures:
            print(f"  ✗ {f}")
        return 1
    print("PASS: all repo identities are canonical or deliberate loop identities")
    return 0


if __name__ == "__main__":
    sys.exit(main())
