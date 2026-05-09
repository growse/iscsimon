tasks:
    just --list

build:
    cargo build --release

test:
    cargo test

check:
    cargo check
    cargo clippy -- -D warnings

deploy: build
    scp target/release/iscsimon storebot:
