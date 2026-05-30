{
  description = "RepeaterPi Evo Development Environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        # Customized Rust toolchain with cross-compilation target added
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          targets = [ "arm-unknown-linux-gnueabihf" ];
          extensions = [ "clippy" "rustfmt" ];
        };

        # ARMv7 hard float cross GCC from Nixpkgs
        crossCC = pkgs.pkgsCross.armv7l-hf-multiplatform.stdenv.cc;
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = [
            rustToolchain
            pkgs.just
            pkgs.gcc-arm-embedded
            crossCC
          ];

          # Environment variables for Cargo to use the correct linker
          CARGO_TARGET_ARM_UNKNOWN_LINUX_GNUEABIHF_LINKER = "armv7l-unknown-linux-gnueabihf-cc";
          CC_arm_unknown_linux_gnueabihf = "armv7l-unknown-linux-gnueabihf-cc";
        };
      }
    );
}
