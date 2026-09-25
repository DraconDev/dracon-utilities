#!/usr/bin/env python3
"""Wait for selected repositories to quiesce and write convergence evidence.

This command is read-only with respect to selected repositories. It refuses to
run unless the sanctioned daemon freeze marker is present, and it writes only
the requested evidence JSON plus no product files.
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

from sync_convergence import (
    DEFAULT_FREEZE,
    DEFAULT_POLICY,
    ConvergenceError,
    atomic_write_json,
    capture_remote_state,
    capture_repository,
    discover_repositories,
    initialize_evidence,
    iso_now,
    load_toml,
    wait_for_quiescence,
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--repo",
        action="append",
        required=True,
        type=Path,
        help="Selected repository root; repeat for each lane. Nested dirty gitlinks are discovered.",
    )
    parser.add_argument(
        "--evidence",
        required=True,
        type=Path,
        help="Output evidence JSON path.",
    )
    parser.add_argument("--policy", type=Path, default=DEFAULT_POLICY)
    parser.add_argument("--freeze-marker", type=Path, default=DEFAULT_FREEZE)
    parser.add_argument("--stable-samples", type=int, default=3)
    parser.add_argument("--interval-seconds", type=float, default=10.0)
    parser.add_argument("--max-wait-seconds", type=int, default=1800)
    parser.add_argument("--remote-attempts", type=int, default=3)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    try:
        if not args.freeze_marker.exists():
            raise ConvergenceError(
                f"freeze marker is absent; run `dracon-sync pause` before capture: {args.freeze_marker}"
            )
        policy = load_toml(args.policy)
        discovered = discover_repositories(args.repo, policy)
        if not discovered:
            raise ConvergenceError("no repositories selected")
        quiescence = wait_for_quiescence(
            discovered,
            policy,
            freeze_marker=args.freeze_marker,
            stable_samples=args.stable_samples,
            interval_seconds=args.interval_seconds,
            max_wait_seconds=args.max_wait_seconds,
            progress=lambda message: print(message, file=sys.stderr, flush=True),
        )
        captured: list[dict] = []
        for record in discovered:
            repo = Path(record["path"])
            snapshot = capture_repository(repo, policy)
            expected = quiescence["fast_tokens"][str(repo)]
            if snapshot["snapshot"]["fast_token"] != expected:
                raise ConvergenceError(
                    f"repository changed while evidence was being captured: {repo}"
                )
            snapshot["role"] = record["role"]
            snapshot["remotes"] = capture_remote_state(
                snapshot, policy, attempts=args.remote_attempts
            )
            captured.append(snapshot)
        evidence = initialize_evidence(
            captured,
            args.policy,
            args.freeze_marker,
            quiescence,
            remote_attempts=args.remote_attempts,
        )
        atomic_write_json(args.evidence, evidence)
        print(
            json.dumps(
                {
                    "ok": True,
                    "evidence": str(args.evidence),
                    "repositories": len(captured),
                    "quiescence": quiescence["span_seconds"],
                    "completed_at": iso_now(),
                },
                sort_keys=True,
            )
        )
        return 0
    except ConvergenceError as exc:
        print(f"FAIL: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
