# Define a default recipe
default:
    @just pi3

# Build the project for the arm-unknown-linux-gnueabihf target in release mode
pi3:
    cargo build --release --target=arm-unknown-linux-gnueabihf

# Run clippy on the codebase
clippy:
    cargo clippy --all-targets -- -D warnings
