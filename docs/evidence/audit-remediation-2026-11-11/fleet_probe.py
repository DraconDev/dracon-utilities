#!/usr/bin/env python3
"""Read-only fleet probe: per-repo mirror-ref honesty, spool count, freeze.

Prints one TSV row per repo. Network calls (ls-remote) are bounded and
parallel; a remote that fails/times out reports `unreachable`.
"""
import json, subprocess, sys, os, glob
from concurrent.futures import ThreadPoolExecutor

GIT = "/run/current-system/sw/bin/git"

def run(cmd, cwd=None, timeout=45):
    try:
        return subprocess.run(cmd, cwd=cwd, capture_output=True, text=True,
                              timeout=timeout, env={**os.environ, "GIT_TERMINAL_PROMPT": "0"})
    except subprocess.TimeoutExpired:
        return None

def ls_remote(repo, name, branch):
    r = run([GIT, "-C", repo, "ls-remote", name, f"refs/heads/{branch}"], timeout=60)
    if r is None or r.returncode != 0:
        return "unreachable"
    out = r.stdout.strip().split()
    return out[0][:10] if out else "no-ref"

def probe(row):
    repo = row["repo"]
    branch = row.get("branch") or "main"
    if branch == "HEAD" or not branch:
        branch = "main"
    r = run([GIT, "-C", repo, "remote"])
    remotes = sorted(r.stdout.split()) if r and r.returncode == 0 else []
    # primary configured upstream
    up = run([GIT, "-C", repo, "rev-parse", "--abbrev-ref", f"{branch}@{'{upstream}'}"])
    upstream = up.stdout.strip() if up and up.returncode == 0 else "-"
    refs, net_jobs = {}, []
    for name in remotes:
        t = run([GIT, "-C", repo, "rev-parse", "--verify", "-q", f"refs/remotes/{name}/{branch}"])
        refs[name] = t.stdout.strip()[:10] if t and t.returncode == 0 else "-"
        if refs[name] != "-":
            net_jobs.append(name)
    head = run([GIT, "-C", repo, "rev-parse", "HEAD"])
    head_sha = head.stdout.strip()[:10] if head and head.returncode == 0 else "?"
    with ThreadPoolExecutor(max_workers=6) as ex:
        futs = {n: ex.submit(ls_remote, repo, n, branch) for n in net_jobs}
        real = {n: f.result() for n, f in futs.items()}
    spools = len(glob.glob(os.path.join(repo, ".git", "dracon-staging-*")))
    # worktree gitdir may be elsewhere for submodules
    gd = run([GIT, "-C", repo, "rev-parse", "--path-format=absolute", "--git-dir"])
    if gd and gd.returncode == 0:
        gdpath = gd.stdout.strip()
        spools += len(glob.glob(os.path.join(gdpath, "dracon-staging-*")))
    return {
        "repo": repo, "branch": branch, "head": head_sha, "upstream": upstream,
        "refs": refs, "real": real, "spools": spools,
        "frozen_secs": row.get("frozen_secs"), "ahead": row.get("ahead"),
        "push_state": row.get("publish_state"),
    }

d = json.load(open(sys.argv[1]))
rows = d["rows"]
with ThreadPoolExecutor(max_workers=6) as ex:
    results = list(ex.map(probe, rows))
# attach the deferred/未 probed names' real sha lazily: names already equal HEAD need no network
json.dump(results, open(sys.argv[2], "w"), indent=1)
print(f"probed {len(results)} repos -> {sys.argv[2]}")
