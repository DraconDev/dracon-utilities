#!/usr/bin/env python3
"""Scaffold GitHub feature-façade repositories for Dracon utilities.

The Dracon utilities source of truth remains the `dracon-utilities` monorepo.
These façade repos are intentionally small GitHub presentation surfaces for
`dracon-sync`, `dracon-system`, and `dracon-warden`. They avoid duplicating
implementation code and instead point users to the canonical monorepo paths.
"""

from __future__ import annotations

import argparse
import json
import shutil
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path

UTILITIES: dict[str, dict[str, str]] = {
    "dracon-sync": {
        "title": "Dracon Sync",
        "description": "Invisible git sync automation for AI-assisted development.",
        "subdir": "dracon-sync",
        "service": "dracon-sync.service",
        "config": "dracon-sync/dracon-sync.example.toml",
        "commands": "dracon-sync status · dracon-sync repos · dracon-sync health · dracon-sync daemon",
        "focus": "Watches configured repositories, waits for changes to settle, commits deterministic diff-based messages, and pushes to origin plus configured mirrors.",
    },
    "dracon-system": {
        "title": "Dracon System",
        "description": "Local disk, process, storage, zram, and service diagnostics for Dracon machines.",
        "subdir": "dracon-system",
        "service": "dracon-system-guard.service",
        "config": "dracon-system/dracon-system.example.toml",
        "commands": "dracon-system status · dracon-system doctor · dracon-system storage · dracon-system guard daemon",
        "focus": "Protects machines from disk/process pressure and provides deterministic diagnostics for storage, links, zram, events, and the guard daemon.",
    },
    "dracon-warden": {
        "title": "Dracon Warden",
        "description": "Git filter encryption and repository hardening for Dracon workspaces.",
        "subdir": "dracon-warden",
        "service": "No systemd service; enforced through global git hooks.",
        "config": "dracon-warden/dracon-warden.example.toml",
        "commands": "dracon-warden status · dracon-warden keygen · dracon-warden setup-hooks --global · dracon-warden scrub-markers",
        "focus": "Encrypts secret-shaped content at rest in git while preserving normal plaintext files in the working tree.",
    },
}


@dataclass(frozen=True)
class RepoSpec:
    name: str
    title: str
    description: str
    subdir: str
    service: str
    config: str
    commands: str
    focus: str


def specs() -> list[RepoSpec]:
    return [RepoSpec(name=name, **data) for name, data in UTILITIES.items()]


def repo_readme(name: str, spec: RepoSpec) -> str:
    return f"""# {spec.title}

{spec.description}

This repository is a GitHub feature façade for {name}. It does **not**
duplicate the implementation code. The canonical source of truth remains the
[`DraconDev/dracon-utilities`](https://github.com/DraconDev/dracon-utilities)
monorepo, with this utility's code and docs under:

- Source: [`{spec.subdir}/`](https://github.com/DraconDev/dracon-utilities/tree/main/{spec.subdir})
- User guide: [`{spec.subdir}/README.md`](https://github.com/DraconDev/dracon-utilities/tree/main/{spec.subdir}/README.md)
- Design notes: [`{spec.subdir}/BLUEPRINT.md`](https://github.com/DraconDev/dracon-utilities/tree/main/{spec.subdir}/BLUEPRINT.md)
- Example config: [`{spec.config}`](https://github.com/DraconDev/dracon-utilities/tree/main/{spec.config})

## Purpose

{spec.focus}

Use this repo to feature the utility on GitHub without splitting the actual
implementation out of the monorepo. Issues, project boards, and roadmap notes can
live here, while commits, releases, tests, and packaging stay anchored in
`dracon-utilities`.

## Runtime

- Binary: `{name}`
- Service: {spec.service}
- Example policy: `{spec.config}`
- Common commands: `{spec.commands}`

## Relationship to the monorepo

| Boundary | Decision |
|----------|----------|
| Source code | Lives in `dracon-utilities/{spec.subdir}` |
| Release artifacts | Built and published from `dracon-utilities` |
| GitHub feature surface | This façade repo |
| Operational policy | `~/.dracon/utilities/` TOML files |
| Shared libraries | Sibling `dracon-libs` workspace where applicable |

## Maintenance

When the monorepo changes the utility README, blueprint, or example config,
regenerate this façade with:

```bash
cd /path/to/dracon-utilities
./scripts/scaffold_feature_repos.py --apply --repo {name}
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
    return """# GitHub Feature Façade Repositories

Dracon utility façade repositories are intentionally small GitHub presentation
surfaces. They make `dracon-sync`, `dracon-system`, and `dracon-warden` easier to
feature on GitHub without splitting the implementation out of the
`DraconDev/dracon-utilities` monorepo.

## Invariants

1. The monorepo is the only source of truth for implementation code, tests,
   release packaging, and changelog entries.
2. Façade repos contain only navigation, issue/project metadata, licenses, and
   links back to the monorepo paths.
3. Do not copy implementation files into façade repos. If code needs a public
   home, create a real separate crate/binary repo and update the monorepo
   architecture docs first.
4. Regenerate façade repos with `scripts/scaffold_feature_repos.py --apply` so
   the presentation layer stays consistent.

## Why this is not a hack

GitHub cannot natively present a subdirectory as a first-class repository with
separate issues, projects, topics, and README without duplicating or moving
files. A façade repo avoids both bad options:

- Moving code would split the implementation and break the current release
  pipeline.
- Copying code would create drift and duplicate maintenance.

The façade repo is therefore a documented, scripted boundary: it owns GitHub
feature metadata only, while `dracon-utilities` owns code and releases.
"""


def write_tree(root: Path, spec: RepoSpec, monorepo_root: Path) -> None:
    root.mkdir(parents=True, exist_ok=True)
    (root / "README.md").write_text(repo_readme(spec.name, spec), encoding="utf-8")
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


def collect_paths(root: Path, spec: RepoSpec) -> list[Path]:
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


def self_test(monorepo_root: Path) -> None:
    with tempfile.TemporaryDirectory() as tmp:
        target = Path(tmp)
        for spec in specs():
            write_tree(target / spec.name, spec, monorepo_root)
            paths = collect_paths(target / spec.name, spec)
            if not paths:
                raise AssertionError(f"no files generated for {spec.name}")
            for path in paths:
                if not path.exists():
                    raise AssertionError(f"missing generated file: {path}")
            readme = (target / spec.name / "README.md").read_text(encoding="utf-8")
            if "does **not**\nduplicate" not in readme:
                raise AssertionError(f"source-of-truth disclaimer missing for {spec.name}")
    print("feature repo scaffold self-test passed")


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(
        description="Scaffold GitHub feature-façade repos for Dracon utilities."
    )
    parser.add_argument(
        "--target-root",
        type=Path,
        default=Path.cwd().parent / "dracon-feature-repos",
        help="Directory that will contain generated façade repos.",
    )
    parser.add_argument(
        "--monorepo-root",
        type=Path,
        default=Path.cwd(),
        help="Path to the dracon-utilities monorepo root.",
    )
    parser.add_argument(
        "--repo",
        choices=sorted(UTILITIES),
        help="Generate only one façade repo.",
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
    args = parser.parse_args(argv)

    monorepo_root = args.monorepo_root.resolve()
    if not (monorepo_root / "dracon-sync").is_dir():
        raise SystemExit(f"monorepo root does not look correct: {monorepo_root}")
    if not (monorepo_root / "LICENSE").is_file():
        raise SystemExit(f"missing LICENSE in monorepo root: {monorepo_root / 'LICENSE'}")

    if args.self_test:
        self_test(monorepo_root)
        return 0

    selected = [next(s for s in specs() if s.name == args.repo)] if args.repo else specs()
    payload = []
    for spec in selected:
        repo_root = args.target_root.resolve() / spec.name
        if args.apply:
            write_tree(repo_root, spec, monorepo_root)
        payload.append(
            {
                "name": spec.name,
                "target": str(repo_root),
                "files": [str(p) for p in collect_paths(repo_root, spec)] if args.apply else [],
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
