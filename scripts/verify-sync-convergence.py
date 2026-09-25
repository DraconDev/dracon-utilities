#!/usr/bin/env python3
"""Verify recorded and live daemon convergence without mutating repositories."""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

from sync_convergence import (
    DEFAULT_FREEZE,
    DEFAULT_POLICY,
    ConvergenceError,
    format_verification,
    verify_evidence,
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--evidence", required=True, type=Path)
    parser.add_argument("--policy", type=Path, default=DEFAULT_POLICY)
    parser.add_argument("--freeze-marker", type=Path, default=DEFAULT_FREEZE)
    parser.add_argument("--remote-attempts", type=int, default=3)
    parser.add_argument(
        "--offline-evidence-only",
        action="store_true",
        help="Validate recorded evidence without querying live daemon or remotes.",
    )
    parser.add_argument("--json", action="store_true")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    try:
        result = verify_evidence(
            args.evidence,
            args.policy,
            freeze_marker=args.freeze_marker,
            remote_attempts=args.remote_attempts,
            check_live_remotes=not args.offline_evidence_only,
        )
    except ConvergenceError as exc:
        result = {
            "ok": False,
            "errors": [str(exc)],
            "warnings": [],
            "checked_remotes": 0,
            "repositories": 0,
            "health": {},
        }
    if args.json:
        print(json.dumps(result, indent=2, sort_keys=True))
    else:
        print(format_verification(result))
    return 0 if result["ok"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
