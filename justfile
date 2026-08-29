default:
    @just pi3

# Pi 3 builds, statically linked 32-bit armv7 musl
pi3:
    cargo zigbuild --release --target=armv7-unknown-linux-musleabihf

check:
    cargo clippy --all-targets -- -D warnings
    cargo fmt --all -- --check

fmt:
    cargo fmt --all
