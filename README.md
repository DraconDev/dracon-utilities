# Dracon Utilities

Three small Rust CLI tools that look after a Linux development machine:
one keeps your git repos committed and backed up, one keeps your disk
and memory from falling over, and one stops secrets from landing in
your git history. They run as ordinary user-level systemd services —
no root required.

| Tool | The problem it solves | In one line |
|------|-----------------------|-------------|
| [`dracon-sync`](#dracon-sync) | "I edited files all day and never committed." | Watches your repos, auto-commits changes with deterministic messages, pushes to GitHub/GitLab/wherever |
| [`dracon-system`](#dracon-system) | "My disk filled up mid-build and everything froze." | Guards disk & memory pressure, cleans known space hogs, diagnoses storage issues |
| [`dracon-warden`](#dracon-warden) | "I nearly pushed my API key to GitHub." | Transparently encrypts secret-shaped files (age) so they're safe at rest in git but plaintext in your editor |

Everything here is one Cargo workspace: `cargo test` at the repo root
builds and tests all three (~1000 tests).

## Try it in 60 seconds

```bash
git clone https://github.com/DraconDev/dracon-utilities.git
cd dracon-utilities
cargo build --release --locked

./target/release/dracon-system doctor      # health check of this machine
./target/release/dracon-sync status        # what sync would watch
```

## Install

```bash
./install.sh                                # binaries -> ~/.local/bin
systemctl --user enable --now dracon-sync.service        # background auto-commit
systemctl --user enable --now dracon-system-guard.service # background guard
```

`dracon-warden` is not a service — after install, run
`dracon-warden setup-hooks --global` once to arm its git hooks, then
`dracon-warden keygen` to create the machine's age keypair.

Each tool then reads its TOML config from `~/.dracon/utilities/`
(examples ship in each tool's directory). Nothing tracked by git ever
lives outside the repo tree; state goes to `~/.dracon/` and
`~/.local/state/dracon/`.

## The tools in practice

### `dracon-sync`

You work; it commits. Give it watch roots (`~/Dev`, say) and it
discovers every git repo beneath them, waits for edits to settle,
commits with deterministic diff-based messages, and pushes to origin
plus any configured mirrors. It classifies push failures
(`STUCK`/`BLOCKED`/`BROKEN`) instead of silently retrying forever, and
flags repos that vanish from disk.

```bash
dracon-sync daemon              # continuous loop (what the service runs)
dracon-sync repos               # live report of every watched repo
dracon-sync sync-now ~/Dev/my-project
dracon-sync health
```

More: [`dracon-sync/README.md`](dracon-sync/README.md)

### `dracon-system`

A watchdog for the machine itself. When disk or memory pressure crosses
warn/critical thresholds it can clean regenerable junk (build dirs,
caches), deprioritize memory hogs so your desktop stays responsive, and
bias the kernel's out-of-memory killer toward the actual offenders.
Also does one-shot diagnostics: storage hotspots, symlink management,
zram status.

```bash
dracon-system doctor            # deterministic diagnostics pass
dracon-system storage ~/Dev     # where did my disk go?
dracon-system guard clean       # reclaim space (dry-run first)
dracon-system guard daemon      # continuous monitoring (the service)
```

More: [`dracon-system/README.md`](dracon-system/README.md)

### `dracon-warden`

Git filter-based encryption using [age](https://age-encryption.org/).
Files matching secret patterns (`.env`, `*.key`, …) are encrypted on
commit and decrypted on checkout, so the working tree stays normal
while the remote holds only ciphertext. Global hooks back this up by
scanning every push for secret-shaped content and blocking risky ones.

```bash
dracon-warden keygen            # machine age keypair
dracon-warden setup-hooks --global
dracon-warden status            # what's protected on this machine
```

More: [`dracon-warden/README.md`](dracon-warden/README.md)

## Repository layout

```
dracon-utilities/
├── dracon-sync/      # auto-commit multi-remote git sync daemon
├── dracon-system/    # disk/process guard, storage & diagnostics
├── dracon-warden/    # age-based git secret encryption (hooks + CLI)
│   └── src/security/ #   embedded dracon-security crate
├── .github/workflows/ci.yml  # lint / test / release-build / deny / nix
├── flake.nix         # optional Nix build path
├── scripts/          # release, checks, audit tooling
└── AGENTS.md         # policies for agents/operators working here
```

## Development

```bash
export DRACON_SYNC_GIT_BIN=/run/current-system/sw/bin/git  # NixOS only
cargo test --workspace --locked
cargo clippy --workspace --locked -- -D warnings
cargo deny check
nix flake check                 # optional
```

Per-tool builds: `cargo build --release --locked -p dracon-{sync,system,warden}`.

## Releases

Latest: **v0.113.53** (2026-08-22) — see
[Releases](https://github.com/DraconDev/dracon-utilities/releases).
Current component versions: `dracon-sync` 0.113.53 ·
`dracon-system` 0.112.38 · `dracon-warden` 0.113.5 (RC).
Details per tool in each directory's `CHANGELOG.md`.

> History note: before 2026-08-22 each utility lived in its own repo
> (`DraconDev/dracon-sync-background-auto-commit-multi-remote`,
> `...-disk-process-guard-doctor`, `...-secret-encrypt-age-git-filter`).
> Those remain as frozen mirrors; all development happens here now.

## Documentation

| Document | Purpose |
|----------|---------|
| [docs/ROADMAP.md](docs/ROADMAP.md) | Documentation map and release status |
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | Service architecture and deterministic commit protocol |
| [docs/OPERATIONS.md](docs/OPERATIONS.md) | Systemd units, incident response, troubleshooting |
| [AGENTS.md](AGENTS.md) | Repo architecture history, daemon policies, commit discipline |
| [SECURITY.md](SECURITY.md) | Security reporting policy |
| [CONTRIBUTING.md](CONTRIBUTING.md) | Contribution workflow |
| [CHANGELOG.md](CHANGELOG.md) | Version history |

## License

AGPL-3.0-only — see [LICENSE](LICENSE) for details.
