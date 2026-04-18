---
description: "Use when manpages may be stale. Discovers commits since the last manpage update, identifies what changed (commands, flags, providers, orchestration, etc.), and updates the affected zag-agent/man/*.md files to match the current CLI implementation."
---

# Updating the Manpages

The `zag-agent/man/` directory contains 35 markdown manpages embedded at compile time via `include_str!()` in `zag-cli/src/commands/manpage.rs` and accessed via `zag man <command>`. They are the authoritative command-level reference documentation. They get stale when CLI flags, commands, behaviors, or providers change without updating the corresponding manpage.

## Tracking Mechanism

The file `.claude/skills/update-manpages/.last-updated` contains the git commit hash from the last time the manpages were comprehensively updated. Use this as the baseline for discovering what changed.

## Discovery Process

1. Read the baseline commit hash:
   ```sh
   BASELINE=$(cat .claude/skills/update-manpages/.last-updated)
   ```

2. List all commits since the baseline, filtering for relevant types:
   ```sh
   git log --oneline "$BASELINE"..HEAD
   ```

3. For each relevant commit, check what files changed:
   ```sh
   git diff --name-only "$BASELINE"..HEAD
   ```

4. Categorize the changes using the manpage mapping below to determine which manpages need updating.

5. Read the affected manpages to understand their current content before making changes.

6. For each affected manpage, compare the current source code against what the manpage documents. Fix any discrepancies.

## Manpage Mapping

Use this table to map changed files/scopes to affected manpages:

| Changed files / commit scope | Manpage(s) to update |
|------------------------------|---------------------|
| `zag-cli/src/cli.rs` (AgentArgs) | `zag.md` (Global Flags), `run.md`, `exec.md`, `spawn.md`, `pipe.md`, `review.md` |
| `zag-cli/src/cli.rs` (SessionIsolationArgs) | `zag.md`, `run.md`, `exec.md` |
| `zag-cli/src/cli.rs` (SessionMetadataArgs) | `run.md`, `exec.md`, `spawn.md` |
| `zag-cli/src/cli.rs` (Commands::Run) | `run.md` |
| `zag-cli/src/cli.rs` (Commands::Exec) | `exec.md` |
| `zag-cli/src/cli.rs` (Commands::Review) | `review.md` |
| `zag-cli/src/cli.rs` (Commands::Config) | `config.md` |
| `zag-cli/src/cli.rs` (Commands::Session) | `session.md` |
| `zag-cli/src/cli.rs` (Commands::Listen) | `listen.md` |
| `zag-cli/src/cli.rs` (Commands::Spawn) | `spawn.md` |
| `zag-cli/src/cli.rs` (Commands::Wait) | `wait.md` |
| `zag-cli/src/cli.rs` (Commands::Pipe) | `pipe.md` |
| `zag-cli/src/cli.rs` (Commands::Events) | `events.md` |
| `zag-cli/src/cli.rs` (Commands::Cancel) | `cancel.md` |
| `zag-cli/src/cli.rs` (Commands::Summary) | `summary.md` |
| `zag-cli/src/cli.rs` (Commands::Watch) | `watch.md` |
| `zag-cli/src/cli.rs` (Commands::Subscribe) | `subscribe.md` |
| `zag-cli/src/cli.rs` (Commands::Broadcast) | `broadcast.md` |
| `zag-cli/src/cli.rs` (Commands::Input) | `input.md` |
| `zag-cli/src/cli.rs` (Commands::Log) | `log.md` |
| `zag-cli/src/cli.rs` (Commands::Output) | `output.md` |
| `zag-cli/src/cli.rs` (Commands::Retry) | `retry.md` |
| `zag-cli/src/cli.rs` (Commands::Gc) | `gc.md` |
| `zag-cli/src/cli.rs` (Commands::Serve) | `serve.md` |
| `zag-cli/src/cli.rs` (Commands::Connect) | `connect.md` |
| `zag-cli/src/cli.rs` (Commands::Search) | `search.md` |
| `zag-cli/src/cli.rs` (Commands::Whoami) | `whoami.md` |
| `zag-cli/src/cli.rs` (Commands::Status) | `status.md` |
| `zag-cli/src/cli.rs` (Commands::Collect) | `collect.md` |
| `zag-cli/src/cli.rs` (Commands::Env) | `env.md` |
| `zag-cli/src/cli.rs` (Commands::Capability) | `capability.md` |
| `zag-cli/src/cli.rs` (Commands::Skills / SkillsCommand) | `skills.md` |
| `zag-cli/src/cli.rs` (Commands::Mcp / McpCommand) | `mcp.md` |
| `zag-cli/src/cli.rs` (Commands::Ps) | `ps.md` |
| `zag-cli/src/cli.rs` (new Command variant) | New `zag-agent/man/<cmd>.md` + `zag.md` (Commands list, See Also) + `manpage.rs` (const, match arm, error list) |
| `zag-cli/src/cli.rs` (new subcommand enum) | Parent command's manpage (add subcommand section) |
| `zag-cli/src/commands/` (behavior changes) | Corresponding command manpage(s) |
| `zag-agent/src/providers/*/mod.rs` | `zag.md` (Providers, Model Size Aliases sections) |
| `zag-agent/src/providers/*/models.rs` | `zag.md` (model lists in Providers section) |
| `zag-agent/src/builder.rs` | Manpages referencing builder-mapped flags |
| `zag-orch/src/` (new primitives) | `orchestration.md` + affected command manpages |
| `zag-orch/src/` (behavior changes) | `orchestration.md` + affected command manpages |
| Session/config changes | `session.md`, `config.md` |

### Shared argument groups

These structs in `cli.rs` are flattened into multiple commands. When they change, all commands that use them need updating:

- **AgentArgs** (provider, model, root, auto_approve, system_prompt, add_dirs, size, show_usage, max_turns) — used by: Run, Exec, Review, Spawn, Pipe
- **SessionIsolationArgs** (worktree, sandbox, session, json, json_schema) — used by: Run, Exec
- **SessionMetadataArgs** (name, description, tags) — used by: Run, Exec, Spawn

## Implementation Files

### Primary

- `zag-agent/man/*.md` — the 35 manpage files being updated
- `zag-cli/src/commands/manpage.rs` — must be updated when adding new manpages (const, match arm, error message)

### Secondary (read-only, for reference)

| Source of truth | What it tells you |
|----------------|-------------------|
| `zag-cli/src/cli.rs` | All CLI flags, commands, subcommands (clap definitions) |
| `zag-cli/src/commands/mod.rs` | Registered command list |
| `zag-cli/src/commands/*.rs` | Command implementations and behavior |
| `zag-agent/src/builder.rs` | Builder options that map to CLI flags |
| `zag-agent/src/providers/*/mod.rs` | Provider models, defaults, size aliases |
| `zag-orch/src/lib.rs` | Orchestration primitives |
| `README.md` | High-level documentation (should be consistent with manpages) |

## Manpage Format Conventions

All manpages follow these conventions — maintain them when editing:

- **H1 title**: `# zag <command>` (matches what `zag man <command>` prints)
- **One-line description**: Immediately after H1, no blank line between them
- **Standard sections in order**: Synopsis, Description, Arguments (if any), Flags, [additional sections], Examples, See Also
- **Synopsis**: 4-space indented code blocks (not fenced), e.g.:
  ```
      zag [flags] exec [options] <prompt>
  ```
- **Flag entries**: 4-space indented with aligned descriptions:
  ```
      -p, --provider <PROVIDER>     AI provider to use
      -m, --model <MODEL>           Model name or size alias
  ```
- **Examples**: 4-space indented with inline comments or preceding description:
  ```
      zag exec "say hello"                    Simple prompt
      zag exec -o json "list files"           Full session as JSON
  ```
- **See Also**: 4-space indented `zag man <cmd>` entries with descriptions
- **Global flags note**: Command manpages include "All global flags apply (see `zag man zag`)." rather than repeating all global flags
- **Subcommand docs**: Commands with subcommands (session, skills, mcp, config) document all subcommands within a single manpage file

## Implementation Patterns

### Adding a new global flag

When a new field is added to `AgentArgs`, `SessionIsolationArgs`, or `SessionMetadataArgs`:

1. Update `zag.md` Global Flags section — add the flag with short form, long form, value placeholder, and description
2. Update each command manpage that flattens those args (see shared argument groups above)
3. Add the flag to the Examples section of affected pages if it warrants demonstration

### Adding a command-specific flag

When a new field is added to a specific `Commands::*` variant:

1. Update that command's manpage Flags section
2. Add examples demonstrating the flag
3. If the flag changes behavior significantly, update the Description section

### Adding a new command

When a new variant is added to the `Commands` enum:

1. Create `zag-agent/man/<cmd>.md` following the standard structure:
   ```markdown
   # zag <cmd>

   One-line description.

   ## Synopsis

       zag [flags] <cmd> [options]

   ## Description

   Detailed explanation.

   ## Flags

       -f, --flag <VALUE>      Description

   All global flags apply (see `zag man zag`).

   ## Examples

       zag <cmd> "example"     Description of example

   ## See Also

       zag man zag       Global flags and options
   ```
2. Update `zag.md` Commands list (keep alphabetical within category groups)
3. Update `zag-cli/src/commands/manpage.rs`:
   - Add `const MAN_<CMD>: &str = include_str!("../../man/<cmd>.md");` (path is relative to `manpage.rs`)
   - Add match arm: `Some("<cmd>") => MAN_<CMD>,`
   - Update the error message "Available:" list
4. Update `help-agent.md` if the command is commonly used

### Adding a new subcommand

When a new variant is added to a subcommand enum (e.g., `SessionCommand`, `SkillsCommand`, `McpCommand`):

1. Update the parent command's manpage with a new subsection documenting the subcommand
2. Add synopsis, flags, and examples for the new subcommand
3. Follow the existing subcommand documentation style in that manpage

### Updating provider models

When `default_model()`, `model_for_size()`, or model lists change in a provider:

1. Update `zag.md` Providers section with new model names
2. Update `zag.md` Model Size Aliases section if size mappings changed
3. Update any command manpage examples that reference specific model names

### Updating orchestration patterns

When new primitives or patterns are added to `zag-orch`:

1. Update `orchestration.md` Key Primitives list
2. Add new pattern sections with shell code examples
3. Update affected command manpages if their behavior changed

## Update Checklist

- [ ] Read baseline from `.last-updated` and run `git log` to identify changes
- [ ] Read `zag-cli/src/cli.rs` to get current clap definitions
- [ ] Read all affected manpages and source-of-truth files
- [ ] Update `zag.md` if global flags, commands, providers, or model aliases changed
- [ ] Update command-specific manpages for changed flags or behavior
- [ ] Create new `zag-agent/man/<cmd>.md` for any new commands
- [ ] Update parent command manpages for any new subcommands
- [ ] Update `orchestration.md` if orchestration primitives changed
- [ ] Update `help-agent.md` if commonly-used commands changed
- [ ] Update `manpage.rs` if new manpages were added (const, match arm, error message)
- [ ] Verify flag names, short forms, and value placeholders match `cli.rs` exactly
- [ ] Verify all examples use correct current syntax
- [ ] Verify See Also references are complete and bidirectional
- [ ] Ensure `zag.md` Commands list is complete (matches all non-hidden `Commands` variants)
- [ ] Consider whether `update-readme` skill should also be run
- [ ] Update `.claude/skills/update-manpages/.last-updated` with current HEAD commit hash:
  ```sh
  git rev-parse HEAD > .claude/skills/update-manpages/.last-updated
  ```

## Verification

1. Build and run tests:
   ```sh
   make build
   cargo test -p zag-cli
   ```
   The `manpage_tests.rs` tests verify manpage content has proper headers and that all commands have registered manpages.
2. For each updated manpage, verify flag names and descriptions match `cli.rs` clap definitions
3. Verify new commands are registered in `manpage.rs` (const, match arm, error message available list)
4. Ensure no sections were accidentally deleted or corrupted
5. Check that `zag.md` Commands list matches the non-hidden variants in the `Commands` enum
6. Confirm `.last-updated` file was updated

## Skill Self-Improvement

After completing an update session, improve this skill file:

1. **Update the mapping table**: If new source-of-truth files or manpage sections were discovered, add them.
2. **Add new patterns**: If you found a recurring update pattern not documented here, add it to Implementation Patterns.
3. **Note format conventions**: If inconsistencies were found and normalized, document the convention.
4. **Update shared argument groups**: If commands that flatten shared args changed, update the list.
5. **Commit the skill update** along with the manpage updates so improvements are preserved.
