run:
    cargo run

# Builds for release and installs to ~/.local/bin
install:
    cargo build --release
    cp target/release/tile_picker ~/.local/bin

fmt:
    cargo fmt

fix:
    cargo clippy --fix

ok:
    just check
    cargo fmt --check
    cargo clippy -- -D warnings
    just test

test:
    cargo test

setup:
    cargo install

doc:
    cargo doc --open

check:
    cargo check
