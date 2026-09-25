#!/usr/bin/env python3
"""Record forward-only convergence actions, gates, and final evidence state."""
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
    finalize_evidence,
    iso_now,
    record_action,
    record_external_blocker,
    record_gate,
    record_phase,
    refresh_evidence,
    load_toml,
)


def read_evidence(path: Path) -> dict:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise ConvergenceError(f"cannot read evidence {path}: {exc}") from exc


def validate_ref_files(evidence_path: Path, refs: list[str]) -> None:
    root = evidence_path.parent.resolve()
    for ref in refs:
        candidate = Path(ref)
        if candidate.is_absolute() or ".." in candidate.parts:
            raise ConvergenceError(f"evidence reference escapes audit directory: {ref}")
        if not (root / candidate).is_file():
            raise ConvergenceError(f"evidence reference does not exist: {ref}")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--evidence", required=True, type=Path)
    parser.add_argument("--policy", type=Path, default=DEFAULT_POLICY)
    parser.add_argument("--freeze-marker", type=Path, default=DEFAULT_FREEZE)
    parser.add_argument("--remote-attempts", type=int, default=3)
    parser.add_argument(
        "--offline-evidence-only",
        action="store_true",
        help="Use only for deterministic temporary-repository rehearsals; final fleet runs must omit this.",
    )
    sub = parser.add_subparsers(dest="command", required=True)

    refresh = sub.add_parser("refresh", help="Refresh live heads, state, and remotes.")
    refresh.add_argument("--require-clean", action="store_true")

    phase = sub.add_parser("record-phase", help="Record pre-resume or post-resume evidence.")
    phase.add_argument("--name", required=True, choices=["pre-resume", "post-resume"])
    phase.add_argument("--evidence-ref", action="append", required=True)
    phase.add_argument("--notes", required=True)

    action = sub.add_parser("record-action", help="Record a reviewed mutation.")
    action.add_argument("--repository", required=True)
    action.add_argument("--kind", required=True)
    action.add_argument("--result", required=True)
    action.add_argument("--evidence-ref", action="append", required=True)
    action.add_argument("--head-before")
    action.add_argument("--head-after")

    blocker = sub.add_parser(
        "record-blocker", help="Record a strict external blocker after three fresh remote attempts."
    )
    blocker.add_argument("--repository", required=True)
    blocker.add_argument("--remote", required=True)
    blocker.add_argument("--evidence-ref", required=True)

    gate = sub.add_parser("record-gate", help="Record a named quality gate.")
    gate.add_argument("--name", required=True)
    gate.add_argument("--command", dest="gate_command", required=True)
    gate.add_argument("--status", required=True)
    gate.add_argument("--notes", required=True)
    gate.add_argument("--evidence-ref", action="append", required=True)

    sub.add_parser("finalize", help="Refresh and mark evidence pass only if final verification succeeds.")
    return parser.parse_args()


def require_clean(evidence: dict) -> None:
    dirty = []
    for repo in evidence.get("repositories", []):
        state = repo.get("state", {})
        if any(int(state.get(key, 0)) != 0 for key in ("modified", "staged", "untracked", "submodule", "ahead", "behind")):
            dirty.append(repo.get("path"))
        if state.get("operation") != "none" or state.get("missing_objects") != 0:
            dirty.append(repo.get("path"))
    if dirty:
        raise ConvergenceError("selected repositories are not clean: " + ", ".join(sorted(set(dirty))))


def main() -> int:
    args = parse_args()
    try:
        if args.command == "finalize":
            if args.freeze_marker.exists():
                raise ConvergenceError("cannot finalize while daemon is frozen")
            evidence, verification = finalize_evidence(
                args.evidence,
                args.policy,
                args.freeze_marker,
                remote_attempts=args.remote_attempts,
                check_live_remotes=not args.offline_evidence_only,
            )
            atomic_write_json(args.evidence, evidence)
            print(json.dumps(verification, indent=2, sort_keys=True))
            return 0 if verification["ok"] else 1

        evidence = read_evidence(args.evidence)
        policy = load_toml(args.policy)
        if args.command == "refresh":
            evidence = refresh_evidence(evidence, policy, remote_attempts=args.remote_attempts)
            if args.require_clean:
                require_clean(evidence)
        elif args.command == "record-phase":
            validate_ref_files(args.evidence, args.evidence_ref)
            evidence = record_phase(
                evidence,
                name=args.name,
                evidence_refs=args.evidence_ref,
                notes=args.notes,
            )
        elif args.command == "record-action":
            validate_ref_files(args.evidence, args.evidence_ref)
            evidence = record_action(
                evidence,
                repository=args.repository,
                kind=args.kind,
                result=args.result,
                evidence_refs=args.evidence_ref,
                head_before=args.head_before,
                head_after=args.head_after,
            )
        elif args.command == "record-blocker":
            validate_ref_files(args.evidence, [args.evidence_ref])
            evidence = record_external_blocker(
                evidence,
                policy,
                repository=args.repository,
                remote=args.remote,
                evidence_ref=args.evidence_ref,
                attempts=args.remote_attempts,
            )
        elif args.command == "record-gate":
            validate_ref_files(args.evidence, args.evidence_ref)
            evidence = record_gate(
                evidence,
                name=args.name,
                command=args.gate_command,
                status=args.status,
                notes=args.notes,
                evidence_refs=args.evidence_ref,
            )
        evidence["updated_at"] = iso_now()
        atomic_write_json(args.evidence, evidence)
        print(json.dumps({"ok": True, "result": evidence.get("result"), "updated_at": evidence["updated_at"]}, sort_keys=True))
        return 0
    except ConvergenceError as exc:
        print(f"FAIL: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
