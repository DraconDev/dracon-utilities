#!/usr/bin/env python3
"""Re-pin every nested utility source across flake.lock and the CI workflow.

WHY THIS EXISTS
---------------
Decision D1 (2026-09-27) pinned the three utilities in two places that
must agree: `flake.lock` and the `ref:` of every `actions/checkout` in
.github/workflows/ci.yml.  A nested release — and the auto-commit daemon
produces those continuously — moves the utility's `main` and the two
pins fall behind.  `scripts/check-nested-pins.py` then fails, which is
correct but leaves the operator hand-editing two files with the same
40-hex revision repeated seven times.

This performs that refresh atomically and verifiably:

    ./scripts/repin-nested-sources.py            # pin to each remote's main
    ./scripts/repin-nested-sources.py --check    # verify only, change nothing

It re-pins to the REMOTE main, never to a local worktree, so a dirty or
mid-release worktree cannot leak into the pins.  Local HEAD may still sit
ahead of the pin after this runs; that is expected while the daemon is
committing, and `check-nested-pins.py --check-local` will say so until
the next refresh.
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
WORKFLOW = ROOT / ".github/workflows/ci.yml"
FLAKE = ROOT / "flake.nix"
LOCK = ROOT / "flake.lock"

# checkout directory -> (flake lock node, CI `repository:` fragment)
TARGETS = {
    "dracon-sync": ("dracon-sync-src", "dracon-sync-background-auto-commit-multi-remote"),
    "dracon-system": ("dracon-system-src", "dracon-system-disk-process-guard-doctor"),
    "dracon-warden": ("dracon-warden-src", "dracon-warden-secret-encrypt-age-git-filter"),
}

# checkout directory -> flake.nix `*-src` variable for the crateVersion
# fallback. Explicit (not string-built): the old
# `"dracon" + checkout.split("-")[1] + "Src"` produced `draconsyncSrc`,
# which never matched `draconSyncSrc` — the rewrite silently rewrote
# nothing and every fallback drifted (audit R4-M-12).
FLAKE_SRC_VAR = {
    "dracon-sync": "draconSyncSrc",
    "dracon-system": "draconSystemSrc",
    "dracon-warden": "draconWardenSrc",
}


def run(argv: list[str], cwd: Path | None = None) -> str:
    return subprocess.check_output(argv, cwd=cwd, text=True).strip()


def remote_main(checkout: str) -> str:
    """The 40-hex `main` of the nested repo's first non-origin remote."""
    repo = ROOT / checkout
    remotes = run(["git", "remote"], cwd=repo).split("\n")
    candidates = [r for r in remotes if r != "origin"] or remotes
    for name in candidates:
        run(["git", "fetch", name, "main"], cwd=repo)
        return run(["git", "rev-parse", "FETCH_HEAD"], cwd=repo)
    raise SystemExit(f"{checkout}: no usable remote")


def package_version_from_manifest(manifest_text: str) -> str | None:
    """The [package] version from a Cargo.toml text, or None when absent.

    Section-scoped: a [workspace.package] version above [package] must
    not win (same class as audit R4-M-11).
    """
    in_package = False
    for line in manifest_text.splitlines():
        stripped = line.strip()
        if stripped.startswith("["):
            in_package = stripped == "[package]"
            continue
        if in_package:
            match = re.match(r'^version\s*=\s*"([^"]+)"', stripped)
            if match:
                return match.group(1)
    return None


def rewrite_crate_fallback(flake_text: str, src_var: str, version: str) -> str:
    """Replace the crateVersion fallback for one src variable.

    Raises SystemExit unless exactly one line is rewritten: the pre-fix
    pattern silently matched nothing, so repin "kept fallbacks in step"
    without ever rewriting them (audit R4-M-12).
    """
    pattern = re.compile(r'(version = crateVersion ")[^"]+(" ' + re.escape(src_var) + r")")
    new_text, count = pattern.subn(
        lambda m: m.group(1) + version + m.group(2), flake_text
    )
    if count != 1:
        raise SystemExit(
            f"expected exactly one crateVersion fallback for {src_var}, rewrote {count}"
        )
    return new_text


def workflow_refs(text: str, repository_fragment: str, checkout: str) -> list[re.Match[str]]:
    """Every `ref:` belonging to one `repository:` in the workflow.

    The workflow writes `repository:`, then `ref:`, then `path:`, so the
    ref must be located relative to its repository line rather than by a
    fixed offset.  Returns one match per checkout block.
    """
    pattern = re.compile(
        r"(repository:[^\n]*" + re.escape(repository_fragment) + r"[^\n]*\n"
        r"\s*ref:\s*)([0-9a-f]{40})(?=\n\s*path:\s*" + re.escape(checkout) + r"\b)"
    )
    return list(pattern.finditer(text))


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--check",
        action="store_true",
        help="report drift without writing anything",
    )
    args = parser.parse_args()

    workflow = WORKFLOW.read_text()
    flake_text = FLAKE.read_text()
    lock = json.loads(LOCK.read_text())

    drift = []
    for checkout, (node, fragment) in TARGETS.items():
        locked = lock["nodes"][node]["locked"]["rev"]
        refs = {m.group(2) for m in workflow_refs(workflow, fragment, checkout)}
        if len(refs) != 1:
            raise SystemExit(
                f"{checkout}: expected exactly one pinned ref in ci.yml, found {sorted(refs)}"
            )
        ci_ref = refs.pop()
        try:
            remote = remote_main(checkout)
        except subprocess.CalledProcessError as error:
            raise SystemExit(f"{checkout}: cannot read remote main: {error}")
        agrees = locked == ci_ref
        print(f"{checkout}: lock={locked[:9]} ci={ci_ref[:9]} remote={remote[:9]}")
        if not agrees:
            raise SystemExit(
                f"{checkout}: lock {locked[:9]} != ci.yml {ci_ref[:9]} — "
                "run check-nested-pins.py to see which is wrong"
            )
        if locked != remote:
            drift.append((checkout, node, fragment, remote))

    if not drift:
        print("Pins already match every utility's remote main.")
        return 0
    if args.check:
        for checkout, _node, _fragment, remote in drift:
            print(f"DRIFT: {checkout} is pinned behind {remote[:9]}")
        return 1

    for checkout, node, fragment, remote in drift:
        # FIXED 2026-10-03 (audit R4-M-05): update ONLY the drifted input.
        # `nix flake update <input>` (nee `lock --update-input`, deprecated
        # alias on modern nix) has existed since Nix 2.4 — the "not
        # available on every nix" comment was stale. The old
        # `--recreate-lock-file` sat INSIDE this per-utility loop and
        # re-resolved every input (nixpkgs included) up to 3x per run:
        # non-hermetic, unreviewable lock churn as a side effect of a
        # utility repin. The `*-src` inputs are `flake = false` plain
        # sources, so updating one input moves exactly one node
        # (verified: nixpkgs/flake-utils/systems byte-identical).
        run(["nix", "flake", "update", node], cwd=ROOT)
        new_lock = json.loads(LOCK.read_text())
        new_rev = new_lock["nodes"][node]["locked"]["rev"]
        if new_rev != remote:
            print(
                f"{checkout}: lock resolved to {new_rev[:9]} but remote main is "
                f"{remote[:9]}; leaving the lock as resolved and re-running",
                file=sys.stderr,
            )
        before = workflow.count(new_rev)
        workflow = re.sub(
            r"(repository:[^\n]*" + re.escape(fragment) + r"[^\n]*\n\s*ref:\s*)[0-9a-f]{40}",
            lambda m: m.group(1) + new_rev,
            workflow,
        )
        after = workflow.count(new_rev)
        if after == before:
            raise SystemExit(f"{checkout}: no workflow ref was rewritten")
        # Keep the crateVersion fallbacks in step with the new pins.
        # FIXED 2026-10-03 (audit R4-M-12, three defects): (1) the old
        # pattern built `draconsyncSrc`, which never matched
        # `draconSyncSrc` — the rewrite was a silent no-op; (2) the
        # version came from the LIVE worktree manifest, contradicting
        # this script's own "REMOTE main, never a local worktree" rule;
        # (3) the version regex read the first `^version =` in the file
        # (M-11 class). The version now comes from the pinned rev,
        # [package]-scoped. A manifest that cannot be read degrades to
        # warn-and-keep (aborting here would leave the already-rewritten
        # lock split from the workflow; a stale fallback is the
        # pre-existing state, a wrong one would be new).
        try:
            manifest_text = run(
                ["git", "show", f"{new_rev}:Cargo.toml"], cwd=ROOT / checkout
            )
        except subprocess.CalledProcessError as error:
            print(
                f"{checkout}: cannot read Cargo.toml at {new_rev[:9]} ({error}); "
                "leaving the crateVersion fallback unchanged",
                file=sys.stderr,
            )
        else:
            version = package_version_from_manifest(manifest_text)
            if version is None:
                print(
                    f"{checkout}: no [package] version at {new_rev[:9]}; "
                    "leaving the crateVersion fallback unchanged",
                    file=sys.stderr,
                )
            else:
                flake_text = rewrite_crate_fallback(
                    flake_text, FLAKE_SRC_VAR[checkout], version
                )
        print(f"{checkout}: repinned to {new_rev[:9]} ({after} workflow refs)")

    WORKFLOW.write_text(workflow)
    FLAKE.write_text(flake_text)
    print("Re-pinned. Verify with: python3 scripts/check-nested-pins.py --check-local")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
