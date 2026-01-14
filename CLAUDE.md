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

# Run with a specific agent (overrides workflow settings)
agent workflow software --agent codex

# List available workflows
agent workflow --list

# Resume interrupted workflow
agent workflow software --resume

# Resume with a specific agent
agent workflow software --resume --agent gemini

# Checkpoint current iteration (for resume)
agent workflow --checkpoint

# List previous runs
agent workflow software --list-runs

# Create a new workflow with AI assistance
agent workflow --create my-workflow

# Create with a different agent
agent workflow --create my-workflow --agent codex

# Create autonomously (skip confirmations)
agent workflow --create my-workflow -a

# Modify an existing workflow with AI assistance
agent workflow --modify my-workflow

# Modify with a different agent
agent workflow --modify my-workflow --agent codex

# Modify autonomously (skip confirmations)
agent workflow --modify my-workflow --auto-approve

# Delete a user-defined workflow
agent workflow --delete my-workflow

# Validate a workflow file
agent workflow --validate ~/.agent/workflows/my-workflow.json
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
| `src/workflow/variables.rs` | Custom variable resolution (env, bash, file, json) |
| `src/workflow/definitions.rs` | Definition formatting for system prompts |
| `src/workflow/manage.rs` | Workflow management (create, modify, delete) |
| `src/workflow/validate.rs` | Workflow validation |
| `src/workflow/memory.rs` | Memory system for persistent learnings |
| `workflows/software.json` | Embedded software workflow |
| `prompts/workflow-reference.md` | System prompt for workflow creation/modification |

### Workflow Loading

1. **User workflows**: `~/.agent/workflows/<name>.json` (takes precedence)
2. **Embedded workflows**: Compiled into binary via `include_str!`

### Agent Selection Priority

When determining which agent to use for a phase:

1. **CLI override** (`--agent`): Takes highest precedence
2. **Phase setting**: Agent specified in the phase definition
3. **Workflow default**: Default agent from workflow's `defaults.agent`

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

Workflows can define variables resolved at startup from environment, bash commands, files, or JSON files:

```json
{
  "variables": [
    { "name": "branch", "type": "bash", "source": "git branch --show-current" },
    { "name": "api_key", "type": "env", "source": "MY_API_KEY", "required": false },
    { "name": "context", "type": "file", "source": "CLAUDE.md" },
    { "name": "version", "type": "json", "source": "package.json", "path": ".version" },
    { "name": "first_dep", "type": "json", "source": "config.json", "path": ".dependencies[0].name" }
  ]
}
```

Access in prompts as `{{var.branch}}`, `{{var.api_key}}`, `{{var.context}}`, `{{var.version}}`.

| Type | Description |
|------|-------------|
| `env` | Environment variable |
| `bash` | Command stdout |
| `file` | File contents |
| `json` | JSON file value at path (dot-notation: `.field`, `.nested.field`, `.array[0]`) |

Variables can reference each other via `{{var.X}}` - dependencies are auto-detected and resolved in correct order. Circular dependencies are reported as errors.

### Definitions

Workflows can define terms and concepts that are injected into the system prompt for all phases. Supports both flat key-value pairs and nested sections:

```json
{
  "definitions": {
    "project": "The current software project",
    "terms": {
      "epic": "A large feature or capability",
      "ticket": "A small, implementable unit of work"
    },
    "guidelines": {
      "code_style": "Use snake_case for variables",
      "testing": "Write unit tests for all functions"
    }
  }
}
```

**Injection format** (prepended to system prompt):

```markdown
## Definitions

**project**: The current software project

### Terms

**epic**: A large feature or capability
**ticket**: A small, implementable unit of work

### Guidelines

**code_style**: Use snake_case for variables
**testing**: Write unit tests for all functions
```

- Flat definitions appear first, then sections (both alphabetically sorted)
- Definition values support template variable expansion (e.g., `{{state_dir}}`, `{{var.name}}`)
- Section names are converted from snake_case/kebab-case to Title Case

### JSON State Files and Dynamic Prompts

Phases can write structured JSON files that later phases consume. This enables dynamic, context-aware prompts:

**Pattern: Phase Chain with JSON State**
1. Phase A writes structured JSON to `{{state_dir}}/analysis.json`
2. JSON variable extracts specific value: `{ "type": "json", "source": "{{state_dir}}/analysis.json", "path": ".summary" }`
3. Phase B prompt uses extracted value: `"Previous analysis: {{var.summary}}"`

**JSON Path Syntax**:
- `.field` - Top-level field
- `.nested.field` - Nested field
- `.[0]` - Array index (root array)
- `.array[0].field` - Field in array element

**Best Practices**:
- Design JSON schemas with downstream extraction in mind
- Include `id`, `status`, `priority` fields for iteratable items
- Use `required: false` with `default` for optional state files
- Specify expected JSON schema in prompts for consistency

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

### Memory System

Memories persist learnings across workflow phases and are automatically injected into system prompts. This helps agents remember project-specific patterns, quirks, and solutions to avoid repeating mistakes.

#### Agent Commands

During workflow execution, agents can save learnings:

```bash
# Add a memory (auto-detects active workflow)
agent memory add "this project uses snake_case for all variables"

# Add with a category
agent memory add "API returns 500 for invalid tokens" --category error_handling

# Remove a memory by ID
agent memory remove 3
```

#### CLI Commands

```bash
# List all memories for a workflow
agent memory list software

# List memories filtered by category
agent memory list software --category code_style

# Search memories
agent memory search "snake_case" --workflow software

# Clear all memories (with confirmation)
agent memory clear software

# Clear without confirmation
agent memory clear software -y
```

#### Memory File Location

Memories are stored in `.agent/workflows/<workflow_name>/memory.jsonl` (project-level).

Each entry is a JSON line:
```json
{"id":1,"timestamp":"2024-01-15T10:30:00Z","content":"learned something","category":"code_style","phase":"spec"}
```

#### System Prompt Injection

Memories are injected into the system prompt in this order:
1. Workflow definitions (prepended)
2. Phase system_prompt
3. **Workflow memories** (injected here)
4. Completion instructions (appended)

Memory format in system prompt:
```markdown
## Workflow Memories

The following are learnings from previous phases in this workflow:

- General learning
- Another learning (from phase: spec)

### Code Style

- Use snake_case for variables
- Follow existing patterns
```

#### Disabling Memories

Disable memory injection for a workflow by setting `memory: false` in defaults:

```json
{
  "defaults": {
    "agent": "claude",
    "memory": false
  }
}
```
