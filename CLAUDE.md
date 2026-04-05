# CLAUDE.md

## Build Commands

- `make build` — Development build
- `make release` — Release build
- `make test` — Run tests
- `make fmt` — Format code
- `make clippy` — Lint (zero warnings required)
- `make website` — Build the website locally (output to `website/dist/`, gitignored)
- `make website-dev` — Start website dev server

## Commit Messages

- Conventional commit style: `type(scope): summary`
- Types: `feat`, `fix`, `refactor`, `docs`, `test` (lowercase)
- Scopes: lowercase, comma-separated if multiple (e.g. `refactor(codex,docs): update lineup`)
- Imperative mood, specific to the change

## Architecture

Cargo workspace with four crates. Dependency graph: `zag-agent ← zag-orch ← zag (published crate) / zag-cli (binary)`.

- **`zag-cli`** (binary) — Thin CLI wrapper: clap arg parsing (`zag-cli/src/cli.rs`), terminal logging (`zag-cli/src/logging.rs`), command handlers (`zag-cli/src/commands/`), dispatch to lib/orch. Commands with subcommands use folder layout (`commands/session/`, `commands/skills/`, `commands/mcp/`) with each subcommand in its own `.rs` file; standalone commands remain as flat files.
- **`zag-agent`** (library) — Agent consolidation: `Agent` trait (`src/agent.rs`), provider implementations (`src/providers/`), `AgentFactory` (`src/factory.rs`), `AgentBuilder` (`src/builder.rs`), config, output types, session logs. Updated when upstream agent CLIs change.
- **`zag-orch`** (library) — Orchestration: spawn, wait, collect, pipe, status, events, cancel, summary, watch, subscribe, retry, gc. Our own multi-session coordination code.
- **`zag`** (published crate, `bindings/rust/`) — Facade that re-exports `zag-agent` (flat) and `zag-orch` (as `zag::orch`). This is the crate users depend on.

Key design: trait-based `Agent` abstraction, factory pattern, builder API, subprocess delegation to upstream CLIs. `ProgressHandler` trait decouples library from terminal UI. Bindings in `bindings/` (Rust, TypeScript, Python, C#, Swift, Java, Kotlin) — Rust re-exports workspace crates directly; others mirror `AgentBuilder` via CLI subprocess.

## Where New Code Goes

1. **Agent/provider logic** → `zag-agent` (trait changes, provider impls, builder options, config, session logs)
2. **Orchestration** → `zag-orch` (multi-session coordination primitives)
3. **CLI flags/dispatch** → `zag-cli/src/cli.rs` + `zag-cli/src/main.rs`
4. **New builder option** → `zag-agent/src/builder.rs`, wire in `create_agent()` or terminal methods
5. **New CLI flag** → `AgentArgs` in `zag-cli/src/cli.rs`, wire in `zag-cli/src/commands/agent_action.rs`
6. **New CLI command handler** → `zag-cli/src/commands/`, declare in `zag-cli/src/commands/mod.rs`. If the command has subcommands, create a folder (`commands/<cmd>/`) with `mod.rs` for dispatch and one `.rs` file per subcommand.
6a. **New subcommand** → `zag-cli/src/commands/<parent>/`, add a new `.rs` file with a `pub(crate) fn run(...)`, register in the parent's `mod.rs` dispatch match
7. **Agent-specific feature** → `Agent` trait or downcast via `as_any_mut()`
8. **New provider** → `zag-agent/src/providers/`, register in `zag-agent/src/factory.rs`
9. **New orch command** → `zag-orch/src/`, declare in `zag-orch/src/lib.rs`, dispatch from `zag-cli/src/main.rs`
10. **Website** → `website/src/` (React components, styles, content)

## Development Process

1. Implement the change
2. Write tests in separate `*_tests.rs` files (not inline `#[cfg(test)]` blocks)
3. `make build` — must compile cleanly
4. `make test` — all tests must pass
5. `make clippy` — zero warnings
6. `make fmt`
7. Update `README.md` if user-facing behavior changed
8. Update `CLAUDE.md` if architecture changed
9. Update `man/*.md` if commands/flags/behavior changed
10. **Do not manually edit `CHANGELOG.md`** — it is auto-generated from conventional commits at release time by CI
11. Commit with `/commit`

## Parity Checklist

When adding a new `AgentBuilder` setter, keep all layers in sync:

1. Add setter to `zag-agent/src/builder.rs`, wire into `create_agent()` or terminal methods
2. If CLI flag: add to `AgentArgs` in `zag-cli/src/cli.rs`, wire in `zag-cli/src/commands/agent_action.rs`
3. Add corresponding method to all six bindings:
   - `bindings/typescript/src/builder.ts` — field, method, `buildGlobalArgs()` or `buildExecArgs()`
   - `bindings/python/src/zag/builder.py` — field, method, `_global_args()` or `_exec_args()`
   - `bindings/csharp/src/Zag/ZagBuilder.cs` — field, method, `BuildGlobalArgs()` or `BuildExecArgs()`
   - `bindings/swift/Sources/Zag/ZagBuilder.swift` — field, method, `buildGlobalArgs()` or `buildExecArgs()`
   - `bindings/java/src/main/java/io/zag/ZagBuilder.java` — field, method, `buildGlobalArgs()` or `buildExecArgs()`
   - `bindings/kotlin/src/main/kotlin/zag/ZagBuilder.kt` — field, method, `buildGlobalArgs()` or `buildExecArgs()`
4. Add tests in all six binding test suites
5. Update builder methods table in all six binding READMEs

### Test file conventions

- Rust: separate `*_tests.rs` files with `use super::*;`, referenced via `#[cfg(test)] #[path = "..._tests.rs"] mod tests;`
- TypeScript: `bindings/typescript/tests/builder.test.ts`
- Python: `bindings/python/tests/test_builder.py`
- C#: `bindings/csharp/tests/Zag.Tests/ZagBuilderTests.cs`
- Swift: `bindings/swift/Tests/ZagTests/ZagBuilderTests.swift`
- Java: `bindings/java/src/test/java/io/zag/ZagBuilderTests.java`
- Kotlin: `bindings/kotlin/src/test/kotlin/zag/ZagBuilderTests.kt`

### Documentation sync points

| Change type | Files to update |
|-------------|----------------|
| New CLI flag | `zag-cli/src/cli.rs`, `README.md`, relevant `man/*.md` |
| New builder option | `zag-agent/src/builder.rs`, all 6 bindings + tests + READMEs |
| New command | `zag-cli/src/cli.rs`, `zag-cli/src/commands/`, `zag-cli/src/main.rs`, `man/<cmd>.md`, `README.md` |
| New subcommand | `zag-cli/src/cli.rs` (enum variant), `zag-cli/src/commands/<parent>/<sub>.rs`, `zag-cli/src/commands/<parent>/mod.rs` (dispatch) |
| New provider | `zag-agent/src/providers/`, `zag-agent/src/factory.rs`, `README.md`, `docs/providers.md` |
| New orch command | `zag-orch/src/`, `zag-orch/src/lib.rs`, `zag-cli/src/cli.rs`, `zag-cli/src/main.rs`, `man/`, `README.md` |
| Provider feature change | `docs/providers.md` |
| Config key change | `docs/configuration.md`, `man/config.md` |
| Event format change | `docs/events-and-logging.md` |
| Website content change | `website/src/`, deployed automatically via GitHub Actions on push to `main` |
| README staleness | Run `update-readme` skill — tracks last update via `.claude/skills/update-readme/.last-updated` |
| Website staleness | Run `update-website` skill — tracks last update via `.claude/skills/update-website/.last-updated` |
| Manpage staleness | Run `update-manpages` skill — tracks last update via `.claude/skills/update-manpages/.last-updated` |
| Release changelog | Auto-generated by CI — do NOT edit `CHANGELOG.md` manually |

## Website

The project landing page lives in `website/` (React + Vite + Tailwind CSS v4). Built output goes to `docs/` for GitHub Pages serving.

- **Source**: `website/src/` — React components and styles
- **Output**: `website/dist/` — gitignored, built in CI
- **Build**: `make website` (or `cd website && npm run build`)
- **Dev server**: `make website-dev` (serves at `http://localhost:5173/zag/`)

### Publishing

The website is built and deployed automatically by `.github/workflows/static.yml` on every push to `main`. The workflow installs dependencies, runs `npm run build` in `website/`, then uploads `docs/` to GitHub Pages.

GitHub Pages must be configured in the repository settings to deploy via GitHub Actions (not from the `docs/` folder).
