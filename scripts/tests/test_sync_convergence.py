#!/usr/bin/env python3
from __future__ import annotations

import datetime as dt
import json
import os
import sys
import tempfile
import time
import unittest
from pathlib import Path
from unittest import mock

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

    def test_repository_discovery_includes_dirty_registered_gitlink(self) -> None:
        parent = self.root / "dracon-platform"
        self.fixture.repo.rename(parent)
        self.fixture.repo = parent
        source = self.root / "child-source"
        source.mkdir()
        self.fixture.git(source, "init", "-q", "-b", "main")
        self.fixture.git(source, "config", "user.name", "Fixture")
        self.fixture.git(source, "config", "user.email", "fixture@example.test")
        (source / "README.md").write_text("child\n", encoding="utf-8")
        self.fixture.git(source, "add", "README.md")
        self.fixture.git(source, "commit", "-qm", "child")
        relative = "web/games/wip/tracked-game"
        self.fixture.git(
            parent,
            "-c",
            "protocol.file.allow=always",
            "submodule",
            "add",
            "--quiet",
            str(source),
            relative,
        )
        self.fixture.git(parent, "commit", "-qm", "add child gitlink")
        child = parent / relative
        (child / "dirty.txt").write_text("dirty\n", encoding="utf-8")
        self.assertEqual(sc.nested_required_repositories(parent, set()), [child.resolve()])
        records = sc.discover_repositories([parent], self.policy)
        self.assertIn(str(child.resolve()), [record["path"] for record in records])

    def test_repository_discovery_ignores_unregistered_nested_worktree(self) -> None:
        parent = self.root / "dracon-platform"
        self.fixture.repo.rename(parent)
        self.fixture.repo = parent
        self.fixture.git(parent, "config", "status.showUntrackedFiles", "all")
        child = parent / "web/games/wip/audit-final"
        child.mkdir(parents=True)
        self.fixture.git(child, "init", "-q", "-b", "main")
        self.fixture.git(child, "config", "user.name", "Fixture")
        self.fixture.git(child, "config", "user.email", "fixture@example.test")
        (child / "README.md").write_text("fixture\n", encoding="utf-8")
        self.fixture.git(child, "add", "README.md")
        self.fixture.git(child, "commit", "-qm", "child")
        head = self.fixture.git(child, "rev-parse", "HEAD")
        self.fixture.git(child, "checkout", "-q", "--detach", head)
        records = sc.discover_repositories([parent], self.policy)
        self.assertEqual([record["path"] for record in records], [str(parent.resolve())])
        self.assertEqual(sc.nested_required_repositories(parent, set()), [])

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

    def test_boundary_digest_handles_binary_tracked_candidate(self) -> None:
        path = self.fixture.repo / "frame.png"
        path.write_bytes(b"\x89PNG\r\n\x1a\n" + b"baseline\x00\xff")
        self.fixture.git(self.fixture.repo, "add", "frame.png")
        self.fixture.git(self.fixture.repo, "commit", "-qm", "add binary frame")
        first, first_details = sc.boundary_content_digest(self.fixture.repo, self.policy)
        self.assertEqual(first_details["unsafe_candidates"], [])
        path.write_bytes(b"\x89PNG\r\n\x1a\n" + b"rewritt3n\x00\xfe")
        second, second_details = sc.boundary_content_digest(self.fixture.repo, self.policy)
        self.assertEqual(second_details["unsafe_candidates"], [])
        self.assertNotEqual(first, second)

    def test_boundary_digest_skips_baseline_diff_for_safe_tracked_file(self) -> None:
        path = self.fixture.write("safe.txt", "baseline\n")
        self.fixture.git(self.fixture.repo, "add", "safe.txt")
        self.fixture.git(self.fixture.repo, "commit", "-qm", "safe baseline")
        path.write_text("safe rewrite\n", encoding="utf-8")
        original = sc.read_git_blob
        sc.read_git_blob = lambda *args, **kwargs: self.fail(  # type: ignore[assignment]
            "safe candidate must not retrieve a baseline blob"
        )
        try:
            _, details = sc.boundary_content_digest(self.fixture.repo, self.policy)
        finally:
            sc.read_git_blob = original
        self.assertEqual(details["unsafe_candidates"], [])

    def test_boundary_digest_does_not_archive_secret_content(self) -> None:
        self.fixture.write("config.env", 'TOKEN="super-secret-value"\n')
        _, details = sc.boundary_content_digest(self.fixture.repo, self.policy)
        self.assertTrue(details["unsafe_candidates"])
        self.assertNotIn("super-secret-value", json.dumps(details))

    def test_head_tree_handles_literal_glob_characters(self) -> None:
        path = self.fixture.write("keys/[slug]/+page.svelte", "unchanged\n")
        self.fixture.git(self.fixture.repo, "add", "--", str(path.relative_to(self.fixture.repo)))
        self.fixture.git(self.fixture.repo, "commit", "-qm", "literal path")
        tracked = sc.head_tracked_paths(self.fixture.repo, sc.head_sha(self.fixture.repo))
        self.assertIn("keys/[slug]/+page.svelte", tracked)

    def test_generic_assignment_is_scoped_to_new_lines(self) -> None:
        existing_value = "existing-" + "example-value"
        new_value = "new-" + "secret-value"
        source = self.fixture.write(
            "src/config.ts",
            f'export const api_key = "{existing_value}";\n',
        )
        self.fixture.git(self.fixture.repo, "add", "src/config.ts")
        self.fixture.git(self.fixture.repo, "commit", "-qm", "existing assignment")
        source.write_text(
            f'export const api_key = "{existing_value}";\n'
            'export const mode = "safe";\n',
            encoding="utf-8",
        )
        _, unchanged_details = sc.boundary_content_digest(
            self.fixture.repo, self.policy
        )
        self.assertEqual(unchanged_details["unsafe_candidates"], [])
        source.write_text(
            f'export const api_key = "{existing_value}";\n'
            'export const mode = "safe";\n'
            f'export const token = "{new_value}";\n',
            encoding="utf-8",
        )
        _, added_details = sc.boundary_content_digest(self.fixture.repo, self.policy)
        self.assertTrue(added_details["unsafe_candidates"])
        self.assertNotIn(new_value, json.dumps(added_details))

    def test_private_key_marker_is_scoped_to_whole_tracked_file(self) -> None:
        source = self.fixture.write(
            "src/legacy.txt",
            "-----BEGIN PRIVATE KEY-----\nexisting fixture marker\n",
        )
        self.fixture.git(self.fixture.repo, "add", "src/legacy.txt")
        self.fixture.git(self.fixture.repo, "commit", "-qm", "legacy marker")
        source.write_text(
            "-----BEGIN PRIVATE KEY-----\nexisting fixture marker\nsafe change\n",
            encoding="utf-8",
        )
        _, details = sc.boundary_content_digest(self.fixture.repo, self.policy)
        self.assertTrue(details["unsafe_candidates"])

    def test_quiescence_requires_two_matching_boundaries(self) -> None:
        freeze = self.root / "freeze"
        freeze.write_text("paused\n", encoding="utf-8")
        records = sc.discover_repositories([self.fixture.repo], self.policy)
        result = sc.wait_for_quiescence(
            records,
            self.policy,
            selected_roots=[self.fixture.repo],
            freeze_marker=freeze,
            stable_samples=2,
            interval_seconds=0.01,
            max_wait_seconds=5,
        )
        self.assertGreaterEqual(result["observations"], 3)
        self.assertIn(str(self.fixture.repo), result["fast_tokens"])
        self.assertEqual(
            [record["path"] for record in result["repositories"]],
            [str(self.fixture.repo)],
        )

    def test_capture_transaction_rejects_change_before_detailed_capture(self) -> None:
        freeze = self.root / "freeze"
        freeze.write_text("paused\n", encoding="utf-8")
        records = sc.discover_repositories([self.fixture.repo], self.policy)
        quiescence = sc.wait_for_quiescence(
            records,
            self.policy,
            selected_roots=[self.fixture.repo],
            freeze_marker=freeze,
            stable_samples=2,
            interval_seconds=0.01,
            max_wait_seconds=5,
        )
        self.fixture.write("race.txt", "changed after quiescence\n")
        with self.assertRaises(sc.SnapshotRaceError):
            sc.capture_evidence_transaction(quiescence, self.policy)

    def test_capture_transaction_rejects_change_during_remote_capture(self) -> None:
        freeze = self.root / "freeze"
        freeze.write_text("paused\n", encoding="utf-8")
        records = sc.discover_repositories([self.fixture.repo], self.policy)
        quiescence = sc.wait_for_quiescence(
            records,
            self.policy,
            selected_roots=[self.fixture.repo],
            freeze_marker=freeze,
            stable_samples=2,
            interval_seconds=0.01,
            max_wait_seconds=5,
        )
        original = sc.capture_remote_state

        def capture_then_change(snapshot, policy, *, attempts=3):
            remotes = original(snapshot, policy, attempts=attempts)
            self.fixture.write("remote-race.txt", "changed during remote capture\n")
            return remotes

        with mock.patch.object(sc, "capture_remote_state", side_effect=capture_then_change):
            with self.assertRaises(sc.SnapshotRaceError):
                sc.capture_evidence_transaction(quiescence, self.policy)

    def test_capture_transaction_retries_are_bounded(self) -> None:
        freeze = self.root / "freeze"
        freeze.write_text("paused\n", encoding="utf-8")
        quiescence = {"repositories": [], "fast_tokens": {}, "boundaries": {}}
        race = sc.SnapshotRaceError("fixture race")
        with mock.patch.object(sc, "wait_for_quiescence", return_value=quiescence) as waiter:
            with mock.patch.object(
                sc,
                "capture_evidence_transaction",
                side_effect=race,
            ) as transaction:
                with self.assertRaises(sc.ConvergenceError) as raised:
                    sc.capture_evidence_with_retries(
                        [self.fixture.repo],
                        self.policy,
                        freeze_marker=freeze,
                        stable_samples=2,
                        interval_seconds=0.01,
                        max_wait_seconds=5,
                        transaction_attempts=2,
                    )
        self.assertEqual(waiter.call_count, 2)
        self.assertEqual(transaction.call_count, 2)
        self.assertIn("did not complete after 2 attempts", str(raised.exception))

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

    def test_nested_repo_metadata_is_opaque_to_internal_writes(self) -> None:
        parent = self.fixture.repo
        child = parent / "child"
        child.mkdir(parents=True)
        self.fixture.git(child, "init", "-q", "-b", "main")
        self.fixture.git(child, "config", "user.name", "Fixture")
        self.fixture.git(child, "config", "user.email", "fixture@example.test")
        (child / "README.md").write_text("child\n", encoding="utf-8")
        self.fixture.git(child, "add", "README.md")
        self.fixture.git(child, "commit", "-qm", "child")
        first = sc.nested_repo_metadata(child, parent)
        self.assertNotIn("status_sha256", first)
        self.assertEqual(first["branch"], "main")
        head = self.fixture.git(child, "rev-parse", "HEAD")
        self.fixture.git(child, "checkout", "-q", "--detach", head)
        (child / "README.md").write_text("child internal change\n", encoding="utf-8")
        second = sc.nested_repo_metadata(child, parent)
        self.assertIsNone(second["branch"])
        self.assertNotIn("status_sha256", second)
        parent_digest, details = sc.boundary_content_digest(
            parent,
            self.policy,
            status=sc.git_status_bytes(parent),
        )
        self.assertNotIn("child/README.md", json.dumps(details))
        self.assertNotIn("child internal change", json.dumps(details))
        self.assertTrue(parent_digest)

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
        nested = sc.nested_repo_metadata(child, parent)
        self.assertIsNotNone(nested)
        self.assertEqual(nested["kind"], "nested-worktree")
        self.assertEqual(nested["head"], child_head)

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
        evidence_dir = self.root / "audit"
        evidence_dir.mkdir()
        (evidence_dir / "commands").mkdir()
        for name in ("push.txt", "gate.txt", "pre.txt", "post.txt", "resume.txt"):
            (evidence_dir / "commands" / name).write_text("fixture evidence\n", encoding="utf-8")
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
            evidence_refs=["commands/gate.txt"],
        )
        evidence = sc.record_phase(
            evidence,
            name="pre-resume",
            evidence_refs=["commands/pre.txt"],
            notes="pre-resume fixture complete",
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

    def test_excluded_repository_has_no_effective_remotes(self) -> None:
        policy = dict(self.policy)
        policy["exclude_repos"] = [str(self.fixture.repo)]
        self.assertEqual(sc.effective_remote_names(policy, self.fixture.repo), [])

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

    def test_reflog_filter_ignores_old_resets_and_captures_new(self) -> None:
        original = sc.head_sha(self.fixture.repo)
        self.fixture.git(self.fixture.repo, "reset", "--hard", "HEAD")
        past = dt.datetime.fromtimestamp(
            1, tz=dt.timezone.utc
        ).isoformat().replace("+00:00", "Z")
        self.assertTrue(
            any(
                action.lower().startswith("reset:")
                for action in sc._forbidden_reflog_actions(self.fixture.repo, past)
            )
        )
        future = dt.datetime.fromtimestamp(
            time.time() + 5, tz=dt.timezone.utc
        ).isoformat().replace("+00:00", "Z")
        self.assertEqual(sc._forbidden_reflog_actions(self.fixture.repo, future), [])
        self.assertEqual(sc.head_sha(self.fixture.repo), original)

    def test_evidence_shape_validation_rejects_missing_fields(self) -> None:
        errors = sc.validate_evidence_shape({"schema_version": 1})
        self.assertIn("missing top-level fields", " ".join(errors))
        self.assertIn("repositories must be a non-empty array", errors)
        shaped = {
            "schema_version": 1,
            "run_id": "run",
            "created_at": sc.iso_now(),
            "updated_at": sc.iso_now(),
            "contract": {
                "forward_only": True,
                "selected_paths": [str(self.fixture.repo)],
                "policy_file": "policy.toml",
                "policy_sha256": "0" * 64,
                "freeze_marker": "freeze",
                "remote_query_attempts": 3,
            },
            "phases": [{"name": "pre-resume", "started_at": sc.iso_now(), "ended_at": sc.iso_now()}],
            "repositories": [{"path": str(self.fixture.repo)}],
            "actions": [],
            "gates": {"unit": {"command": "test", "status": "pass", "notes": "ok"}},
            "external_blockers": [],
            "result": "pending",
        }
        shaped_errors = sc.validate_evidence_shape(shaped)
        self.assertTrue(any("requires evidence references" in error for error in shaped_errors))

    def test_finalize_requires_live_checks_by_default(self) -> None:
        policy = self.fixture.policy()
        freeze = self.root / "freeze"
        freeze.write_text("paused\n", encoding="utf-8")
        record = sc.capture_repository(self.fixture.repo, policy)
        record["role"] = "standalone"
        record["remotes"] = sc.capture_remote_state(record, policy)
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
                str(self.fixture.repo): {**record["snapshot"], "head": record["head"]}
            },
            "completed_at": sc.iso_now(),
        }
        evidence = sc.initialize_evidence([record], policy_path, freeze, quiescence)
        evidence_dir = self.root / "audit"
        (evidence_dir / "commands").mkdir(parents=True)
        names = [*sc.REQUIRED_FINAL_GATES, "pre-resume", "post-resume", "resume"]
        refs = {}
        for name in names:
            ref = f"commands/{name}.txt"
            refs[name] = ref
            (evidence_dir / ref).write_text("fixture\n", encoding="utf-8")
        for name in sc.REQUIRED_FINAL_GATES:
            evidence = sc.record_gate(
                evidence,
                name=name,
                command="fixture",
                status="pass",
                notes="pass",
                evidence_refs=[refs[name]],
            )
        evidence = sc.record_phase(
            evidence,
            name="pre-resume",
            evidence_refs=[refs["pre-resume"]],
            notes="pass",
        )
        evidence = sc.record_action(
            evidence,
            repository="fleet",
            kind="resume",
            result="ok",
            evidence_refs=[refs["resume"]],
        )
        evidence = sc.record_phase(
            evidence,
            name="post-resume",
            evidence_refs=[refs["post-resume"]],
            notes="pass",
        )
        evidence_path = evidence_dir / "evidence.json"
        sc.atomic_write_json(evidence_path, evidence)
        freeze.unlink()
        _, blocked = sc.finalize_evidence(
            evidence_path,
            policy_path,
            freeze,
            check_live_remotes=True,
        )
        self.assertFalse(blocked["ok"])
        self.assertTrue(any("daemon" in error.lower() for error in blocked["errors"]))

    def test_evidence_references_must_exist_inside_audit_directory(self) -> None:
        evidence = {
            "phases": [
                {
                    "name": "pre-resume",
                    "evidence": ["commands/missing.txt", "../escape.txt"],
                }
            ],
            "actions": [],
            "gates": {},
            "external_blockers": [],
        }
        errors = sc.validate_evidence_references(evidence, self.root / "audit" / "evidence.json")
        self.assertTrue(any("does not exist" in error for error in errors))
        self.assertTrue(any("escapes audit directory" in error for error in errors))

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
        evidence_dir = self.root / "audit"
        (evidence_dir / "commands").mkdir(parents=True)
        refs: dict[str, str] = {}
        for name in [*sc.REQUIRED_FINAL_GATES, "pre-resume", "post-resume", "resume"]:
            ref = f"commands/{name}.txt"
            refs[name] = ref
            (evidence_dir / ref).write_text("fixture evidence\n", encoding="utf-8")
        for name in sc.REQUIRED_FINAL_GATES:
            evidence = sc.record_gate(
                evidence,
                name=name,
                command="fixture",
                status="pass",
                notes="fixture pass",
                evidence_refs=[refs[name]],
            )
        evidence = sc.record_phase(
            evidence,
            name="pre-resume",
            evidence_refs=[refs["pre-resume"]],
            notes="fixture pre-resume complete",
        )
        evidence = sc.record_action(
            evidence,
            repository="fleet",
            kind="resume",
            result="ok",
            evidence_refs=[refs["resume"]],
        )
        evidence = sc.record_phase(
            evidence,
            name="post-resume",
            evidence_refs=[refs["post-resume"]],
            notes="fixture post-resume complete",
        )
        evidence["result"] = "pass"
        evidence_path = evidence_dir / "evidence.json"
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
