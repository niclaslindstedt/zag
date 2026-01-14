# CLAUDE.md

Keep this file updated when making architectural changes to the codebase.

## Build Commands

- `make build` - Development build
- `make release` - Release build
- `make test` - Run tests
- `make fmt` - Format code
- `make clippy` - Lint

## Architecture

Rust CLI that provides a unified interface for multiple AI coding agents (Claude, Codex, Gemini, Copilot).

### Design

- **Trait-based abstraction**: Common `Agent` trait defines the interface for all agent implementations
- **Subprocess delegation**: Each agent spawns its respective CLI tool, passing configuration via arguments or temporary files
- **Session management**: Tracks active processes and handles graceful shutdown via signal forwarding

### Key Files

| File | Purpose |
|------|---------|
| `src/agent.rs` | Agent trait definition |
| `src/session.rs` | AgentSession and run_sessions() |
| `src/main.rs` | CLI entry point with clap |
| `src/claude.rs` | Claude agent implementation |
| `src/codex.rs` | Codex agent implementation |
| `src/gemini.rs` | Gemini agent implementation |
| `src/copilot.rs` | Copilot agent implementation |
| `src/interrupt.rs` | CTRL+C signal handling |
| `src/pid.rs` | Session PID and workflow context |

## Workflow System

Multi-phase workflow orchestration for complex AI agent tasks.

### CLI Usage

```bash
# Run a workflow
agent workflow software

# List available workflows
agent workflow --list

# Resume interrupted workflow
agent workflow software --resume

# Checkpoint current iteration (for resume)
agent workflow --checkpoint

# List previous runs
agent workflow software --list-runs

# Create a new workflow with AI assistance
agent workflow --create my-workflow

# Create with a different agent
agent workflow --create my-workflow --agent codex

# Modify an existing workflow with AI assistance
agent workflow --modify my-workflow

# Modify with a different agent
agent workflow --modify my-workflow --agent codex

# Delete a user-defined workflow
agent workflow --delete my-workflow
```

### Workflow Files

| File | Purpose |
|------|---------|
| `src/workflow/mod.rs` | Module exports |
| `src/workflow/types.rs` | Data structures (Workflow, Phase, etc.) |
| `src/workflow/engine.rs` | Main orchestrator |
| `src/workflow/phase.rs` | Phase execution with recursion |
| `src/workflow/state.rs` | State directory management |
| `src/workflow/loader.rs` | Load embedded + custom workflows |
| `src/workflow/template.rs` | Variable expansion (`{{var}}`) |
| `src/workflow/variables.rs` | Custom variable resolution (env, bash, file) |
| `src/workflow/manage.rs` | Workflow management (create, modify, delete) |
| `workflows/software.json` | Embedded software workflow |
| `prompts/workflow-create-system.md` | System prompt for workflow creation |
| `prompts/workflow-modify-system.md` | System prompt for workflow modification |

### Workflow Loading

1. **User workflows**: `~/.agent/workflows/<name>.json` (takes precedence)
2. **Embedded workflows**: Compiled into binary via `include_str!`

### State Directory

```
.agent/state/<workflow>/<run_id>/
├── manifest.json      # Run status and progress tracking
├── spec.md            # Phase outputs
├── epics.json
└── epics/
    └── epic-001/
        ├── tickets.json
        └── tickets/
            └── T001/
                └── implementation.md
```

### Execution Modes

- **once**: Run phase single time
- **iterate**: Run for each item in JSON array file

### Template Variables

| Variable | Description |
|----------|-------------|
| `{{state_dir}}` | Run's state directory path |
| `{{index}}` | Current iteration index |
| `{{item.field}}` | Field from iteration item |
| `{{var.name}}` | Custom variable (see below) |

### Custom Variables

Workflows can define variables resolved at startup from environment, bash commands, or files:

```json
{
  "variables": [
    { "name": "branch", "type": "bash", "source": "git branch --show-current" },
    { "name": "api_key", "type": "env", "source": "MY_API_KEY", "required": false },
    { "name": "context", "type": "file", "source": "CLAUDE.md" }
  ]
}
```

Access in prompts as `{{var.branch}}`, `{{var.api_key}}`, `{{var.context}}`.

| Type | Description |
|------|-------------|
| `env` | Environment variable |
| `bash` | Command stdout |
| `file` | File contents |

Variables can reference each other via `{{var.X}}` - dependencies are auto-detected and resolved in correct order. Circular dependencies are reported as errors.

### Nested Phases

For epic → ticket → follow-up patterns:
- Parent phase: `"nested_phases": ["child-id"]`
- Child phase: `"parent": "parent-id"`

### Automatic Completion

For interactive phases, the workflow engine automatically injects a completion instruction telling the agent to:
1. Run `agent workflow --checkpoint` to save progress
2. Run `agent kill` to continue to the next phase

### Checkpoints and Resume

- **Checkpoint**: Marks current iteration as complete in manifest
- **Resume**: Skips iterations that have been checkpointed
- Context stored in `~/.agent/workflow.json` for auto-detection

### Signal Handling

- **CTRL+C (SIGINT)**: Interrupts current phase, marks it as failed (workflow is resumable)
- **`agent kill` (SIGTERM)**: Terminates current session, continues to next phase/iteration

### Software Workflow Phases

1. **spec** - Write technical specification
2. **epics** - Break spec into epics (features)
3. **epic-loop** - Iterate over epics
   - **create-tickets** - Create tickets for current epic
   - **ticket-loop** - Iterate over tickets
     - **implement** - Implement ticket
     - **review** - Review and create follow-ups
     - **followup-loop** - Complete follow-ups first

### Creating Custom Workflows

Run `agent workflow --create <name>` to create a new workflow with AI assistance. The AI will guide you through defining phases and write the workflow JSON to `~/.agent/workflows/<name>.json`.

### Modifying Workflows

Run `agent workflow --modify <name>` to modify an existing workflow with AI assistance. The AI will read the current workflow, ask what you want to change, and make the modifications. For embedded workflows (like `software`), a copy is created in `~/.agent/workflows/` for modification.
