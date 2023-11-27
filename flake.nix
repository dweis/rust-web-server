{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    rust-overlay.url = "github:oxalica/rust-overlay";
    systems.url = "github:nix-systems/default";
    # Dev tools
    treefmt-nix.url = "github:numtide/treefmt-nix";
  };

  outputs = { self, flake-parts, rust-overlay, ... }@ inputs:
    inputs.flake-parts.lib.mkFlake { inherit inputs; } {
      systems = import inputs.systems;
      imports = [
        inputs.treefmt-nix.flakeModule
      ];
      perSystem = { config, self', pkgs, lib, system, rust-overlay, ... }:
        let
          cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
          rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ./toolchain.toml;
          buildInputs = [
            pkgs.libiconv
          ];
          nativeBuildInputs = with pkgs; [
            rustToolchain
          ] ++ (
            lib.optionals stdenv.isDarwin [
              pkgs.darwin.apple_sdk.frameworks.Security
              pkgs.darwin.apple_sdk.frameworks.CoreServices
              pkgs.darwin.apple_sdk.frameworks.CoreFoundation
              pkgs.darwin.apple_sdk.frameworks.Foundation
              pkgs.darwin.apple_sdk.frameworks.AppKit
              pkgs.darwin.apple_sdk.frameworks.WebKit
              pkgs.darwin.apple_sdk.frameworks.Cocoa
            ]
          );
        in
        {
          # Apply Rust overlay
          _module.args.pkgs = import self.inputs.nixpkgs {
            inherit system;
            overlays = [ (import self.inputs.rust-overlay) ];
          };

          # Rust package
          packages.default = pkgs.rustPlatform.buildRustPackage {
            inherit (cargoToml.package) name version;
            src = ./.;
            cargoLock.lockFile = ./Cargo.lock;
            inherit buildInputs nativeBuildInputs;
          };

          # Docker image
          packages.docker = pkgs.dockerTools.buildLayeredImage {
            name = cargoToml.package.name;
            tag = cargoToml.package.version;
            config = {
              Cmd = [ "${self'.packages.default}/bin/${cargoToml.package.name}" ];
            };
          };

          # Rust dev environment
          devShells.default = pkgs.mkShell {
            inputsFrom = [
              config.treefmt.build.devShell
            ];
            shellHook = ''
              echo
              echo "🦀 Run 'just <recipe>' to get started 🦀"
              just
            '';

            # Enable backtrace
            RUST_BACKTRACE = 1;
            # For rust-analyzer 'hover' tooltips to work.
            RUST_SRC_PATH = rustToolchain + /lib/rustlib/src/rust/library;

            inherit buildInputs;
            nativeBuildInputs = nativeBuildInputs ++ (with pkgs; [
              just
              cargo-watch
              rust-analyzer
              (python3.withPackages (ps: with ps; [notebook]))
              evcxr
            ]);
          };

          # Add your auto-formatters here.
          # cf. https://numtide.github.io/treefmt/
          treefmt.config = {
            projectRootFile = "flake.nix";
            programs = {
              nixpkgs-fmt.enable = true;
              rustfmt.enable = true;
            };
          };
        };
    };
}
