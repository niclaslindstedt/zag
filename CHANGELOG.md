# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.0] - 2026-04-04

### Added

- Support "self" as process ID alias for agent self-termination (#48)
- Add --interactive flag for FIFO-based long-lived sessions (#47)
- Enforce HTTPS with auto-generated TLS certs and tokens (#46)
- Add iOS support via remote HTTP/WebSocket bindings (#45)
- Add Kotlin bindings for zag (#44)
- Add Java bindings for zag (#43)
- Add Swift bindings for zag (#41)

### Fixed

- Fix changelog generation and update released versions (#40)

## [0.2.4] - 2026-04-04

### Fixed

- Prevent panic on multi-byte UTF-8 truncation in debug logs (#39)

## [0.2.3] - 2026-04-04

Initial public release.

### Added

- Multi-agent orchestration: spawn, wait, collect, pipe, status, events, cancel, summary, watch, subscribe, retry, gc
- Provider support for Claude, Copilot, Codex, Gemini, and Ollama
- Language bindings for Rust, TypeScript, Python, and C#
- Builder pattern API for agent configuration
- CLI with session management, skills, and MCP support
- Cross-platform release builds (Linux, macOS, Windows; x86_64 and aarch64)
- Publishing to crates.io, npm, NuGet, and PyPI
- GitHub Pages landing page
