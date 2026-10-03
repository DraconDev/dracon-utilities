"""Hermetic regressions for the parent audit gates."""
import importlib.util
import json
import contextlib
import io
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest
from unittest.mock import patch

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


    def test_checker_rejects_lock_metadata_matching_only_live_sources(self):
        spec = importlib.util.spec_from_file_location("pins", ROOT / "scripts/check-nested-pins.py")
        pins = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(pins)
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            workflow = []
            nodes = {}
            urls = []
            for utility, node in pins.SOURCES.items():
                repo = root / utility
                repo.mkdir()
                def git(*args):
                    return subprocess.check_output(
                        ["git", "-c", "core.hooksPath=/dev/null", "-c", "user.name=Audit Fixture",
                         "-c", "user.email=audit-fixture@invalid", *args], cwd=repo, text=True).strip()
                git("init", "--quiet")
                (repo / "Cargo.toml").write_text(f'[package]\nname="{utility}"\nversion="1.0.0"\n')
                if utility == "dracon-warden":
                    (repo / "src/security").mkdir(parents=True)
                    (repo / "src/security/Cargo.toml").write_text(
                        '[package]\nname="dracon-security"\nversion="1.0.0"\n')
                git("add", "--", "Cargo.toml", *( ["src/security/Cargo.toml"] if utility == "dracon-warden" else []))
                git("commit", "--quiet", "-m", "pinned metadata")
                revision = git("rev-parse", "HEAD")
                (repo / "Cargo.toml").write_text(f'[package]\nname="{utility}"\nversion="2.0.0"\n')
                workflow.append(f"      repository: DraconDev/{pins.GITHUB_REPOS[utility]}\n"
                                f"      ref: {revision}\n      path: {utility}\n")
                nodes[node] = {"locked": {"rev": revision}, "original": {"ref": "main"}}
                urls.append(f'url = "github:DraconDev/{pins.GITHUB_REPOS[utility]}/main";')
            (root / ".github/workflows").mkdir(parents=True)
            (root / ".github/workflows/ci.yml").write_text("\n".join(workflow))
            (root / "flake.nix").write_text("\n".join(urls))
            (root / "flake.lock").write_text(json.dumps({"nodes": nodes}))
            def lock(version):
                (root / "Cargo.lock").write_text("\n".join(
                    f'[[package]]\nname="{name}"\nversion="{version if name != "dracon-security" else "1.0.0"}"\n'
                    for name in pins.LOCK_PACKAGES))
            with patch.object(pins, "ROOT", root), patch("sys.argv", ["check-nested-pins.py"]), \
                    contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()) as errors:
                lock("2.0.0")
                with self.assertRaises(SystemExit) as failure:
                    pins.main()
                self.assertEqual(failure.exception.code, 1)
                self.assertIn("Cargo.lock version 2.0.0", errors.getvalue())
                lock("1.0.0")
                self.assertEqual(pins.main(), 0)


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


class LockedInstallerBuilds(unittest.TestCase):
    def test_install_sh_builds_are_locked(self):
        # ADDED 2026-10-02 (audit M11): install.sh is the deployment
        # path — every cargo build it runs must carry --locked, or the
        # installed binary drifts from the Cargo.lock/deny pin chain CI
        # tested. A stale lock must fail loudly, never resolve silently.
        lines = (ROOT / "install.sh").read_text().splitlines()
        builds = [line.strip() for line in lines
                  if "cargo build" in line and not line.strip().startswith("#")]
        self.assertGreater(len(builds), 0, "no cargo build lines found in install.sh")
        for line in builds:
            self.assertIn("--locked", line,
                          f"floating installer build (add --locked): {line}")


class InstallAtomicBinaryReplace(unittest.TestCase):
    def test_install_sh_replaces_live_binary_atomically(self):
        # ADDED 2026-10-03 (audit L12): the live binary must be
        # replaced by same-dir rename (atomic) — never `rm -f` + `cp`,
        # whose window exec-fails a concurrent git filter spawn and
        # wedges add/checkout mid-install.
        text = (ROOT / "install.sh").read_text()
        self.assertNotIn('rm -f ~/.local/bin/"$binary"', text,
                         "non-atomic live-binary remove is back (use temp+rename)")
        self.assertIn('mv -f "$tmp_bin" ~/.local/bin/"$binary"', text,
                      "atomic temp-then-rename of the live binary is missing")
        self.assertIn('cp "$resolved" "$tmp_bin"', text,
                      "staged copy into the same dir is missing")


if __name__ == "__main__":
    unittest.main()
