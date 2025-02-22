{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  buildInputs = [
    pkgs.rustup
    pkgs.rustc
    pkgs.gcc-arm-embedded
    pkgs.cargo-cross
    pkgs.just
  ];
}
