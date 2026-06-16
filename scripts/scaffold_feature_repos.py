#!/usr/bin/env python3
"""Scaffold feature-façade repositories for Dracon utilities.

The Dracon utilities source of truth remains the `dracon-utilities` monorepo.
Each façade repo is a **canonical "main"** for its utility: it contains the
actual source code, `Cargo.toml`, tests, examples, and the per-utility README
mirrored from the monorepo. It is **independently buildable** with a sibling
`dracon-libs` repo (and, for `dracon-warden`, a sibling `dracon-utilities`
repo for the security kit). The auto-sync mechanism
(`scripts/regenerate_facade_repos.py` + monorepo `post-commit` hook) keeps
the per-utility content in sync with the monorepo's per-utility subdirs.

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
        "name": "dracon-system-di[DRACON_SECRET:YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSAxY2hRSDNDYnVSMlpkTFppWDhpRkV1M0FzYjFQTWduTXNwdkM1azV0RzBnCmN1Zkx3MzNkbTIvR216U21Tc3pDS3JHNEtUZ1pybXkwWlM5L0pVeElzMTQKLT4gWDI1NTE5IFJWSkxucHBJWklla1hrNUczTGFXMTlwUHp4R21KUHZQYm84WnRiSXRKMncKejJFTkdkN2ZiQlNvUWZCSzV2VlNFU09IRkJyTGpVQmFVenJUVnVDYUxqdwotPiBYMjU1MTkgeHdWSyt4ZUU4MGhPSk1nWE9NWEMxK1V5Vnh4UXJxNFdSZVFtbFo0aWJ4TQpCeHVCN09Tc2ZmRHYrazEvdVY1UFBEek1uL1M0TjBsMDhUeGFvR1BUTXlBCi0+IFgyNTUxOSB5WUhBVkJXdGtVZUdEMnJRVFBSN0NTU0kvNnY0WGwzQjlvclBTNEN0VjBFCmNlSFVmM0RtUldaK2Y0ODZGZmRDUXBHUnkxRFJ6M0ZVQWtrUGFlVmFNU1UKLT4gWDI1NTE5IFlMK2daRlRYUzZHL1R6SWJxMUJ6bGNTY1c0cFZiRjFIbTNTckpjSkFQenMKbGlaVFBEakRVYzdWVXIycEdtQWorcVRKNzFDWXZXV25QOU1mQVJqZGl5dwotPiA9OCpFLWdyZWFzZSAnb00kIFs+YF1NbwplWG1Na0Vjb01yMU8rOE1hVzljLwotLS0gaHRjbjhTZk9pMXlUUFU3NlBXTS82d0U4RTZ0MXJvUW51Y25LMjFzZEkzMAoUfhRcScqnOYSfVZx7Hy7tOd4UwSlKW9u9EOMHfwShxKfbLXZr7nAeLgyhkjxE5Lsuvc271IZc]",
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

This repository is the **canonical "main"** for `{spec.short}` on GitHub,
GitLab, and Codeberg. It contains the actual source code (mirrored from the
[`DraconDev/dracon-utilities`](https://github.com/DraconDev/dracon-utilities)
monorepo), the `Cargo.toml`, tests, examples, and the per-utility README.
You can build and install this utility directly from this repo.

## Quick start (standalone build)

```bash
# Clone this repo
git clone https://github.com/DraconDev/{spec.name}.git
cd {spec.name}

# Clone required siblings (path-dep layout)
git clone https://github.com/DraconDev/dracon-libs.git ../dracon-libs
{ '# dracon-warden also needs the monorepo for the security kit' if spec.short == 'dracon-warden' else '' }
{ 'git clone https://github.com/DraconDev/dracon-utilities.git ../dracon-utilities' if spec.short == 'dracon-warden' else '' }

# Build
cargo build --release

# Install (binary lands in target/release/)
sudo cp target/release/{spec.short} /usr/local/bin/
```

## What is in this repo

- `src/` — utility source code
- `tests/` — integration tests (if present)
- `Cargo.toml` — standalone build manifest with path-dep siblings
- `README.md` — this file (the per-utility README from the monorepo is at `monorepo-README.md`)
- `BLUEPRINT.md` — design notes
- `{spec.config.split('/')[-1]}` — example config
- `{spec.service}` — systemd user-service unit
- `LICENSE`, `SECURITY.md`, `.gitignore`, `.github/` — repo metadata
- `docs/SOURCE_OF_TRUTH.md` — architecture + invariants

## Relationship to the monorepo

| Boundary | Decision |
|----------|----------|
| Source code | Mirrored from `dracon-utilities/{spec.subdir}` via `scripts/regenerate_facade_repos.py` on every monorepo commit |
| Source of truth | `dracon-utilities` monorepo (the auto-sync is one-way) |
| Feature surface | This repo (canonical main for `{spec.short}`) |
| Shared libraries | Sibling `dracon-libs` workspace (`../dracon-libs`) |
| Operational policy | `~/.dracon/utilities/` TOML files |

## Why this name?

The descriptive name is a deliberate choice for Codeberg/Forgejo, where
descriptive repo names get upvotes and free attention because readers
immediately know what the project does. The full word list (no fillers, no
audience/UX claims) is documented in
[`docs/design/github-feature-repos.md`](https://github.com/DraconDev/dracon-utilities/blob/main/docs/design/github-feature-repos.md).

## Purpose

{spec.focus}

## Runtime

- Binary: `{spec.short}`
- Service: {spec.service}
- Example policy: `{spec.config}`
- Common commands: `{spec.commands}`

## Maintenance

When the monorepo changes the utility source code, README, or example config,
the monorepo's `post-commit` hook calls `scripts/regenerate_facade_repos.py`
which mirrors the changes to this repo. The `dracon-sync` daemon picks up
the local change in `/home/dracon/Dev/facade-repos/{spec.name}` and
auto-pushes to the 3 remotes (github, gitlab, codeberg). No manual
`--apply` or `--push-all-remotes` invocation is needed in the normal flow.

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
    return """# GitHub / GitLab / Codeberg Feature Façade Repositories (v0.112.7+)

Dracon utility façade repositories are the **canonical "mains"** for each
utility. They make `dracon-sync`, `dracon-system`, and `dracon-warden`
discoverable on GitHub, GitLab, and Codeberg and provide an independently
buildable install target for each.

## Architecture (v0.112.7)

- **Each façade repo contains real source code** (not just navigation
  metadata). The source is mirrored from the `DraconDev/dracon-utilities`
  monorepo's per-utility subdir by `scripts/regenerate_facade_repos.py` on
  every monorepo commit.
- **Each façade repo is independently buildable** with a sibling
  `dracon-libs` repo (and, for `dracon-warden`, a sibling `dracon-utilities`
  repo for the security kit). The `Cargo.toml` uses path deps to the
  siblings.
- **The monorepo is the dev workspace** — it owns the development workflow
  and the source-of-truth content. The 3 façade repos are downstream
  one-way mirrors.
- **Auto-sync** is driven by a monorepo `post-commit` hook that calls
  `scripts/regenerate_facade_repos.py`. The script detects which utility's
  source files changed and regenerates that façade. The `dracon-sync` daemon
  picks up the local change in the façade repo clone and auto-pushes to the
  3 remotes (github, gitlab, codeberg).

## Invariants

1. The monorepo is the source of truth for implementation code, tests,
   release packaging, and changelog entries.
2. Each façade repo mirrors its utility's source code from the monorepo via
   `regenerate_facade_repos.py`. The mirror is one-way (monorepo → façade).
3. Each façade repo's `Cargo.toml` uses path deps to siblings
   (`../dracon-libs` for `dracon-git` / `dracon-system-lib`,
   `../dracon-utilities/dracon-warden/src/security` for the
   `dracon-security` kit).
4. The 3 façade repos are 4-remote aligned (github, gitlab, codeberg, + a
   local clone at `/home/dracon/Dev/facade-repos/` that the daemon watches).

## Why this is not a hack

The 3 façade repos give each utility a discoverable, installable home on
GitHub, GitLab, and Codeberg. The auto-sync mechanism keeps them aligned
with the monorepo's source of truth, so the duplication is mechanical (a
scripted mirror) and never drifts. The alternative — keeping the
implementation only in the monorepo — would mean each utility had no
standalone install target, which is what the operator pushed back on:
"are they mains? we are not pushing to them they are still shells".
"""


#: Internal crates that need path-dep rewrites in the standalone Cargo.toml.
#: Maps the dep name (in the per-utility Cargo.toml) to the path-dep
#: replacement (relative to the façade repo root).
INTERNAL_CRATE_PATH_DEPS: dict[str, str] = {
    "dracon-git": "../dracon-libs/tools/sync/dracon-git",
    "dracon-system-lib": "../dracon-libs/tools/system/dracon-system",
    "dracon-security-kit": "../dracon-utilities/dracon-warden/src/security",
}

#: Workspace crate to package name remapping (for `package = "..."` aliases).
PACKAGE_REMAP: dict[str, str] = {
    "dracon-security-kit": "dracon-security",
}


def _parse_workspace_deps(monorepo_root: Path) -> dict[str, str]:
    """Parse `[workspace.dependencies]` from the monorepo's root Cargo.toml.

    Returns a dict of dep-name -> stringified dep spec (e.g. 'anyhow = "1.0"',
    'clap = { version = "4.5", features = ["derive"] }'). Used to inline
    `workspace = true` deps in a standalone Cargo.toml.
    """
    cargo_toml = monorepo_root / "Cargo.toml"
    text = cargo_toml.read_text(encoding="utf-8")
    # Find the [workspace.dependencies] block
    start = text.find("[workspace.dependencies]")
    if start < 0:
        return {}
    # Find the end of the block (next [section] or EOF)
    rest = text[start + len("[workspace.dependencies]"):]
    end = rest.find("\n[")
    if end < 0:
        block = rest
    else:
        block = rest[:end]
    # Parse each line: `name = "version"` or `name = { version = "X", features = [...] }`
    deps: dict[str, str] = {}
    for line in block.splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        if "=" not in line:
            continue
        name, _, value = line.partition("=")
        name = name.strip()
        value = value.strip()
        if name:
            deps[name] = value
    return deps


def _standalone_cargo_toml(
    spec: RepoSpec, monorepo_root: Path
) -> str:
    """Generate a standalone Cargo.toml for the façade repo.

    Starts from the per-utility Cargo.toml in the monorepo, then:
    1. Inlines `workspace = true` deps using the monorepo's
       `[workspace.dependencies]` block.
    2. Rewrites internal crates (`dracon-git`, `dracon-system-lib`,
       `dracon-security-kit`) to use `path = "..."` deps.
    3. Strips the per-utility `[package]` `repository` / `homepage` /
       `documentation` fields that point to the monorepo (replaces them
       with this façade repo's URLs).
    """
    src_path = monorepo_root / spec.subdir / "Cargo.toml"
    text = src_path.read_text(encoding="utf-8")
    workspace_deps = _parse_workspace_deps(monorepo_root)

    # In a tolerant way, parse the [package] + [dependencies] + [dev-dependencies]
    # sections. We do a targeted text-rewrite to keep formatting deterministic.
    lines = text.splitlines()
    out: list[str] = []
    in_workspace_dep = False
    for line in lines:
        stripped = line.strip()
        # Detect a `name = { workspace = true }` line and rewrite it.
        if "workspace = true" in line and "=" in line and not stripped.startswith("#"):
            name_part, _, value_part = line.partition("=")
            name = name_part.strip()
            # Internal crate → path dep
            if name in INTERNAL_CRATE_PATH_DEPS:
                path = INTERNAL_CRATE_PATH_DEPS[name]
                pkg = PACKAGE_REMAP.get(name)
                if pkg:
                    rewrite = f'{name} = {{ package = "{pkg}", path = "{path}" }}'
                else:
                    rewrite = f'{name} = {{ path = "{path}" }}'
                # Preserve leading whitespace
                indent = line[: len(line) - len(line.lstrip())]
                out.append(indent + rewrite)
                in_workspace_dep = True
                continue
            # Regular dep → inline version from [workspace.dependencies]
            if name in workspace_deps:
                indent = line[: len(line) - len(line.lstrip())]
                out.append(f"{indent}{name} = {workspace_deps[name]}")
                in_workspace_dep = True
                continue
            in_workspace_dep = True
            # Unknown workspace dep — keep as-is (will likely fail at build
            # time, which is the right behavior: it forces a fix)
            out.append(line)
            continue
        # Rewrite repository / homepage / documentation in [package] to
        # point at the façade repo.
        if stripped.startswith("repository =") and "github.com/DraconDev/dracon-utilities" in line:
            indent = line[: len(line) - len(line.lstrip())]
            out.append(
                f'{indent}repository = "https://github.com/DraconDev/{spec.name}"'
            )
            continue
        if stripped.startswith("homepage =") and "github.com/DraconDev/dracon-utilities" in line:
            indent = line[: len(line) - len(line.lstrip())]
            out.append(
                f'{indent}homepage = "https://github.com/DraconDev/{spec.name}"'
            )
            continue
        out.append(line)

    out.append("")
    out.append("# Auto-generated by scripts/scaffold_feature_repos.py")
    out.append(f"# from {spec.subdir}/Cargo.toml in dracon-utilities monorepo.")
    out.append(
        f"# Source: https://github.com/DraconDev/dracon-utilities/blob/main/{spec.subdir}/Cargo.toml"
    )
    return "\n".join(out) + "\n"


#: Files / directories in the per-utility subdir to NOT mirror (build
#: artifacts, generated files, repo-specific config, or files that would
#: overwrite the façade's own generated content).
EXCLUDE_FROM_MIRROR = {
    "target",
    "Cargo.lock",
    ".git",
    "proptest-regressions",
    "node_modules",
    # The per-utility README is renamed below; the façade's main README is
    # the generated `repo_readme()` which documents the sibling layout.
}


def _copy_utility_source(root: Path, spec: RepoSpec, monorepo_root: Path) -> int:
    """Copy the per-utility source code from the monorepo to the façade repo.

    Returns the number of files copied. Excludes build artifacts and
    generated files. The per-utility README is renamed to
    `monorepo-README.md` so it does not overwrite the façade's own
    navigation README.
    """
    src_dir = monorepo_root / spec.subdir
    if not src_dir.is_dir():
        return 0
    count = 0
    for src in src_dir.rglob("*"):
        if not src.is_file():
            continue
        rel = src.relative_to(src_dir)
        # Skip excluded paths
        parts = rel.parts
        if any(p in EXCLUDE_FROM_MIRROR for p in parts):
            continue
        # Special-case the per-utility README: rename so it does not
        # overwrite the façade's main README (the navigation README).
        if rel == Path("README.md"):
            dst = root / "monorepo-README.md"
        else:
            dst = root / rel
        dst.parent.mkdir(parents=True, exist_ok=True)
        # Special-case Cargo.toml: rewrite to standalone
        if src.name == "Cargo.toml" and rel == Path("Cargo.toml"):
            dst.write_text(
                _standalone_cargo_toml(spec, monorepo_root), encoding="utf-8"
            )
            count += 1
            continue
        shutil.copy2(src, dst)
        count += 1
    return count


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
    # v0.112.7+: copy the per-utility source code so the façade repo is
    # independently buildable, not a navigation shell.
    n = _copy_utility_source(root, spec, monorepo_root)
    print(f"  [scaffold] copied {n} source files from monorepo/{spec.subdir}")


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
            if "mirrored from the\n[`DraconDev/dracon-utilities`]" not in readme:
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
