#!/usr/bin/env python3
"""Compare commit activity in matching windows without manufacturing work.

The report is intentionally descriptive: a lower count is evidence to
investigate, not a condition that should be "fixed" by creating empty commits.
"""
from __future__ import annotations

import argparse
import datetime as dt
import os
import subprocess
import sys
import tomllib
from pathlib import Path

DEFAULT_POLICY = Path.home() / ".dracon/utilities/sync/dracon-sync.toml"
DEFAULT_WINDOWS = (("1h", 3600), ("6h", 6 * 3600), ("24h", 24 * 3600))


def run(args: list[str], timeout: int = 20) -> tuple[int, str, str]:
    try:
        p = subprocess.run(args, text=True, capture_output=True, timeout=timeout)
    except (OSError, subprocess.TimeoutExpired) as exc:
        return 125, "", str(exc)
    return p.returncode, p.stdout.strip(), p.stderr.strip()


def load_policy(path: Path) -> dict:
    try:
        with path.open("rb") as fh:
            return tomllib.load(fh)
    except (OSError, tomllib.TOMLDecodeError) as exc:
        raise SystemExit(f"cannot read policy {path}: {exc}")


def discover_repos(policy: dict) -> list[Path]:
    excluded = set(policy.get("exclude_dir_names", []))
    excluded.update({".git", "target", "node_modules"})
    found: set[Path] = set()
    for root_text in policy.get("watch_roots", []):
        root = Path(root_text).expanduser()
        if not root.exists():
            continue
        if (root / ".git").exists():
            found.add(root.resolve())
        for current, dirs, _files in os.walk(root):
            dirs[:] = [d for d in dirs if d not in excluded]
            current_path = Path(current)
            if (current_path / ".git").exists():
                found.add(current_path.resolve())
    return sorted(found, key=lambda p: str(p))


def usable_ref(repo: Path) -> str | None:
    for ref in ("HEAD", "origin/main", "gitlab/main", "github/main"):
        code, _out, _err = run(["git", "-C", str(repo), "rev-parse", "--verify", ref])
        if code == 0:
            return ref
    return None


def count_commits(repo: Path, ref: str, start: int, end: int) -> int | None:
    code, out, _err = run(
        [
            "git",
            "-C",
            str(repo),
            "rev-list",
            "--count",
            f"--since=@{start}",
            f"--until=@{end}",
            ref,
        ]
    )
    if code != 0:
        return None
    try:
        return int(out or "0")
    except ValueError:
        return None


def classify(current: int | None, baseline: int | None) -> str:
    if current is None or baseline is None:
        return "history unavailable"
    if current == 0 and baseline > 0:
        return "no current commits — inspect workload/ownership"
    if current < baseline:
        return "lower than baseline — explain workload or blocker"
    if current > baseline:
        return "higher than baseline"
    return "matches baseline"


def fmt_time(epoch: int) -> str:
    return dt.datetime.fromtimestamp(epoch, dt.timezone.utc).strftime("%Y-%m-%d %H:%M UTC")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--policy", type=Path, default=DEFAULT_POLICY)
    parser.add_argument("--compare-days", type=int, default=3)
    parser.add_argument("--write", type=Path)
    args = parser.parse_args()
    if args.compare_days < 1:
        parser.error("--compare-days must be positive")

    policy = load_policy(args.policy)
    repos = discover_repos(policy)
    if not repos:
        raise SystemExit("no repositories found beneath configured watch roots")

    end = int(dt.datetime.now(dt.timezone.utc).timestamp())
    baseline_end = end - args.compare_days * 86400
    rows: list[dict] = []
    for repo in repos:
        ref = usable_ref(repo)
        row: dict = {"repo": str(repo), "ref": ref or "unavailable"}
        for label, seconds in DEFAULT_WINDOWS:
            row[f"{label}_current"] = count_commits(repo, ref, end - seconds, end) if ref else None
            row[f"{label}_baseline"] = (
                count_commits(repo, ref, baseline_end - seconds, baseline_end) if ref else None
            )
        row["classification"] = classify(row["24h_current"], row["24h_baseline"])
        rows.append(row)

    current_totals = {
        label: sum(row[f"{label}_current"] or 0 for row in rows) for label, _ in DEFAULT_WINDOWS
    }
    baseline_totals = {
        label: sum(row[f"{label}_baseline"] or 0 for row in rows) for label, _ in DEFAULT_WINDOWS
    }
    lines = [
        "# Commit activity audit",
        "",
        f"Generated: {fmt_time(end)}",
        f"Policy: `{args.policy}`",
        f"Comparison: current windows ending {fmt_time(end)} vs matching windows ending {fmt_time(baseline_end)} ({args.compare_days} days earlier)",
        "",
        "This audit measures existing commit activity only. Lower activity is classified for investigation; no commits are manufactured to match the baseline.",
        "",
        "## Fleet totals",
        "",
        "| Window | Current | Baseline | Delta |",
        "|---|---:|---:|---:|",
    ]
    for label, _seconds in DEFAULT_WINDOWS:
        current = current_totals[label]
        baseline = baseline_totals[label]
        lines.append(f"| {label} | {current} | {baseline} | {current - baseline:+d} |")
    lines += [
        "",
        "## Repositories",
        "",
        "| Repository | Ref | 1h | 6h | 24h | Classification |",
        "|---|---|---:|---:|---:|---|",
    ]
    for row in rows:
        short = row["repo"].replace("|", "\\|")
        lines.append(
            f"| `{short}` | `{row['ref']}` | "
            f"{row['1h_current'] if row['1h_current'] is not None else '?'} / {row['1h_baseline'] if row['1h_baseline'] is not None else '?'} | "
            f"{row['6h_current'] if row['6h_current'] is not None else '?'} / {row['6h_baseline'] if row['6h_baseline'] is not None else '?'} | "
            f"{row['24h_current'] if row['24h_current'] is not None else '?'} / {row['24h_baseline'] if row['24h_baseline'] is not None else '?'} | "
            f"{row['classification']} |"
        )
    lines += [
        "",
        "Window cells are `current / baseline`; the baseline is time-matched rather than a lifetime total.",
    ]
    report = "\n".join(lines) + "\n"
    if args.write:
        args.write.parent.mkdir(parents=True, exist_ok=True)
        args.write.write_text(report, encoding="utf-8")
    print(report, end="")
    return 0


if __name__ == "__main__":
    sys.exit(main())
