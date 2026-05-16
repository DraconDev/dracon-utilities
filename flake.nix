{
  description = "Dracon Utilities — CLI binaries for dracon system services";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    dracon-libs = {
      url = "github:DraconDev/dracon-libs";
      flake = false;
    };
  };

  outputs = { self, nixpkgs, flake-utils, dracon-libs }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };

        # Merge dracon-utilities and dracon-libs into a single source tree
        # so that Cargo path dependencies (../../dracon-libs/...) resolve.
        #
        # Layout inside mergedSrc:
        #   dracon-utilities/   <- workspace root (Cargo.toml, Cargo.lock, crates)
        #   dracon-libs/        <- sibling (tools/sync/dracon-git, tools/system/dracon-system)
        #
        # dracon-sync depends on ../../dracon-libs/tools/sync/dracon-git
        # dracon-system depends on ../../dracon-libs/tools/system/dracon-system
        # Both resolve because dracon-libs is a sibling of dracon-utilities.
        mergedSrc = pkgs.runCommand "dracon-merged-src" {
          # Force rebuild when inputs change
          inherit dracon-libs;
        } ''
          mkdir -p $out
          cp -r ${./.} $out/dracon-utilities
          cp -r ${dracon-libs} $out/dracon-libs
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

      in {
        packages = {
          dracon-sync = pkgs.rustPlatform.buildRustPackage (commonArgs // {
            pname = "dracon-sync";
            version = "0.1.5";
            buildAndTestSubdir = "dracon-sync";
            cargoBuildFeatures = [ "scribe" "ai-bumper" ];
            # Tests need git, serial execution, and network access (some tests hang
            # in the Nix sandbox). Tests run via 'cargo test' in CI.
            doCheck = false;
          });

          dracon-system = pkgs.rustPlatform.buildRustPackage (commonArgs // {
            pname = "dracon-system";
            version = "0.2.0";
            buildAndTestSubdir = "dracon-system";
            nativeCheckInputs = [ pkgs.git ];
            checkFlags = [
              "--test-threads=1"
              # Skip tests that require D-Bus (no D-Bus in Nix sandbox)
              "--skip" "guard_report_completes_for_ok_disk"
            ];
          });

          dracon-warden = pkgs.rustPlatform.buildRustPackage (commonArgs // {
            pname = "dracon-warden";
            version = "0.1.1";
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
            # Link dracon-libs if not present (for Cargo path deps)
            if [ ! -d "../dracon-libs" ]; then
              echo "NOTE: dracon-libs not found at ../dracon-libs"
              echo "  git clone https://github.com/DraconDev/dracon-libs.git ../dracon-libs"
            fi
          '';
        };
      }
    ) // {
      # Home Manager module for declarative systemd user services
      homeManagerModules.dracon = { config, lib, pkgs, ... }:
        with lib;
        let
          cfg = config.services.dracon;
          draconPkgs = self.packages.${pkgs.system};
        in {
          options.services.dracon = {
            sync.enable = mkEnableOption "dracon-sync daemon (git sync automation)";
            system.enable = mkEnableOption "dracon-system guard daemon (disk/process protection)";
            warden.enable = mkEnableOption "dracon-warden daemon (secret hardening)";

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
            warden.package = mkOption {
              type = types.package;
              default = draconPkgs.dracon-warden;
              description = "dracon-warden package to use";
            };

            sync.policyPath = mkOption {
              type = types.str;
              default = "%h/.dracon/utilities/sync/dracon-sync.toml";
              description = "Path to dracon-sync policy file";
            };
          };

          config = {
            # --- dracon-sync ---
            systemd.user.services.dracon-sync = mkIf cfg.sync.enable {
              Unit = {
                Description = "Dracon Sync (deterministic sync runtime)";
                Documentation = "https://github.com/DraconDev/dracon-utilities";
                After = [ "default.target" ];
              };
              Service = {
                Type = "simple";
                Environment = [
                  "PATH=%h/.local/bin:/run/wrappers/bin:%h/.nix-profile/bin:%h/.local/state/nix/profile/bin:/etc/profiles/per-user/%u/bin:/nix/var/nix/profiles/default/bin:/run/current-system/sw/bin"
                  "DRACON_SYNC_POLICY=${cfg.sync.policyPath}"
                  "GIT_TERMINAL_PROMPT=0"
                ];
                PassEnvironment = [ "SSH_AUTH_SOCK" ];
                ExecStartPre = "-pkill -x -f 'dracon-git pulse'";
                ExecStart = "${cfg.sync.package}/bin/dracon-sync daemon";
                Restart = "on-failure";
                RestartSec = "5";
                RestartPreventExitStatus = "2 78";
                Nice = "10";
                CPUQuota = "15%";
                MemoryHigh = "768M";
                MemoryMax = "2G";
                TasksMax = "96";
                NoNewPrivileges = true;
                ProtectSystem = "strict";
                ProtectHome = "read-only";
                ReadWritePaths = [ "%h/.dracon" "%h/Dev" "%h/.local/state/dracon" "%h/.ssh" ];
                PrivateTmp = true;
              };
              Install = {
                WantedBy = [ "default.target" ];
              };
            };

            # --- dracon-system ---
            systemd.user.services.dracon-system-guard = mkIf cfg.system.enable {
              Unit = {
                Description = "Dracon System Guard - Proactive disk space monitoring and cleanup";
                Documentation = "https://github.com/DraconDev/dracon-utilities";
                After = [ "network.target" ];
              };
              Service = {
                Type = "simple";
                Environment = [
                  "PATH=%h/.local/bin:/run/wrappers/bin:%h/.nix-profile/bin:%h/.local/state/nix/profile/bin:/etc/profiles/per-user/%u/bin:/nix/var/nix/profiles/default/bin:/run/current-system/sw/bin"
                ];
                ExecStart = "${cfg.system.package}/bin/dracon-system guard daemon";
                Restart = "on-failure";
                RestartSec = "10";
                RestartPreventExitStatus = "2 78";
                MemoryMax = "250M";
                CPUQuota = "20%";
                TasksMax = "64";
                NoNewPrivileges = true;
                ProtectSystem = "strict";
                ProtectHome = "read-only";
                ReadWritePaths = [ "%h/.dracon" "%h/Dev" "%h/.local/state/dracon" "%h/.local/share/Trash" "%h/.cargo" "%h/.cache" "%h/.npm" ];
                PrivateTmp = true;
              };
              Install = {
                WantedBy = [ "default.target" ];
              };
            };

            # --- dracon-warden ---
            systemd.user.services.dracon-warden = mkIf cfg.warden.enable {
              Unit = {
                Description = "Dracon Warden (lightweight runtime)";
                Documentation = "https://github.com/DraconDev/dracon-utilities";
                After = [ "default.target" ];
              };
              Service = {
                Type = "simple";
                Environment = [
                  "PATH=%h/.local/bin:/run/wrappers/bin:%h/.nix-profile/bin:%h/.local/state/nix/profile/bin:/etc/profiles/per-user/%u/bin:/nix/var/nix/profiles/default/bin:/run/current-system/sw/bin"
                ];
                ExecStart = "${cfg.warden.package}/bin/dracon-warden daemon";
                Restart = "on-failure";
                RestartSec = "3";
                RestartPreventExitStatus = "2 78";
                Nice = "10";
                CPUQuota = "10%";
                MemoryHigh = "384M";
                MemoryMax = "1G";
                TasksMax = "64";
                NoNewPrivileges = true;
                ProtectSystem = "strict";
                ProtectHome = "read-only";
                ReadWritePaths = [ "%h/.dracon" "%h/Dev" "%h/.local/state/dracon" ];
                PrivateTmp = true;
              };
              Install = {
                WantedBy = [ "default.target" ];
              };
            };
          };
        };
    };
}
