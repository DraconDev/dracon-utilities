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

HERMETICITY (learned the hard way, 2026-09-28): `install.sh` scans `PATH` for
"shadowing" `dracon-*` binaries and removes anything outside
`$HOME/.local/bin`. A first version of this file inherited the caller's `PATH`,
which contained the operator's REAL `~/.local/bin` while `HOME` pointed at the
sandbox — so the script deleted the live installed `dracon-warden` and broke
every filtered git operation on the rig. The stub `PATH` below therefore
contains only the stub directory and the system directories, and
`test_the_sandbox_cannot_see_the_real_install` pins that.
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
DOCTOR_SH = ROOT / "doctor.sh"

SERVICES = ("dracon-sync.service", "dracon-system-guard.service")


def copy_unit_destinations() -> list[str]:
    """Every destination path the installer's `copy_unit` calls copy, in order.

    Same parser discipline as `copy_unit_sources`, second field instead of
    first. `DoctorParity` uses it so doctor.sh cannot silently fall behind the
    installer's copy list again (audit F127, 2026-10-09).
    """
    dests: list[str] = []
    for line in INSTALL_SH.read_text().splitlines():
        stripped = line.strip()
        if not stripped.startswith("copy_unit "):
            continue
        parts = stripped.split()
        if len(parts) >= 3:
            dests.append(parts[2])
    return dests


def copy_unit_sources() -> list[str]:
    """Every source path the installer's `copy_unit` calls copy, in file order.

    Parsed from the real install.sh so the sandbox fixture can never drift from
    the installer's copy list again: when audit M8 added the watchdog units +
    scripts (2026-10-03), the hardcoded fixture still created only
    `<crate>/<crate>.service`, and this suite went 11/14 red while nothing ran
    it. The parser is deliberately dumb — one source per `copy_unit <src> <dst>`
    line, first word after the call name — and the definition line
    (`copy_unit() {`) does not match because the next character is `(`.
    """
    sources: list[str] = []
    for line in INSTALL_SH.read_text().splitlines():
        stripped = line.strip()
        if not stripped.startswith("copy_unit "):
            continue
        parts = stripped.split()
        if len(parts) >= 2:
            sources.append(parts[1])
    return sources

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

# show <unit> -p ExecStart --value  (install.sh's live-unit guard)
if [ "$1" = "--user" ] && [ "$2" = "show" ] && [ "$4" = "-p" ] && [ "$5" = "ExecStart" ]; then
    [ -n "${UNIT_EXEC_PATH:-}" ] && echo "{ path=$UNIT_EXEC_PATH ; argv[]=$UNIT_EXEC_PATH daemon ; }"
    exit 0
fi

# getent is stubbed by the sandbox when a test needs a specific "real" home.

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


# The sandbox PATH: stub directory + the directories coreutils live in on
# NixOS. Deliberately NOT the caller's PATH: install.sh removes "shadowing"
# dracon-* binaries from every PATH dir that is not $HOME/.local/bin, so
# inheriting the real PATH (which holds the operator's installed binaries)
# would let a test mutate the rig.
SANDBOX_PATH_DIRS = ("/run/current-system/sw/bin", "/usr/bin", "/bin")


class InstallRun:
    """One hermetic `install.sh` run: sandbox repo+HOME, stubbed tools.

    The script is a BYTE-IDENTICAL COPY of the real one inside a sandbox
    tree, because `install.sh` starts with `cd "$(dirname "$0")"` and then
    builds in `./dracon-<name>/target/release/`. Running the real file in
    place made the stub `cargo` drop fake 17-byte `dracon-*` binaries into
    the REAL per-crate target directories — a trap for the next
    `install.sh`, which resolves `$subdir/target/release/$binary` first and
    would have installed a no-op "daemon". The copy is verified against the
    real file's hash in `test_the_sandbox_script_is_a_verified_copy`, so the
    run is still the real logic.
    """

    def __init__(self, root: Path, active: str, existing: str) -> None:
        self.root = root
        self.home = root / "home"
        self.home.mkdir(parents=True, exist_ok=True)
        self.stub_dir = root / "stubs"
        self.stub_dir.mkdir(parents=True, exist_ok=True)
        self.log = root / "stub.log"
        self.log.touch()

        # Sandbox repo tree: the script under test plus the crate dirs it
        # builds in. No source is copied in — the stub `cargo` only needs
        # the directories to exist, but install.sh refuses to start when a
        # utility directory has no manifest, so each gets a minimal one.
        self.repo = root / "repo"
        self.repo.mkdir(parents=True, exist_ok=True)
        self.script = self.repo / "install.sh"
        self.script.write_bytes(INSTALL_SH.read_bytes())
        self.script.chmod(0o755)
        for crate in ("dracon-sync", "dracon-system", "dracon-warden"):
            crate_dir = self.repo / crate
            crate_dir.mkdir(parents=True, exist_ok=True)
            (crate_dir / "Cargo.toml").write_text(
                f'[package]\nname = "{crate}"\nversion = "0.0.0"\nedition = "2021"\n'
            )
        # Materialize every unit/script the installer copies, parsed from the
        # real install.sh (copy_unit_sources above) so the fixture follows the
        # installer instead of a hardcoded list that rots. The real files carry
        # the warden encryption filter, so the sandbox uses inert stubs — the
        # installer only cp's them and chmod's the notify scripts.
        for src in copy_unit_sources():
            target = self.repo / src
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_text(f"# sandbox stub for {src}\n")

        self._write_stub("systemctl", STUB_SYSTEMCTL)
        self._write_stub("pgrep", STUB_PGREP)
        self._write_stub("pkill", STUB_PKILL)
        self._write_stub("cargo", STUB_CARGO)
        self.env = {
            "HOME": str(self.home),
            "PATH": os.pathsep.join((str(self.stub_dir), *SANDBOX_PATH_DIRS)),
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
            ["bash", str(self.script), *args],
            cwd=str(cwd or self.repo),
            env=dict(self.env),
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


class InstallerCopyListParity(unittest.TestCase):
    """The sandbox must materialize everything the installer copies.

    Structural guard: the fixture parses install.sh's copy list, so the two
    failure modes that survive a future installer change are a source missing
    from the REAL tree and a source the parser missed. This class pins both.
    """

    def test_every_copy_source_exists_in_the_real_repo(self) -> None:
        missing = [s for s in copy_unit_sources() if not (ROOT / s).is_file()]
        self.assertEqual(
            missing, [], f"install.sh copies sources missing from the repo: {missing}"
        )

    def test_the_sandbox_materializes_every_copy_source(self) -> None:
        tmp, run_obj = sandbox(active="", existing="")
        self.addCleanup(tmp.cleanup)
        missing = [s for s in copy_unit_sources() if not (run_obj.repo / s).is_file()]
        self.assertEqual(
            missing, [], f"sandbox fixture lacks installer copy sources: {missing}"
        )

    def test_the_parser_finds_the_m8_watchdog_sources(self) -> None:
        # Pin that the parser sees the M8 additions — a silent parse regression
        # would recreate the 11/14 red suite.
        sources = copy_unit_sources()
        self.assertIn("dracon-system/dracon-system-guard.service", sources)
        self.assertIn("dracon-sync/dracon-sync-watchdog.timer", sources)
        self.assertGreaterEqual(
            len([s for s in sources if "watchdog" in s]),
            8,
            "the M8 watchdog units + scripts must be part of the parsed copy list",
        )


class PreFlightGuards(unittest.TestCase):
    """install.sh must fail before ANY mutation when a copy source is missing.

    Added 2026-10-08 (audit F105): the pre-M8 prerequisite loop checked only
    Cargo.toml, so a partial checkout installed new binaries first and died on
    the fatal `copy_unit` afterwards — a half-installed machine.
    """

    def _drop_one_copy_source(self, run_obj: InstallRun) -> str:
        src = copy_unit_sources()[0]
        (run_obj.repo / src).unlink()
        return src

    def test_a_missing_copy_source_fails_before_any_mutation(self) -> None:
        tmp, run_obj = sandbox(active="", existing="")
        self.addCleanup(tmp.cleanup)
        missing = self._drop_one_copy_source(run_obj)
        res = run_obj.run()
        self.assertNotEqual(res.returncode, 0, "a missing copy source must abort the install")
        self.assertIn(missing, res.stdout + res.stderr, "the error must name the missing source")
        self.assertIn(
            "missing from this checkout",
            res.stdout + res.stderr,
            "the pre-flight error must identify itself",
        )
        # Fail-fast proof: the installer must not even have built or shipped a
        # binary. The stub cargo is the only thing that would log "cargo" here.
        self.assertEqual(
            [c for c in run_obj.calls("cargo")], [],
            "the pre-flight must reject the run before any binary is built or installed",
        )

    def test_the_preflight_list_tracks_the_installer(self) -> None:
        # The check parses its own copy list; pin that it catches MORE than the
        # two manifest units (i.e. it really parses the watchdog M8 additions)
        # by removing a watchdog script and asserting the same abort.
        tmp, run_obj = sandbox(active="", existing="")
        self.addCleanup(tmp.cleanup)
        watchdog = next(s for s in copy_unit_sources() if "watchdog.sh" in s)
        (run_obj.repo / watchdog).unlink()
        res = run_obj.run()
        self.assertNotEqual(res.returncode, 0)
        self.assertIn(watchdog, res.stdout + res.stderr)
        self.assertEqual([c for c in run_obj.calls("cargo")], [])


class SandboxIsHermetic(unittest.TestCase):
    """The suite must not be able to mutate the operator's real installation."""

    def setUp(self) -> None:
        self.tmp, self.run_obj = sandbox(active="", existing="")
        self.addCleanup(self.tmp.cleanup)

    def test_the_sandbox_cannot_see_the_real_install(self) -> None:
        real_bin = Path(os.path.expanduser("~/.local/bin"))
        self.assertNotIn(
            str(real_bin),
            self.run_obj.env["PATH"].split(os.pathsep),
            "the sandbox PATH must not contain the operator's real ~/.local/bin: "
            "install.sh deletes 'shadowing' dracon-* binaries from any PATH dir "
            "that is not $HOME/.local/bin, and a test with the real bin dir on "
            "PATH deletes the live installation (incident 2026-09-28)",
        )
        self.assertNotEqual(
            self.run_obj.env["HOME"],
            os.path.expanduser("~"),
            "the sandbox HOME must differ from the real home",
        )

    def test_a_sandbox_run_leaves_the_real_bin_untouched(self) -> None:
        # Plant a canary in a PATH dir the sandbox DOES see and confirm the
        # script only ever touches files under the sandbox HOME.
        canary_dir = self.run_obj.stub_dir
        canary = canary_dir / "dracon-warden"
        canary.write_text("#!/bin/sh\nexit 0\n")
        canary.chmod(0o755)
        real_bin = Path(os.path.expanduser("~/.local/bin"))
        before = sorted(p.name for p in real_bin.glob("dracon-*")) if real_bin.is_dir() else []
        res = self.run_obj.run()
        self.assertEqual(res.returncode, 0, msg=f"installer failed:\n{res.stdout}\n{res.stderr}")
        after = sorted(p.name for p in real_bin.glob("dracon-*")) if real_bin.is_dir() else []
        self.assertEqual(
            before, after, "a sandboxed install.sh run must not add or remove anything in the real ~/.local/bin"
        )


class DeletionGuards(unittest.TestCase):
    """install.sh must never delete a live or the operator's own installation.

    The 2026-09-28 incident: running the installer with a HOME that is not
    the operator's made the real `~/.local/bin` look like a "shadowing" PATH
    directory, and its shadow-scan deleted the live `dracon-warden`, which
    broke every filtered git operation on the rig. install.sh now has two
    guards; these tests pin both, hermetically.
    """

    def _canary(self, directory: Path, name: str = "dracon-warden") -> Path:
        p = directory / name
        p.write_text("#!/bin/sh\nexit 0\n")
        p.chmod(0o755)
        return p

    def test_the_operators_real_bin_dir_is_never_a_shadowing_target(self) -> None:
        tmp, run_obj = sandbox(active="", existing="")
        self.addCleanup(tmp.cleanup)
        # `getent` reports a "real" home that is NOT the sandbox HOME; its
        # ~/.local/bin is on PATH and holds a live-looking binary. Without
        # the guard the shadow-scan deletes it (incident 2026-09-28).
        real_home = Path(tmp.name) / "real-home"
        real_bin = real_home / ".local" / "bin"
        real_bin.mkdir(parents=True, exist_ok=True)
        canary = self._canary(real_bin)
        run_obj.env["PATH"] = os.pathsep.join((str(run_obj.stub_dir), str(real_bin), *SANDBOX_PATH_DIRS))
        getent = run_obj.stub_dir / "getent"
        getent.write_text(f'#!/usr/bin/env bash\necho "dracon:x:1000:1000:Test:{real_home}:/bin/bash"\n')
        getent.chmod(0o755)

        res = run_obj.run()
        self.assertEqual(res.returncode, 0, msg=f"installer failed:\n{res.stdout}\n{res.stderr}")
        self.assertTrue(
            canary.exists(),
            "the operator's real ~/.local/bin must never be treated as a shadowing "
            "directory (incident 2026-09-28: this deleted the live dracon-warden)",
        )

    def test_a_binary_a_live_unit_executes_is_never_removed(self) -> None:
        tmp, run_obj = sandbox(active="", existing=" ".join(SERVICES))
        self.addCleanup(tmp.cleanup)
        # The daemon binary a live unit is executing lives in a plain PATH
        # dir (not the install target) — e.g. /usr/local/bin/dracon-sync.
        foreign_bin = Path(tmp.name) / "usr-local-bin"
        foreign_bin.mkdir(parents=True, exist_ok=True)
        canary = self._canary(foreign_bin, "dracon-sync")
        run_obj.env["PATH"] = os.pathsep.join((str(run_obj.stub_dir), str(foreign_bin), *SANDBOX_PATH_DIRS))
        run_obj.env["UNIT_EXEC_PATH"] = str(canary)

        res = run_obj.run()
        self.assertEqual(res.returncode, 0, msg=f"installer failed:\n{res.stdout}\n{res.stderr}")
        self.assertTrue(
            canary.exists(),
            "a binary a live systemd unit executes must never be removed as a "
            "shadowing stale copy — the unit would fail to restart",
        )
        self.assertIn("a live systemd unit runs it", res.stdout, msg=res.stdout)

    def test_a_unit_exec_path_containing_a_space_is_not_truncated(self) -> None:
        # The guard compares the candidate path against the unit's ExecStart
        # verbatim, so the extraction must not cut a path at a space (a
        # truncated comparison silently matches nothing and the guard fails
        # OPEN, i.e. the binary gets deleted).
        tmp, run_obj = sandbox(active="", existing=" ".join(SERVICES))
        self.addCleanup(tmp.cleanup)
        spaced_dir = Path(tmp.name) / "bin with spaces"
        spaced_dir.mkdir(parents=True, exist_ok=True)
        canary = self._canary(spaced_dir, "dracon-system")
        run_obj.env["PATH"] = os.pathsep.join((str(run_obj.stub_dir), str(spaced_dir), *SANDBOX_PATH_DIRS))
        run_obj.env["UNIT_EXEC_PATH"] = str(canary)

        res = run_obj.run()
        self.assertEqual(res.returncode, 0, msg=f"installer failed:\n{res.stdout}\n{res.stderr}")
        self.assertTrue(
            canary.exists(),
            "an ExecStart path containing a space must still be recognised as live",
        )

    def test_a_stale_foreign_copy_is_still_removed(self) -> None:
        # The guards must not turn the feature off: a genuine stale copy in
        # a foreign PATH dir is still cleaned up.
        tmp, run_obj = sandbox(active="", existing=" ".join(SERVICES))
        self.addCleanup(tmp.cleanup)
        foreign_bin = Path(tmp.name) / "usr-local-bin"
        foreign_bin.mkdir(parents=True, exist_ok=True)
        canary = self._canary(foreign_bin, "dracon-sync")
        run_obj.env["PATH"] = os.pathsep.join((str(run_obj.stub_dir), str(foreign_bin), *SANDBOX_PATH_DIRS))

        res = run_obj.run()
        self.assertEqual(res.returncode, 0, msg=f"installer failed:\n{res.stdout}\n{res.stderr}")
        self.assertFalse(
            canary.exists(),
            "a stale foreign copy that no unit runs must still be removed — the "
            "shadow-scan feature is unchanged",
        )
        self.assertIn("Removed shadowing binary", res.stdout, msg=res.stdout)


    def test_the_sandbox_script_is_a_verified_copy(self) -> None:
        # The suite runs install.sh from a sandbox copy (so the stub cargo
        # cannot drop fake binaries into the real per-crate target dirs), so
        # the copy must be provably the same file.
        tmp, run_obj = sandbox(active="", existing="")
        self.addCleanup(tmp.cleanup)
        self.assertEqual(
            run_obj.script.read_bytes(),
            INSTALL_SH.read_bytes(),
            "the sandbox install.sh must be a byte-identical copy of the real one",
        )

    def test_a_sandbox_run_writes_nothing_into_the_real_repo(self) -> None:
        tmp, run_obj = sandbox(active="", existing="")
        self.addCleanup(tmp.cleanup)
        before = {
            crate: sorted(
                p.name
                for p in (ROOT / crate / "target" / "release").glob("dracon-*")
            )
            if (ROOT / crate / "target" / "release").is_dir()
            else []
            for crate in ("dracon-sync", "dracon-system", "dracon-warden")
        }
        res = run_obj.run()
        self.assertEqual(res.returncode, 0, msg=f"installer failed:\n{res.stdout}\n{res.stderr}")
        after = {
            crate: sorted(
                p.name
                for p in (ROOT / crate / "target" / "release").glob("dracon-*")
            )
            if (ROOT / crate / "target" / "release").is_dir()
            else []
            for crate in ("dracon-sync", "dracon-system", "dracon-warden")
        }
        self.assertEqual(
            before,
            after,
            "a sandboxed run must not create files in the real crates' target dirs — "
            "install.sh resolves $subdir/target/release/$binary first, so a fake binary "
            "left there is what the next real install would install",
        )


class DoctorParity(unittest.TestCase):
    """doctor.sh must report every unit and script install.sh installs.

    Added 2026-10-09 (audit F127). M8 (2026-10-02) made install.sh copy 8
    unit files and 3 watchdog scripts, but doctor.sh still looped over two
    services, so a failed `copy_unit` or a disabled watchdog timer produced no
    signal at all — while uninstall.sh already documents the consequence of
    missing timers (they fire every 2 min against removed units). The two files
    are now pinned to each other: this test parses the installer's copy list
    and fails when doctor.sh stops mentioning any installed unit or script.
    """

    def _doctor_text(self) -> str:
        return DOCTOR_SH.read_text()

    def test_every_installed_unit_is_covered_by_the_doctor(self) -> None:
        doctor = self._doctor_text()
        units = sorted(
            Path(dest).name
            for dest in copy_unit_destinations()
            if dest.endswith((".service", ".timer"))
        )
        self.assertGreaterEqual(len(units), 8, f"expected the M8 unit set, parsed: {units}")
        missing = [u for u in units if u not in doctor]
        self.assertEqual(
            missing,
            [],
            f"doctor.sh does not cover units install.sh installs: {missing}",
        )

    def test_every_installed_watchdog_script_is_covered(self) -> None:
        doctor = self._doctor_text()
        scripts = sorted(
            Path(dest).name
            for dest in copy_unit_destinations()
            if dest.endswith(".sh")
        )
        self.assertGreaterEqual(len(scripts), 3, f"expected the 3 notify scripts, parsed: {scripts}")
        missing = [s for s in scripts if s not in doctor]
        self.assertEqual(
            missing,
            [],
            f"doctor.sh does not cover the watchdog scripts install.sh installs: {missing}",
        )

    def test_oneshot_units_are_not_required_to_be_active(self) -> None:
        """The watchdog .service files are Type=oneshot, started by their timer.

        On a healthy machine they are inactive between runs, so a doctor that
        demands `is-active` for them emits a permanent false WARN — the same
        defect class that audits F79 and F10 removed from other sections.
        """
        doctor = self._doctor_text()
        self.assertIn("Type=oneshot", doctor)
        for unit in (
            "dracon-sync-watchdog.service",
            "dracon-freeze-watchdog.service",
            "dracon-system-guard-watchdog.service",
        ):
            src = ROOT / "dracon-sync" / unit
            if not src.is_file():
                src = ROOT / "dracon-system" / unit
            self.assertTrue(src.is_file(), f"missing unit source for {unit}")
            self.assertIn("Type=oneshot", src.read_text(), f"{unit} must stay a oneshot")


if __name__ == "__main__":
    unittest.main(verbosity=2)
