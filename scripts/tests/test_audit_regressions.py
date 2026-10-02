"""Hermetic regressions for the parent audit gates."""
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]


class SpecExitStatus(unittest.TestCase):
    def test_cargo_failures_cannot_report_success(self):
        for status, message in [(101, "error: could not compile fixture"),
                                (1, "test result: FAILED"), (0, "test result: ok")]:
            with self.subTest(status=status), tempfile.TemporaryDirectory() as tmp:
                root = Path(tmp)
                for utility in ("dracon-sync", "dracon-system", "dracon-warden"):
                    (root / utility / ".git").mkdir(parents=True)
                    (root / utility / "src").mkdir()
                    (root / utility / "Cargo.toml").touch()
                stubs = root / "bin"
                stubs.mkdir()
                shell = shutil.which("bash")
                cargo = stubs / "cargo"
                cargo.write_text(f'#!{shell}\ncase " $* " in *" --locked "*) ;; *) exit 99;; esac\n'
                                 f'if [ "$1" = test ]; then echo "{message}"; exit {status}; fi\nexit 0\n')
                cargo.chmod(0o755)
                python = stubs / "python3"
                python.write_text(f"#!{shell}\nexit 0\n")
                python.chmod(0o755)
                result = subprocess.run([shell, str(ROOT / "scripts/verify-spec.sh")],
                                        cwd=root, capture_output=True, text=True,
                                        env={**os.environ, "PATH": str(stubs) + ":" + os.environ["PATH"]})
                self.assertEqual(result.returncode == 0, status == 0, result.stdout)
                self.assertEqual("PASS: Core unit tests pass" in result.stdout, status == 0)


class PinnedMetadata(unittest.TestCase):
    def test_pinned_metadata_ignores_a_newer_live_manifest(self):
        spec = importlib.util.spec_from_file_location("pins", ROOT / "scripts/check-nested-pins.py")
        pins = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(pins)
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            def git(*args):
                return subprocess.check_output(["git", "-c", "core.hooksPath=/dev/null",
                                                "-c", "user.name=Audit Fixture",
                                                "-c", "user.email=audit-fixture@invalid", *args],
                                               cwd=root, text=True).strip()
            git("init", "--quiet")
            manifest = root / "Cargo.toml"
            manifest.write_text('[package]\nname="fixture"\nversion="1.0.0"\n')
            git("add", "--", "Cargo.toml")
            git("commit", "--quiet", "-m", "pinned fixture")
            revision = git("rev-parse", "HEAD")
            manifest.write_text('[package]\nname="fixture"\nversion="2.0.0"\n')
            self.assertEqual(pins.pinned_package(root, revision, "Cargo.toml"), ("fixture", "1.0.0"))


@unittest.skipUnless(shutil.which("nix"), "Nix is required for the source isolation regression")
class NixSourceIsolation(unittest.TestCase):
    def test_actual_flake_reference_excludes_ignored_private_files(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "target").mkdir()
            (root / "target/artifact").write_text("synthetic artifact")
            private = root / ".env.fixture"
            private.write_text("SYNTHETIC=not-a-real-secret\n")
            private.chmod(0o600)
            (root / ".gitignore").write_text("target/\n.env.fixture\n")
            (root / "flake.nix").write_text("{ outputs = { self }: {}; }\n")
            for args in [("init", "--quiet"), ("add", "--", "flake.nix", ".gitignore"),
                         ("commit", "--quiet", "-m", "fixture")]:
                subprocess.run(["git", "-c", "core.hooksPath=/dev/null", "-c", "user.name=Audit Fixture",
                                "-c", "user.email=audit-fixture@invalid", *args], cwd=root, check=True)
            reference = next(line for line in (ROOT / "scripts/check-flake.sh").read_text().splitlines()
                             if "flake = builtins.getFlake" in line)
            expression = ('let ' + reference + ' in { artifact = builtins.pathExists '
                          '(flake.outPath + "/target/artifact"); private = builtins.pathExists '
                          '(flake.outPath + "/.env.fixture"); }')
            result = subprocess.run(["nix", "eval", "--impure", "--json", "--expr", expression],
                                    cwd=root, capture_output=True, text=True, timeout=60)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(json.loads(result.stdout), {"artifact": False, "private": False})


if __name__ == "__main__":
    unittest.main()
