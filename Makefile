.PHONY: build release release-tag install run clean test check fmt fmt-check clippy lint shellcheck coverage coverage-report extract-website-data website website-dev website-clean

build:
	cargo build

release:
	cargo build --release

release-tag:
	scripts/release.sh $(BUMP)

install:
	cargo install --path zag-cli

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

lint:
	cargo clippy --workspace --all-targets -- -D warnings

fmt-check:
	cargo fmt --all -- --check

shellcheck:
	shellcheck scripts/*.sh

coverage:
	cargo llvm-cov --summary-only

coverage-report:
	cargo llvm-cov --html --output-dir .coverage
	@echo "Report: .coverage/html/index.html"

extract-website-data:
	cd website && npm run extract

website:
	cd website && npm install && npm run build

website-dev:
	cd website && npm install && npm run dev

website-clean:
	rm -rf website/dist
