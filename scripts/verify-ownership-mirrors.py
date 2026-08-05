#!/usr/bin/env python3
"""Verify path ownership, operator push targets, and mirror convergence.

This is deliberately read-only. Identity mismatches are warnings because
configured path ownership controls synchronization; unexpected push targets or
non-converged permitted remotes are failures.
"""
from __future__ import annotations

import argparse
import os
import json
import subprocess
import sys
import tomllib
from pathlib import Path

DEFAULT_POLICY = Path.home() / ".dracon/utilities/sync/dracon-sync.toml"


def run(args: list[str], timeout: int = 35) -> tuple[int, str, str]:
    try:
        p = subprocess.run(args, text=True, capture_output=True, timeout=timeout)
    except (OSError, subprocess.TimeoutExpired) as exc:
        return 125, "", str(exc)
    return p.returncode, p.stdout.strip(), p.stderr.strip()


def load_toml(path: Path) -> dict:
    try:
        with path.open("rb") as fh:
            return tomllib.load(fh)
    except (OSError, tomllib.TOMLDecodeError):
        return {}


def discover_repos(policy: dict) -> list[Path]:
    excluded = set(policy.get("exclude_dir_names", [])) | {".git", "target", "node_modules"}
    found: set[Path] = set()
    for root_text in policy.get("watch_roots", []):
        root = Path(root_text).expanduser()
        if not root.exists():
            continue
        if (root / ".git").exists():
            found.add(root.resolve())
        for current, dirs, _files in os.walk(root):
            dirs[:] = [d for d in dirs if d not in excluded]
            path = Path(current)
            if (path / ".git").exists():
                found.add(path.resolve())
    return sorted(found, key=lambda p: str(p))


def repo_override(repo: Path) -> dict:
    return load_toml(repo / ".dracon/dracon-sync.toml")


def canonical_url(url: str) -> str:
    value = url.strip().lower()
    if "://" in value:
        value = value.split("://", 1)[1]
    if "@" in value.split("/", 1)[0]:
        value = value.split("@", 1)[1]
    value = value.replace(":", "/", 1) if ":" in value.split("/", 1)[0] else value
    return value.removesuffix(".git").rstrip("/")


def default_account(push_url: str) -> str:
    if "codeberg.org" in push_url.lower():
        return "dracondev"
    return "DraconDev"


def expected_url(remote: dict, local_name: str) -> str:
    mapping = remote.get("repo_name_map", {})
    remote_name = mapping.get(local_name, local_name)
    account = remote.get("auto_create_account") or default_account(remote.get("push_url", ""))
    return (
        remote.get("push_url", "")
        .replace("{repo}", str(remote_name))
        .replace("{account}", str(account))
    )


def local_remote(repo: Path, name: str) -> str | None:
    code, out, _err = run(["git", "-C", str(repo), "remote", "get-url", "--push", name])
    return out if code == 0 and out else None


def find_matching_remote(repo: Path, expected: str) -> tuple[str, str] | None:
    code, names, _err = run(["git", "-C", str(repo), "remote"])
    if code != 0:
        return None
    expected_key = canonical_url(expected)
    for name in names.splitlines():
        actual = local_remote(repo, name)
        if actual and canonical_url(actual) == expected_key:
            return name, actual
    return None


def branch_and_head(repo: Path) -> tuple[str | None, str | None]:
    code, branch, _err = run(["git", "-C", str(repo), "symbolic-ref", "--short", "HEAD"])
    if code != 0:
        code, branch, _err = run(["git", "-C", str(repo), "rev-parse", "--abbrev-ref", "HEAD"])
        if code != 0 or branch == "HEAD":
            return None, None
    code, head, _err = run(["git", "-C", str(repo), "rev-parse", "HEAD"])
    return (branch, head) if code == 0 else (branch, None)


def remote_head(repo: Path, remote: str, branch: str) -> tuple[str | None, bool]:
    code, out, err = run(
        ["git", "-C", str(repo), "ls-remote", remote, f"refs/heads/{branch}"],
        timeout=45,
    )
    if code != 0 or not out:
        lower = err.lower()
        missing = any(
            phrase in lower
            for phrase in (
                "cannot find repository",
                "repository not found",
                "could not be found",
                "does not exist",
                "push to create is not enabled",
                "404",
            )
        )
        return None, missing
    return out.split()[0], False


def live_report_rows() -> dict[str, dict]:
    code, out, _err = run(["dracon-sync", "repos", "--json"], timeout=120)
    if code != 0:
        return {}
    try:
        data = json.loads(out)
    except json.JSONDecodeError:
        return {}
    return {row.get("repo", ""): row for row in data.get("rows", [])}


def identity_warning(repo: Path) -> str | None:
    code, email, _err = run(["git", "-C", str(repo), "config", "--local", "--get", "user.email"])
    if code == 0 and email and email not in {
        "dracsharp@gmail.com", "darklord@dracon.local", "hellhunter@dracon.local",
        "endless-td@dracon.local", "hegemon@dracon.local", "junk-runner@dracon.local",
        "neonbreak@dracon.local", "deathrun@dracon.local", "ai-auto-writer@dracon.local",
        "pully@dracon.local", "polis@dracon.local", "mojo@local",
    }:
        return f"local user.email={email}"
    code, author, _err = run(["git", "-C", str(repo), "log", "-1", "--format=%an <%ae>"])
    if code == 0 and author and "--global" in author:
        return f"HEAD author={author}"
    return None


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--policy", type=Path, default=DEFAULT_POLICY)
    args = parser.parse_args()
    policy = load_toml(args.policy)
    remotes = policy.get("remotes", [])
    repos = discover_repos(policy)
    if not repos:
        raise SystemExit("no repositories found beneath configured watch roots")

    failures: list[str] = []
    warnings: list[str] = []
    checked_refs = 0
    report_rows = live_report_rows()
    for repo in repos:
        override = repo_override(repo)
        if override.get("owned") is False:
            warnings.append(f"{repo}: explicit owned=false opt-out")
            continue
        branch, head = branch_and_head(repo)
        if head is None:
            warnings.append(f"{repo}: empty or detached repository (no comparable HEAD)")
            continue
        if not any(repo.resolve().is_relative_to(Path(r).expanduser().resolve()) for r in policy.get("watch_roots", [])):
            failures.append(f"{repo}: discovered path is outside configured watch roots")
        identity = identity_warning(repo)
        if identity:
            warnings.append(f"{repo}: path-owned identity warning ({identity})")

        excluded = set(override.get("exclude_remotes", []))
        for remote in remotes:
            name = remote.get("name", "")
            if not name or name in excluded:
                continue
            # Codeberg remains a public-only mirror. Absence is expected until
            # a fresh positive public visibility result authorizes creation;
            # an existing private mirror is preserved but intentionally not
            # required to advance while all owned forges are private.
            if name == "codeberg" and policy.get("codeberg_public_only", True):
                row = report_rows.get(str(repo), {})
                if row.get("codeberg_skip_reason") in {"private", "unknown"}:
                    continue
                # A positively public row is expected to have a Codeberg
                # mirror; let the normal missing-remote failure below catch
                # an unprovisioned mirror instead of silently accepting it.
            expected = expected_url(remote, repo.name)
            matching = find_matching_remote(repo, expected)
            if matching is None:
                failures.append(
                    f"{repo}: missing permitted remote {name} targeting {expected}"
                )
                continue
            actual_name, actual = matching
            remote_tip, missing = remote_head(repo, actual_name, branch)
            if missing:
                failures.append(f"{repo}: permitted {actual_name}/{branch} repository is missing on the forge")
            elif remote_tip is None:
                warnings.append(f"{repo}: {name}/{branch} could not be queried (unknown, not treated as publishable)")
            elif remote_tip != head:
                row = report_rows.get(str(repo), {})
                worktree_dirty = bool(run(["git", "-C", str(repo), "status", "--porcelain"], timeout=20)[1])
                sync_active = row.get("push_status") in {"PENDING", "PUSHING", "ACTIVE"} or bool(row.get("ahead", 0))
                if worktree_dirty or sync_active:
                    warnings.append(
                        f"{repo}: {actual_name}/{branch} differs while local activity is in flight; "
                        "daemon sync is allowed to converge it"
                    )
                    checked_refs += 1
                    continue
                ancestor = run(
                    ["git", "-C", str(repo), "merge-base", "--is-ancestor", remote_tip, head],
                    timeout=20,
                )[0] == 0
                if not ancestor:
                    warnings.append(
                        f"{repo}: {actual_name} mirror diverges or is ahead "
                        f"({remote_tip[:12]} vs local {head[:12]}); preserved because "
                        "history rewrites are forbidden"
                    )
                    checked_refs += 1
                else:
                    failures.append(f"{repo}: {actual_name}/{branch}={remote_tip[:12]} differs from local {head[:12]}")
            else:
                checked_refs += 1

        origin = local_remote(repo, "origin")
        if origin:
            trusted = [canonical_url(expected_url(r, repo.name)).split("/", 1)[0] for r in remotes]
            origin_host = canonical_url(origin).split("/", 1)[0]
            if origin_host not in trusted:
                warnings.append(f"{repo}: foreign origin is fetch-only by policy ({origin})")

    print(f"ownership/mirror verification: {len(repos)} repos, {checked_refs} permitted refs equal local HEAD")
    for warning in warnings:
        print(f"WARN: {warning}")
    for failure in failures:
        print(f"FAIL: {failure}")
    if failures:
        print(f"FAILED: {len(failures)} invariant(s)")
        return 1
    print("PASS: path ownership, configured operator targets, and comparable mirrors are consistent")
    return 0


if __name__ == "__main__":
    sys.exit(main())
