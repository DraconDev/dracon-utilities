#!/usr/bin/env python3
"""Verify that CI, the Nix flake, and Cargo metadata agree on nested crates.

The parent repository is intentionally meta-only.  This check makes a nested
utility revision update an explicit, reviewable operation instead of allowing
CI and Nix to silently build different source commits.
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
import tomllib
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SOURCES = {
    "dracon-sync": "dracon-sync-src",
    "dracon-system": "dracon-system-src",
    "dracon-warden": "dracon-warden-src",
}
GITHUB_REPOS = {
    "dracon-sync": "dracon-sync-background-auto-commit-multi-remote",
    "dracon-system": "dracon-system-disk-process-guard-doctor",
    "dracon-warden": "dracon-warden-secret-encrypt-age-git-filter",
}
LOCK_PACKAGES = {
    "dracon-sync": ("dracon-sync", ROOT / "dracon-sync"),
    "dracon-system": ("dracon-system", ROOT / "dracon-system"),
    "dracon-warden": ("dracon-warden", ROOT / "dracon-warden"),
    "dracon-security": (
        "dracon-security",
        ROOT / "dracon-warden" / "src" / "security",
    ),
}


def fail(message: str) -> None:
    print(f"FAIL: {message}", file=sys.stderr)
    raise SystemExit(1)


def read_package_version(crate_dir: Path) -> tuple[str, str]:
    manifest = crate_dir / "Cargo.toml"
    try:
        data = tomllib.loads(manifest.read_text())
        package = data["package"]
        return str(package["name"]), str(package["version"])
    except (OSError, KeyError, TypeError, tomllib.TOMLDecodeError) as error:
        fail(f"cannot read package metadata from {manifest}: {error}")


def local_head(crate_dir: Path) -> str:
    try:
        return subprocess.check_output(
            ["git", "-C", str(crate_dir), "rev-parse", "HEAD"],
            text=True,
            stderr=subprocess.STDOUT,
        ).strip()
    except subprocess.CalledProcessError as error:
        fail(f"cannot read {crate_dir} HEAD: {error.output.strip()}")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--check-local",
        action="store_true",
        help="also require each checked-out nested repository HEAD to equal its pins",
    )
    args = parser.parse_args()

    workflow = (ROOT / ".github/workflows/ci.yml").read_text()
    flake = (ROOT / "flake.nix").read_text()
    flake_lock = json.loads((ROOT / "flake.lock").read_text())
    cargo_lock = tomllib.loads((ROOT / "Cargo.lock").read_text())
    lock_by_name = {}
    for package in cargo_lock.get("package", []):
        # The workspace path package is the source-less entry.  A package name
        # may also occur in the lock file as a registry dependency.
        if package.get("source") is None:
            lock_by_name[package["name"]] = str(package["version"])

    for checkout_path, flake_node in SOURCES.items():
        expected_url = (
            f'url = "github:DraconDev/{GITHUB_REPOS[checkout_path]}/main";'
        )
        if expected_url not in flake:
            fail(f"flake.nix must pin {checkout_path} to the main branch explicitly")

        matches = re.findall(
            rf"path:\s*{re.escape(checkout_path)}\s*\n\s*ref:\s*([0-9a-f]{{40}})",
            workflow,
        )
        if not matches:
            fail(f"CI has no pinned checkout for {checkout_path}")
        if len(set(matches)) != 1:
            fail(f"CI uses inconsistent pins for {checkout_path}: {sorted(set(matches))}")
        ci_rev = matches[0]

        try:
            flake_rev = flake_lock["nodes"][flake_node]["locked"]["rev"]
            flake_ref = flake_lock["nodes"][flake_node]["original"]["ref"]
        except KeyError as error:
            fail(f"flake.lock has no locked revision for {flake_node}: {error}")
        if flake_ref != "main":
            fail(f"flake.lock input {flake_node} is not locked from main: {flake_ref}")
        if ci_rev != flake_rev:
            fail(f"{checkout_path}: CI pin {ci_rev} != flake pin {flake_rev}")

        if args.check_local:
            head = local_head(ROOT / checkout_path)
            if head != ci_rev:
                fail(f"{checkout_path}: local HEAD {head} != pinned revision {ci_rev}")

        print(f"PASS: {checkout_path} source pin {ci_rev}")

    for label, (package_name, crate_dir) in LOCK_PACKAGES.items():
        manifest_name, manifest_version = read_package_version(crate_dir)
        if manifest_name != package_name:
            fail(f"{label}: manifest name is {manifest_name}, expected {package_name}")
        lock_version = lock_by_name.get(package_name)
        if lock_version is None:
            fail(f"Cargo.lock has no source-less workspace package {package_name}")
        if manifest_version != lock_version:
            fail(
                f"{package_name}: Cargo.toml version {manifest_version} "
                f"!= Cargo.lock version {lock_version}"
            )
        print(f"PASS: {package_name} Cargo.lock version {lock_version}")

    print("All nested source and workspace lock pins agree.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
