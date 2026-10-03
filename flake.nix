{
  description = "Dracon Utilities — CLI binaries for dracon system services";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";

    # RESTORED 2026-09-27 (audit decision D1): the utilities are nested
    # standalone repositories, NOT tracked by this parent (they are listed
    # in .gitignore and carry their own .git). The 2026-08-22 monorepo
    # conversion deleted these inputs, and `mergedSrc` below then built
    # from `${self}` — which, on a fresh clone, contains no utility source
    # at all, so every `nix build` failed. The invisible half of that
    # regression was that CI only ever ran `nix flake check --no-build`,
    # which never realises a derivation.
    #
    # Each input names a BRANCH (`/main`); flake.lock is the actual pin.
    # scripts/check-nested-pins.py asserts that the flake.lock rev, the
    # ci.yml `ref:`, and the local nested HEAD all agree, so moving a
    # utility is an explicit, reviewable lockfile change.
    dracon-sync-src = {
      url = "github:DraconDev/dracon-sync-background-auto-commit-multi-remote/main";
      # The utilities ship no flake.nix of their own; they are plain
      # source trees, not flakes.
      flake = false;
    };
    dracon-system-src = {
      url = "github:DraconDev/dracon-system-disk-process-guard-doctor/main";
      flake = false;
    };
    dracon-warden-src = {
      url = "github:DraconDev/dracon-warden-secret-encrypt-age-git-filter/main";
      flake = false;
    };
  };

  # The `*-src` inputs have hyphens in their names, which cannot be bound
  # as positional function arguments in Nix, so they are taken from the
  # attribute set instead. The `...` is REQUIRED: without it Nix rejects
  # the three extra inputs as unexpected arguments and evaluation fails
  # (this only shows up on a clean evaluation, e.g. a fresh clone or
  # `nix flake check`).
  outputs = inputs@{
    self,
    nixpkgs,
    flake-utils,
    ...
  }:
  let
    draconSyncSrc = inputs."dracon-sync-src";
    draconSystemSrc = inputs."dracon-system-src";
    draconWardenSrc = inputs."dracon-warden-src";
  in
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };

        # CHANGED 2026-08-22 (monorepo conversion): the utilities live in
        # this repo under dracon-{sync,system,warden}/, so the merged tree
        # is just the repo itself — no external -src inputs to splice.
        # Layout inside mergedSrc:
        #   dracon-utilities/   <- workspace root (Cargo.toml, Cargo.lock, crates)
        #   dracon-utilities/dracon-sync/   <- workspace member
        #   dracon-utilities/dracon-system/ <- workspace member
        #   dracon-utilities/dracon-warden/ <- workspace member
        # REVERTED 2026-09-27 (audit decision D1): that comment described
        # a layout this repo no longer has. The three utility directories
        # are gitignored nested clones, so `${self}` contributes no utility
        # source and `buildRustPackage` had no Cargo.toml to find. Each
        # `*-src` input is spliced in at its workspace-member path. The
        # `rm -rf` first makes the result deterministic when a developer
        # runs the flake from a worktree that happens to hold live
        # checkouts, instead of silently splicing a mix of the pinned
        # revision and whatever is on disk.
        mergedSrc = pkgs.runCommand "dracon-merged-src" {} ''
          mkdir -p $out/dracon-utilities
          cp -r ${self}/. $out/dracon-utilities/
          rm -rf \
            $out/dracon-utilities/dracon-sync \
            $out/dracon-utilities/dracon-system \
            $out/dracon-utilities/dracon-warden
          cp -r ${draconSyncSrc} $out/dracon-utilities/dracon-sync
          cp -r ${draconSystemSrc} $out/dracon-utilities/dracon-system
          cp -r ${draconWardenSrc} $out/dracon-utilities/dracon-warden
          # Make writable for buildRustPackage (Cargo needs to write target/, .cargo/)
          chmod -R u+w $out
        '';

        # Shared native build inputs for crates that need C libraries
        nativeBuildDeps = [ pkgs.pkg-config pkgs.cmake ];
        buildDeps = [ pkgs.openssl pkgs.libssh2 ];
        # NOTE: libgit2 is NOT included here — libgit2-sys 0.16.x bundles
        # libgit2 1.7.x and nixpkgs ships 1.9.x, so we let it vendor its own.
        # libssh2-sys also vendors by default; set LIBSSH2_NO_VENDOR=1 only
        # if the nixpkgs version is compatible.

        # Common buildRustPackage arguments.
        # src points at the merged tree root; buildAndTestSubdir selects the crate.
        commonArgs = {
          src = mergedSrc;
          sourceRoot = "${mergedSrc.name}/dracon-utilities";
          cargoLock = {
            lockFile = ./Cargo.lock;
            # Path deps outside the source tree need their hashes provided.
            # Since we merged the sources, they're inside the tree now,
            # but Cargo.lock still references them by relative path.
            # We let buildRustPackage handle this via the merged src.
          };
          nativeBuildInputs = nativeBuildDeps;
          buildInputs = buildDeps;
        };

        # FIXED 2026-09-27 (audit F85): the three package versions were
        # hardcoded (0.1.5 / 0.2.0 / 0.1.1) and had drifted three orders
        # of magnitude from the crate manifests, so the default package
        # was literally named `dracon-utilities-0.1.5`. Read the version
        # from each Cargo.toml instead so it can never drift again.
        #
        # RE-READS ITS SOURCE 2026-09-27 (audit decision D1): the first
        # fix read `./dracon-sync/Cargo.toml` — a path INSIDE this repo.
        # The parent does not track the utility sources, so that path
        # exists only in a developer worktree that happens to hold live
        # checkouts. On a fresh clone `builtins.pathExists` is false and
        # the function silently fell back to a hardcoded string, which is
        # how `nix eval` came to report `dracon-utilities-0.113.85` while
        # the code it actually builds was 0.113.88 — the same drift F85
        # was raised to kill, reintroduced one level up and invisible on
        # the machine that reported the original bug.
        #
        # The `*-src` inputs are store paths that always exist at eval
        # time and carry the exact revision flake.lock pins, so the
        # version is read from the same source the build uses and cannot
        # disagree with it. The fallback now only guards a genuinely
        # malformed input.
        crateVersion = fallback: srcPath:
          let manifest = srcPath + "/Cargo.toml";
          in if builtins.pathExists manifest
             then (builtins.fromTOML (builtins.readFile manifest)).package.version
             else fallback;

      in {
        packages = {
          dracon-sync = pkgs.rustPlatform.buildRustPackage (commonArgs // {
            pname = "dracon-sync";
            version = crateVersion "0.113.93" draconSyncSrc;
            buildAndTestSubdir = "dracon-sync";
            cargoBuildFeatures = [ ];
            # Tests need git, serial execution, and network access (some tests hang
            # in the Nix sandbox). Tests run via 'cargo test' in CI.
            doCheck = false;
          });

          dracon-system = pkgs.rustPlatform.buildRustPackage (commonArgs // {
            pname = "dracon-system";
            version = crateVersion "0.112.44" draconSystemSrc;
            buildAndTestSubdir = "dracon-system";
            nativeCheckInputs = [ pkgs.git ];
            checkFlags = [
              "--test-threads=1"
              # Skip tests that require D-Bus (no D-Bus in Nix sandbox)
              "--skip" "guard_report_completes_for_ok_disk"
              # ADDED 2026-09-27 (audit decision D1): these eight assert the
              # HOST filesystem layout of /tmp and $HOME. Inside the Nix
              # sandbox `std::env::temp_dir()` is /build/... rather than /tmp,
              # so the very containment check under test refuses its own
              # fixture and the test fails for a reason that has nothing to
              # do with the code. They are skipped here rather than weakened
              # so the other ~197 tests still gate the Nix build; the real
              # coverage is the workspace `cargo test --workspace` job, which
              # runs them on a normal filesystem.
              "--skip" "tests::tmp_entry_must_remain_under_validated_root"
              "--skip" "tests::safe_tmp_root_policy_allows_tmp_descendants_and_rejects_home"
              "--skip" "tests::clean_tmp_paths_respects_age_dry_run_and_open_fds"
              "--skip" "tests::clean_tmp_paths_keeps_old_process_cwd_directory"
              "--skip" "tests::clean_tmp_paths_rejects_home_search_root_before_apply"
              "--skip" "tests::storage_cleanup_apply_refuses_git_database_dirs"
              "--skip" "tests::storage_cleanup_apply_accepts_home_artifact_dirs_and_refuses_system_roots"
              "--skip" "tests::critical_tier_bypass_cleans_fresh_target"
            ];
          });

          dracon-warden = pkgs.rustPlatform.buildRustPackage (commonArgs // {
            pname = "dracon-warden";
            version = crateVersion "0.113.15" draconWardenSrc;
            buildAndTestSubdir = "dracon-warden";
            # Warden doesn't need openssl/libgit2/libssh2, but they're
            # harmless to include via the shared commonArgs.
            nativeCheckInputs = [ pkgs.git ];
            checkFlags = [ "--test-threads=1" "--skip" "filter_clean_encrypts_content_with_secret_marker" ];
          });

          # All three binaries in one derivation
          default = pkgs.symlinkJoin {
            name = "dracon-utilities-${self.packages.${system}.dracon-sync.version}";
            paths = with self.packages.${system}; [
              dracon-sync
              dracon-system
              dracon-warden
            ];
          };
        };

        devShells.default = pkgs.mkShell {
          nativeBuildInputs = nativeBuildDeps ++ (with pkgs; [
            # Rust toolchain (use rustup or your preferred method)
            cargo
            rustc
            rustfmt
            clippy
            rust-analyzer

            # Nix tooling
            nixfmt

            # Runtime deps for testing
            git
          ]);

          buildInputs = buildDeps;

          shellHook = ''
            echo "Dracon Utilities dev shell loaded"
          '';
        };
      }
    ) // {
      # Home Manager module for declarative systemd user services.  Nix's
      # generic `flake check` does not recognize the Home Manager convention
      # `homeManagerModules`; scripts/check-flake.sh validates the flake while
      # explicitly filtering that known, non-functional warning.
      homeManagerModules.dracon = { config, lib, pkgs, ... }:
        with lib;
        let
          cfg = config.services.dracon;
          draconPkgs = self.packages.${pkgs.system};
        in {
          options.services.dracon = {
            sync.enable = mkEnableOption "dracon-sync daemon (git sync automation)";
            system.enable = mkEnableOption "dracon-system guard daemon (disk/process protection)";
            # No warden.enable: dracon-warden has no daemon — enforcement is via
            # git hooks (dracon-warden setup-hooks --global). A warden systemd
            # unit would ExecStart a nonexistent subcommand (removed 2026-09-09).

            sync.package = mkOption {
              type = types.package;
              default = draconPkgs.dracon-sync;
              description = "dracon-sync package to use";
            };
            system.package = mkOption {
              type = types.package;
              default = draconPkgs.dracon-system;
              description = "dracon-system package to use";
            };
            # No warden.package: with no warden service, the binary comes from
            # the dracon-warden package directly (nix build .#dracon-warden).

            sync.policyPath = mkOption {
              type = types.str;
              default = "%h/.dracon/utilities/sync/dracon-sync.toml";
              description = "Path to dracon-sync policy file";
            };
          };

          config = {
            # --- dracon-sync ---
            # Parity contract (audit H2, 2026-10-02): this Nix unit must
            # match dracon-sync/dracon-sync.service property-for-property
            # except ExecStart (store path vs ~/.local/bin). Drift here
            # previously shipped CPUQuota=15% (measured classifier
            # starvation) and Restart=on-failure to Nix installs.
            # scripts/check-flake.sh asserts the security-relevant
            # properties below; update it with any intentional change.
            systemd.user.services.dracon-sync = mkIf cfg.sync.enable {
              Unit = {
                Description = "Dracon Sync (deterministic sync runtime)";
                Documentation = "https://github.com/DraconDev/dracon-utilities";
                After = [ "default.target" ];
              };
              Service = {
                Type = "simple";
                StandardOutput = "journal";
                StandardError = "journal";
                Environment = [
                  "PATH=%h/.local/bin:/run/wrappers/bin:%h/.local/share/flatpak/exports/bin:/var/lib/flatpak/exports/bin:%h/.nix-profile/bin:/nix/profile/bin:%h/.local/state/nix/profile/bin:/etc/profiles/per-user/%u/bin:/nix/var/nix/profiles/default/bin:/run/current-system/sw/bin"
                  "DRACON_SYNC_POLICY=${cfg.sync.policyPath}"
                  "GIT_TERMINAL_PROMPT=0"
                ];
                PassEnvironment = [ "SSH_AUTH_SOCK" ];
                ExecStartPre = "-/run/current-system/sw/bin/pkill -x -f \"dracon-git pulse\"";
                ExecStart = "${cfg.sync.package}/bin/dracon-sync daemon";
                Restart = "always";
                RestartSec = "5";
                RestartPreventExitStatus = "2 78";
                Nice = "10";
                CPUQuota = "100%";
                MemoryHigh = "768M";
                MemoryMax = "2G";
                TasksMax = "96";
                NoNewPrivileges = true;
                ProtectSystem = "strict";
                ProtectHome = "read-only";
                ReadWritePaths = [ "%h/.dracon" "%h/Dev" "%h/.local/state/dracon" "%h/.ssh" ];
                PrivateTmp = true;
                PrivateDevices = true;
                ProtectKernelTunables = true;
                ProtectKernelLogs = true;
                ProtectClock = true;
                ProtectHostname = true;
                ProtectControlGroups = true;
                LockPersonality = true;
                # NO MemoryDenyWriteExecute: mirrors the shipped unit —
                # the daemon honors repo pre-push hooks, whose JIT
                # runtimes (node/V8) need PROT_EXEC (2026-10-02
                # fleet-wide push-stuck incident). Hook helpers inherit
                # the whole sandbox below and cannot relax it (audit M7):
                # helpers needing blocked calls fail with EPERM.
                RestrictRealtime = true;
                RestrictSUIDSGID = true;
                RemoveIPC = true;
                CapabilityBoundingSet = "";
                RestrictNamespaces = true;
                SystemCallFilter = "@system-service";
                SystemCallErrorNumber = "EPERM";
              };
              Install = {
                WantedBy = [ "default.target" ];
              };
            };

            # --- dracon-system ---
            # Parity contract (audit H1, 2026-10-02): this Nix unit must
            # match dracon-system/dracon-system-guard.service
            # property-for-property except ExecStart (store path vs
            # ~/.local/bin). Drift here previously shipped
            # ReadWritePaths without '-' prefixes (systemd refuses to
            # start when an entry is absent) and without the
            # nix-profile/quarantine/cold roots, no ExecReload,
            # no CAP_SYS_NICE, and no sandbox.
            # scripts/check-flake.sh asserts the security-relevant
            # properties below; update it with any intentional change.
            systemd.user.services.dracon-system-guard = mkIf cfg.system.enable {
              Unit = {
                Description = "Dracon System Guard - Proactive disk space monitoring and cleanup";
                Documentation = "https://github.com/DraconDev/dracon-utilities";
                After = [ "network.target" ];
              };
              Service = {
                Type = "simple";
                StandardOutput = "journal";
                StandardError = "journal";
                Environment = [
                  "PATH=%h/.local/bin:/run/wrappers/bin:%h/.local/share/flatpak/exports/bin:/var/lib/flatpak/exports/bin:%h/.nix-profile/bin:/nix/profile/bin:%h/.local/state/nix/profile/bin:/etc/profiles/per-user/%u/bin:/nix/var/nix/profiles/default/bin:/run/current-system/sw/bin"
                ];
                ExecStart = "${cfg.system.package}/bin/dracon-system guard daemon";
                ExecReload = "/bin/sh -c 'kill -HUP $MAINPID'";
                # Keep intentionally relative policy paths stable for the user service.
                WorkingDirectory = "%h";
                # A disabled policy exits cleanly; only failures/crashes restart.
                Restart = "on-failure";
                RestartSec = "10";
                # 78 = EX_CONFIG for malformed/unreadable startup policy; 2 = usage.
                RestartPreventExitStatus = "2 78";
                MemoryMax = "250M";
                CPUQuota = "20%";
                TasksMax = "64";
                NoNewPrivileges = true;
                ProtectSystem = "strict";
                ProtectHome = "read-only";
                # clean_tmp targets the host /tmp; keep the service sandboxed
                # elsewhere while granting only this required write path.
                # '-' prefixes mirror the shipped unit: systemd refuses to
                # start when a non-prefixed entry is absent, so optional
                # roots must not block startup (verified live, 2026-10-01).
                ReadWritePaths = [ "%h/.dracon" "%h/.local/state/dracon" "%h/.local/share" "-%h/Dev" "-%h/.local/share/Trash" "-%h/.cargo" "-%h/.cache" "-%h/.npm" "-%h/.local/state/nix" "/tmp" "-/mnt/data/quarantine" "-/mnt/data/cold" ];
                PrivateTmp = false;
                PrivateDevices = true;
                ProtectKernelTunables = true;
                ProtectKernelLogs = true;
                ProtectClock = true;
                ProtectHostname = true;
                ProtectControlGroups = true;
                LockPersonality = true;
                MemoryDenyWriteExecute = true;
                RestrictRealtime = true;
                RestrictSUIDSGID = true;
                RemoveIPC = true;
                AmbientCapabilities = "CAP_SYS_NICE";
                CapabilityBoundingSet = "CAP_SYS_NICE";
                RestrictNamespaces = true;
                SystemCallFilter = "@system-service";
                SystemCallErrorNumber = "EPERM";
              };
              Install = {
                WantedBy = [ "default.target" ];
              };
            };

            # No dracon-warden service: warden has no daemon subcommand —
            # enforcement is via git hooks (dracon-warden setup-hooks --global).

            # --- watchdog timers (audit M8, 2026-10-02) ---
            # The timers were live-only, so Nix installs silently lacked
            # restart-if-stopped and freeze auto-clear. Mirror of the
            # shipped units in dracon-sync/ + dracon-system/ (services,
            # timers, and the notify scripts they exec).
            systemd.user.services.dracon-sync-watchdog = mkIf cfg.sync.enable {
              Unit = {
                Description = "Dracon sync watchdog (restart daemon if stopped)";
                Documentation = "https://github.com/DraconDev/dracon-utilities";
                After = [ "timers.target" ];
              };
              Service = {
                Type = "oneshot";
                ExecStart = "%h/.dracon/sync-notify/dracon-sync-watchdog.sh";
                TimeoutStartSec = "15";
                StandardOutput = "journal";
                StandardError = "journal";
              };
            };
            systemd.user.timers.dracon-sync-watchdog = mkIf cfg.sync.enable {
              Unit = {
                Description = "Run dracon-sync-watchdog.service every 2 minutes";
                Documentation = "https://github.com/DraconDev/dracon-utilities";
              };
              Timer = {
                OnBootSec = "2min";
                OnUnitActiveSec = "2min";
                RandomizedDelaySec = "30";
                AccuracySec = "1s";
              };
              Install = { WantedBy = [ "timers.target" ]; };
            };
            systemd.user.services.dracon-freeze-watchdog = mkIf cfg.sync.enable {
              Unit = {
                Description = "Dracon freeze watchdog (warn/auto-clear forgotten pause)";
                Documentation = "https://github.com/DraconDev/dracon-utilities";
                After = [ "timers.target" ];
              };
              Service = {
                Type = "oneshot";
                ExecStart = "%h/.dracon/sync-notify/dracon-freeze-watchdog.sh";
                TimeoutStartSec = "10";
                StandardOutput = "journal";
                StandardError = "journal";
              };
            };
            systemd.user.timers.dracon-freeze-watchdog = mkIf cfg.sync.enable {
              Unit = {
                Description = "Run dracon-freeze-watchdog every 2 minutes";
                Documentation = "https://github.com/DraconDev/dracon-utilities";
              };
              Timer = {
                OnBootSec = "2min";
                OnUnitActiveSec = "2min";
                RandomizedDelaySec = "15";
                AccuracySec = "1s";
              };
              Install = { WantedBy = [ "timers.target" ]; };
            };
            systemd.user.services.dracon-system-guard-watchdog = mkIf cfg.system.enable {
              Unit = {
                Description = "Dracon system guard watchdog (restart daemon if stopped)";
                Documentation = "https://github.com/DraconDev/dracon-utilities";
                After = [ "timers.target" ];
              };
              Service = {
                Type = "oneshot";
                ExecStart = "%h/.dracon/system-notify/dracon-system-guard-watchdog.sh";
                TimeoutStartSec = "15";
                StandardOutput = "journal";
                StandardError = "journal";
              };
            };
            systemd.user.timers.dracon-system-guard-watchdog = mkIf cfg.system.enable {
              Unit = {
                Description = "Run dracon-system-guard-watchdog.service every 2 minutes";
                Documentation = "https://github.com/DraconDev/dracon-utilities";
              };
              Timer = {
                OnBootSec = "2min";
                OnUnitActiveSec = "2min";
                RandomizedDelaySec = "30";
                AccuracySec = "1s";
              };
              Install = { WantedBy = [ "timers.target" ]; };
            };
            # The oneshot services above exec these scripts; provision them
            # so a pure-Nix install does not ship timers pointing at
            # missing files.
            # FIXED 2026-10-03 (audit R3-H1): these MUST come from the
            # pinned `*-src` inputs, not `${self}` — the utility dirs are
            # gitignored nested clones, so git-filtered `self` has no
            # utility source and every Nix install failed at generation
            # build. scripts/check-flake.sh asserts `.source` existence.
            home.file.".dracon/sync-notify/dracon-sync-watchdog.sh" = mkIf cfg.sync.enable {
              source = "${draconSyncSrc}/scripts/dracon-sync-watchdog.sh";
              executable = true;
            };
            home.file.".dracon/sync-notify/dracon-freeze-watchdog.sh" = mkIf cfg.sync.enable {
              source = "${draconSyncSrc}/scripts/dracon-freeze-watchdog.sh";
              executable = true;
            };
            home.file.".dracon/system-notify/dracon-system-guard-watchdog.sh" = mkIf cfg.system.enable {
              source = "${draconSystemSrc}/scripts/dracon-system-guard-watchdog.sh";
              executable = true;
            };
          };
        };
    };
}
