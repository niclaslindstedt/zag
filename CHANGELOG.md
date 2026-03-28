# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-03-28

### Added

- Unified CLI for Claude, Codex, Gemini, Copilot, and Ollama agents
- `AgentBuilder` programmatic Rust API (`zag` library crate)
- `StreamingSession` for bidirectional communication with Claude
- Model size aliases (`small`, `medium`, `large`) across all providers
- Automatic provider/model selection (`-p auto -m auto`)
- Structured JSON output with JSON Schema validation and retry
- Git worktree isolation (`--worktree`)
- Docker sandbox isolation (`--sandbox`)
- Unified session logging in NDJSON format
- Session search with full-text and metadata filters
- Provider-agnostic skills management (Agent Skills standard)
- MCP server management across providers
- Process tracking and management (`zag ps`)
- Code review via Codex (`zag review`)
- Language bindings for TypeScript, Python, and C#
- Per-project configuration (`~/.zag/projects/`)
- Built-in manual pages (`zag man`)
