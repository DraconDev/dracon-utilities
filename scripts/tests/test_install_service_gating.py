#!/usr/bin/env python3
"""D4 regression test: install.sh must not touch service state it was not asked to.

`install.sh --help` promises:

    --upgrade   Stop services, install, restart (default: only restart if running)

The pre-D4 code gated the per-binary `systemctl --user stop` / `pkill -x` and the
following `start` on `NO_RESTART != true` alone, and the final `restart_service`
block restarted every ENABLED unit, so a plain `./install.sh` stopped every
service, killed its binary, and started it again — resurrecting a daemon the
operator had deliberately stopped.

This drives the REAL script end to end in a sandbox: a temp `HOME` and a
prepended stub directory whose `systemctl`, `pgrep`, `pkill` and `cargo` are
recording shims. Nothing touches the host's systemd user session or `cargo`, and
every service action the script takes is captured in a log file the test asserts
against. That makes the regression a behaviour test rather than a grep over the
script text.
"""

from __future__ import annotations

import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
INSTALL_SH = ROOT / "install.sh"

SERVICES = ("dracon-sync.service", "dracon-system-guard.service")

# The stub `systemctl` reports these services as ACTIVE and as already
# EXISTING on disk (i.e. an established installation).
STUB_SYSTEMCTL = r"""#!/usr/bin/env bash
# Recording stub for systemctl. $STUB_LOG gets one line per invocation.
printf 'systemctl %s\n' "$*" >> "$STUB_LOG"

# is-active <unit>
if [ "$1" = "--user" ] && [ "$2" = "is-active" ]; then
    for svc in ${STUB_ACTIVE_SERVICES}; do
        [ "$svc" = "$3" ] && exit 0
    done
    exit 3
fi

# list-unit-files (the script captures this once at the top)
if [ "$1" = "--user" ] && [ "$2" = "list-unit-files" ]; then
    for svc in ${STUB_EXISTING_SERVICES}; do
        echo "$svc enabled"
    done
    exit 0
fi

# list-units (the post-install "running from" verification)
if [ "$1" = "--user" ] && [ "$2" = "list-units" ]; then
    exit 0
fi

# is-enabled <unit>: every unit that exists is enabled on this rig.
if [ "$1" = "--user" ] && [ "$2" = "is-enabled" ]; then
    for svc in ${STUB_EXISTING_SERVICES}; do
        [ "$svc" = "$3" ] && exit 0
    done
    exit 1
fi

# Everything else (stop/start/restart/daemon-reload) is a no-op that succeeds.
exit 0
"""

STUB_PGREP = r"""#!/usr/bin/env bash
printf 'pgrep %s\n' "$*" >> "$STUB_LOG"
# A stale manual process is modelled as absent unless told otherwise.
exit 1
"""

STUB_PKILL = r"""#!/usr/bin/env bash
printf 'pkill %s\n' "$*" >> "$STUB_LOG"
exit 0
"""

# `cargo build --release` is stubbed: the test is about service gating, not
# compilation. It just materialises the binary the installer expects.
STUB_CARGO = r"""#!/usr/bin/env bash
printf 'cargo %s\n' "$*" >> "$STUB_LOG"
for arg in "$@"; do
    case "$arg" in
        --package|-p) next_is_pkg=1 ;;
        *)
            if [ "${next_is_pkg:-}" = 1 ]; then
                pkg=$arg; next_is_pkg=0
                mkdir -p "target/release"
                printf '#!/bin/sh\nexit 0\n' > "target/release/${pkg}"
                chmod +x "target/release/${pkg}"
            fi
            ;;
    esac
done
exit 0
"""


class InstallRun:
    """One hermetic `install.sh` run: sandbox HOME, stubbed side-effecting tools."""

    def __init__(self, root: Path, active: str, existing: str) -> None:
        self.root = root
        self.home = root / "home"
        self.home.mkdir(parents=True, exist_ok=True)
        self.stub_dir = root / "stubs"
        self.stub_dir.mkdir(parents=True, exist_ok=True)
        self.log = root / "stub.log"
        self.log.touch()

        self._write_stub("systemctl", STUB_SYSTEMCTL)
        self._write_stub("pgrep", STUB_PGREP)
        self._write_stub("pkill", STUB_PKILL)
        self._write_stub("cargo", STUB_CARGO)
        # `cargo` above writes into cwd, and the installer runs the build in
        # the crate subdirectory; a writable repo copy keeps the real tree
        # clean. The script only needs the three crate dirs to exist.
        self.env = {
            "HOME": str(self.home),
            "PATH": f"{self.stub_dir}:{os.environ.get('PATH', '')}",
            "STUB_LOG": str(self.log),
            "STUB_ACTIVE_SERVICES": active,
            "STUB_EXISTING_SERVICES": existing,
        }

    def _write_stub(self, name: str, body: str) -> None:
        p = self.stub_dir / name
        p.write_text(body)
        p.chmod(0o755)

    def run(self, *args: str, cwd: Path | None = None) -> subprocess.CompletedProcess:
        return subprocess.run(
            ["bash", str(INSTALL_SH), *args],
            cwd=str(cwd or ROOT),
            env={**self.env, "PATH": self.env["PATH"]},
            capture_output=True,
            text=True,
            timeout=300,
        )

    # --- log queries ---------------------------------------------------

    def calls(self, *verbs: str) -> list[str]:
        """Logged stub invocations whose first word is one of `verbs`."""
        out = []
        for line in self.log.read_text().splitlines():
            parts = line.split()
            if not parts:
                continue
            if any(parts[0] == v or parts[0:2] == ["systemctl", v] for v in verbs):
                out.append(line)
        return out

    def service_actions(self, verb: str, service: str) -> int:
        """How many times `systemctl --user <verb> <service>` was called."""
        return sum(1 for line in self.calls("systemctl") if line.split()[:3] == ["systemctl", "--user", verb] and service in line.split()[3:4])

    def pkill_calls(self, binary: str) -> int:
        return sum(1 for line in self.calls("pkill") if line.split()[1:2] == [f"-x"] and binary in line.split()[2:3])

    def all_lines(self) -> list[str]:
        return self.log.read_text().splitlines()


def sandbox(active: str, existing: str) -> tuple[tempfile.TemporaryDirectory, InstallRun]:
    tmp = tempfile.TemporaryDirectory()
    return tmp, InstallRun(Path(tmp.name), active=active, existing=existing)


class PlainInstallDoesNotTouchServices(unittest.TestCase):
    """A plain `./install.sh` must leave service state alone."""

    def setUp(self) -> None:
        self.tmp, self.run_obj = sandbox(active=" ".join(SERVICES), existing=" ".join(SERVICES))
        self.addCleanup(self.tmp.cleanup)

    def test_plain_install_never_stops_kills_or_starts(self) -> None:
        res = self.run_obj.run()
        self.assertEqual(res.returncode, 0, msg=f"installer failed:\n{res.stdout}\n{res.stderr}")
        for svc in SERVICES:
            self.assertEqual(
                self.run_obj.service_actions("stop", svc), 0, f"{svc} was stopped by a plain install"
            )
            self.assertEqual(
                self.run_obj.service_actions("start", svc), 0, f"{svc} was started by a plain install"
            )
        # Running services are still restarted (the "only restart if running"
        # promise); the point is that nothing was quiesced to get there.
        for svc in SERVICES:
            self.assertEqual(
                self.run_obj.service_actions("restart", svc), 1, f"{svc} should be restarted when running"
            )
        self.assertEqual(self.run_obj.pkill_calls("dracon-sync"), 0, "plain install pkill-ed dracon-sync")
        self.assertEqual(self.run_obj.pkill_calls("dracon-system"), 0, "plain install pkill-ed dracon-system")

    def test_plain_install_with_binaries_only_does_not_touch_services(self) -> None:
        res = self.run_obj.run("--binaries-only")
        self.assertEqual(res.returncode, 0, msg=f"installer failed:\n{res.stdout}\n{res.stderr}")
        for svc in SERVICES:
            self.assertEqual(self.run_obj.service_actions("stop", svc), 0)
            self.assertEqual(self.run_obj.service_actions("start", svc), 0)
        self.assertEqual(
            [c for c in self.run_obj.calls("pkill")],
            [],
            "--binaries-only must not kill any process",
        )


class UpgradeRespectsRunState(unittest.TestCase):
    """`--upgrade` stops what is running — and only restarts what it stopped."""

    def test_upgrade_stops_and_restarts_an_active_service(self) -> None:
        tmp, run_obj = sandbox(active="dracon-sync.service", existing=" ".join(SERVICES))
        self.addCleanup(tmp.cleanup)
        res = run_obj.run("--upgrade")
        self.assertEqual(res.returncode, 0, msg=f"installer failed:\n{res.stdout}\n{res.stderr}")
        # `--upgrade` stops early (pre-install block) and again per binary,
        # so the count is "at least once", not exactly once.
        self.assertGreaterEqual(
            run_obj.service_actions("stop", "dracon-sync.service"), 1, "--upgrade must stop a running service"
        )
        self.assertEqual(
            run_obj.service_actions("start", "dracon-sync.service"), 1, "--upgrade must bring it back exactly once"
        )
        # The guard is not active: --upgrade must still clean up processes.
        self.assertGreaterEqual(run_obj.pkill_calls("dracon-sync"), 1, "--upgrade must pkill the running binary")

    def test_upgrade_does_not_resurrect_an_operator_stopped_service(self) -> None:
        # dracon-sync is enabled on disk but NOT running: the operator
        # stopped it. The install must not start it.
        tmp, run_obj = sandbox(active="", existing=" ".join(SERVICES))
        self.addCleanup(tmp.cleanup)
        res = run_obj.run("--upgrade")
        self.assertEqual(res.returncode, 0, msg=f"installer failed:\n{res.stdout}\n{res.stderr}")
        self.assertEqual(
            run_obj.service_actions("stop", "dracon-sync.service"), 0, "nothing to stop when it is already stopped"
        )
        self.assertEqual(
            run_obj.service_actions("start", "dracon-sync.service"),
            0,
            "--upgrade must not resurrect a service the operator stopped",
        )
        self.assertEqual(
            run_obj.service_actions("restart", "dracon-sync.service"),
            0,
            "the final restart block must leave a deliberately stopped service alone",
        )

    def test_upgrade_with_no_restart_flag_does_not_quiesce(self) -> None:
        tmp, run_obj = sandbox(active=" ".join(SERVICES), existing=" ".join(SERVICES))
        self.addCleanup(tmp.cleanup)
        res = run_obj.run("--upgrade", "--no-restart")
        self.assertEqual(res.returncode, 0, msg=f"installer failed:\n{res.stdout}\n{res.stderr}")
        for svc in SERVICES:
            self.assertEqual(run_obj.service_actions("stop", svc), 0)
            self.assertEqual(run_obj.service_actions("start", svc), 0)


class FirstInstallBehaviourIsUnchanged(unittest.TestCase):
    """D4 must not alter what a first install does."""

    def test_fresh_install_leaves_absent_units_alone(self) -> None:
        # No units on disk yet and nothing active: the shipped installer
        # reports "not found" for these and starts nothing (it never runs
        # `systemctl --user enable`). D4 must not change that — the help
        # text promises "only restart if running", and there is no
        # established daemon to restart.
        tmp, run_obj = sandbox(active="", existing="")
        self.addCleanup(tmp.cleanup)
        res = run_obj.run()
        self.assertEqual(res.returncode, 0, msg=f"installer failed:\n{res.stdout}\n{res.stderr}")
        for svc in SERVICES:
            self.assertEqual(run_obj.service_actions("stop", svc), 0)
            self.assertEqual(run_obj.service_actions("start", svc), 0)
            self.assertEqual(
                run_obj.service_actions("restart", svc),
                0,
                f"a unit that did not exist before the install is not started ({svc})",
            )


if __name__ == "__main__":
    unittest.main(verbosity=2)
