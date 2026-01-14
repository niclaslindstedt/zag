# Agent Workflow System

Workflows define multi-phase AI agent sessions that execute sequentially with filesystem-based data passing. User-defined workflows are stored in `~/.agent/workflows/<name>.json`.

## Workflow JSON Schema

```json
{
  "name": "workflow-name",
  "version": "1.0.0",
  "description": "What this workflow accomplishes",
  "defaults": {
    "agent": "claude",
    "model": "opus",
    "interactive": true,
    "skip_permissions": false
  },
  "variables": [
    {
      "name": "variable_name",
      "type": "env|bash|file",
      "source": "SOURCE_VALUE",
      "required": true,
      "default": "fallback value"
    }
  ],
  "phases": [
    {
      "id": "unique-phase-id",
      "name": "Human-readable Phase Name",
      "execution": {
        "mode": "once",
        "iterate_over": "{{state_dir}}/items.json",
        "item_variable": "item",
        "skip_if_empty": false
      },
      "agent": "claude",
      "model": "opus",
      "interactive": true,
      "skip_permissions": false,
      "system_prompt": "System prompt with {{variables}}",
      "prompt": "User prompt with {{variables}}",
      "output": {
        "filename": "output.md",
        "format": "markdown"
      },
      "depends_on": ["previous-phase-id"],
      "parent": "parent-phase-id",
      "nested_phases": ["child-phase-id"]
    }
  ]
}
```

## Execution Modes

### `once`
Run the phase a single time.

```json
{
  "execution": { "mode": "once" }
}
```

### `iterate`
Run the phase for each item in a JSON array file.

```json
{
  "execution": {
    "mode": "iterate",
    "iterate_over": "{{state_dir}}/items.json",
    "item_variable": "item",
    "skip_if_empty": true
  }
}
```

- `iterate_over`: Path to JSON array file (supports template variables)
- `item_variable`: Variable name for the current item (default: "item")
- `skip_if_empty`: Skip iteration if file doesn't exist or array is empty

## Template Variables

Use `{{variable}}` syntax in prompts. Available variables:

| Variable | Description |
|----------|-------------|
| `{{state_dir}}` | Current run's state directory |
| `{{index}}` | Current iteration index (0-based) |
| `{{item.field}}` | Field from current iteration item |
| `{{var.name}}` | Custom variables defined in `variables` array |

### Example
```json
{
  "prompt": "Process item {{item.id}}: {{item.name}}. Save to {{state_dir}}/output/{{item.id}}.md"
}
```

## Custom Variables

Define variables at workflow level that are resolved before execution. Custom variables use the `{{var.name}}` syntax to distinguish them from built-in variables:

```json
{
  "variables": [
    {
      "name": "branch",
      "type": "bash",
      "source": "git branch --show-current"
    },
    {
      "name": "api_key",
      "type": "env",
      "source": "MY_API_KEY",
      "required": false,
      "default": ""
    },
    {
      "name": "context",
      "type": "file",
      "source": "CLAUDE.md",
      "required": false,
      "default": "No context available"
    }
  ],
  "phases": [
    {
      "prompt": "Working on branch {{var.branch}}. Context:\n{{var.context}}"
    }
  ]
}
```

### Variable Types

| Type | Description | Source Field |
|------|-------------|--------------|
| `env` | Read from environment variable | Environment variable name |
| `bash` | Execute command and capture stdout | Shell command string |
| `file` | Read file contents | File path (supports `{{state_dir}}`) |

### Variable Properties

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `name` | string | Yes | Variable name (accessed as `{{var.name}}`) |
| `type` | enum | Yes | `env`, `bash`, or `file` |
| `source` | string | Yes | Source specification |
| `required` | bool | No | Fail if unavailable (default: true) |
| `default` | string | No | Fallback value if source unavailable |

### Dependency Resolution

Variables are resolved once at workflow start. Dependencies are automatically detected via `{{var.X}}` patterns and resolved in the correct order, so variables can be defined in any order:

```json
{
  "variables": [
    { "name": "config", "type": "file", "source": "{{var.project}}/config.json" },
    { "name": "project", "type": "env", "source": "PROJECT_NAME" }
  ]
}
```

Circular dependencies are detected and reported as errors.

## Nested Phases

For complex workflows with nested iterations (e.g., epic -> tickets -> follow-ups):

1. Create parent phase with `"nested_phases": ["child-id"]`
2. Create child phases with `"parent": "parent-id"`

```json
{
  "id": "epic-loop",
  "execution": {
    "mode": "iterate",
    "iterate_over": "{{state_dir}}/epics.json",
    "item_variable": "epic"
  },
  "nested_phases": ["create-tickets", "ticket-loop"]
},
{
  "id": "create-tickets",
  "parent": "epic-loop",
  "execution": { "mode": "once" },
  "prompt": "Create tickets for {{epic.name}}..."
}
```

## State Directory Structure

Each workflow run creates:

```
.agent/state/<workflow>/<run_id>/
├── manifest.json          # Run status and progress
└── <phase outputs>        # Files created by agents
```

Recommended filenames:
- `spec.md` - Specifications
- `items.json` - JSON arrays for iteration
- `<id>/` - Subdirectories for nested items

## Agent Configuration

Each phase can override defaults:

| Field | Description |
|-------|-------------|
| `agent` | "claude", "codex", "gemini", "copilot" |
| `model` | Agent-specific model name |
| `interactive` | Run in interactive mode (true/false) |
| `skip_permissions` | Auto-approve actions (true/false) |

## Dependencies

Use `depends_on` to ensure phases run in order:

```json
{
  "id": "review",
  "depends_on": ["implement"],
  "prompt": "Review the implementation..."
}
```

## Automatic Completion Handling

For **interactive** phases, the workflow engine automatically injects a completion instruction. This does not need to be included in the workflow JSON.

For **non-interactive** phases (`interactive: false`), the agent exits naturally when done.

## Best Practices

1. **Use descriptive IDs**: `create-tickets` not `phase1`
2. **Document state paths**: Include paths in system prompts
3. **Create systematic filenames**: Makes data easy to find
4. **Add skip_if_empty for optional iterations**: Prevents errors
5. **Keep prompts focused**: One clear task per phase
6. **Include context in system prompts**: Reference relevant files
7. **Use interactive mode for complex tasks**: Allows agent to ask clarifying questions
8. **Write user-input prompts in first-person**: Use "Ask me which files..." not "Ask the user which files..."

## Common Modification Patterns

### Adding a new phase
- Determine where in the workflow it should run (use `depends_on`)
- Consider if it's part of an iteration loop (use `parent`)

### Fixing iteration issues
- Check `iterate_over` path uses correct variables
- Verify `item_variable` matches usage in prompts
- Add `skip_if_empty: true` if the file might not exist

### Changing prompts
- Keep prompts focused on a single task
- Use `{{state_dir}}` for file paths
- Use `{{item.field}}` for iteration data

### Adjusting agent settings
- Change `agent`, `model`, `interactive`, or `skip_permissions`
- Can be set per-phase or in `defaults`

### Adding variables
- Add to `variables` array at workflow level
- Use `type: env` for environment variables
- Use `type: bash` for dynamic command output
- Use `type: file` for file content injection
- Set `required: false` with `default` for optional variables

## Example: Code Review Workflow

```json
{
  "name": "review",
  "version": "1.0.0",
  "description": "Multi-file code review workflow",
  "defaults": { "agent": "claude", "interactive": true },
  "phases": [
    {
      "id": "identify-files",
      "name": "Identify Files to Review",
      "execution": { "mode": "once" },
      "prompt": "List files that need review. Save to {{state_dir}}/files.json as [{\"path\": \"...\", \"priority\": 1}]"
    },
    {
      "id": "review-loop",
      "name": "Review Files",
      "execution": {
        "mode": "iterate",
        "iterate_over": "{{state_dir}}/files.json",
        "item_variable": "file"
      },
      "depends_on": ["identify-files"],
      "nested_phases": ["review-file"]
    },
    {
      "id": "review-file",
      "name": "Review Single File",
      "parent": "review-loop",
      "execution": { "mode": "once" },
      "prompt": "Review {{file.path}}. Save findings to {{state_dir}}/reviews/{{index}}.md"
    },
    {
      "id": "summary",
      "name": "Generate Summary",
      "execution": { "mode": "once" },
      "depends_on": ["review-loop"],
      "prompt": "Read all reviews in {{state_dir}}/reviews/. Create summary at {{state_dir}}/summary.md"
    }
  ]
}
```

## Testing Commands

```bash
# List available workflows
agent workflow --list

# Run workflow
agent workflow <name>

# Resume interrupted workflow
agent workflow <name> --resume

# List previous runs
agent workflow <name> --list-runs

# Resume specific run
agent workflow <name> --resume --run-id <id>
```
