.PHONY: build release install run clean test check fmt clippy coverage coverage-report

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

coverage:
	cargo llvm-cov --summary-only

coverage-report:
	cargo llvm-cov --html --output-dir .coverage
	@echo "Report: .coverage/html/index.html"
