#!/usr/bin/env python3
"""Scaffold feature-façade repositories for Dracon utilities.

The Dracon utilities source of truth remains the `dracon-utilities` monorepo.
These façade repos are intentionally small presentation surfaces (GitHub,
GitLab, Codeberg) for `dracon-sync`, `dracon-system`, and `dracon-warden`. They
avoid duplicating implementation code and instead point users to the canonical
monorepo paths.

Brutally-descriptive names are used so the project is self-explanatory on
Codeberg/Forgejo where descriptive names surface well in search and discovery.
The short names (`dracon-sync`, `dracon-system`, `dracon-warden`) are kept as
backwards-compatible aliases — the GitHub façades still respond under the short
names for now, but the canonical name on every remote is the descriptive one.

See `docs/design/github-feature-repos.md` for the full specification.
"""

from __future__ import annotations

import argparse
import json
import re
import shutil
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path

#: Words the operator has explicitly excluded from façade names.
FILLER_WORDS = frozenset({
    "the", "for", "in", "and", "with", "of",
    "workspace", "infrastructure", "tool", "utility", "framework",
    "development", "coding", "platform", "software", "app",
    "ai",
})

#: Primary feature keyword that MUST appear in the descriptive name.
PRIMARY_KEYWORDS = {
    "dracon-sync": ("sync", "watch", "commit", "push", "mirror"),
    "dracon-system": ("system", "disk", "process", "guard", "doctor"),
    "dracon-warden": ("warden", "age", "secret", "encrypt"),
}

UTILITIES: dict[str, dict[str, str]] = {
    "dracon-sync": {
        # Short alias kept for backwards compatibility
        "short": "dracon-sync",
        # Brutally-descriptive canonical name (no filler, no "ai")
        "name": "dracon-sync-background-auto-commit-multi-remote",
        "title": "Dracon Sync",
        "description": (
            "Background, auto-commit, multi-remote — invisible git sync "
            "for developer workspaces."
        ),
        "subdir": "dracon-sync",
        "service": "dracon-sync.service",
        "config": "dracon-sync/dracon-sync.example.toml",
        "commands": (
            "dracon-sync status · dracon-sync repos · dracon-sync health · "
            "dracon-sync daemon"
        ),
        "focus": (
            "Watches configured repositories, waits for changes to settle "
            "(fingerprint stability / debounce), commits deterministic "
            "diff-based messages, and pushes to origin plus configured "
            "mirrors. Invisible: runs in the background, no user interaction "
            "required."
        ),
        "keywords": ("background", "auto-commit", "multi-remote"),
    },
    "dracon-system": {
        "short": "dracon-system",
        "name": "dracon-system-di[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBJREJWTjk2N0VLZUs2WlVxcXA2aEQ5cWloaUtQbnZmZGRnd3FXNEhodGhBCitHRVVxTGJ4YkQ0UnVlQkhPTkdSUndvb1BDckZCQURUeTlKSWZmZ3VVa28KLT4gWDI1NTE5IHFBMGRHaGhnOCs2VE5zQVgyZUxKWEl1VlZXNnJ2cEoyTUUzZ2dySzI5ejAKRzhYVnZVL25JeGhSb0ovN1pZRllZVXp4RncrajlzL1FKUjJoc1loL0dSOAotPiBYMjU1MTkgenlINUpNc3JlSXFQMmp1WXg4RFdrdHRiNitRVnlVRmtuNG9EVmdROHAzSQpaY2tPcnROdDFsaGh3emg0N251R0FEVm5WSE1XbDVNeE9GWVZLdlp4N2FnCi0+IFgyNTUxOSBYLzh6cjBTaHE5UmRXa0Z3aGcyUHZTb3lqYkhmNkQ3Q1NDZE5vSXFvdFRnClp1WlJMZmhqaU9pZVB0SVJoTXlySmhVQ0YwU3l4QUtqOG9SNHJrZnNYUjgKLT4gWDI1NTE5IEsydDVjQmF6NEwxcVRQWW5Bb3BMRzJWYUEzSmN6ajBqWHQvYzNTU0x2aEEKdU5Ta0paL1JrNytVZmFkT3BleTlac0JBWlR0VEhWdFc2MDJNU2ZFZWFnQQotPiB7V19mIi1ncmVhc2UgYGdpY1pwSCBHagpEYTdxNHFreUdlYlhKZHZpR0RCeWYwU3pZZE9CCi0tLSBaUm5jWU9kZ1NpUnI2cFpSNThNZmhwZjZWaFh2ZFVGMjNKZmd2N3phcGljCjAsV99lwF5EMj8om8gNvwKV9E4eXUmVhdtZjVXZqEa/JvCkYwwbtVrUx8TA0mJSf/I69dkdqL4=]",
        "title": "Dracon System",
        "description": (
            "Disk, process, guard, doctor — local machine diagnostics and "
            "watchdog for Dracon workspaces."
        ),
        "subdir": "dracon-system",
        "service": "dracon-system-guard.service",
        "config": "dracon-system/dracon-system.example.toml",
        "commands": (
            "dracon-system status · dracon-system doctor · "
            "dracon-system storage · dracon-system guard daemon"
        ),
        "focus": (
            "Protects machines from disk/process pressure and provides "
            "deterministic diagnostics for storage, links, zram, events, "
            "and the guard daemon."
        ),
        "keywords": ("disk", "process", "guard", "doctor"),
    },
    "dracon-warden": {
        "short": "dracon-warden",
        "name": "dracon-warden-secret-encrypt-age-git-filter",
        "title": "Dracon Warden",
        "description": (
            "Secret, encrypt, age, git-filter — repository hardening and "
            "smudge/clean encryption for Dracon workspaces."
        ),
        "subdir": "dracon-warden",
        "service": "No systemd service; enforced through global git hooks.",
        "config": "dracon-warden/dracon-warden.example.toml",
        "commands": (
            "dracon-warden status · dracon-warden keygen · "
            "dracon-warden setup-hooks --global · "
            "dracon-warden scrub-markers"
        ),
        "focus": (
            "Encrypts secret-shaped content at rest in git while preserving "
            "normal plaintext files in the working tree. Uses age encryption "
            "and git smudge/clean filters plus a pre-commit hook for "
            "plaintext-secret prevention."
        ),
        "keywords": ("secret", "encrypt", "age", "git-filter"),
    },
}


@dataclass(frozen=True)
class RepoSpec:
    name: str
    short: str
    title: str
    description: str
    subdir: str
    service: str
    config: str
    commands: str
    focus: str
    keywords: tuple[str, ...]


def specs() -> list[RepoSpec]:
    return [
        RepoSpec(
            name=data["name"],
            short=data["short"],
            title=data["title"],
            description=data["description"],
            subdir=data["subdir"],
            service=data["service"],
            config=data["config"],
            commands=data["commands"],
            focus=data["focus"],
            keywords=data["keywords"],
        )
        for data in UTILITIES.values()
    ]


def _validate_name(name: str, spec_short: str) -> list[str]:
    """Return a list of constraint violations; empty if all constraints pass."""
    errors: list[str] = []
    if not name.startswith("dracon-"):
        errors.append(f"name must start with 'dracon-': {name!r}")
    if not re.match(r"^[a-z0-9-]+$", name):
        errors.append(f"name must be lowercase letters/digits/hyphens: {name!r}")
    if len(name) < 30:
        errors.append(f"name must be at least 30 chars: {name!r} (len={len(name)})")
    if len(name) > 60:
        errors.append(f"name must be at most 60 chars: {name!r} (len={len(name)})")
    # No filler words
    tokens = name.split("-")
    for token in tokens:
        if token.lower() in FILLER_WORDS:
            errors.append(
                f"filler word in name: {token!r} is in the excluded set: "
                f"{sorted(FILLER_WORDS)}"
            )
    # At least one primary keyword from this utility must appear
    primary_set = set(k.lower() for k in PRIMARY_KEYWORDS.get(spec_short, ()))
    name_tokens = set(t.lower() for t in tokens)
    if not (name_tokens & primary_set):
        errors.append(
            f"name must contain at least one of the primary keywords "
            f"{sorted(primary_set)}: {name!r}"
        )
    return errors


def repo_readme(spec: RepoSpec) -> str:
    return f"""# {spec.title}

{spec.description}

This repository is a feature façade for `{spec.short}`. It does **not**
duplicate the implementation code. The canonical source of truth remains the
[`DraconDev/dracon-utilities`](https://github.com/DraconDev/dracon-utilities)
monorepo, with this utility's code and docs under:

- Source: [`{spec.subdir}/`](https://github.com/DraconDev/dracon-utilities/tree/main/{spec.subdir})
- User guide: [`{spec.subdir}/README.md`](https://github.com/DraconDev/dracon-utilities/tree/main/{spec.subdir}/README.md)
- Design notes: [`{spec.subdir}/BLUEPRINT.md`](https://github.com/DraconDev/dracon-utilities/tree/main/{spec.subdir}/BLUEPRINT.md)
- Example config: [`{spec.config}`](https://github.com/DraconDev/dracon-utilities/tree/main/{spec.config})

## Why this name?

The descriptive name is a deliberate choice for Codeberg/Forgejo, where
descriptive repo names get upvotes and free attention because readers
immediately know what the project does. The full word list (no fillers, no
audience/UX claims) is documented in
[`docs/design/github-feature-repos.md`](https://github.com/DraconDev/dracon-utilities/blob/main/docs/design/github-feature-repos.md).

## Purpose

{spec.focus}

Use this repo to feature the utility on GitHub, GitLab, and Codeberg without
splitting the actual implementation out of the monorepo. Issues, project
boards, and roadmap notes can live here, while commits, releases, tests, and
packaging stay anchored in `dracon-utilities`.

## Runtime

- Binary: `{spec.short}`
- Service: {spec.service}
- Example policy: `{spec.config}`
- Common commands: `{spec.commands}`

## Relationship to the monorepo

| Boundary | Decision |
|----------|----------|
| Source code | Lives in `dracon-utilities/{spec.subdir}` |
| Release artifacts | Built and published from `dracon-utilities` |
| Feature surface | This façade repo (and short-name alias) |
| Operational policy | `~/.dracon/utilities/` TOML files |
| Shared libraries | Sibling `dracon-libs` workspace where applicable |

## Maintenance

When the monorepo changes the utility README, blueprint, or example config,
regenerate this façade with:

```bash
cd /path/to/dracon-utilities
./scripts/scaffold_feature_repos.py --apply --repo {spec.short}
./scripts/scaffold_feature_repos.py --push-all-remotes --repo {spec.short} \\
    --ssh-target /path/to/{spec.name}
```

Do not paste implementation code into this façade repo. Keep it as a stable
navigation and feature surface so the monorepo remains the single source of
truth.

## License

AGPL-3.0-only — see [LICENSE](LICENSE).
"""


def issue_template() -> str:
    return """---
name: Feature or problem report
about: Track utility-specific work without duplicating the monorepo source.
title: ""
labels: ""
assignees: ""
---

## Summary

<!-- Keep implementation details linked to DraconDev/dracon-utilities when possible. -->

## Utility

<!-- dracon-sync / dracon-system / dracon-warden -->

## Expected behavior

## Actual behavior

## Links

- Monorepo path:
- Related issue/commit/release:

"""


def codeowners() -> str:
    return "@DraconDev\n"


def gitignore() -> str:
    return """target/
node_modules/
*.swp
.DS_Store
"""


def source_of_truth() -> str:
    return """# GitHub / GitLab / Codeberg Feature Façade Repositories

Dracon utility façade repositories are intentionally small presentation
surfaces. They make `dracon-sync`, `dracon-system`, and `dracon-warden` easier
to feature on GitHub, GitLab, and Codeberg without splitting the
implementation out of the `DraconDev/dracon-utilities` monorepo.

## Invariants

1. The monorepo is the only source of truth for implementation code, tests,
   release packaging, and changelog entries.
2. Façade repos contain only navigation, issue/project metadata, licenses,
   and links back to the monorepo paths.
3. Do not copy implementation files into façade repos. If code needs a public
   home, create a real separate crate/binary repo and update the monorepo
   architecture docs first.
4. Regenerate façade repos with `scripts/scaffold_feature_repos.py --apply`
   so the presentation layer stays consistent.

## Why this is not a hack

GitHub, GitLab, and Codeberg cannot natively present a subdirectory as a
first-class repository with separate issues, projects, topics, and README
without duplicating or moving files. A façade repo avoids both bad options:

- Moving code would split the implementation and break the current release
  pipeline.
- Copying code would create drift and duplicate maintenance.

The façade repo is therefore a documented, scripted boundary: it owns feature
metadata only, while `dracon-utilities` owns code and releases.
"""


def write_tree(root: Path, spec: RepoSpec, monorepo_root: Path) -> None:
    root.mkdir(parents=True, exist_ok=True)
    (root / "README.md").write_text(repo_readme(spec), encoding="utf-8")
    shutil.copy2(monorepo_root / "LICENSE", root / "LICENSE")
    shutil.copy2(monorepo_root / "SECURITY.md", root / "SECURITY.md")
    (root / ".gitignore").write_text(gitignore(), encoding="utf-8")
    (root / ".github").mkdir(parents=True, exist_ok=True)
    (root / ".github" / "ISSUE_TEMPLATE").mkdir(parents=True, exist_ok=True)
    (root / ".github" / "ISSUE_TEMPLATE" / "feature-or-problem.md").write_text(
        issue_template(), encoding="utf-8"
    )
    (root / ".github" / "CODEOWNERS").write_text(codeowners(), encoding="utf-8")
    (root / "docs").mkdir(parents=True, exist_ok=True)
    (root / "docs" / "SOURCE_OF_TRUTH.md").write_text(source_of_truth(), encoding="utf-8")


def collect_paths(root: Path) -> list[Path]:
    paths = [
        root / "README.md",
        root / "LICENSE",
        root / "SECURITY.md",
        root / ".gitignore",
        root / ".github" / "ISSUE_TEMPLATE" / "feature-or-problem.md",
        root / ".github" / "CODEOWNERS",
        root / "docs" / "SOURCE_OF_TRUTH.md",
    ]
    return sorted(p for p in paths if p.exists())


def _init_local_git_repo(repo_root: Path, spec: RepoSpec) -> None:
    """Initialize a local git repo for the generated façade and commit."""
    subprocess.run(["git", "init", "-b", "main", str(repo_root)], check=True)
    subprocess.run(
        ["git", "-C", str(repo_root), "config", "user.email", "dracsharp@gmail.com"],
        check=True,
    )
    subprocess.run(
        ["git", "-C", str(repo_root), "config", "user.name", "DraconDev"],
        check=True,
    )
    subprocess.run(["git", "-C", str(repo_root), "add", "."], check=True)
    subprocess.run(
        [
            "git",
            "-C",
            str(repo_root),
            "commit",
            "--no-verify",
            "-m",
            f"docs: scaffold feature façade for {spec.short} ({spec.name})",
        ],
        check=True,
    )


def _remote_url(remote: str, spec: RepoSpec) -> str:
    """Return the canonical clone URL for a given remote and spec."""
    if remote == "github":
        return f"https://github.com/DraconDev/{spec.name}.git"
    if remote == "gitlab":
        return f"git@gitlab.com:DraconDev/{spec.name}.git"
    if remote == "codeberg":
        return f"ssh://git@codeberg.org/dracondev/{spec.name}.git"
    raise ValueError(f"unknown remote: {remote}")


def push_all_remotes(repo_root: Path, spec: RepoSpec) -> None:
    """Add the 3 remote targets and push main to each (sequentially)."""
    remotes = ("github", "gitlab", "codeberg")
    for remote in remotes:
        url = _remote_url(remote, spec)
        # Replace or add
        subprocess.run(
            [
                "git",
                "-C",
                str(repo_root),
                "remote",
                "remove",
                remote,
            ],
            check=False,
            capture_output=True,
        )
        subprocess.run(
            ["git", "-C", str(repo_root), "remote", "add", remote, url],
            check=True,
        )
        print(f"  remote {remote}: {url}")

    # Push sequentially (no concurrent race; same lesson as dracon-sync multi-remote)
    for remote in remotes:
        print(f"  pushing main to {remote}…")
        result = subprocess.run(
            [
                "git",
                "-C",
                str(repo_root),
                "push",
                "-u",
                remote,
                "main",
            ],
            capture_output=True,
            text=True,
        )
        if result.returncode != 0:
            print(f"    ✗ push to {remote} failed: {result.stderr.strip()}")
            raise SystemExit(1)
        print(f"    ✓ pushed main to {remote}")


def self_test(monorepo_root: Path) -> None:
    # 1. Validate all spec names
    print("[1/2] validating descriptive names against constraints…")
    for spec in specs():
        errors = _validate_name(spec.name, spec.short)
        if errors:
            raise AssertionError(
                f"name constraints violated for {spec.short}: {errors}"
            )
        # At least one keyword from spec.keywords must be in the name
        name_tokens = set(spec.name.split("-"))
        if not (name_tokens & set(spec.keywords)):
            raise AssertionError(
                f"spec.keywords {spec.keywords} not reflected in name {spec.name!r}"
            )
        print(f"  ✓ {spec.short} → {spec.name} (len={len(spec.name)})")

    # 2. Generate full tree in a temp dir
    print("[2/2] generating façade tree in a temp dir…")
    with tempfile.TemporaryDirectory() as tmp:
        target = Path(tmp)
        for spec in specs():
            write_tree(target / spec.name, spec, monorepo_root)
            paths = collect_paths(target / spec.name)
            if not paths:
                raise AssertionError(f"no files generated for {spec.name}")
            for path in paths:
                if not path.exists():
                    raise AssertionError(f"missing generated file: {path}")
            readme = (target / spec.name / "README.md").read_text(encoding="utf-8")
            if "does **not**\nduplicate" not in readme:
                raise AssertionError(
                    f"source-of-truth disclaimer missing for {spec.name}"
                )
            sot = (target / spec.name / "docs" / "SOURCE_OF_TRUTH.md").read_text(
                encoding="utf-8"
            )
            if "monorepo" not in sot:
                raise AssertionError(
                    f"SOURCE_OF_TRUTH.md must mention the monorepo for {spec.name}"
                )
    print("feature repo scaffold self-test passed")


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(
        description=(
            "Scaffold feature-façade repos for Dracon utilities on "
            "GitHub, GitLab, and Codeberg."
        )
    )
    parser.add_argument(
        "--target-root",
        type=Path,
        default=Path(__file__).resolve().parent.parent / "dracon-feature-repos",
        help="Directory that will contain generated façade repos.",
    )
    parser.add_argument(
        "--monorepo-root",
        type=Path,
        default=Path(__file__).resolve().parent.parent,
        help="Path to the dracon-utilities monorepo root.",
    )
    parser.add_argument(
        "--repo",
        choices=sorted(UTILITIES),
        help="Generate only one façade repo (by short name).",
    )
    parser.add_argument(
        "--apply",
        action="store_true",
        help="Write generated files. Without this flag, print a JSON dry-run.",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Alias for the default behavior: print generated paths as JSON without writing files.",
    )
    parser.add_argument(
        "--self-test",
        action="store_true",
        help="Run an internal generation self-test and exit.",
    )
    parser.add_argument(
        "--init-git",
        action="store_true",
        help=(
            "Initialize a git repo at the target path and commit the generated "
            "files. Does not push."
        ),
    )
    parser.add_argument(
        "--push-all-remotes",
        action="store_true",
        help=(
            "Add github, gitlab, and codeberg remotes to the target repo and "
            "push main to each. Requires --init-git to have been run (or the "
            "target to already be a git repo with commits)."
        ),
    )
    parser.add_argument(
        "--validate-name",
        action="store_true",
        help="Only validate the descriptive names against the constraint set and exit.",
    )
    args = parser.parse_args(argv)

    monorepo_root = args.monorepo_root.resolve()
    if not (monorepo_root / "dracon-sync").is_dir():
        raise SystemExit(f"monorepo root does not look correct: {monorepo_root}")
    if not (monorepo_root / "LICENSE").is_file():
        raise SystemExit(f"missing LICENSE in monorepo root: {monorepo_root / 'LICENSE'}")

    if args.self_test:
        self_test(monorepo_root)
        return 0

    if args.validate_name:
        for spec in specs():
            errors = _validate_name(spec.name, spec.short)
            if errors:
                print(f"  ✗ {spec.short} → {spec.name}: {errors}")
                return 1
            print(f"  ✓ {spec.short} → {spec.name} (len={len(spec.name)})")
        print("all names pass the constraint set")
        return 0

    selected = (
        [next(s for s in specs() if s.short == args.repo)] if args.repo else specs()
    )
    payload = []
    for spec in selected:
        repo_root = args.target_root.resolve() / spec.name
        if args.apply:
            write_tree(repo_root, spec, monorepo_root)
            if args.init_git:
                _init_local_git_repo(repo_root, spec)
            if args.push_all_remotes:
                if not (repo_root / ".git").is_dir():
                    raise SystemExit(
                        f"--push-all-remotes requires a git repo at {repo_root}; "
                        "run with --init-git first"
                    )
                push_all_remotes(repo_root, spec)
        payload.append(
            {
                "name": spec.name,
                "short": spec.short,
                "target": str(repo_root),
                "files": [str(p) for p in collect_paths(repo_root)]
                if args.apply and (repo_root).exists()
                else [],
                "source_subdir": spec.subdir,
            }
        )

    if args.apply:
        for item in payload:
            print(item["target"])
    else:
        print(json.dumps(payload, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
