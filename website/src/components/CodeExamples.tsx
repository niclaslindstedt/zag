import { useState } from "react";

const tabs = [
  {
    label: "Basic Usage",
    code: `# Interactive session with Claude (default)
$ zag run

# Non-interactive: execute and return
$ zag exec "Add error handling to src/api.rs"

# Use a different provider
$ zag exec -p gemini "Explain this function"

# Use size aliases instead of model names
$ zag exec -p codex -m small "Quick formatting fix"

# Auto-select the best provider and model
$ zag exec -p auto -m auto "Refactor auth module"`,
  },
  {
    label: "JSON & Streaming",
    code: `# Get structured JSON output
$ zag exec --json "List all API endpoints"

# Validate output against a JSON schema
$ zag exec --json-schema schema.json \\
    "Extract metadata from README"
# Auto-retries up to 3x if validation fails

# Stream events as NDJSON
$ zag exec -o stream-json "Write a test suite"
{"type":"init","session_id":"a1b2c3..."}
{"type":"assistant_message","content":"..."}
{"type":"tool_execution","tool":"write","path":"..."}
{"type":"result","output":"Done.","tokens":{...}}`,
  },
  {
    label: "Sessions",
    code: `# Name and tag sessions for easy discovery
$ zag run --name "auth-refactor" --tag sprint-12

# List recent sessions
$ zag session list

# Resume a previous session
$ zag run --session <session-id>

# Search through all session logs
$ zag search "error handling"

# Export session environment
$ eval $(zag env <session-id>)`,
  },
  {
    label: "Isolation",
    code: `# Run in an isolated git worktree
$ zag exec --worktree "Experiment with new API design"
# Changes stay in the worktree — your branch is untouched

# Run in a Docker sandbox
$ zag exec --sandbox "Run untrusted build script"
# Full isolation from your host system

# Combine with any provider
$ zag exec -p codex --worktree --auto-approve \\
    "Rewrite the database layer"`,
  },
];

export default function CodeExamples() {
  const [active, setActive] = useState(0);

  return (
    <section className="border-t border-border bg-surface-alt py-20 md:py-28">
      <div className="mx-auto max-w-4xl px-6">
        <h2 className="text-center text-3xl font-bold text-text-primary md:text-4xl">
          See it in action
        </h2>
        <p className="mx-auto mt-4 max-w-2xl text-center text-text-secondary">
          From simple one-liners to complex multi-agent pipelines — zag keeps the interface consistent.
        </p>

        <div className="mt-12 overflow-hidden rounded-xl border border-border bg-surface shadow-2xl">
          {/* Tab bar */}
          <div className="flex overflow-x-auto border-b border-border">
            {tabs.map((t, i) => (
              <button
                key={t.label}
                onClick={() => setActive(i)}
                className={`shrink-0 whitespace-nowrap px-5 py-3 text-sm font-medium transition-colors ${
                  i === active
                    ? "border-b-2 border-accent text-accent bg-surface-alt"
                    : "text-text-dim hover:text-text-secondary"
                }`}
              >
                {t.label}
              </button>
            ))}
          </div>
          {/* Code */}
          <pre className="overflow-x-auto p-6 text-sm leading-relaxed text-text-secondary">
            <code>{tabs[active].code}</code>
          </pre>
        </div>
      </div>
    </section>
  );
}
