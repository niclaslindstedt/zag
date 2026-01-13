.PHONY: build release install run clean test check fmt clippy

build:
	cargo build

release:
	cargo build --release

install:
	cargo install --path .

run:
	cargo run

clean:
	cargo clean

test:
	cargo test

check:
	cargo check

fmt:
	cargo fmt

clippy:
	cargo clippy
