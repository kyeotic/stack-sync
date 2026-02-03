default:
    @just --list

run *args:
    cargo run -- {{ args }}

build:
    cargo build --release

check:
    cargo check

test:
    cargo test

lint:
    cargo clippy -- -D warnings

fmt:
    cargo fmt

fmt-check:
    cargo fmt -- --check

release *args:
    cargo release {{ args }}

install-local:
    cargo install --path .
