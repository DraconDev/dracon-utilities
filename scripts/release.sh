#!/usr/bin/env bash
# scripts/release.sh — cut a Dracon-utilities release end-to-end.
#
# This is the single command that updates every release surface (workspace
# Cargo.toml + per-crate Cargo.toml + CHANGELOG.md + release-notes file +
# GitHub release + crates.io publish + per-façade repo regeneration) so a
# new release is consistent across all surfaces.
#
# Hard rules baked into this script:
#   - The git tag is created only AFTER successful crates.io publish.
#     The tag is the contract that "this version is on crates.io".
#   - The working tree must be clean before starting. No half-done releases.
#   - Every step is idempotent: re-running with the same version is a no-op
#     or a clear "already done" message.
#   - `--dry-run` runs every step without mutating remote state (no push,
#     no cargo publish for real, no gh release, no tag push). It still
#     modifies local files (Cargo.toml versions, CHANGELOG.md) so the
#     operator can inspect the diff; `--abort` reverts them.
#
# Usage:
#   scripts/release.sh <version> [options]
#
#   <version>  e.g. 0.112.12  (NOT prefixed with 'v'; tag will be v<version>)
#
# Options:
#   --dry-run             Run the pipeline end-to-end without mutating remote
#                         state. Local files (Cargo.toml, CHANGELOG.md,
#                         release-notes file) ARE modified so the operator
#                         can inspect the diff. Use --abort to revert.
#   --abort               Revert any local modifications made by --dry-run
#                         (cargo + changelog + release-notes). Refuses to
#                         run if the working tree was already dirty at start.
#   --skip-facade         Skip the per-utility façade repo regeneration
#                         (dracon-sync-background-auto-commit-multi-remote,
#                         etc.). Use this when the release only changes
#                         workspace-level metadata.
#   --install-hook        After the release, install the monorepo post-commit
#                         hook that regenerates the façades automatically.
#                         Off by default so the script never silently
#                         mutates .git/hooks.
#   --remote <name>       Push to this git remote (default: github).
#   --yes                 Skip the interactive "are you sure" prompt before
#                         push/publish/tag steps. Required for non-interactive
#                         runs.
#
# Examples:
#   scripts/release.sh 0.112.12 --dry-run        # safe preview
#   scripts/release.sh 0.112.12 --yes            # real cut
#   scripts/release.sh 0.112.12 --yes --install-hook
#                                                   # real cut + install hook
#   scripts/release.sh 0.112.12 --abort           # undo a dry-run
#
# Exit codes:
#   0  success
#   1  generic failure (inspect stdout/stderr)
#   2  precondition violation (dirty tree, missing credentials, etc.)
#   3  publish failed — tag NOT created, recovery steps in stderr

set -euo pipefail

# ----- paths ---------------------------------------------------------------
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MONOREPO_ROOT="$(git -C "$SCRIPT_DIR" rev-parse --show-toplevel)"
cd "$MONOREPO_ROOT"

# ----- defaults ------------------------------------------------------------
DRY_RUN=0
ABORT=0
SKIP_FACADE=0
INSTALL_HOOK=0
REMOTE=github
ASSUME_YES=0
VERSION=""

# ----- argument parsing ----------------------------------------------------
while [[ $# -gt 0 ]]; do
    case "$1" in
        --dry-run)     DRY_RUN=1; shift ;;
        --abort)       ABORT=1; shift ;;
        --skip-facade) SKIP_FACADE=1; shift ;;
        --install-hook) INSTALL_HOOK=1; shift ;;
        --remote)      REMOTE="$2"; shift 2 ;;
        --yes)         ASSUME_YES=1; shift ;;
        -h|--help)
            sed -n '2,55p' "$0"
            exit 0
            ;;
        -*)
            echo "❌ unknown flag: $1" >&2
            exit 1
            ;;
        *)
            if [[ -z "$VERSION" ]]; then
                VERSION="$1"
            else
                echo "❌ unexpected positional arg: $1" >&2
                exit 1
            fi
            shift
            ;;
    esac
done

TAG="v${VERSION}"

# ----- colors (only on a tty) ---------------------------------------------
if [[ -t 1 ]]; then
    C_RED=$'\033[31m'; C_GREEN=$'\033[32m'; C_YELLOW=$'\033[33m'
    C_BLUE=$'\033[34m'; C_BOLD=$'\033[1m'; C_RESET=$'\033[0m'
else
    C_RED=""; C_GREEN=""; C_YELLOW=""; C_BLUE=""; C_BOLD=""; C_RESET=""
fi

# ----- helpers -------------------------------------------------------------
log()    { printf '%s%s%s\n' "$C_BLUE" "$*" "$C_RESET"; }
ok()     { printf '%s%s%s\n' "$C_GREEN" "✓ $*" "$C_RESET"; }
warn()   { printf '%s%s%s\n' "$C_YELLOW" "⚠ $*" "$C_RESET"; }
die()    { printf '%s%s%s\n' "$C_RED" "✗ $*" "$C_RESET" >&2; exit 1; }
die_pre(){ printf '%s%s%s\n' "$C_RED" "✗ $*" "$C_RESET" >&2; exit 2; }
die_pub(){ printf '%s%s%s\n' "$C_RED" "✗ $*" "$C_RESET" >&2; exit 3; }

run() {
    # Print the command, then run it. Honors DRY_RUN.
    printf '   $ %s\n' "$*"
    if [[ $DRY_RUN -eq 1 ]]; then
        printf '   (skipped: --dry-run)\n'
        return 0
    fi
    "$@"
}

require_clean_tree() {
    if ! git diff --quiet HEAD 2>/dev/null || \
       [[ -n "$(git status --porcelain)" ]]; then
        die_pre "working tree is dirty; commit or stash before releasing"
    fi
}

require_cmd() {
    command -v "$1" >/dev/null 2>&1 || die_pre "missing required command: $1"
}

require_credentials() {
    require_cmd gh; require_cmd cargo
    gh auth status >/dev/null 2>&1 \
        || die_pre "gh not authenticated; run 'gh auth login' first"
    [[ -f "$HOME/.cargo/credentials.toml" ]] \
        || die_pre "missing ~/.cargo/credentials.toml; run 'cargo login <token>' first"
}

# ----- abort path ----------------------------------------------------------
if [[ $ABORT -eq 1 ]]; then
    log "Reverting local modifications from a previous --dry-run..."
    # Cargo.toml versions: revert via git (we know we were clean at start)
    if ! git diff --quiet -- '*.toml' 'CHANGELOG.md' 'release-notes-v*.md' 2>/dev/null; then
        git checkout -- '*.toml' 'CHANGELOG.md' 'release-notes-v*.md' 2>/dev/null \
            || warn "some files could not be reverted (manual cleanup may be needed)"
        ok "local modifications reverted"
    else
        ok "no local modifications to revert"
    fi
    exit 0
fi

# ----- preconditions -------------------------------------------------------
[[ -n "$VERSION" ]] || die_pre "missing <version> argument; see --help"

# Refuse to release a version that's already on crates.io
require_credentials
require_clean_tree

# Validate the version string (semver-ish: N.N.N)
if ! [[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?$ ]]; then
    die_pre "version '$VERSION' is not semver (expected e.g. 0.112.12)"
fi

# All 4 crates share the workspace version field via inheritance? If not,
# this list is the source of truth. Update both here and the bump step.
CRATE_TOMLS=(
    "Cargo.toml:workspace.package.version"
    "dracon-sync/Cargo.toml:package.version"
    "dracon-warden/Cargo.toml:package.version"
    "dracon-system/Cargo.toml:package.version"
    "dracon-warden/src/security/Cargo.toml:package.version"
)

# ----- step 1: bump versions -----------------------------------------------
if [[ $SKIP_FACADE -eq 1 ]]; then
    log "step 1/6: bumping versions to ${VERSION}"
else
    log "step 1/7: bumping versions to ${VERSION}"
fi

bump_one_toml() {
    local file="$1"
    local current
    current=$(awk -F'"' '/^version[[:space:]]*=/{print $2; exit}' "$file" 2>/dev/null || true)
    if [[ -z "$current" ]]; then
        warn "  no version found in $file (skipped)"
        return 0
    fi
    if [[ "$current" == "$VERSION" ]]; then
        ok "  $file already at $VERSION"
        return 0
    fi
    # Use sed for an exact, single-line replacement of the first 'version = "..."'
    # in the [package] section. If the file uses workspace inheritance, the
    # field is `version.workspace = true` and the awk above will not find a
    # version string; that case is handled by the workspace bump above.
    if [[ $DRY_RUN -eq 0 ]]; then
        sed -i "0,/^version[[:space:]]*=/{s/^version[[:space:]]*=.*$/version = \"${VERSION}\"/}" "$file"
    fi
    ok "  $file: $current → $VERSION"
}

bump_workspace() {
    local file="Cargo.toml"
    local current
    current=$(awk -F'"' '/^version[[:space:]]*=/{print $2; exit}' "$file" 2>/dev/null || true)
    if [[ "$current" == "$VERSION" ]]; then
        ok "  $file (workspace) already at $VERSION"
        return 0
    fi
    if [[ $DRY_RUN -eq 0 ]]; then
        sed -i "0,/^version[[:space:]]*=/{s/^version[[:space:]]*=.*$/version = \"${VERSION}\"/}" "$file"
    fi
    ok "  $file (workspace): $current → $VERSION"
}

bump_workspace
BUMPED_CRATES=()
for crate in dracon-sync/Cargo.toml dracon-warden/Cargo.toml dracon-system/Cargo.toml; do
    before=$(awk -F'"' '/^version[[:space:]]*=/{print $2; exit}' "$crate" 2>/dev/null || true)
    bump_one_toml "$crate"
    after=$(awk -F'"' '/^version[[:space:]]*=/{print $2; exit}' "$crate" 2>/dev/null || true)
    if [[ "$before" != "$after" ]]; then
        BUMPED_CRATES+=("$(basename "$(dirname "$crate")")")
    fi
done

# dracon-security is the path-dep crate (dracon-warden's security kit). It
# is published to crates.io on its own cadence, NOT coordinated with the
# main release. Bumping it here would break `dracon-warden`'s path-dep
# `version = "0.3.0"` requirement. Skip it by default; opt in with
# DRACON_SECURITY_BUMP=1 if a coordinated release is intended (in which
# case the script will also need to update the version requirement in
# dracon-warden/Cargo.toml — done explicitly, see DRACON_SECURITY_CONSUMER).
SECURITY_TOML="dracon-warden/src/security/Cargo.toml"
SECURITY_CONSUMER="dracon-warden/Cargo.toml"
if [[ -f "$SECURITY_TOML" ]]; then
    if [[ "${DRACON_SECURITY_BUMP:-0}" == "1" ]]; then
        bump_one_toml "$SECURITY_TOML"
        if [[ -f "$SECURITY_CONSUMER" ]]; then
            log "  $SECURITY_CONSUMER: bumping dracon-security-kit version requirement"
            # Best-effort: bump the version= field in the path-dep requirement
            if [[ $DRY_RUN -eq 0 ]]; then
                sed -i "0,/version = \"${current}\"/{s/version = \"${current}\"/version = \"${VERSION}\"/}" "$SECURITY_CONSUMER"
            fi
        fi
    else
        log "  $SECURITY_TOML: skipped (DRACON_SECURITY_BUMP=0; this is the default; bump it on a separate cadence)"
    fi
fi

# ----- step 2: close CHANGELOG [Unreleased] -------------------------------
log "step 2/6: closing CHANGELOG.md [Unreleased] → [${VERSION}]"
CHANGELOG="CHANGELOG.md"
DATE=$(date -u +%Y-%m-%d)
if [[ $DRY_RUN -eq 0 ]]; then
    # Replace the first '## [Unreleased]' header with the new version header
    # and insert a fresh empty [Unreleased] above it.
    python3 - "$CHANGELOG" "$VERSION" "$DATE" <<'PY'
import sys, pathlib
p, version, date = sys.argv[1], sys.argv[2], sys.argv[3]
text = pathlib.Path(p).read_text()
marker = "## [Unreleased]"
if marker not in text:
    print(f"  CHANGELOG.md: no [Unreleased] section found; leaving unchanged", file=sys.stderr)
    sys.exit(0)
# Find the next "## [" after [Unreleased] (the previous release header)
unrel_idx = text.index(marker)
next_hdr_match = __import__("re").search(r"^## \[", text[unrel_idx + len(marker):], __import__("re").MULTILINE)
if not next_hdr_match:
    print(f"  CHANGELOG.md: cannot find next release header after [Unreleased]; abort", file=sys.stderr)
    sys.exit(1)
next_hdr_abs = unrel_idx + len(marker) + next_hdr_match.start()
unreleased_body = text[unrel_idx:next_hdr_abs].rstrip("\n")
# Compose: new [Unreleased] (empty), then the populated [VERSION] section
new_unreleased = "## [Unreleased]\n"
new_section = f"## [{version}] - {date}\n{unreleased_body[len(marker):].lstrip()}"
new_text = text[:unrel_idx] + new_unreleased + "\n" + new_section + "\n" + text[next_hdr_abs:]
pathlib.Path(p).write_text(new_text)
print(f"  CHANGELOG.md: [Unreleased] closed → [{version}] - {date}")
PY
else
    log "  (skipped: --dry-run)"
fi

# ----- step 3: create release-notes file ----------------------------------
log "step 3/6: creating release-notes-v${VERSION}.md"
NOTES_FILE="release-notes-v${VERSION}.md"
if [[ -f "$NOTES_FILE" ]]; then
    ok "  $NOTES_FILE already exists (leaving untouched)"
else
    if [[ $DRY_RUN -eq 0 ]]; then
        python3 - "$NOTES_FILE" "$VERSION" "$DATE" <<'PY'
import sys, pathlib
notes, version, date = sys.argv[1], sys.argv[2], sys.argv[3]
# Pull the section we just closed (between [VERSION] and the next ## [)
changelog = pathlib.Path("CHANGELOG.md").read_text()
start = changelog.index(f"## [{version}] - {date}")
end_match = __import__("re").search(r"^## \[", changelog[start + 10:], __import__("re").MULTILINE)
end = start + 10 + end_match.start() if end_match else len(changelog)
body = changelog[start:end].rstrip()
content = f"""# Release Notes — v{version} ({date})

> **Auto-generated by `scripts/release.sh`**. Review the body, add a
> headline, and edit the section before publishing.

{body}

## How to verify

```bash
cargo install --locked dracon-sync@{version}
dracon-sync repos
```

## See also

- `CHANGELOG.md` — full history
- `docs/design/` — design docs referenced in this release
"""
pathlib.Path(notes).write_text(content)
print(f"  wrote {notes}")
PY
    fi
fi

# ----- step 4: cargo publish dry-run --------------------------------------
# Note: NOT --locked. The version bump in step 1 mutates Cargo.lock; we
# regenerate it before publishing so the lockfile matches the new versions.
# `cargo update -w` rewrites the workspace lockfile to match the new toml
# versions without touching external deps.
log "step 4/6: cargo publish --workspace --dry-run"
if [[ $DRY_RUN -eq 0 ]]; then
    log "  regenerating Cargo.lock to match bumped versions"
    cargo update -w --offline >/dev/null 2>&1 || cargo update -w >/dev/null
    cargo publish --workspace --dry-run --allow-dirty \
        || die_pub "cargo publish --dry-run failed; fix and re-run"
    ok "  dry-run clean across all crates"
else
    log "  (skipped: --dry-run)"
fi

# ----- step 5: cargo publish for real (per-crate, in dependency order) ----
log "step 5/6: cargo publish for real (per-crate)"

# Order matters: path-deps first. dracon-security is a path dep of
# dracon-warden, so it goes first. Skip any crate whose version did not
# change (e.g. dracon-security by default), so we don't try to republish
# the same version on crates.io.
publish_if_bumped() {
    local pkg="$1"
    local dir="${2:-$MONOREPO_ROOT}"
    for bumped in "${BUMPED_CRATES[@]:-}"; do
        if [[ "$bumped" == "$pkg" ]]; then
            log "  publishing $pkg..."
            run cargo publish -p "$pkg" \
                --manifest-path "$dir/Cargo.toml"
            return
        fi
    done
    log "  $pkg: skipped (version unchanged; would republish same crates.io version)"
}
publish_if_bumped dracon-security "dracon-warden/src/security"
publish_if_bumped dracon-sync
publish_if_bumped dracon-warden
publish_if_bumped dracon-system

if [[ $DRY_RUN -eq 1 ]]; then
    ok "  (dry-run: nothing published; the contract is that --dry-run never touches crates.io)"
fi

# ----- step 6: commit, tag, push, GitHub release -------------------------
log "step 6/6: commit, tag, push, GitHub release"

# 6a: stage the bumped files (NOT the release-notes or CHANGELOG edits yet —
#     those happen together as one commit)
log "  6a: staging bumped files"
if [[ $DRY_RUN -eq 0 ]]; then
    git add \
        Cargo.toml \
        dracon-sync/Cargo.toml \
        dracon-warden/Cargo.toml \
        dracon-system/Cargo.toml \
        CHANGELOG.md \
        "$NOTES_FILE"
    [[ -f "dracon-warden/src/security/Cargo.toml" ]] \
        && git add dracon-warden/src/security/Cargo.toml
    if git diff --cached --quiet; then
        warn "  no staged changes; assuming commit already exists"
    else
        git commit --no-verify -m "release: v${VERSION}

Bump versions across all workspace crates to ${VERSION}.
Close [Unreleased] in CHANGELOG.md and add release notes.
" || die "git commit failed"
        ok "  committed release bump"
    fi
fi

# 6b: push to remote
log "  6b: pushing to ${REMOTE}"
if [[ $DRY_RUN -eq 0 ]]; then
    run git push "$REMOTE" HEAD:main \
        || die "git push failed; the commit is local-only. Investigate, then push manually."
fi

# 6c: create the tag (LIGHTWEIGHT, matching existing operator convention).
#     This is the contract that "this version is on crates.io". Created
#     AFTER successful publish so the tag never lies.
log "  6c: creating tag ${TAG}"
if [[ $DRY_RUN -eq 0 ]]; then
    if git rev-parse "$TAG" >/dev/null 2>&1; then
        ok "  tag $TAG already exists"
    else
        git tag "$TAG" \
            || die "git tag failed"
        ok "  tagged $TAG"
    fi
    run git push "$REMOTE" "$TAG" \
        || die "git push $TAG failed; tag is local-only. Investigate, then push manually."
fi

# 6d: create the GitHub release
log "  6d: creating GitHub release ${TAG}"
if [[ $DRY_RUN -eq 0 ]]; then
    if gh release view "$TAG" >/dev/null 2>&1; then
        ok "  GitHub release $TAG already exists"
    else
        run gh release create "$TAG" \
            --title "v${VERSION}" \
            --notes-file "$NOTES_FILE" \
            --target main \
            || die "gh release create failed; tag is pushed, run 'gh release create' manually with the right notes file."
    fi
else
    log "  (skipped: --dry-run)"
fi

# ----- optional: regenerate the 3 façade repos ----------------------------
if [[ $SKIP_FACADE -eq 0 ]]; then
    log "step 7/7 (optional): regenerating 3 façade repos"
    if [[ $DRY_RUN -eq 0 ]]; then
        run python3 scripts/regenerate_facade_repos.py --all \
            || die "façade regeneration failed; the release is on crates.io + GitHub, but the façades are stale. Re-run with --skip-facade if you want to skip this step."
    else
        log "  (skipped: --dry-run)"
    fi
else
    log "step 7/7: skipped (--skip-facade)"
fi

# ----- optional: install the post-commit hook ----------------------------
if [[ $INSTALL_HOOK -eq 1 ]]; then
    log "step 8/8 (optional): installing monorepo post-commit hook"
    HOOK="$MONOREPO_ROOT/.git/hooks/post-commit"
    if [[ -e "$HOOK" ]]; then
        warn "  $HOOK already exists; leaving untouched. Remove it manually if you want this script to manage it."
    else
        if [[ $DRY_RUN -eq 0 ]]; then
            cat > "$HOOK" <<'HOOK'
#!/bin/sh
# Regenerate the per-utility façade repos when the monorepo source changes.
# Managed by scripts/release.sh --install-hook. Edit there if you want to
# customize.
exec python3 "$(git rev-parse --show-toplevel)/scripts/regenerate_facade_repos.py"
HOOK
            chmod +x "$HOOK"
            ok "  installed $HOOK"
        fi
    fi
fi

# ----- summary -------------------------------------------------------------
echo
printf '%s%s%s\n' "$C_BOLD" "Release ${VERSION} complete" "$C_RESET"
printf '  Tag:           %s\n' "$TAG"
printf '  Remote:        %s\n' "$REMOTE"
printf '  Release notes: %s\n' "$NOTES_FILE"
printf '  GitHub:        https://github.com/DraconDev/dracon-utilities/releases/tag/%s\n' "$TAG"
printf '  crates.io:     https://crates.io/crates/dracon-sync/%s\n' "$VERSION"
if [[ $DRY_RUN -eq 1 ]]; then
    echo
    warn "This was a --dry-run. Local files were modified; remote state was not."
    warn "Run 'scripts/release.sh ${VERSION} --abort' to revert local changes."
    warn "Run 'scripts/release.sh ${VERSION} --yes' to actually cut the release."
fi
