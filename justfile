# Define a default recipe
default:
    @just pi3

# Build the project for the aarch64-unknown-linux-musl target in release mode
pi3:
    cross build --release --target=arm-unknown-linux-gnueabihf
