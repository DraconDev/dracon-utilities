#!/usr/bin/env python3
from __future__ import annotations

import json
import os
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))

import sync_convergence as sc  # noqa: E402


class GitFixture:
    def __init__(self, root: Path) -> None:
        self.root = root
        self.repo = root / "repo"
        self.remote = root / "remote.git"
        self.repo.mkdir()
        self.remote.mkdir()
        self.git(self.repo, "init", "-q", "-b", "main")
        self.git(self.repo, "config", "user.name", "Fixture")
        self.git(self.repo, "config", "user.email", "fixture@example.test")
        self.git(self.remote, "init", "-q", "--bare")
        self.write("README.md", "initial\n")
        self.git(self.repo, "add", "README.md")
        self.git(self.repo, "commit", "-qm", "initial")
        self.git(self.repo, "remote", "add", "origin", str(self.remote))
        self.git(self.repo, "push", "-q", "-u", "origin", "main")

    @staticmethod
    def git(cwd: Path, *args: str) -> str:
        import subprocess

        return subprocess.check_output(
            ["git", "-C", str(cwd), *args], text=True, stderr=subprocess.STDOUT
        ).strip()

    def write(self, relative: str, content: str) -> Path:
        path = self.repo / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content, encoding="utf-8")
        return path

    def policy(self, remote_name: str = "origin") -> dict:
        return {
            "remotes": [{"name": remote_name, "push_url": str(self.remote)}],
            "exclude_dir_names": ["node_modules", "target"],
            "max_stage_file_bytes": 100 * 1024 * 1024,
        }


class SyncConvergenceTests(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory(prefix="sync-convergence-test-")
        self.root = Path(self.tmp.name)
        self.fixture = GitFixture(self.root)
        self.policy = self.fixture.policy()

    def tearDown(self) -> None:
        self.tmp.cleanup()

    def test_redaction_and_remote_canonicalization(self) -> None:
        self.assertEqual(
            sc.canonical_remote_url("https://user:token@example.com/Org/Repo.git"),
            "example.com/org/repo",
        )
        self.assertEqual(
            sc.canonical_remote_url("git@github.com:Org/Repo.git"),
            "github.com/org/repo",
        )
        redacted = sc.redact(
            "url=https://alice:secret@example.com/x token=abcdefgh123"
        )
        self.assertNotIn("alice:secret", redacted)
        self.assertNotIn("abcdefgh123", redacted)

    def test_remote_failure_classification_is_strict(self) -> None:
        self.assertEqual(
            sc.classify_remote_failure("Permission denied (publickey)"), "authentication"
        )
        self.assertEqual(
            sc.classify_remote_failure("Could not resolve hostname github.com"),
            "provider_outage",
        )
        self.assertIsNone(sc.classify_remote_failure("remote branch is missing", missing=True))
        self.assertIsNone(sc.classify_remote_failure("protocol error"))
        self.assertNotIn("dirty", sc.ALLOWED_EXTERNAL_BLOCKERS)
        self.assertNotIn("diverged", sc.ALLOWED_EXTERNAL_BLOCKERS)

    def test_effective_remote_names_respect_repo_override(self) -> None:
        override = self.fixture.repo / ".dracon/dracon-sync.toml"
        override.parent.mkdir(parents=True)
        override.write_text('exclude_remotes = ["origin"]\n', encoding="utf-8")
        self.assertEqual(sc.effective_remote_names(self.policy, self.fixture.repo), [])
        override.write_text('exclude_remotes = ["codeberg"]\n', encoding="utf-8")
        self.assertEqual(
            sc.effective_remote_names(self.policy, self.fixture.repo), ["origin"]
        )
        override.write_text('owned = false\n', encoding="utf-8")
        self.assertEqual(sc.effective_remote_names(self.policy, self.fixture.repo), [])
        policy = dict(self.policy)
        policy["exclude_repos"] = [str(self.fixture.repo)]
        self.assertEqual(sc.effective_remote_names(policy, self.fixture.repo), [])

    def test_expected_remote_identity_uses_policy_mapping(self) -> None:
        policy = {
            "remotes": [
                {
                    "name": "origin",
                    "push_url": "git@example.com:{account}/{repo}.git",
                    "auto_create_account": "Operator",
                    "repo_name_map": {"repo": "canonical-repository"},
                }
            ]
        }
        expected = sc.expected_remote_project(policy, self.fixture.repo, "origin")
        self.assertEqual(
            sc.canonical_remote_url(expected), "example.com/operator/canonical-repository"
        )

    def test_detached_head_is_rejected(self) -> None:
        head = sc.head_sha(self.fixture.repo)
        sc.run_command(["git", "-C", self.fixture.repo, "checkout", "-q", "--detach", head])
        with self.assertRaises(sc.ConvergenceError):
            sc.attached_branch(self.fixture.repo)

    def test_boundary_digest_detects_same_size_rewrite(self) -> None:
        path = self.fixture.write("data.txt", "AAAA\n")
        self.fixture.git(self.fixture.repo, "add", "data.txt")
        first, first_details = sc.boundary_content_digest(self.fixture.repo, self.policy)
        self.assertEqual(first_details["unsafe_candidates"], [])
        path.write_text("BBBB\n", encoding="utf-8")
        second, second_details = sc.boundary_content_digest(self.fixture.repo, self.policy)
        self.assertNotEqual(first, second)

    def test_boundary_digest_does_not_archive_secret_content(self) -> None:
        self.fixture.write("config.env", 'TOKEN="super-secret-value"\n')
        _, details = sc.boundary_content_digest(self.fixture.repo, self.policy)
        self.assertTrue(details["unsafe_candidates"])
        self.assertNotIn("super-secret-value", json.dumps(details))

    def test_quiescence_requires_two_matching_boundaries(self) -> None:
        freeze = self.root / "freeze"
        freeze.write_text("paused\n", encoding="utf-8")
        records = sc.discover_repositories([self.fixture.repo], self.policy)
        result = sc.wait_for_quiescence(
            records,
            self.policy,
            freeze_marker=freeze,
            stable_samples=2,
            interval_seconds=0.01,
            max_wait_seconds=5,
        )
        self.assertGreaterEqual(result["observations"], 3)
        self.assertIn(str(self.fixture.repo), result["fast_tokens"])

    def test_remote_snapshot_records_ancestor_relation(self) -> None:
        record = sc.capture_repository(self.fixture.repo, self.policy)
        remote = sc.capture_remote_state(record, self.policy)[0]
        self.assertEqual(remote["relation"], "equal")
        self.fixture.write("next.txt", "next\n")
        self.fixture.git(self.fixture.repo, "add", "next.txt")
        self.fixture.git(self.fixture.repo, "commit", "-qm", "next")
        record = sc.capture_repository(self.fixture.repo, self.policy)
        remote = sc.capture_remote_state(record, self.policy)[0]
        self.assertEqual(remote["relation"], "local-ahead")
        self.assertEqual(
            remote["before_sha"],
            self.fixture.git(self.fixture.remote, "rev-parse", "refs/heads/main"),
        )

    def test_parent_gitlink_and_forward_ancestry(self) -> None:
        parent = self.root / "parent"
        child = parent / "child"
        child.mkdir(parents=True)
        self.fixture.git(child, "init", "-q", "-b", "main")
        self.fixture.git(child, "config", "user.name", "Fixture")
        self.fixture.git(child, "config", "user.email", "fixture@example.test")
        (child / "README.md").write_text("child\n", encoding="utf-8")
        self.fixture.git(child, "add", "README.md")
        self.fixture.git(child, "commit", "-qm", "child")
        self.fixture.git(parent, "init", "-q", "-b", "main")
        self.fixture.git(parent, "config", "user.name", "Fixture")
        self.fixture.git(parent, "config", "user.email", "fixture@example.test")
        child_head = sc.head_sha(child)
        self.fixture.git(
            parent,
            "update-index",
            "--add",
            "--cacheinfo",
            f"160000,{child_head},child",
        )
        tree = self.fixture.git(parent, "write-tree")
        parent_commit = self.fixture.git(
            parent, "commit-tree", tree, "-m", "child pin"
        )
        self.fixture.git(parent, "update-ref", "refs/heads/main", parent_commit)
        self.assertEqual(sc._parent_gitlink(parent, child), child_head)

        before = sc.head_sha(child)
        (child / "forward.txt").write_text("forward\n", encoding="utf-8")
        self.fixture.git(child, "add", "forward.txt")
        self.fixture.git(child, "commit", "-qm", "forward")
        after = sc.head_sha(child)
        self.assertTrue(sc._is_ancestor(child, before, after))
        self.assertFalse(sc._is_ancestor(child, after, before))

    def test_evidence_actions_and_gates_are_recorded(self) -> None:
        policy = self.fixture.policy()
        freeze = self.root / "freeze"
        freeze.write_text("paused\n", encoding="utf-8")
        record = sc.capture_repository(self.fixture.repo, policy)
        record["role"] = "standalone"
        record["remotes"] = sc.capture_remote_state(record, policy)
        policy_path = self.root / "policy.toml"
        policy_path.write_text(
            f'remotes = [{{name = "origin", push_url = "{self.fixture.remote}"}}]\n'
            'exclude_dir_names = ["node_modules", "target"]\n'
            "max_stage_file_bytes = 104857600\n",
            encoding="utf-8",
        )
        quiescence = {
            "stable_samples": 3,
            "observations": 3,
            "interval_seconds": 1,
            "span_seconds": 2,
            "fast_tokens": {str(self.fixture.repo): record["snapshot"]["fast_token"]},
            "boundaries": {
                str(self.fixture.repo): {
                    **record["snapshot"],
                    "head": record["head"],
                }
            },
            "completed_at": sc.iso_now(),
        }
        evidence = sc.initialize_evidence(
            [record], policy_path, freeze, quiescence
        )
        evidence = sc.record_action(
            evidence,
            repository=str(self.fixture.repo),
            kind="push",
            result="ok",
            evidence_refs=["commands/push.txt"],
            head_before=record["head"],
            head_after=record["head"],
        )
        evidence = sc.record_gate(
            evidence,
            name="fixture",
            command="python3 -m unittest",
            status="pass",
            notes="all fixture tests pass",
        )
        self.assertEqual(evidence["actions"][0]["kind"], "push")
        self.assertEqual(evidence["gates"]["fixture"]["status"], "pass")
        with self.assertRaises(sc.ConvergenceError):
            sc.record_action(
                evidence,
                repository=str(self.fixture.repo),
                kind="rebase",
                result="ok",
                evidence_refs=["commands/rebase.txt"],
            )

    def test_internal_failure_cannot_be_recorded_as_external_blocker(self) -> None:
        policy = self.fixture.policy()
        record = sc.capture_repository(self.fixture.repo, policy)
        record["role"] = "standalone"
        record["remotes"] = sc.capture_remote_state(record, policy)
        freeze = self.root / "freeze"
        freeze.write_text("paused\n", encoding="utf-8")
        policy_path = self.root / "policy.toml"
        policy_path.write_text(
            f'remotes = [{{name = "origin", push_url = "{self.fixture.remote}"}}]\n',
            encoding="utf-8",
        )
        quiescence = {
            "stable_samples": 3,
            "observations": 3,
            "interval_seconds": 1,
            "span_seconds": 2,
            "fast_tokens": {str(self.fixture.repo): record["snapshot"]["fast_token"]},
            "boundaries": {
                str(self.fixture.repo): {
                    **record["snapshot"],
                    "head": record["head"],
                }
            },
            "completed_at": sc.iso_now(),
        }
        evidence = sc.initialize_evidence([record], policy_path, freeze, quiescence)
        self.fixture.git(self.fixture.repo, "remote", "set-url", "origin", str(self.root / "missing.git"))
        with self.assertRaises(sc.ConvergenceError):
            sc.record_external_blocker(
                evidence,
                {**policy, "remotes": [{"name": "origin", "push_url": "unused"}]},
                repository=str(self.fixture.repo),
                remote="origin",
                evidence_ref="commands/missing.txt",
            )

    def test_evidence_shape_validation_rejects_missing_fields(self) -> None:
        errors = sc.validate_evidence_shape({"schema_version": 1})
        self.assertIn("missing top-level fields", " ".join(errors))
        self.assertIn("repositories must be a non-empty array", errors)

    def test_offline_verifier_accepts_complete_local_evidence(self) -> None:
        policy_path = self.root / "policy.toml"
        policy_path.write_text(
            f'remotes = [{{name = "origin", push_url = "{self.fixture.remote}"}}]\n'
            'exclude_dir_names = ["node_modules", "target"]\n'
            "max_stage_file_bytes = 104857600\n",
            encoding="utf-8",
        )
        freeze = self.root / "freeze"
        freeze.write_text("paused\n", encoding="utf-8")
        record = sc.capture_repository(self.fixture.repo, self.policy)
        record["role"] = "standalone"
        record["remotes"] = sc.capture_remote_state(record, self.policy)
        quiescence = {
            "stable_samples": 3,
            "observations": 3,
            "interval_seconds": 1,
            "span_seconds": 2,
            "fast_tokens": {str(self.fixture.repo): record["snapshot"]["fast_token"]},
            "boundaries": {
                str(self.fixture.repo): {
                    **record["snapshot"],
                    "head": record["head"],
                }
            },
            "completed_at": sc.iso_now(),
        }
        evidence = sc.initialize_evidence(
            [record], policy_path, freeze, quiescence
        )
        evidence["result"] = "pass"
        evidence["phases"].extend(
            [
                {"name": "pre-resume", "started_at": sc.iso_now(), "ended_at": sc.iso_now()},
                {"name": "post-resume", "started_at": sc.iso_now(), "ended_at": sc.iso_now()},
            ]
        )
        for name in sc.REQUIRED_FINAL_GATES:
            evidence = sc.record_gate(
                evidence,
                name=name,
                command="fixture",
                status="pass",
                notes="fixture pass",
            )
        evidence = sc.record_action(
            evidence,
            repository="fleet",
            kind="resume",
            result="ok",
            evidence_refs=["commands/resume.txt"],
        )
        evidence_path = self.root / "evidence.json"
        sc.atomic_write_json(evidence_path, evidence)
        freeze.unlink()
        result = sc.verify_evidence(
            evidence_path,
            policy_path,
            freeze_marker=freeze,
            check_live_remotes=False,
        )
        self.assertTrue(result["ok"], result["errors"])


if __name__ == "__main__":
    unittest.main()
