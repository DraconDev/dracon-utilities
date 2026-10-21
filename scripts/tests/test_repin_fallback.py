#!/usr/bin/env python3
"""Unit tests for the repin-nested-sources.py crateVersion fallback path.

Audit R4-M-12: the fallback rewrite had three defects — a src-variable
pattern that never matched (`draconsyncSrc` vs `draconSyncSrc`), a
version read from the live worktree instead of the pinned rev, and an
unscoped first-`^version =` regex. The rewrite is pure (flake text in,
flake text out), so it is tested directly; the `git show <rev>` call
stays in main() and is covered by inspection + the live --check path.
"""

from __future__ import annotations

import importlib.util
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))


def _load_repin():
    # The script filename has dashes, so it cannot be imported by name.
    spec = importlib.util.spec_from_file_location(
        "repin_nested_sources", ROOT / "scripts" / "repin-nested-sources.py"
    )
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


repin = _load_repin()

FLAKE_SNIPPET = """\
        packages = {
          dracon-sync = pkgs.rustPlatform.buildRustPackage (commonArgs // {
            pname = "dracon-sync";
            version = crateVersion "0.113.88" draconSyncSrc;
          });
          dracon-system = pkgs.rustPlatform.buildRustPackage (commonArgs // {
            pname = "dracon-system";
            version = crateVersion "0.112.41" draconSystemSrc;
          });
          dracon-warden = pkgs.rustPlatform.buildRustPackage (commonArgs // {
            pname = "dracon-warden";
            version = crateVersion "0.113.14" draconWardenSrc;
          });
        };
"""


class PackageVersionTest(unittest.TestCase):
    def test_package_scoped_beats_workspace_package_above(self):
        manifest = (
            '[workspace.package]\nversion = "9.9.9"\n\n'
            '[package]\nname = "dracon-sync"\nversion = "0.113.93"\n'
        )
        self.assertEqual(repin.package_version_from_manifest(manifest), "0.113.93")

    def test_no_package_section_is_none(self):
        self.assertIsNone(repin.package_version_from_manifest('[workspace]\n'))

    def test_package_without_version_is_none(self):
        self.assertIsNone(repin.package_version_from_manifest('[package]\nname = "x"\n'))

    def test_version_after_package_in_other_sections_ignored(self):
        manifest = (
            '[package]\nname = "x"\nversion = "1.2.3"\n\n'
            '[dependencies]\nfoo = "4.5.6"\n'
        )
        self.assertEqual(repin.package_version_from_manifest(manifest), "1.2.3")


class RewriteFallbackTest(unittest.TestCase):
    def test_rewrites_each_src_var_exactly_once(self):
        for var, version in [
            ("draconSyncSrc", "0.113.93"),
            ("draconSystemSrc", "0.112.44"),
            ("draconWardenSrc", "0.113.15"),
        ]:
            with self.subTest(var=var):
                out = repin.rewrite_crate_fallback(FLAKE_SNIPPET, var, version)
                self.assertEqual(out.count(f'crateVersion "{version}" {var};'), 1)
                # The other two fallbacks are untouched.
                for other, other_version in [
                    ("draconSyncSrc", "0.113.88"),
                    ("draconSystemSrc", "0.112.41"),
                    ("draconWardenSrc", "0.113.14"),
                ]:
                    if other != var:
                        self.assertIn(f'crateVersion "{other_version}" {other};', out)

    def test_old_case_mangled_var_matches_nothing(self):
        # Regression for the silent no-op: the old builder produced
        # `draconsyncSrc`, which must NOT match `draconSyncSrc`.
        with self.assertRaises(SystemExit):
            repin.rewrite_crate_fallback(FLAKE_SNIPPET, "draconsyncSrc", "1.0.0")

    def test_unknown_var_is_fatal(self):
        with self.assertRaises(SystemExit):
            repin.rewrite_crate_fallback(FLAKE_SNIPPET, "draconNopeSrc", "1.0.0")

    def test_real_flake_matches_every_configured_var(self):
        # Structural guard: if flake.nix is restructured so a pattern
        # stops matching, this fails instead of drifting silently.
        flake_text = (ROOT / "flake.nix").read_text()
        for checkout, var in repin.FLAKE_SRC_VAR.items():
            with self.subTest(checkout=checkout):
                out = repin.rewrite_crate_fallback(flake_text, var, "9.9.9-test")
                self.assertIn(f'crateVersion "9.9.9-test" {var};', out)


class RemoteSelection(unittest.TestCase):
    """Which remote the pin is read from.

    FIXED 2026-10-21: the remote used to be "the first remote that is not
    called origin", decided by `git remote`'s alphabetical listing. For
    dracon-system that was gitlab, so a repin pinned gitlab's main — a
    revision the CI checkout (github.com) may not have at all, and one that
    lagged what the daemon had already published to GitHub. The pin's
    consumers (the CI checkout and the flake.lock input) both fetch from the
    repository the CI names, so that repository decides.
    """

    def test_remote_repo_spellings(self):
        for url, expected in [
            (
                "git@github.com:DraconDev/dracon-sync.git",
                ("github.com", "DraconDev/dracon-sync"),
            ),
            (
                "https://github.com/DraconDev/dracon-sync",
                ("github.com", "DraconDev/dracon-sync"),
            ),
            (
                "https://github.com/DraconDev/dracon-sync.git",
                ("github.com", "DraconDev/dracon-sync"),
            ),
            (
                "ssh://git@gitlab.com/DraconDev/dracon-sync.git",
                ("gitlab.com", "DraconDev/dracon-sync"),
            ),
            (
                "git@gitlab.com:DraconDev/dracon-sync.git",
                ("gitlab.com", "DraconDev/dracon-sync"),
            ),
        ]:
            with self.subTest(url=url):
                self.assertEqual(repin.remote_repo(url), expected)

    def test_same_path_on_two_forges_is_not_the_same_repository(self):
        # GitHub and GitLab both host DraconDev/<same-repo>; a path-only
        # comparison would accept either, which is exactly the defect this
        # change fixes.
        self.assertNotEqual(
            repin.remote_repo("git@github.com:DraconDev/x.git"),
            repin.remote_repo("git@gitlab.com:DraconDev/x.git"),
        )

    def test_ci_repository_is_read_from_the_workflow(self):
        workflow = (ROOT / ".github/workflows/ci.yml").read_text()
        for checkout, fragment in repin.TARGETS.values():
            with self.subTest(checkout=checkout):
                host, path = repin.ci_repository(workflow, fragment)
                self.assertEqual(host, "github.com")
                self.assertIn(fragment, path)

    def test_first_remote_holds_the_ci_repository(self):
        workflow = (ROOT / ".github/workflows/ci.yml").read_text()
        for checkout, (_node, fragment) in repin.TARGETS.items():
            with self.subTest(checkout=checkout):
                repo_path = ROOT / checkout
                if not (repo_path / ".git").exists():
                    self.skipTest(f"{checkout} is not checked out here")
                wanted = repin.ci_repository(workflow, fragment)
                ordered = repin.ordered_remotes(repo_path, wanted)
                self.assertTrue(ordered, "no remote found")
                url = repin.run(
                    ["git", "remote", "get-url", ordered[0]], cwd=repo_path
                )
                self.assertEqual(
                    repin.remote_repo(url),
                    wanted,
                    f"{checkout}: the pin would be read from {url}, not the "
                    "repository the CI checks out",
                )

    def test_missing_ci_repository_is_fatal(self):
        # Pinning from a forge the CI does not use would break both the CI
        # checkout and the flake input, so an absent remote must stop the
        # repin rather than fall back to another one.
        with self.assertRaises(SystemExit):
            repin.ordered_remotes(
                ROOT / "dracon-sync", ("github.com", "SomeoneElse/Not-A-Repo")
            )


if __name__ == "__main__":
    unittest.main()
