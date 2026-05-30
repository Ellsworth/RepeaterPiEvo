default:
    @just pi3

# Pi 3 builds, targets 32-bit armv7hf
pi3:
    cargo build --release --target=arm-unknown-linux-gnueabihf

check:
    cargo clippy --all-targets -- -D warnings
    cargo fmt --all -- --check

fmt:
    cargo fmt --all
