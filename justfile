run := "nix develop --command"

default:
    @just --list

# Pi 3 builds, statically linked 32-bit armv7 musl
pi3:
    @just check
    {{run}} cargo zigbuild --release --target=armv7-unknown-linux-musleabihf

# Host build for local development
build:
    @just check
    {{run}} cargo build --release

check:
    {{run}} cargo clippy --all-targets -- -D warnings
    {{run}} cargo fmt --all -- --check

fmt:
    {{run}} cargo fmt --all

# Interactive Nix development shell
shell:
    nix develop

