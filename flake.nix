{
  description = "grove: keep your git checkouts in view (read only)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    crane.url = "github:ipetkov/crane";
  };

  outputs =
    { self, nixpkgs, crane }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "aarch64-darwin"
      ];
      forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});

      perSystem =
        pkgs:
        let
          inherit (pkgs) lib;
          craneLib = crane.mkLib pkgs;

          # Cargo sources plus what the tests read: the transcripts (docs/), fixtures (tests/), deny.toml.
          src = lib.fileset.toSource {
            root = ./.;
            fileset = lib.fileset.unions [
              (craneLib.fileset.commonCargoSources ./.)
              ./docs
              ./tests
              ./deny.toml
            ];
          };

          commonArgs = {
            inherit src;
            strictDeps = true;
            # Tests build throwaway git worlds and run grove against them.
            nativeCheckInputs = [ pkgs.git ];
          };

          cargoArtifacts = craneLib.buildDepsOnly commonArgs;

          grove = craneLib.buildPackage (
            commonArgs
            // {
              inherit cargoArtifacts;
              doCheck = false; # tests run once, in checks.grove-test
              meta.mainProgram = "grove";
            }
          );
        in
        {
          package = grove;

          checks = {
            inherit grove;

            grove-clippy = craneLib.cargoClippy (
              commonArgs
              // {
                inherit cargoArtifacts;
                cargoClippyExtraArgs = "--all-targets -- --deny warnings";
              }
            );

            grove-fmt = craneLib.cargoFmt { inherit src; };

            # Advisories need the network, which the build sandbox does not have:
            # run `cargo deny check advisories` in `nix develop`.
            grove-deny = craneLib.cargoDeny {
              inherit src;
              cargoDenyChecks = "bans licenses sources";
            };

            grove-test = craneLib.cargoTest (commonArgs // { inherit cargoArtifacts; });
          };

          devShell = craneLib.devShell {
            checks = self.checks.${pkgs.stdenv.hostPlatform.system};
            packages = [ pkgs.cargo-deny ];
          };
        };
    in
    {
      packages = forAllSystems (pkgs: {
        default = (perSystem pkgs).package;
        grove = (perSystem pkgs).package;
      });
      checks = forAllSystems (pkgs: (perSystem pkgs).checks);
      devShells = forAllSystems (pkgs: {
        default = (perSystem pkgs).devShell;
      });
    };
}
