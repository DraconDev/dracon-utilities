# Ownership and Codeberg auto-provisioning

**Implemented in**: dracon-sync v0.113.39
**Scope**: watched-path ownership, forge provisioning, and mirror visibility

## Ownership

A repository discovered beneath a configured `watch_roots` entry is owned by
path policy. This is the operator's explicit synchronization scope and is the
strongest signal for daemon operation.

- `owned = false` remains a hard per-repository opt-out.
- Legacy `owned = true` remains accepted for backwards compatibility, but is
  unnecessary under the new default.
- Local `user.email`, HEAD author, and `origin` host/account checks remain
  useful diagnostics. A mismatch produces an ownership warning on a
  path-owned repository; it does not prevent a commit or a push to a
  configured operator remote.
- Repositories outside configured watch roots retain the conservative
  trusted-signal heuristic.
- Trusted lists are never expanded automatically.
- Empty initialized repositories under a watch root are claimed and
  provisioned; the first commit still has to exist before a branch can be
  pushed.

This separates two concerns that were previously conflated:

1. **Which local paths the operator asked the daemon to manage?** Path policy.
2. **Where may the daemon publish?** Configured operator remote namespaces.

A foreign `origin` is therefore fetch-only and is reported as a warning. The
daemon does not push to it merely because it exists in `.git/config`.

## Forge provisioning

New GitHub and GitLab repositories are private by default. The daemon never
turns an unknown visibility result into a public repository.

Codeberg is a public-only marketing mirror under the quota posture:

1. Query every configured operator-owned GitHub/GitLab forge.
2. If any forge positively reports **public**, Codeberg creation and pushes
   are enabled for that repository and a newly created Codeberg repository is
   public.
3. If every positively queried forge reports **private**, new Codeberg
   creation and pushes are skipped.
4. If any required visibility query fails or is unknown, Codeberg publication
   is skipped. The cache is not overwritten with a guessed private/public
   result, and stale cache entries are not sufficient to authorize new
   publication.
5. An existing Codeberg mirror is never deleted. When a repository becomes
   private everywhere, future Codeberg pushes stop while the mirror remains
   available for operator review or a later public transition.

The global Codeberg `auto_create = false` setting remains the quota-safe
fallback. A fresh positive public visibility result is an effective,
repository-scoped opt-in; enabling Codeberg globally is not required.

## Convergence and safety

All permitted configured remotes are expected to converge to the selected
local branch tip, regardless of whether GitHub/GitLab are private. The daemon
uses ordinary fast-forward/merge reconciliation and stops on conflicts. It
does not rewrite published remote history or force-push divergent branches.

The `dracon-sync repos --json` report exposes effective push/exclude decisions,
so verification tools can distinguish an intentionally skipped private or
unknown Codeberg mirror from a failed permitted push. A clean repository is
not permanently marked provisioned while Codeberg eligibility is private or
unknown: a later positive visibility refresh can authorize creation without a
synthetic commit.

A pre-existing Codeberg mirror that is already divergent is reported as a
warning and is not force-pushed. Normal merge reconciliation remains available,
but choosing to merge a large historical mirror is an operator decision; the
no-rewrite policy takes precedence over making the branch tips identical.

## Live incident outcome

On 2026-08-04, `ai-auto-writer` recovered from a damaged local HEAD/object
store through the existing recovery activity: the healthy remote branch was
restored, current worktree content was preserved, `.git.corrupt-bak/` was
removed from tracking and added to `.gitignore`, and GitHub/GitLab were
verified at the same branch tip. No remote history rewrite was used.

`darklord` had local identity values literally set to `--global`. Its local
identity was corrected to `darklord-dev <darklord@dracon.local>` without
rewriting its existing commits, then its normal fast-forward tip was pushed
to the configured GitLab and GitHub mirrors.

The public watched repositories were then provisioned on Codeberg and their
existing local `main` tips were pushed without rewriting history. The one
pre-existing divergent `dracon-strategy/DraconDev` Codeberg mirror remains
explicitly preserved and reported as a warning.
