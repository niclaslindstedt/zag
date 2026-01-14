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
      "type": "env|bash|file|json",
      "source": "SOURCE_VALUE",
      "path": ".json.path",
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
    },
    {
      "name": "project_name",
      "type": "json",
      "source": "package.json",
      "path": ".name"
    }
  ],
  "phases": [
    {
      "prompt": "Working on {{var.project_name}} branch {{var.branch}}. Context:\n{{var.context}}"
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
| `json` | Extract value from JSON file | JSON file path |

### Variable Properties

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| `name` | string | Yes | Variable name (accessed as `{{var.name}}`) |
| `type` | enum | Yes | `env`, `bash`, `file`, or `json` |
| `source` | string | Yes | Source specification |
| `path` | string | No | JSON path for `json` type (e.g., `.field`, `.nested.field`, `.array[0]`) |
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

## JSON State Files and Dynamic Prompts

JSON state files are the backbone of dynamic workflows. Design them carefully to enable powerful data passing between phases.

### Why JSON State Files Matter

1. **Structure enables automation**: Well-designed JSON schemas let later phases extract specific values
2. **Iteration support**: JSON arrays drive `iterate` mode for batch processing
3. **Dynamic prompts**: Extract specific fields to craft context-aware prompts
4. **Traceability**: Structured data makes workflow state inspectable and debuggable

### Designing JSON Schemas

When a phase produces JSON output, design the schema with downstream consumption in mind:

**Good Schema (flat, extractable fields)**:
```json
[
  {
    "id": "feature-001",
    "name": "User Authentication",
    "description": "Implement login and session management",
    "priority": 1,
    "estimated_complexity": "medium",
    "dependencies": [],
    "tags": ["security", "core"]
  }
]
```

**Why it's good**:
- Flat structure with named fields
- Each item has an `id` for identification
- Fields like `priority` and `estimated_complexity` can inform later prompts
- `dependencies` array supports ordering logic

### Using JSON Variables for Dynamic Prompts

The `json` variable type extracts specific values from JSON files to inject into prompts:

```json
{
  "variables": [
    {
      "name": "analysis_summary",
      "type": "json",
      "source": "{{state_dir}}/analysis.json",
      "path": ".summary",
      "required": false,
      "default": "No analysis available"
    },
    {
      "name": "issue_count",
      "type": "json",
      "source": "{{state_dir}}/analysis.json",
      "path": ".metrics.issues_found",
      "required": false,
      "default": "0"
    }
  ],
  "phases": [
    {
      "id": "report",
      "prompt": "Found {{var.issue_count}} issues. Summary: {{var.analysis_summary}}"
    }
  ]
}
```

### JSON Path Syntax

| Pattern | Description | Example |
|---------|-------------|---------|
| `.field` | Top-level field | `.name` → `"value"` |
| `.nested.field` | Nested field | `.config.timeout` → `30` |
| `.[0]` | Array index (root array) | `.[0]` → first element |
| `.array[0]` | Array index | `.items[0]` → first item |
| `.array[0].field` | Field in array element | `.users[0].email` |

### Pattern: Phase Chain with JSON State

This pattern shows how phases can write JSON that later phases read via variables:

```json
{
  "name": "analysis-workflow",
  "variables": [
    {
      "name": "summary",
      "type": "json",
      "source": "{{state_dir}}/analysis.json",
      "path": ".summary",
      "required": false,
      "default": "Analysis not yet complete"
    },
    {
      "name": "risk_level",
      "type": "json",
      "source": "{{state_dir}}/analysis.json",
      "path": ".risk_level",
      "required": false,
      "default": "unknown"
    }
  ],
  "phases": [
    {
      "id": "analyze",
      "name": "Analyze Codebase",
      "execution": { "mode": "once" },
      "prompt": "Analyze the codebase and write results to {{state_dir}}/analysis.json:\n{\n  \"summary\": \"Brief summary of findings\",\n  \"risk_level\": \"low|medium|high\",\n  \"issues\": [{\"id\": \"...\", \"severity\": \"...\", \"description\": \"...\"}],\n  \"recommendations\": [\"...\"]\n}"
    },
    {
      "id": "report",
      "name": "Generate Report",
      "execution": { "mode": "once" },
      "depends_on": ["analyze"],
      "prompt": "Previous analysis found risk level: {{var.risk_level}}\nSummary: {{var.summary}}\n\nRead full analysis from {{state_dir}}/analysis.json and generate a detailed report at {{state_dir}}/report.md"
    },
    {
      "id": "fix-issues",
      "name": "Fix Issues",
      "execution": {
        "mode": "iterate",
        "iterate_over": "{{state_dir}}/analysis.json",
        "item_variable": "issue",
        "skip_if_empty": true
      },
      "depends_on": ["analyze"],
      "prompt": "Fix issue {{issue.id}} ({{issue.severity}}): {{issue.description}}"
    }
  ]
}
```

### Recommended JSON Schemas

**For iteration (tasks/tickets/items)**:
```json
[
  {
    "id": "unique-id",
    "name": "Human readable name",
    "description": "Detailed description",
    "status": "pending|in_progress|completed",
    "metadata": { "any": "additional data" }
  }
]
```

**For analysis results**:
```json
{
  "summary": "Brief overview",
  "status": "success|warning|error",
  "findings": [
    { "id": "F001", "severity": "high", "message": "...", "location": "..." }
  ],
  "metrics": {
    "total_files": 42,
    "issues_found": 3
  }
}
```

**For configuration/context**:
```json
{
  "project": {
    "name": "...",
    "version": "...",
    "type": "..."
  },
  "settings": {
    "key": "value"
  }
}
```

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

## Role-Based Prompts

Assigning a specific role to the agent in each phase significantly improves output quality. Roles establish context, vocabulary, and perspective that shape how the agent approaches the task.

### Why Roles Matter

- **Domain expertise**: A "security auditor" naturally focuses on vulnerabilities; a "UX designer" considers user experience
- **Appropriate vocabulary**: A "lawyer" uses precise legal terminology; a "developer" uses technical jargon
- **Consistent perspective**: Roles maintain focus throughout complex tasks
- **Better reasoning**: Agents perform better when given clear identity context

### Role Prompt Structure

Use this pattern for phase prompts:

```
You are a <role>. You are tasked with <task>.

<additional context and instructions>
```

### Examples

**Technical specification writer**:
```json
{
  "id": "spec",
  "prompt": "You are a senior software architect. You are tasked with writing a technical specification for the requested feature.\n\nAnalyze the requirements and produce a detailed spec covering architecture, data models, APIs, and edge cases. Save to {{state_dir}}/spec.md"
}
```

**Code reviewer**:
```json
{
  "id": "review",
  "prompt": "You are a principal engineer conducting a code review. You are tasked with reviewing the implementation for correctness, security, and maintainability.\n\nFocus on:\n- Logic errors and edge cases\n- Security vulnerabilities\n- Code clarity and documentation\n- Performance implications\n\nSave findings to {{state_dir}}/review.md"
}
```

**Security analyst**:
```json
{
  "id": "security-audit",
  "prompt": "You are a security analyst specializing in application security. You are tasked with auditing the codebase for vulnerabilities.\n\nCheck for OWASP Top 10 issues, authentication flaws, and data exposure risks. Document findings with severity ratings in {{state_dir}}/security-audit.json"
}
```

**Technical writer**:
```json
{
  "id": "docs",
  "prompt": "You are a technical writer. You are tasked with creating user-facing documentation for the new feature.\n\nWrite clear, concise documentation with examples. Target audience: developers integrating with this API. Save to {{state_dir}}/docs.md"
}
```

### Role Selection Guidelines

| Task Type | Recommended Role |
|-----------|------------------|
| Architecture/design | Senior software architect |
| Implementation | Senior developer, Backend/Frontend engineer |
| Code review | Principal engineer, Staff engineer |
| Security | Security analyst, Penetration tester |
| Testing | QA engineer, Test automation engineer |
| Documentation | Technical writer |
| API design | API designer, Integration architect |
| Performance | Performance engineer |
| DevOps/Infrastructure | Platform engineer, SRE |
| Data modeling | Data architect, Database engineer |

### Combining Roles with System Prompts

For complex phases, use `system_prompt` for persistent role context and `prompt` for task-specific instructions:

```json
{
  "id": "implement",
  "system_prompt": "You are a senior backend engineer with expertise in distributed systems. You write clean, testable code and consider edge cases carefully.",
  "prompt": "Implement {{ticket.name}}: {{ticket.description}}\n\nFollow the patterns established in the codebase. Write tests for your implementation."
}
```

## Best Practices

1. **Use descriptive IDs**: `create-tickets` not `phase1`
2. **Document state paths**: Include paths in system prompts
3. **Create systematic filenames**: Makes data easy to find
4. **Add skip_if_empty for optional iterations**: Prevents errors
5. **Keep prompts focused**: One clear task per phase
6. **Include context in system prompts**: Reference relevant files
7. **Use interactive mode for complex tasks**: Allows agent to ask clarifying questions
8. **Write user-input prompts in first-person**: Use "Ask me which files..." not "Ask the user which files..."
9. **Assign roles to agents**: Start prompts with "You are a <role>. You are tasked with <task>." to set appropriate context and vocabulary
10. **Design JSON schemas for extraction**: Include fields that later phases will need to reference
11. **Use JSON variables for dynamic context**: Extract specific values from state files into prompts
12. **Prefer flat JSON structures**: Deeply nested objects are harder to extract from
13. **Include metadata in JSON outputs**: Fields like `status`, `priority`, `id` enable filtering and ordering
14. **Specify JSON schemas in prompts**: Tell agents exactly what structure to produce for consistency

## When Modifying Workflows

Common modification patterns:

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
- Use `type: json` with `path` to extract specific values from JSON files
- Set `required: false` with `default` for optional variables

### Adding JSON state files for data passing
- Have phases write structured JSON to `{{state_dir}}/filename.json`
- Define JSON schemas in prompts so output is consistent
- Add JSON variables to extract specific values for later phases
- Use `iterate_over` to loop through JSON arrays

### Making prompts more dynamic with JSON variables
- Identify static context that could come from previous phase output
- Add `type: json` variables with appropriate `path` to extract values
- Update prompts to use `{{var.name}}` for dynamic context
- Always set `required: false` with `default` for state files that may not exist yet

### Improving JSON schemas in prompts
- Add explicit schemas showing expected structure
- Include all fields that later phases will need to reference
- Prefer flat structures over deeply nested objects
- Include `id`, `status`, and `priority` fields for items that will be iterated

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
      "prompt": "You are a senior engineer triaging code changes. You are tasked with identifying files that need review.\n\nAnalyze recent changes and prioritize files by risk and complexity. Save to {{state_dir}}/files.json as [{\"path\": \"...\", \"priority\": 1, \"reason\": \"...\"}]"
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
      "prompt": "You are a principal engineer conducting a thorough code review. You are tasked with reviewing {{file.path}}.\n\nPriority reason: {{file.reason}}\n\nExamine for correctness, security vulnerabilities, performance issues, and maintainability. Save findings to {{state_dir}}/reviews/{{index}}.md"
    },
    {
      "id": "summary",
      "name": "Generate Summary",
      "execution": { "mode": "once" },
      "depends_on": ["review-loop"],
      "prompt": "You are a technical lead preparing a review summary for the team. You are tasked with synthesizing all review findings.\n\nRead all reviews in {{state_dir}}/reviews/. Create an executive summary highlighting critical issues, patterns, and recommendations at {{state_dir}}/summary.md"
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
