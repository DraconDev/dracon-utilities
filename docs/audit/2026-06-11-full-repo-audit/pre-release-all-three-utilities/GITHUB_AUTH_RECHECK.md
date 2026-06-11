# GitHub login prompt re-check

Date: 2026-06-11

User report: GitHub login prompts were still appearing, possibly because the PAT helper was not updated or not applied.

## Checks performed

- `gh auth status -h github.com` shows logged in as `DraconDev` via keyring.
- `~/.gitconfig` had the PAT helper first, but also had empty `helper =` lines in the GitHub/Gist URL sections.
- `git credential fill` worked and returned a GitHub credential without prompting.
- `GIT_TERMINAL_PROMPT=0 git ls-remote origin HEAD` worked.
- A scan of all watched repos with GitHub `origin` URLs using `GIT_TERMINAL_PROMPT=0 git ls-remote ... HEAD` succeeded for all tested repos.

Evidence:

- `github-auth-prompt-recheck/github-auth-recheck.log`
- `github-auth-prompt-recheck/github-helper-check.log`
- `github-auth-prompt-recheck/github-lsremote-summary.tsv`

## Safe cleanup applied

Removed only the empty per-URL helper entries from `~/.gitconfig`.

Before:

```ini
[credential "https://github.com"]
    helper = !/home/dracon/.dracon/secrets/pat/git-credential-github.sh
    helper = 
    helper = !/etc/profiles/per-user/dracon/bin/gh auth git-credential
```

After:

```ini
[credential "https://github.com"]
    helper = !/home/dracon/.dracon/secrets/pat/git-credential-github.sh
    helper = !/etc/profiles/per-user/dracon/bin/gh auth git-credential
```

The PAT helper remains first, so it should still bypass the keyring/`gh` helper for GitHub HTTPS operations.

Evidence after cleanup:

- `github-auth-prompt-recheck/github-helper-after-empty-cleanup.log`
- `github-auth-prompt-recheck/github-helper-after-empty-cleanup.exit`

## Remaining push blocker

`git push --dry-run origin HEAD` is still blocked, but not by a GitHub login prompt. It is blocked by the local warden pre-push hook because commits being pushed contain secret-shaped fixture/evidence lines.

Evidence:

- `github-auth-prompt-recheck/github-push-dryrun-after-helper-cleanup.log`

No secret values were printed. No token was rotated. No keyring or `~/.git-credentials` contents were changed.
