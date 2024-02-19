{
  description = "A small utility library for parsing GGUF file info";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [
          rust-overlay.overlays.default
          (final: prev: with prev.rust-bin; {
            rustToolchain = fromRustupToolchainFile ./rust-toolchain.toml;
          })
          (final: prev: {
            cargo-deny = prev.cargo-deny.overrideAttrs (oldAttrs: {
              buildInputs = oldAttrs.buildInputs ++ (prev.lib.optionals prev.stdenv.isDarwin [ prev.darwin.apple_sdk.frameworks.SystemConfiguration ]);
            });
          })
        ];
        pkgs = import nixpkgs { inherit overlays system; };
        rust_packages = with pkgs; [
          rustToolchain
          openssl
          pkg-config
          cargo-deny
          cargo-edit
          cargo-watch
          rust-analyzer
        ];
        gguf-info = pkgs.rustPlatform.buildRustPackage rec {
          pname = "gguf-info";
          version = "0.1.2";
          src = ./.;
          cargoLock = {
            lockFile = ./Cargo.lock;
          };
          cargoBuildFlags = [ "--features" "bin" "--bin" pname ];
          binName = pname;
        };
      in
      {
        packages.default = gguf-info;
        devShells = with pkgs; {
          default = mkShell
            ({
              packages = rust_packages ++ [ gguf-info ];
              shellHook = ''
                export CARGO_HOME="$PWD/.cargo "
                export PATH="$CARGO_HOME/bin:$PATH" 
              '';
            });
        };
      });
}
