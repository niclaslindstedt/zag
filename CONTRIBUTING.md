# Contributing to zag

Thank you for your interest in contributing to zag! This document provides guidelines and instructions for contributing.

Please read our [Code of Conduct](CODE_OF_CONDUCT.md) before participating. To report security vulnerabilities, see [SECURITY.md](SECURITY.md).

## Getting started

### Prerequisites

- Rust 1.85+ (edition 2024)
- At least one agent CLI installed: `claude`, `codex`, `gemini`, `copilot`, or `ollama`
- GNU Make

### Building

```bash
git clone https://github.com/niclaslindstedt/zag.git
cd zag
make build
```

### Running tests

```bash
make test
```

### Full check (build + test + lint + format)

```bash
make build && make test && make clippy && make fmt
```

## Development workflow

1. Fork the repository and create a feature branch
2. Make your changes
3. Add or update tests in the corresponding `*_tests.rs` file
4. Run `make build && make test && make clippy && make fmt`
5. Update documentation if your change affects user-facing behavior:
   - `README.md` for usage changes
   - `CLAUDE.md` for architecture or development pattern changes
   - `man/*.md` for command/flag changes
6. Commit with a conventional commit message
7. Open a pull request

## Project structure

```
zag/
├── src/              # Binary crate — thin CLI wrapper (argument parsing, dispatch)
├── zag-lib/src/      # Library crate — agent trait, providers, builder API, config
├── zag-orch/src/     # Orchestration crate — spawn, wait, collect, pipe, and more
├── examples/         # Example projects (cv-review, orchestration scripts, React UI)
├── bindings/         # Language SDKs (TypeScript, Python, C#)
├── man/              # Manpages (embedded in `zag man`)
└── prompts/          # Prompt templates (auto-selector, json-wrap)
```

Dependency graph: `zag-lib` ← `zag-orch` ← `zag` (binary). The binary also depends directly on `zag-lib`.

See `CLAUDE.md` for detailed architecture documentation.

## Commit messages

Follow the conventional commit style:

```
type(scope): summary
```

- **Types**: `feat`, `fix`, `refactor`, `docs`, `test`, `chore`
- **Scopes**: lowercase, comma-separated if multiple (e.g., `refactor(codex,docs): ...`)
- **Summary**: imperative mood, specific to the change

Examples:
```
feat(builder): add timeout option to AgentBuilder
fix(gemini): handle empty response in exec mode
docs(readme): update install instructions for crates.io
test(claude): add streaming session edge case tests
```

## Code style

- Run `make fmt` before committing
- Run `make clippy` and fix all warnings
- Tests live in separate `*_tests.rs` files, not inline
- Core logic belongs in `zag-lib`; the binary crate is a thin CLI wrapper
- Follow existing patterns — check `CLAUDE.md` for architecture details

## Running individual tests

```bash
# Run a specific test by name
cargo test test_name

# Run tests in a specific crate
cargo test -p zag          # zag-lib tests
cargo test -p zag-cli      # binary crate tests

# Run tests with output shown
cargo test -- --nocapture
```

## Code coverage

```bash
# Summary (requires cargo-llvm-cov)
make coverage

# HTML report
make coverage-report
# Opens .coverage/html/index.html
```

Install the coverage tool with: `cargo install cargo-llvm-cov`

## Adding a new provider

1. Create `zag-lib/src/providers/<name>.rs` implementing the `Agent` trait
2. Register the provider in `zag-lib/src/factory.rs`
3. Add model validation and size mappings
4. Add tests in `zag-lib/src/providers/<name>_tests.rs`
5. Update `CLAUDE.md`, `README.md`, and `man/zag.md`

## Language bindings

SDK bindings live under `bindings/` for TypeScript, Python, and C#. Each wraps the `zag` CLI binary and parses its JSON output.

### Testing bindings

```bash
# TypeScript
cd bindings/typescript && npm run build && npm test

# Python
cd bindings/python && pip install -e . && pytest

# C#
cd bindings/csharp && dotnet test
```

When modifying the `AgentOutput` or `Event` types in `zag-lib/src/output.rs`, update the corresponding type definitions in all three binding packages.

## Release process

Releases are managed via `scripts/release.sh`, which builds a release binary, creates a git tag, and publishes to GitHub Releases.

```bash
# Bump patch version and release
./scripts/release.sh --bump patch

# Set an explicit version
./scripts/release.sh --version 1.0.0

# Dry-run (build only, no publish)
./scripts/release.sh --dry-run
```

Requires `cargo`, `gh` (GitHub CLI), and `git`.

## Reporting issues

- Use [GitHub Issues](https://github.com/niclaslindstedt/zag/issues)
- Include your Rust version (`rustc --version`), OS, and steps to reproduce

## License

By contributing, you agree that your contributions will be licensed under the [MIT License](LICENSE).
