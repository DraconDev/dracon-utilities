#!/usr/bin/env python3
"""Shared, read-only primitives for daemon convergence evidence.

The module deliberately separates three concerns:

* cheap generation tokens used while other writers are active;
* boundary content proofs used to establish an immutable snapshot; and
* final verification against live Git refs, effective remotes, and gitlinks.

It never commits, pulls, merges, pushes, resumes the daemon, or modifies a
selected repository.  Callers own those actions and record them in evidence.
"""
from __future__ import annotations

import dataclasses
import datetime as dt
import difflib
import fnmatch
import hashlib
import json
import os
import re
import shutil
import stat
import subprocess
import sys
import tempfile
import time
import tomllib
import urllib.parse
from pathlib import Path
from typing import Any, Iterable, Sequence

SCHEMA_VERSION = 1
DEFAULT_POLICY = Path.home() / ".dracon/utilities/sync/dracon-sync.toml"
DEFAULT_FREEZE = Path.home() / ".dracon/dracon-sync.freeze"
DEFAULT_EXCLUDED_DIRS = {
    ".git",
    "target",
    "node_modules",
    ".cache",
    ".venv",
    "dist",
    "build",
    "archives",
    "repo-runtime",
}
SECRET_NAME_RE = re.compile(
    r"(?:^|[._-])(?:env|pem|key|age|secret|credential|credentials)(?:$|[._-])"
    r"|(?:^|[._-])(?:id_rsa|id_ed25519|credentials_json|service_account)(?:$|[._-])",
    re.IGNORECASE,
)
PRIVATE_KEY_RE = re.compile(
    r"-----BEGIN (?:RSA |EC |OPENSSH |DSA )?PRIVATE KEY-----", re.IGNORECASE
)
REMOTE_USERINFO_RE = re.compile(r"(?P<scheme>[a-z][a-z0-9+.-]*://)[^/@\s]+@", re.IGNORECASE)
ASSIGNED_SECRET_RE = re.compile(
    r"(?i)\b(?:password|passwd|secret|token|api[_-]?key|access[_-]?key|private[_-]?key)"
    r"\b\s*[:=]\s*(?:\"[^\"\n]{8,}\"|'[^'\n]{8,}'|[A-Za-z0-9_./+=~-]{8,})"
)
ASSIGNED_SECRET_BYTES_RE = re.compile(
    rb"(?i)\b(?:password|passwd|secret|token|api[_-]?key|access[_-]?key|private[_-]?key)"
    rb"\b\s*[:=]\s*(?:\"[^\"\n]{8,}\"|'[^'\n]{8,}'|[A-Za-z0-9_./+=~-]{8,})"
)
ALLOWED_EXTERNAL_BLOCKERS = {
    "authentication",
    "permission",
    "provider_outage",
    "remote_unavailable",
}
REQUIRED_FINAL_GATES = {
    "daemon-health",
    "daemon-repos",
    "cargo-test-workspace",
    "cargo-build-release",
    "cargo-clippy-workspace",
    "cargo-deny",
}
REQUIRED_FINAL_PHASES = {
    "quiescent-snapshot",
    "pre-resume",
    "post-resume",
}
PROHIBITED_REFLOG_PREFIXES = (
    "reset:",
    "rebase (start):",
    "rebase (pick):",
    "rebase (finish):",
    "filter-branch:",
    "filter-repo:",
    "amend:",
)


class ConvergenceError(RuntimeError):
    """A convergence invariant failed."""


@dataclasses.dataclass(frozen=True)
class CommandResult:
    argv: tuple[str, ...]
    returncode: int
    stdout: str
    stderr: str
    duration_ms: int

    @property
    def ok(self) -> bool:
        return self.returncode == 0


def utc_now() -> dt.datetime:
    return dt.datetime.now(dt.timezone.utc)


def iso_now() -> str:
    return utc_now().isoformat().replace("+00:00", "Z")


def run_command(
    argv: Sequence[str | os.PathLike[str]],
    *,
    cwd: Path | None = None,
    timeout: int = 120,
    env: dict[str, str] | None = None,
    max_output_chars: int = 1_000_000,
) -> CommandResult:
    """Run a command without a shell and capture bounded text output."""
    rendered = tuple(os.fspath(item) for item in argv)
    started = time.monotonic()
    try:
        proc = subprocess.run(
            rendered,
            cwd=cwd,
            env=env,
            text=True,
            capture_output=True,
            timeout=timeout,
            check=False,
        )
        return CommandResult(
            rendered,
            proc.returncode,
            proc.stdout[-max_output_chars:],
            proc.stderr[-max_output_chars:],
            int((time.monotonic() - started) * 1000),
        )
    except subprocess.TimeoutExpired as exc:
        return CommandResult(
            rendered,
            124,
            (exc.stdout or "")[-1_000_000:] if isinstance(exc.stdout, str) else "",
            f"timed out after {timeout}s",
            int((time.monotonic() - started) * 1000),
        )
    except OSError as exc:
        return CommandResult(rendered, 125, "", str(exc), int((time.monotonic() - started) * 1000))


def require_command(
    argv: Sequence[str | os.PathLike[str]], *, cwd: Path | None = None, timeout: int = 120
) -> str:
    result = run_command(argv, cwd=cwd, timeout=timeout)
    if not result.ok:
        raise ConvergenceError(
            f"command failed ({result.returncode}): {' '.join(result.argv)}: "
            f"{redact(result.stderr or result.stdout).strip()}"
        )
    return result.stdout.strip()


def load_toml(path: Path) -> dict[str, Any]:
    try:
        with path.open("rb") as handle:
            return tomllib.load(handle)
    except (OSError, tomllib.TOMLDecodeError) as exc:
        raise ConvergenceError(f"cannot load TOML {path}: {exc}") from exc


def file_sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        while chunk := handle.read(1024 * 1024):
            digest.update(chunk)
    return digest.hexdigest()


def bytes_sha256(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def canonical_json_bytes(value: Any) -> bytes:
    return json.dumps(
        value, sort_keys=True, separators=(",", ":"), ensure_ascii=False
    ).encode("utf-8")


def canonical_digest(value: Any) -> str:
    return bytes_sha256(canonical_json_bytes(value))


def atomic_write_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, tmp_name = tempfile.mkstemp(prefix=f".{path.name}.", dir=path.parent)
    tmp = Path(tmp_name)
    try:
        with os.fdopen(fd, "wb") as handle:
            handle.write(json.dumps(value, indent=2, sort_keys=True, ensure_ascii=False).encode())
            handle.write(b"\n")
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(tmp, path)
    finally:
        tmp.unlink(missing_ok=True)


def redact(value: str) -> str:
    """Redact common credential forms before evidence is persisted."""
    text = PRIVATE_KEY_RE.sub("-----BEGIN PRIVATE KEY----- [REDACTED]", value)
    text = REMOTE_USERINFO_RE.sub(r"\g<scheme>[REDACTED]@", text)
    text = ASSIGNED_SECRET_RE.sub("[REDACTED SECRET ASSIGNMENT]", text)
    return text


def safe_remote_name(value: str) -> bool:
    return bool(re.fullmatch(r"[A-Za-z0-9._-]+", value))


def excluded_directory(name: str, patterns: Iterable[str]) -> bool:
    return any(fnmatch.fnmatch(name, pattern) for pattern in patterns)


def policy_excluded_dir_names(policy: dict[str, Any]) -> set[str]:
    names = set(DEFAULT_EXCLUDED_DIRS)
    names.update(str(item) for item in policy.get("exclude_dir_names", []))
    return names


def repo_override(repo: Path) -> dict[str, Any]:
    override_path = repo / ".dracon/dracon-sync.toml"
    return load_toml(override_path) if override_path.exists() else {}


def repo_excluded_by_policy(policy: dict[str, Any], repo: Path) -> bool:
    resolved = repo.expanduser().resolve()
    for value in policy.get("exclude_repos", []):
        try:
            if resolved == Path(str(value)).expanduser().resolve():
                return True
        except OSError:
            continue
    return False


def effective_remote_names(policy: dict[str, Any], repo: Path) -> list[str]:
    if repo_excluded_by_policy(policy, repo):
        return []
    override = repo_override(repo)
    if override.get("owned") is False:
        return []
    excluded = {str(item) for item in override.get("exclude_remotes", [])}
    names = [str(remote.get("name", "")) for remote in policy.get("remotes", [])]
    return sorted(name for name in names if safe_remote_name(name) and name not in excluded)


def expected_remote_project(policy: dict[str, Any], repo: Path, remote_name: str) -> str | None:
    for remote in policy.get("remotes", []):
        if str(remote.get("name", "")) != remote_name:
            continue
        template = str(remote.get("push_url", ""))
        if not template:
            return None
        repo_name = str(remote.get("repo_name_map", {}).get(repo.name, repo.name))
        account = str(remote.get("auto_create_account") or "DraconDev")
        return template.replace("{repo}", repo_name).replace("{account}", account)
    return None


def canonical_remote_url(url: str) -> str:
    """Return a credential-free canonical repository identity."""
    value = url.strip()
    if "://" in value:
        parsed = urllib.parse.urlsplit(value)
        host = parsed.hostname or ""
        port = f":{parsed.port}" if parsed.port else ""
        return f"{host}{port}{parsed.path}".removesuffix(".git").rstrip("/").lower()
    if "@" in value and ":" in value:
        authority, path = value.split(":", 1)
        host = authority.rsplit("@", 1)[-1]
        return f"{host}/{path}".removesuffix(".git").rstrip("/").lower()
    return value.removesuffix(".git").rstrip("/").lower()


def git_repo_info(repo: Path) -> dict[str, str]:
    repo = repo.expanduser().resolve()
    if not repo.is_dir():
        raise ConvergenceError(f"repository path does not exist: {repo}")
    try:
        top = Path(require_command(["git", "-C", repo, "rev-parse", "--show-toplevel"]))
        git_dir = Path(require_command(["git", "-C", repo, "rev-parse", "--absolute-git-dir"]))
        common_dir = Path(
            require_command(["git", "-C", repo, "rev-parse", "--path-format=absolute", "--git-common-dir"])
        )
    except ConvergenceError as exc:
        raise ConvergenceError(f"not a Git repository: {repo}: {exc}") from exc
    return {
        "path": str(top),
        "git_dir": str(git_dir),
        "common_dir": str(common_dir),
    }


def attached_branch(repo: Path) -> str:
    branch = require_command(["git", "-C", repo, "symbolic-ref", "--quiet", "--short", "HEAD"])
    if not branch:
        raise ConvergenceError(f"detached or unborn HEAD is not admissible: {repo}")
    return branch


def head_sha(repo: Path) -> str:
    value = require_command(["git", "-C", repo, "rev-parse", "--verify", "HEAD^{commit}"])
    if not re.fullmatch(r"[0-9a-f]{40,64}", value):
        raise ConvergenceError(f"invalid HEAD for {repo}: {value!r}")
    return value


def git_status_bytes(repo: Path) -> bytes:
    result = run_command(
        ["git", "-C", repo, "status", "--porcelain=v2", "-z", "--untracked-files=all"],
        timeout=180,
    )
    if not result.ok:
        raise ConvergenceError(f"git status failed for {repo}: {redact(result.stderr).strip()}")
    return result.stdout.encode("utf-8", "surrogateescape")


def parse_status_v2(raw: bytes) -> list[dict[str, Any]]:
    records: list[dict[str, Any]] = []
    parts = raw.split(b"\0")
    index = 0
    while index < len(parts):
        record = parts[index]
        index += 1
        if not record:
            continue
        text = record.decode("utf-8", "surrogateescape")
        if text.startswith("#"):
            records.append({"kind": "header", "record": text})
            continue
        if text.startswith("u "):
            fields = text.split(" ", 10)
            records.append(
                {
                    "kind": "untracked",
                    "path": fields[10] if len(fields) > 10 else "",
                    "record": text,
                }
            )
            continue
        if text.startswith("? "):
            records.append({"kind": "untracked", "path": text[2:], "record": text})
            continue
        fields = text.split(" ", 8)
        if len(fields) < 9:
            records.append({"kind": "unknown", "record": text})
            continue
        xy = fields[1]
        mode = fields[2]
        path = fields[8]
        item: dict[str, Any] = {
            "kind": "tracked",
            "xy": xy,
            "submodule": mode == "160000",
            "path": path,
            "record": text,
        }
        if "R" in xy or "C" in xy:
            if index < len(parts) and parts[index]:
                item["orig_path"] = parts[index].decode("utf-8", "surrogateescape")
                index += 1
        records.append(item)
    return records


def status_counts(repo: Path) -> dict[str, int]:
    records = parse_status_v2(git_status_bytes(repo))
    counts = {"modified": 0, "staged": 0, "untracked": 0, "submodule": 0}
    for record in records:
        kind = record.get("kind")
        if kind == "untracked":
            counts["untracked"] += 1
        elif kind == "tracked":
            xy = str(record.get("xy", ".."))
            if xy[0] != ".":
                counts["staged"] += 1
            if xy[1] != ".":
                counts["modified"] += 1
            if record.get("submodule"):
                counts["submodule"] += 1
    return counts


def ahead_behind(repo: Path, branch: str, remote: str = "origin") -> tuple[int, int]:
    result = run_command(
        ["git", "-C", repo, "rev-list", "--left-right", "--count", f"HEAD...{remote}/{branch}"],
        timeout=120,
    )
    if not result.ok:
        return (0, 0)
    parts = result.stdout.strip().split()
    if len(parts) != 2:
        return (0, 0)
    return int(parts[0]), int(parts[1])


def operation_state(repo: Path) -> str:
    info = git_repo_info(repo)
    git_dir = Path(info["git_dir"])
    common_dir = Path(info["common_dir"])
    markers = [
        git_dir / "MERGE_HEAD",
        git_dir / "CHERRY_PICK_HEAD",
        git_dir / "REVERT_HEAD",
        git_dir / "rebase-merge",
        git_dir / "rebase-apply",
        common_dir / "rebase-merge",
        common_dir / "rebase-apply",
    ]
    active = [path for path in markers if path.exists()]
    locks = [path for path in (git_dir / "index.lock", git_dir / "HEAD.lock") if path.exists()]
    if active:
        return "operation:" + ",".join(path.name for path in active)
    if locks:
        return "lock:" + ",".join(path.name for path in locks)
    return "none"


def nested_required_repositories(parent: Path, excluded_names: set[str]) -> list[Path]:
    """Discover only child repositories that make the parent dirty."""
    result = run_command(
        ["git", "-C", parent, "status", "--porcelain=v1", "-z", "--ignore-submodules=none"],
        timeout=180,
    )
    if not result.ok:
        return []
    selected: set[Path] = set()
    for record in result.stdout.split("\0"):
        if len(record) < 4:
            continue
        path_text = record[3:]
        if " -> " in path_text:
            path_text = path_text.split(" -> ", 1)[1]
        path = (parent / path_text).resolve()
        try:
            path.relative_to(parent)
        except ValueError:
            continue
        if path == parent or not (path / ".git").exists():
            continue
        if any(fnmatch.fnmatch(part, pattern) for part in path.parts for pattern in excluded_names):
            continue
        selected.add(path)
    return sorted(selected, key=str)


def discover_repositories(
    selected_paths: Sequence[Path], policy: dict[str, Any]
) -> list[dict[str, str]]:
    excluded_names = policy_excluded_dir_names(policy)
    selected = {Path(path).expanduser().resolve() for path in selected_paths}
    platform_roots = [path for path in selected if path.name == "dracon-platform"]
    for platform in platform_roots:
        selected.update(nested_required_repositories(platform, excluded_names))

    records: list[dict[str, str]] = []
    seen: set[Path] = set()
    for repo in sorted(selected, key=str):
        if repo in seen:
            continue
        seen.add(repo)
        info = git_repo_info(repo)
        if repo.name == "dracon-platform":
            role = "parent"
        elif any(repo.is_relative_to(root) for root in platform_roots):
            role = "nested-required"
        else:
            role = "standalone"
        records.append({**info, "role": role})
    return records


def _safe_candidate_names(paths: Iterable[str]) -> list[str]:
    return sorted({path for path in paths if path and "\0" not in path})


def candidate_paths_from_status(raw: bytes) -> list[str]:
    """Extract every dirty/staged/untracked path from one porcelain-v2 scan."""
    paths: set[str] = set()
    for record in parse_status_v2(raw):
        path = record.get("path")
        original = record.get("orig_path")
        if isinstance(path, str) and path:
            paths.add(path)
        if isinstance(original, str) and original:
            paths.add(original)
    return _safe_candidate_names(paths)


def changed_candidate_paths(repo: Path) -> list[str]:
    """Compatibility helper for callers that do not already hold status bytes."""
    return candidate_paths_from_status(git_status_bytes(repo))


def _path_metadata(path: Path, repo: Path) -> dict[str, Any] | None:
    try:
        info = path.lstat()
    except FileNotFoundError:
        return {"path": str(path.relative_to(repo)), "kind": "missing"}
    except OSError as exc:
        return {"path": str(path.relative_to(repo)), "kind": "error", "error": str(exc)}
    item: dict[str, Any] = {
        "path": str(path.relative_to(repo)),
        "kind": stat.S_IFMT(info.st_mode),
        "mode": stat.S_IMODE(info.st_mode),
        "size": info.st_size,
        "mtime_ns": info.st_mtime_ns,
        "ctime_ns": info.st_ctime_ns,
    }
    if stat.S_ISLNK(info.st_mode):
        try:
            item["target"] = os.readlink(path)
        except OSError:
            item["target"] = "<unreadable>"
    return item


def nested_repo_metadata(path: Path, repo: Path) -> dict[str, Any] | None:
    if path.resolve() == repo.resolve() or not path.is_dir() or not (path / ".git").exists():
        return None
    try:
        info = path.lstat()
        child = git_repo_info(path)
        return {
            "path": str(path.relative_to(repo)),
            "kind": "nested-worktree",
            "head": head_sha(path),
            "branch": attached_branch(path),
            "status_sha256": bytes_sha256(git_status_bytes(path)),
            "mtime_ns": info.st_mtime_ns,
            "ctime_ns": info.st_ctime_ns,
            "git_dir": child["git_dir"],
            "common_dir": child["common_dir"],
        }
    except (ConvergenceError, OSError, ValueError):
        return None


def _walk_candidate_tree(
    root: Path, repo: Path, excluded_names: set[str]
) -> Iterable[Path]:
    if root.is_file() or root.is_symlink():
        yield root
        return
    if not root.is_dir():
        return
    for current, dirs, files in os.walk(root, topdown=True, followlinks=False):
        current_path = Path(current)
        dirs[:] = sorted(
            name
            for name in dirs
            if not excluded_directory(name, excluded_names)
            and not (current_path / name / ".git").exists()
        )
        for name in sorted(files):
            yield current_path / name
        for name in list(dirs):
            path = current_path / name
            if path.is_symlink():
                yield path


def fast_token(
    repo: Path,
    policy: dict[str, Any],
    *,
    status: bytes | None = None,
    branch: str | None = None,
    head: str | None = None,
) -> str:
    """Cheap generation token that detects normal concurrent writes."""
    info = git_repo_info(repo)
    git_dir = Path(info["git_dir"])
    common_dir = Path(info["common_dir"])
    branch = branch or attached_branch(repo)
    head = head or head_sha(repo)
    status = git_status_bytes(repo) if status is None else status
    candidates = candidate_paths_from_status(status)
    excluded_names = policy_excluded_dir_names(policy)

    metadata: list[dict[str, Any]] = []
    for relative in candidates:
        path = repo / relative
        if any(
            excluded_directory(part, excluded_names)
            for part in Path(relative).parts[:-1]
        ):
            continue
        nested = nested_repo_metadata(path, repo)
        if nested is not None:
            metadata.append(nested)
            continue
        if path.is_dir() and not path.is_symlink():
            try:
                info = path.stat()
                metadata.append(
                    {
                        "path": relative,
                        "kind": "directory",
                        "mtime_ns": info.st_mtime_ns,
                        "ctime_ns": info.st_ctime_ns,
                    }
                )
            except OSError:
                metadata.append({"path": relative, "kind": "unreadable-directory"})
            for child in _walk_candidate_tree(path, repo, excluded_names):
                child_meta = _path_metadata(child, repo)
                if child_meta:
                    metadata.append(child_meta)
        else:
            item = _path_metadata(path, repo)
            if item:
                metadata.append(item)

    special: list[dict[str, Any]] = []
    for path in [git_dir / "index", git_dir / "HEAD", common_dir / "packed-refs"]:
        item = _path_metadata(path, repo) if path.is_relative_to(repo) else None
        if item is None and path.exists():
            info = path.stat()
            item = {
                "path": str(path),
                "kind": "git-metadata",
                "size": info.st_size,
                "mtime_ns": info.st_mtime_ns,
                "ctime_ns": info.st_ctime_ns,
            }
        if item:
            special.append(item)
    for ref_dir in [git_dir / "refs", common_dir / "refs"]:
        if ref_dir.exists():
            info = ref_dir.stat()
            special.append(
                {
                    "path": str(ref_dir),
                    "kind": "git-ref-dir",
                    "mtime_ns": info.st_mtime_ns,
                    "ctime_ns": info.st_ctime_ns,
                }
            )
    return canonical_digest(
        {
            "repo": str(repo),
            "branch": branch,
            "head": head,
            "status_sha256": bytes_sha256(status),
            "candidates": sorted(metadata, key=lambda item: item["path"]),
            "git_metadata": sorted(special, key=lambda item: item["path"]),
            "operation": operation_state(repo),
        }
    )


def head_tracked_paths(repo: Path, head: str) -> set[str]:
    tree = run_command(
        ["git", "-C", repo, "ls-tree", "-r", "-z", "--name-only", head],
        timeout=180,
        max_output_chars=32_000_000,
    )
    if not tree.ok:
        raise ConvergenceError(f"cannot list HEAD tree for {repo}")
    return {
        item.decode("utf-8", "surrogateescape")
        for item in tree.stdout.encode("utf-8", "surrogateescape").split(b"\0")
        if item
    }


def added_line_numbers(
    repo: Path,
    path: Path,
    *,
    head: str,
    tracked_paths: set[str],
) -> set[int] | None:
    """Return current 1-based lines added relative to HEAD.

    ``None`` means the path is untracked and every line must be scanned.
    """
    relative = str(path.relative_to(repo))
    if relative not in tracked_paths:
        return None
    baseline_result = run_command(
        ["git", "-C", repo, "show", f"{head}:{relative}"],
        timeout=180,
    )
    if not baseline_result.ok:
        raise ConvergenceError(
            f"cannot read HEAD version of tracked candidate {relative} in {repo}"
        )
    baseline_text = baseline_result.stdout.encode("utf-8", "surrogateescape")
    current_bytes = path.read_bytes()
    baseline_lines = baseline_text.splitlines(keepends=True)
    current_lines = current_bytes.splitlines(keepends=True)
    matcher = difflib.SequenceMatcher(
        a=baseline_lines,
        b=current_lines,
        autojunk=False,
    )
    added: set[int] = set()
    for tag, _i1, _i2, j1, j2 in matcher.get_opcodes():
        if tag != "equal":
            added.update(range(j1 + 1, j2 + 1))
    return added


def boundary_content_digest(
    repo: Path,
    policy: dict[str, Any],
    *,
    status: bytes | None = None,
    branch: str | None = None,
    head: str | None = None,
) -> tuple[str, dict[str, Any]]:
    """Hash current eligible candidates and classify unsafe paths without reading secrets."""
    info = git_repo_info(repo)
    git_dir = Path(info["git_dir"])
    status = git_status_bytes(repo) if status is None else status
    candidates = candidate_paths_from_status(status)
    excluded_names = policy_excluded_dir_names(policy)
    max_bytes = int(policy.get("max_stage_file_bytes", 100 * 1024 * 1024))
    records: list[dict[str, Any]] = []
    unsafe: list[dict[str, str]] = []
    resolved_head = head or head_sha(repo)
    tracked_paths = head_tracked_paths(repo, resolved_head)

    for relative in candidates:
        path = repo / relative
        if any(excluded_directory(part, excluded_names) for part in Path(relative).parts[:-1]):
            continue
        nested = nested_repo_metadata(path, repo)
        if nested is not None:
            records.append({key: value for key, value in nested.items() if key not in {"git_dir", "common_dir"}})
            continue
        if path.is_dir() and not path.is_symlink():
            files = list(_walk_candidate_tree(path, repo, excluded_names))
        else:
            files = [path]
        for candidate in files:
            try:
                relative_candidate = str(candidate.relative_to(repo))
            except ValueError:
                continue
            base = candidate
            while base != repo and not base.is_file() and not base.is_symlink():
                base = base.parent
            secret_path = any(SECRET_NAME_RE.search(part) for part in Path(relative_candidate).parts)
            try:
                stat_info = candidate.lstat()
            except FileNotFoundError:
                records.append({"path": relative_candidate, "kind": "missing"})
                continue
            except OSError as exc:
                unsafe.append({"path": relative_candidate, "reason": f"stat failed: {exc}"})
                continue
            if stat.S_ISLNK(stat_info.st_mode):
                records.append(
                    {
                        "path": relative_candidate,
                        "kind": "symlink",
                        "target": os.readlink(candidate),
                        "mtime_ns": stat_info.st_mtime_ns,
                    }
                )
                continue
            if not stat.S_ISREG(stat_info.st_mode):
                records.append(
                    {
                        "path": relative_candidate,
                        "kind": "special",
                        "mode": stat.S_IMODE(stat_info.st_mode),
                        "mtime_ns": stat_info.st_mtime_ns,
                    }
                )
                continue
            if stat_info.st_size > max_bytes:
                unsafe.append(
                    {
                        "path": relative_candidate,
                        "reason": f"file exceeds {max_bytes} bytes",
                    }
                )
                records.append(
                    {
                        "path": relative_candidate,
                        "kind": "oversize-metadata",
                        "size": stat_info.st_size,
                        "mtime_ns": stat_info.st_mtime_ns,
                    }
                )
                continue
            if secret_path:
                unsafe.append(
                    {
                        "path": relative_candidate,
                        "reason": "secret-like path requires Warden/operator handling",
                    }
                )
                records.append(
                    {
                        "path": relative_candidate,
                        "kind": "secret-metadata",
                        "size": stat_info.st_size,
                        "mtime_ns": stat_info.st_mtime_ns,
                    }
                )
                continue
            try:
                content = candidate.read_bytes()
            except OSError as exc:
                unsafe.append({"path": relative_candidate, "reason": f"read failed: {exc}"})
                continue
            content_text = content.decode("utf-8", "ignore")
            current_lines = content.splitlines(keepends=True)
            try:
                added_lines = added_line_numbers(
                    repo,
                    candidate,
                    head=resolved_head,
                    tracked_paths=tracked_paths,
                )
            except ConvergenceError as exc:
                unsafe.append({"path": relative_candidate, "reason": str(exc)})
                continue
            if added_lines is None:
                added_lines = set(range(1, len(current_lines) + 1))
            added_text = b"".join(
                line
                for number, line in enumerate(current_lines, 1)
                if number in added_lines
            )
            if PRIVATE_KEY_RE.search(content_text) or ASSIGNED_SECRET_BYTES_RE.search(added_text):
                unsafe.append(
                    {
                        "path": relative_candidate,
                        "reason": "credential-like content requires Warden/operator handling",
                    }
                )
                records.append(
                    {
                        "path": relative_candidate,
                        "kind": "credential-metadata",
                        "size": stat_info.st_size,
                        "mtime_ns": stat_info.st_mtime_ns,
                    }
                )
                continue
            content_hash = bytes_sha256(content)
            records.append(
                {
                    "path": relative_candidate,
                    "kind": "file",
                    "mode": stat.S_IMODE(stat_info.st_mode),
                    "size": stat_info.st_size,
                    "sha256": content_hash,
                }
            )

    index_path = git_dir / "index"
    index_digest = file_sha256(index_path) if index_path.is_file() else None
    payload = {
        "repo": info["path"],
        "head": resolved_head,
        "branch": branch or attached_branch(repo),
        "status_sha256": bytes_sha256(status),
        "index_sha256": index_digest,
        "candidates": sorted(records, key=lambda item: item["path"]),
        "operation": operation_state(repo),
    }
    return canonical_digest(payload), {
        "digest": canonical_digest(payload),
        "head": payload["head"],
        "branch": payload["branch"],
        "status_sha256": payload["status_sha256"],
        "index_sha256": index_digest,
        "candidate_count": len(records),
        "unsafe_candidates": sorted(unsafe, key=lambda item: item["path"]),
        "captured_at": iso_now(),
    }


def capture_repository(repo: Path, policy: dict[str, Any]) -> dict[str, Any]:
    info = git_repo_info(repo)
    repo = Path(info["path"])
    branch = attached_branch(repo)
    head = head_sha(repo)
    status = git_status_bytes(repo)
    content_digest, content = boundary_content_digest(
        repo, policy, status=status, branch=branch, head=head
    )
    ahead, behind = ahead_behind(repo, branch)
    fsck = run_command(
        ["git", "-C", repo, "fsck", "--connectivity-only", "--no-dangling"],
        timeout=600,
    )
    fsck_output = f"{fsck.stdout}\n{fsck.stderr}"
    if fsck.returncode not in {0, 1}:
        missing_objects = -1
    else:
        missing_objects = len(
            {
                line.strip()
                for line in fsck_output.splitlines()
                if re.search(r"\bmissing\b", line, re.IGNORECASE)
            }
        )
    return {
        **info,
        "branch": branch,
        "head": head,
        "state": {
            **status_counts(repo),
            "ahead": ahead,
            "behind": behind,
            "missing_objects": missing_objects,
            "operation": operation_state(repo),
        },
        "snapshot": {
            "fast_token": fast_token(
                repo, policy, status=status, branch=branch, head=head
            ),
            "content_digest": content_digest,
            "status_sha256": bytes_sha256(status),
            "index_sha256": content["index_sha256"],
            "candidate_count": content["candidate_count"],
            "unsafe_candidates": content["unsafe_candidates"],
            "captured_at": iso_now(),
        },
    }


def wait_for_quiescence(
    repositories: Sequence[dict[str, str]],
    policy: dict[str, Any],
    *,
    selected_roots: Sequence[Path] | None = None,
    freeze_marker: Path = DEFAULT_FREEZE,
    stable_samples: int = 3,
    interval_seconds: float = 10.0,
    max_wait_seconds: int = 1800,
    progress=None,
) -> dict[str, Any]:
    """Wait for stable generation tokens and two identical boundary proofs."""
    if stable_samples < 2:
        raise ConvergenceError("stable_samples must be at least 2")
    if interval_seconds <= 0 or max_wait_seconds <= 0:
        raise ConvergenceError("quiescence intervals must be positive")
    records_by_path = {record["path"]: dict(record) for record in repositories}
    roots = list(selected_roots or [Path(record["path"]) for record in repositories])
    started = time.monotonic()
    stable_count = 0
    previous_tokens: dict[str, str] = {}
    boundary: dict[str, dict[str, Any]] | None = None
    boundary_token: dict[str, str] | None = None
    observations = 0

    while time.monotonic() - started < max_wait_seconds:
        if not freeze_marker.exists():
            raise ConvergenceError(f"freeze marker disappeared before snapshot: {freeze_marker}")
        discovered = discover_repositories(roots, policy)
        for record in discovered:
            records_by_path.setdefault(record["path"], record)
        paths = [Path(path) for path in sorted(records_by_path)]
        current_tokens = {str(repo): fast_token(repo, policy) for repo in paths}
        observations += 1
        if current_tokens == previous_tokens:
            stable_count += 1
        else:
            stable_count = 1
            boundary = None
            boundary_token = None
        previous_tokens = current_tokens
        if progress:
            progress(
                f"quiescence observation {observations}: stable={stable_count}/{stable_samples} "
                f"elapsed={int(time.monotonic() - started)}s"
            )
        if stable_count < stable_samples:
            time.sleep(interval_seconds)
            continue

        candidate_boundary: dict[str, dict[str, Any]] = {}
        candidate_token: dict[str, str] = {}
        for repo in paths:
            digest, details = boundary_content_digest(repo, policy)
            if details["unsafe_candidates"]:
                raise ConvergenceError(
                    f"unsafe adoption candidates in {repo}: "
                    + json.dumps(details["unsafe_candidates"], sort_keys=True)
                )
            candidate_boundary[str(repo)] = details
            candidate_token[str(repo)] = digest
        if boundary_token == candidate_token and boundary is not None:
            final_tokens = {str(repo): fast_token(repo, policy) for repo in paths}
            if final_tokens != current_tokens:
                stable_count = 0
                boundary = None
                boundary_token = None
                time.sleep(interval_seconds)
                continue
            return {
                "stable_samples": stable_samples,
                "observations": observations,
                "interval_seconds": interval_seconds,
                "span_seconds": int(time.monotonic() - started),
                "fast_tokens": final_tokens,
                "boundaries": candidate_boundary,
                "repositories": [records_by_path[path] for path in sorted(records_by_path)],
                "completed_at": iso_now(),
            }
        boundary = candidate_boundary
        boundary_token = candidate_token
        time.sleep(interval_seconds)

    raise ConvergenceError(
        f"selected repositories did not quiesce within {max_wait_seconds}s"
    )


def classify_remote_failure(stderr: str, *, missing: bool = False) -> str | None:
    if missing:
        return None
    value = stderr.lower()
    auth_markers = (
        "permission denied",
        "authentication failed",
        "access denied",
        "could not read username",
        "terminal prompts disabled",
        "http 403",
        "403 forbidden",
    )
    outage_markers = (
        "could not resolve hostname",
        "connection refused",
        "connection timed out",
        "operation timed out",
        "network is unreachable",
        "temporary failure in name resolution",
        "remote end hung up unexpectedly",
        "http 502",
        "http 503",
        "http 504",
        "service unavailable",
        "bad gateway",
    )
    if any(marker in value for marker in auth_markers):
        return "authentication"
    if any(marker in value for marker in outage_markers):
        return "provider_outage"
    return None


def query_remote(
    repo: Path, remote: str, branch: str, attempts: int = 3, timeout: int = 60
) -> dict[str, Any]:
    if not safe_remote_name(remote):
        raise ConvergenceError(f"unsafe remote name: {remote!r}")
    result = run_command(
        ["git", "-C", repo, "ls-remote", remote, f"refs/heads/{branch}"],
        timeout=timeout,
    )
    advertised: str | None = None
    missing = False
    if result.ok and result.stdout.strip():
        fields = result.stdout.strip().splitlines()
        if len(fields) == 1 and re.fullmatch(r"[0-9a-f]{40,64}", fields[0].split()[0]):
            advertised = fields[0].split()[0]
        else:
            result = dataclasses.replace(result, returncode=1, stderr="ambiguous ls-remote output")
    elif result.ok:
        missing = True
        result = dataclasses.replace(result, returncode=2, stderr="remote branch is missing")
    else:
        lower = result.stderr.lower()
        missing = any(
            marker in lower
            for marker in ("repository not found", "could not read from remote repository", "not found: 404")
        ) and "permission denied" not in lower
    blocker = classify_remote_failure(result.stderr, missing=missing)
    return {
        "name": remote,
        "advertised_sha": advertised,
        "returncode": result.returncode,
        "duration_ms": result.duration_ms,
        "stderr": redact(result.stderr.strip()),
        "missing": missing,
        "blocker": blocker,
        "checked_at": iso_now(),
    }


def query_remote_with_retries(
    repo: Path,
    remote: str,
    branch: str,
    *,
    attempts: int = 3,
    timeout: int = 60,
) -> dict[str, Any]:
    evidence: list[dict[str, Any]] = []
    for attempt in range(1, attempts + 1):
        result = query_remote(repo, remote, branch, attempts=1, timeout=timeout)
        evidence.append(result)
        if result["advertised_sha"] or result["blocker"] or result["missing"]:
            break
        if attempt < attempts:
            time.sleep(min(2 ** (attempt - 1), 4))
    final = evidence[-1]
    return {**final, "attempts": evidence}


def capture_remote_state(
    repo_record: dict[str, Any], policy: dict[str, Any], *, attempts: int = 3
) -> list[dict[str, Any]]:
    repo = Path(repo_record["path"])
    branch = repo_record["branch"]
    remotes: list[dict[str, Any]] = []
    for remote in effective_remote_names(policy, repo):
        url_result = run_command(
            ["git", "-C", repo, "remote", "get-url", "--push", remote], timeout=30
        )
        if not url_result.ok:
            remotes.append(
                {
                    "name": remote,
                    "configured": False,
                    "eligible": True,
                    "canonical_project": None,
                    "attempts": [],
                    "blocker": None,
                }
            )
            continue
        queried = query_remote_with_retries(
            repo, remote, branch, attempts=attempts, timeout=60
        )
        advertised = queried.get("advertised_sha")
        local_head = repo_record.get("head")
        if advertised == local_head:
            relation = "equal"
        elif advertised and local_head and _is_ancestor(repo, advertised, local_head):
            relation = "local-ahead"
        elif advertised and local_head and _is_ancestor(repo, local_head, advertised):
            relation = "remote-ahead"
        elif advertised:
            relation = "diverged"
        else:
            relation = "missing" if queried.get("missing") else "unavailable"
        canonical = canonical_remote_url(url_result.stdout.strip())
        expected = expected_remote_project(policy, repo, remote)
        remotes.append(
            {
                "name": remote,
                "configured": True,
                "eligible": True,
                "canonical_project": canonical,
                "expected_project": canonical_remote_url(expected) if expected else None,
                "policy_identity_ok": expected is not None and canonical == canonical_remote_url(expected),
                "before_sha": advertised,
                "relation": relation,
                **queried,
            }
        )
    return remotes


def initialize_evidence(
    repositories: Sequence[dict[str, Any]],
    policy_path: Path,
    freeze_marker: Path,
    quiescence: dict[str, Any],
    *,
    remote_attempts: int = 3,
) -> dict[str, Any]:
    return {
        "schema_version": SCHEMA_VERSION,
        "run_id": f"sync-convergence-{int(time.time())}-{os.getpid()}",
        "created_at": iso_now(),
        "updated_at": iso_now(),
        "contract": {
            "forward_only": True,
            "selected_paths": sorted(record["path"] for record in repositories),
            "policy_file": str(policy_path),
            "policy_sha256": file_sha256(policy_path),
            "freeze_marker": str(freeze_marker),
            "remote_query_attempts": remote_attempts,
        },
        "phases": [
            {
                "name": "quiescent-snapshot",
                "started_at": quiescence["boundaries"][repositories[0]["path"]]["captured_at"],
                "ended_at": quiescence["completed_at"],
                "quiescence": quiescence,
            }
        ],
        "repositories": sorted(repositories, key=lambda item: item["path"]),
        "actions": [],
        "gates": {},
        "external_blockers": [],
        "result": "pending",
    }


def refresh_evidence(
    evidence: dict[str, Any],
    policy: dict[str, Any],
    *,
    remote_attempts: int = 3,
) -> dict[str, Any]:
    updated = dict(evidence)
    repositories: list[dict[str, Any]] = []
    for previous in evidence.get("repositories", []):
        current = capture_repository(Path(previous["path"]), policy)
        preserved = {
            key: value
            for key, value in previous.items()
            if key not in {"state", "snapshot", "remotes"}
        }
        previous_remotes = {
            item.get("name"): item for item in previous.get("remotes", [])
        }
        current_remotes = capture_remote_state(
            current, policy, attempts=remote_attempts
        )
        for remote in current_remotes:
            prior = previous_remotes.get(remote.get("name"), {})
            remote.setdefault("before_sha", prior.get("before_sha"))
            if remote.get("before_sha") is None:
                remote["before_sha"] = remote.get("advertised_sha")
        current["remotes"] = current_remotes
        repositories.append({**preserved, **current})
    updated["repositories"] = sorted(repositories, key=lambda item: item["path"])
    updated["updated_at"] = iso_now()
    return updated


def record_action(
    evidence: dict[str, Any],
    *,
    repository: str,
    kind: str,
    result: str,
    evidence_refs: Sequence[str],
    head_before: str | None = None,
    head_after: str | None = None,
) -> dict[str, Any]:
    allowed_kinds = {"commit", "pull", "merge", "push", "config", "hook", "ignore", "resume"}
    allowed_results = {"ok", "blocked"}
    if kind not in allowed_kinds:
        raise ConvergenceError(f"unsupported action kind: {kind}")
    if result not in allowed_results:
        raise ConvergenceError(f"unsupported action result: {result}")
    if repository not in {record["path"] for record in evidence.get("repositories", [])} and repository != "fleet":
        raise ConvergenceError(f"action names an unselected repository: {repository}")
    refs = sorted({redact(str(ref)) for ref in evidence_refs if str(ref).strip()})
    if not refs:
        raise ConvergenceError("action requires at least one evidence reference")
    updated = dict(evidence)
    updated["actions"] = [
        *evidence.get("actions", []),
        {
            "repository": repository,
            "kind": kind,
            "result": result,
            "head_before": head_before,
            "head_after": head_after,
            "evidence": refs,
            "recorded_at": iso_now(),
        },
    ]
    updated["updated_at"] = iso_now()
    return updated


def record_external_blocker(
    evidence: dict[str, Any],
    policy: dict[str, Any],
    *,
    repository: str,
    remote: str,
    evidence_ref: str,
    attempts: int = 3,
) -> dict[str, Any]:
    records = {record["path"]: record for record in evidence.get("repositories", [])}
    if repository not in records:
        raise ConvergenceError(f"blocker names an unselected repository: {repository}")
    repo = Path(repository)
    if remote not in effective_remote_names(policy, repo):
        raise ConvergenceError(
            f"remote {remote} is not eligible for {repository}; exclusions are not blockers"
        )
    if not evidence_ref.strip():
        raise ConvergenceError("external blocker requires an evidence reference")
    queried = query_remote_with_retries(
        repo,
        remote,
        records[repository]["branch"],
        attempts=attempts,
        timeout=60,
    )
    kind = queried.get("blocker")
    if kind not in ALLOWED_EXTERNAL_BLOCKERS:
        raise ConvergenceError(
            f"remote failure is not an allowed external blocker: {queried.get('stderr') or queried}"
        )
    blocker = {
        "repository": repository,
        "remote": remote,
        "kind": kind,
        "attempts": queried.get("attempts", []),
        "evidence": [redact(evidence_ref.strip())],
        "recorded_at": iso_now(),
    }
    updated = dict(evidence)
    updated["external_blockers"] = [
        *evidence.get("external_blockers", []),
        blocker,
    ]
    updated["updated_at"] = iso_now()
    return updated


def record_gate(
    evidence: dict[str, Any],
    *,
    name: str,
    command: str,
    status: str,
    notes: str,
) -> dict[str, Any]:
    if status not in {"pass", "fail", "blocked"}:
        raise ConvergenceError(f"unsupported gate status: {status}")
    if not command.strip() or not notes.strip():
        raise ConvergenceError("gate requires command and notes")
    updated = dict(evidence)
    gates = dict(evidence.get("gates", {}))
    gates[name] = {
        "command": command.strip(),
        "status": status,
        "notes": redact(notes.strip()),
        "recorded_at": iso_now(),
    }
    updated["gates"] = gates
    updated["updated_at"] = iso_now()
    return updated


def finalize_evidence(
    evidence_path: Path,
    policy_path: Path,
    freeze_marker: Path,
    *,
    remote_attempts: int = 3,
) -> tuple[dict[str, Any], dict[str, Any]]:
    evidence = json.loads(evidence_path.read_text(encoding="utf-8"))
    policy = load_toml(policy_path)
    refreshed = refresh_evidence(evidence, policy, remote_attempts=remote_attempts)
    candidate = dict(refreshed)
    candidate["result"] = "pass"
    candidate_path = evidence_path.with_suffix(".candidate.json")
    atomic_write_json(candidate_path, candidate)
    try:
        verification = verify_evidence(
            candidate_path,
            policy_path,
            freeze_marker=freeze_marker,
            remote_attempts=remote_attempts,
            check_live_remotes=True,
        )
    finally:
        candidate_path.unlink(missing_ok=True)
    if verification["ok"]:
        candidate["result"] = "pass"
        candidate["updated_at"] = iso_now()
    else:
        candidate["result"] = "blocked"
        candidate["updated_at"] = iso_now()
    return candidate, verification


def validate_evidence_shape(evidence: dict[str, Any]) -> list[str]:
    """Validate the checked-in schema's required structural invariants.

    The runtime intentionally has no third-party JSON Schema dependency. This
    function checks the schema's required fields and core enums directly.
    """
    errors: list[str] = []
    required_top = {
        "schema_version",
        "run_id",
        "created_at",
        "updated_at",
        "contract",
        "phases",
        "repositories",
        "actions",
        "gates",
        "external_blockers",
        "result",
    }
    missing = sorted(required_top - set(evidence))
    if missing:
        errors.append(f"missing top-level fields: {missing}")
    if evidence.get("schema_version") != SCHEMA_VERSION:
        errors.append("schema_version must be 1")
    if evidence.get("result") not in {"pending", "pass", "blocked"}:
        errors.append("result must be pending, pass, or blocked")
    contract = evidence.get("contract", {})
    for field in (
        "forward_only",
        "selected_paths",
        "policy_file",
        "policy_sha256",
        "freeze_marker",
        "remote_query_attempts",
    ):
        if field not in contract:
            errors.append(f"missing contract field: {field}")
    if not isinstance(evidence.get("repositories"), list) or not evidence["repositories"]:
        errors.append("repositories must be a non-empty array")
    else:
        paths = [record.get("path") for record in evidence["repositories"]]
        if len(paths) != len(set(paths)):
            errors.append("repository paths must be unique")
        for record in evidence["repositories"]:
            for field in ("path", "role", "git_dir", "common_dir", "branch", "head", "state", "snapshot", "remotes"):
                if field not in record:
                    errors.append(f"repository record missing {field}: {record.get('path')}")
    if not isinstance(evidence.get("phases"), list) or not evidence["phases"]:
        errors.append("phases must be a non-empty array")
    if not isinstance(evidence.get("actions"), list):
        errors.append("actions must be an array")
    if not isinstance(evidence.get("gates"), dict):
        errors.append("gates must be an object")
    if not isinstance(evidence.get("external_blockers"), list):
        errors.append("external_blockers must be an array")
    return errors


def evidence_text(evidence_path: Path) -> str:
    return evidence_path.read_text(encoding="utf-8")


def assert_evidence_safe(evidence: dict[str, Any]) -> None:
    text = json.dumps(evidence, sort_keys=True)
    if PRIVATE_KEY_RE.search(text):
        raise ConvergenceError("evidence contains private-key material")
    if ASSIGNED_SECRET_RE.search(text):
        raise ConvergenceError("evidence contains a secret assignment")
    if REMOTE_USERINFO_RE.search(text):
        raise ConvergenceError("evidence contains URL userinfo")


def _is_ancestor(repo: Path, ancestor: str, descendant: str) -> bool:
    return run_command(
        ["git", "-C", repo, "merge-base", "--is-ancestor", ancestor, descendant],
        timeout=120,
    ).returncode == 0


def _parent_gitlink(parent: Path, child: Path) -> str | None:
    try:
        relative = child.relative_to(parent).as_posix()
    except ValueError:
        return None
    result = run_command(["git", "-C", parent, "ls-tree", "HEAD", "--", relative], timeout=60)
    if not result.ok or not result.stdout.strip():
        return None
    fields = result.stdout.split()
    return fields[2] if len(fields) >= 3 and fields[0] == "160000" else None


def _validate_external_blocker(blocker: dict[str, Any]) -> list[str]:
    errors: list[str] = []
    kind = blocker.get("kind")
    if kind not in ALLOWED_EXTERNAL_BLOCKERS:
        errors.append(f"external blocker kind is not allowed: {kind!r}")
    attempts = blocker.get("attempts")
    if not isinstance(attempts, list) or len(attempts) < 3:
        errors.append("external blocker requires at least three attempts")
    if not blocker.get("evidence"):
        errors.append("external blocker requires a redacted evidence reference")
    return errors


def _forbidden_reflog_actions(repo: Path, since: str) -> list[str]:
    result = run_command(
        ["git", "-C", repo, "reflog", "--all", f"--since={since}", "--format=%gs"],
        timeout=120,
    )
    if not result.ok:
        return []
    actions: list[str] = []
    for line in result.stdout.splitlines():
        action = line.strip()
        if action.lower().startswith(PROHIBITED_REFLOG_PREFIXES):
            actions.append(action)
    return actions


def verify_evidence(
    evidence_path: Path,
    policy_path: Path = DEFAULT_POLICY,
    *,
    freeze_marker: Path = DEFAULT_FREEZE,
    remote_attempts: int = 3,
    check_live_remotes: bool = True,
) -> dict[str, Any]:
    """Verify evidence and current live state without mutating any repository."""
    try:
        evidence = json.loads(evidence_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise ConvergenceError(f"cannot read evidence {evidence_path}: {exc}") from exc
    assert_evidence_safe(evidence)
    shape_errors = validate_evidence_shape(evidence)
    if shape_errors:
        raise ConvergenceError("evidence schema violations: " + "; ".join(shape_errors[:10]))
    if evidence.get("schema_version") != SCHEMA_VERSION:
        raise ConvergenceError("unsupported evidence schema_version")
    if evidence.get("result") != "pass":
        raise ConvergenceError(
            f"evidence result must be 'pass' before final verification, got {evidence.get('result')!r}"
        )
    policy = load_toml(policy_path)
    contract = evidence.get("contract", {})
    if contract.get("forward_only") is not True:
        raise ConvergenceError("evidence contract is not forward-only")
    if Path(contract.get("policy_file", "")) != policy_path:
        raise ConvergenceError("evidence names a different policy file")
    if contract.get("policy_sha256") != file_sha256(policy_path):
        raise ConvergenceError("policy changed after evidence capture")
    if freeze_marker.exists():
        raise ConvergenceError("daemon is still frozen; final convergence is not yet valid")
    if not evidence.get("repositories"):
        raise ConvergenceError("evidence contains no repositories")

    errors: list[str] = []
    warnings: list[str] = []
    checked_remotes = 0
    current_rows: dict[str, dict[str, Any]] = {}
    health: dict[str, Any] = {}
    if check_live_remotes:
        health_raw = run_command(["dracon-sync", "health", "--json"], timeout=180)
        repos_raw = run_command(["dracon-sync", "repos", "--json"], timeout=240)
        if not health_raw.ok:
            errors.append("dracon-sync health --json failed")
        else:
            try:
                health = json.loads(health_raw.stdout)
            except json.JSONDecodeError:
                errors.append("dracon-sync health output is not JSON")
        if not repos_raw.ok:
            errors.append("dracon-sync repos --json failed")
        else:
            try:
                report = json.loads(repos_raw.stdout)
                current_rows = {row["repo"]: row for row in report.get("rows", [])}
            except (json.JSONDecodeError, KeyError, TypeError):
                errors.append("dracon-sync repos output has an invalid shape")
        if health:
            if health.get("frozen"):
                errors.append("daemon health still reports frozen")
            if not health.get("policy_valid"):
                errors.append("daemon policy is invalid")
            if not health.get("daemon_running"):
                errors.append("daemon is not running")

    repositories = evidence["repositories"]
    repo_paths = {record["path"]: Path(record["path"]) for record in repositories}
    parent_paths = [path for path in repo_paths.values() if path.name == "dracon-platform"]

    for record in repositories:
        repo = repo_paths[record["path"]]
        label = record["path"]
        if not repo.is_dir():
            errors.append(f"{label}: repository path disappeared")
            continue
        try:
            live = capture_repository(repo, policy)
        except ConvergenceError as exc:
            errors.append(f"{label}: live capture failed: {exc}")
            continue
        if live["head"] != record.get("head"):
            errors.append(
                f"{label}: evidence is stale; recorded HEAD {record.get('head')} != {live['head']}"
            )
        if live["branch"] != "main":
            errors.append(f"{label}: attached branch is {live['branch']}, expected main")
        counts = live["state"]
        for field in ("modified", "staged", "untracked", "submodule"):
            if counts[field] != 0:
                errors.append(f"{label}: {field}={counts[field]}, expected 0")
        if counts["ahead"] != 0 or counts["behind"] != 0:
            errors.append(
                f"{label}: origin divergence ahead={counts['ahead']} behind={counts['behind']}"
            )
        if counts["operation"] != "none":
            errors.append(f"{label}: active Git state {counts['operation']}")
        if counts["missing_objects"] != 0:
            errors.append(f"{label}: missing_objects={counts['missing_objects']}")
        snapshot_head = (
            evidence.get("phases", [{}])[0]
            .get("quiescence", {})
            .get("boundaries", {})
            .get(label, {})
            .get("head")
        )
        if snapshot_head and not _is_ancestor(repo, snapshot_head, live["head"]):
            errors.append(
                f"{label}: final HEAD is not a forward descendant of quiescent snapshot"
            )
        forbidden = _forbidden_reflog_actions(
            repo, evidence.get("created_at", "1970-01-01T00:00:00Z")
        )
        if forbidden:
            errors.append(f"{label}: prohibited reflog actions recorded: {forbidden[:3]}")

        remote_records = record.get("remotes", [])
        eligible_names = effective_remote_names(policy, repo)
        if sorted(item.get("name") for item in remote_records) != eligible_names:
            errors.append(
                f"{label}: remote evidence does not match effective remotes {eligible_names}"
            )
        for remote_record in remote_records:
            remote = remote_record["name"]
            url_result = run_command(
                ["git", "-C", repo, "remote", "get-url", "--push", remote], timeout=30
            )
            if not url_result.ok:
                errors.append(f"{label}: eligible remote {remote} is not configured")
                continue
            canonical = canonical_remote_url(url_result.stdout.strip())
            if canonical != remote_record.get("canonical_project"):
                errors.append(f"{label}: remote {remote} canonical identity changed")
            if canonical != remote_record.get("expected_project"):
                errors.append(
                    f"{label}: remote {remote} targets {canonical}, expected "
                    f"{remote_record.get('expected_project')}"
                )
            queried = (
                query_remote_with_retries(
                    repo, remote, live["branch"], attempts=remote_attempts, timeout=60
                )
                if check_live_remotes
                else remote_record
            )
            remote_before = remote_record.get("before_sha")
            if remote_before and not _is_ancestor(repo, remote_before, live["head"]):
                errors.append(
                    f"{label}: final HEAD does not descend from remote {remote} snapshot {remote_before}"
                )
            if queried.get("advertised_sha") == live["head"]:
                checked_remotes += 1
                continue
            blocker = queried.get("blocker")
            matching_blockers = [
                item
                for item in evidence.get("external_blockers", [])
                if item.get("repository") == label
                and item.get("remote") == remote
                and item.get("kind") == blocker
                and len(item.get("attempts", [])) >= 3
                and bool(item.get("evidence"))
            ]
            if blocker in ALLOWED_EXTERNAL_BLOCKERS and matching_blockers:
                warnings.append(
                    f"{label}: remote {remote} allowed external blocker {blocker}"
                )
                continue
            errors.append(
                f"{label}: remote {remote} advertises {queried.get('advertised_sha')} "
                f"instead of local HEAD {live['head']}"
            )

        row = current_rows.get(label)
        if check_live_remotes:
            if row is None:
                errors.append(f"{label}: absent from dracon-sync repos --json")
            else:
                for field in ("modified", "staged", "untracked", "excluded_dirty"):
                    if int(row.get(field, -1)) != 0:
                        errors.append(f"{label}: daemon row {field}={row.get(field)}")
                if int(row.get("ahead", -1)) != 0 or int(row.get("behind", -1)) != 0:
                    errors.append(
                        f"{label}: daemon row divergence {row.get('ahead')}/{row.get('behind')}"
                    )
                if row.get("push_status") != "OK":
                    errors.append(f"{label}: daemon push status {row.get('push_status')}")
                if row.get("active") or row.get("warn") or row.get("concern"):
                    errors.append(
                        f"{label}: daemon row active/warn/concern="
                        f"{row.get('active')}/{row.get('warn')}/{row.get('concern')}"
                    )

    for record in repositories:
        repo = repo_paths[record["path"]]
        if record.get("role") != "nested-required":
            continue
        parents = [
            parent
            for parent in parent_paths
            if repo.is_relative_to(Path(parent))
        ]
        if len(parents) != 1:
            errors.append(f"{record['path']}: cannot identify exactly one selected parent")
            continue
        parent = Path(parents[0])
        gitlink = _parent_gitlink(parent, repo)
        if gitlink is None:
            errors.append(f"{record['path']}: parent has no mode-160000 gitlink")
        elif gitlink != head_sha(repo):
            errors.append(
                f"{record['path']}: parent gitlink {gitlink} != child HEAD {head_sha(repo)}"
            )

    selected_paths = set(contract.get("selected_paths", []))
    repository_paths = set(repo_paths)
    if selected_paths != repository_paths:
        errors.append(
            "contract selected_paths do not exactly match repository evidence paths"
        )
    phase_names = {
        str(phase.get("name"))
        for phase in evidence.get("phases", [])
        if isinstance(phase, dict)
    }
    missing_phases = sorted(REQUIRED_FINAL_PHASES - phase_names)
    if missing_phases:
        errors.append(f"missing required final phases: {missing_phases}")
    gates = evidence.get("gates", {})
    missing_gates = sorted(REQUIRED_FINAL_GATES - set(gates))
    if missing_gates:
        errors.append(f"missing required final gates: {missing_gates}")
    for name in sorted(REQUIRED_FINAL_GATES & set(gates)):
        if gates[name].get("status") != "pass":
            errors.append(f"required gate {name} is not pass")
    resume_actions = [
        action
        for action in evidence.get("actions", [])
        if action.get("kind") == "resume" and action.get("result") == "ok"
    ]
    if not resume_actions:
        errors.append("evidence lacks a successful fleet resume action")

    for blocker in evidence.get("external_blockers", []):
        errors.extend(
            f"external blocker {blocker.get('repository')}: {message}"
            for message in _validate_external_blocker(blocker)
        )

    return {
        "ok": not errors,
        "errors": errors,
        "warnings": warnings,
        "checked_remotes": checked_remotes,
        "repositories": len(repositories),
        "health": health,
        "verified_at": iso_now(),
    }


def format_verification(result: dict[str, Any]) -> str:
    lines = [
        f"sync convergence: {result['repositories']} repositories, "
        f"{result['checked_remotes']} eligible remote refs equal local HEAD"
    ]
    lines.extend(f"WARN: {warning}" for warning in result["warnings"])
    lines.extend(f"FAIL: {error}" for error in result["errors"])
    lines.append("PASS: selected repositories are clean and forward-only converged" if result["ok"] else "FAILED")
    return "\n".join(lines)


def main() -> int:
    """Importable module entrypoint; CLIs live in thin wrapper scripts."""
    print(__doc__, file=sys.stderr)
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
