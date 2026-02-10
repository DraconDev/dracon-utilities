{
  description = "Security Manager";
  inputs = { nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable"; };
  outputs = { self, nixpkgs }:
    let
      forAllSystems = nixpkgs.lib.genAttrs [ "x86_64-linux" "aarch64-linux" ];
    in {
      packages = forAllSystems (system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
          src = pkgs.lib.cleanSourceWith {
            filter = name: type: let base = baseNameOf name; in !(type == "directory" && (base == "target" || base == ".git"));
            src = ./.;
          };
        in {
          default = pkgs.rustPlatform.buildRustPackage {
            pname = "dracon-security";
            version = "0.1.0";
            inherit src;
            cargoLock.lockFile = ./Cargo.lock;
          };
        }
      );
    };
}
