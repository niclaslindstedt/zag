# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.12.3] - 2026-04-13

### Fixed

- Use unique temp filenames in atomic_write to prevent race condition (#141)

## [0.12.2] - 2026-04-13

### Fixed

- Fall back to assistant text when Result.result is empty (#140)

## [0.12.1] - 2026-04-12

### Fixed

- Apply rustfmt to AVAILABLE_MODELS array (#138)
- Remove versioned model aliases that Claude CLI rejects

### Tests

- Fix health cache test race on shared global path (#122)

## [0.12.0] - 2026-04-11

### Added

- Add turn_complete streaming event with stop_reason (#121)

## [0.11.0] - 2026-04-10

### Added

- Capability-aware errors in feature-gated builder methods (#120)
- Add streaming_input mid-turn semantics (#118)
- Add StreamingSession.close() and opt-in autoCleanup (#115)
- Downgrade provider through tier list on failure (#105)

### Fixed

- Include stderr in streaming error messages (#116)
- Surface per-turn result events in bidirectional streaming (#114)

### Documentation

- Document execStreaming defaults and per-provider flag support (#117)

## [0.10.0] - 2026-04-10

- No notable changes

## [0.9.0] - 2026-04-09

### Added

- Add --resume and --continue to exec command (#103)

## [0.8.3] - 2026-04-09

- No notable changes

## [0.8.2] - 2026-04-08

### Fixed

- Place subcommand before agent args in CLI invocation (#101)

## [0.8.1] - 2026-04-08

### Fixed

- Fix package exports pointing to wrong path (#100)

## [0.8.0] - 2026-04-08

### Added

- Add file attachment support for embedding files in agent prompts (#99)
- Add expandable tool details and sub-agent nesting to ZagChat (#95)
- Add meta-bootstrap orchestration example (#96)
- Add remote health check with auto-disconnect (#93)
- Add ZagChat example app and streaming session control (#94)
- Add realistic terminal output to hero demos (#89)
- Add exit error messages and structured exit codes (#88)
- Add discover command for provider/model discovery (#87)
- Add plan command and --plan flag for run/exec/spawn (#84)
- Force sandbox for connected users (#82)

### Fixed

- Extract source data from latest version tag (#92)
- Make mobile sidebar full-width on small screens (#86)
- Fix copy button scroll and burger menu close on mobile (#85)

### Documentation

- Add package registry badges to README (#98)
- Add comprehensive reference documentation for all language bindings (#97)

## [0.7.0] - 2026-04-06

### Added

- Add user account support with per-user isolation (#81)
- Extract code-sourced data from Rust source for website (#80)
- Add CLI version tracking and compatibility checking (#79)
- Add prompt-based review support for all providers (#78)
- Add --timeout flag to exec and spawn commands (#76)

### Fixed

- Improve mobile responsiveness across landing page (#77)
- Add id attributes to headings for in-page anchor links (#75)
- Improve mobile support and UI fixes (#74)

## [0.6.0] - 2026-04-05

### Added

- Add Manual page compiling CLI man pages (#73)
- Add copy-to-clipboard buttons on docs code blocks (#72)
- Add documentation pages with markdown rendering (#70)
- Link crates.io release badge to crates.io page (#69)
- Add copy button to install command in hero section (#67)
- Add animated terminal component with tabbed demos (#65)
- Add --env flag for passing environment variables to agent subprocesses (#63)
- Add --mcp-config support for per-invocation MCP injection (#62)
- Log spawned commands at info level and always print token

### Fixed

- Install rustls crypto provider to prevent startup panic (#60)
- Integrate sync-releases.sh into release workflow (#59)

### Documentation

- Add comprehensive documentation for orchestration, bindings, and features (#68)
- Sync --env/--mcp-config flags and add missing bindings (#64)
- Document squash-merge PR workflow and add commit skill (#61)

## [0.5.0] - 2026-04-05

### Added

- Add update-manpages skill for keeping man/ in sync (#57)
- Resolve host and port from config file (#56)
- Add update-readme and update-website skills (#53)

### Fixed

- Correct provider models and binding method names (#55)

### Documentation

- Update baseline hash after audit (#58)
- Add missing session, skills, and mcp subcommands (#54)

## [0.4.0] - 2026-04-05

### Added

- Support all non-TTY commands in remote mode (#52)
- Update agent implementations (#51)
- Add update-bindings skill for syncing language bindings (#50)
- Add sync-releases.sh to update GitHub releases from changelog (#49)

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
