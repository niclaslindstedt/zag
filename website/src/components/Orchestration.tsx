const patterns = [
  {
    name: "Sequential Pipeline",
    description: "Chain tasks with dependency tracking",
    code: `# Each step depends on the previous
S1=$(zag spawn "Analyze codebase")
S2=$(zag spawn --depends-on $S1 "Write tests")
S3=$(zag spawn --depends-on $S2 "Review changes")
zag wait $S3 && zag collect $S3`,
  },
  {
    name: "Fan-Out / Gather",
    description: "Parallelize work across agents, then collect results",
    code: `# Spawn parallel tasks with tags
zag spawn --tag batch "Fix auth module"
zag spawn --tag batch "Fix logging module"
zag spawn --tag batch "Fix config module"

# Wait for all and gather results
zag wait --tag batch
zag collect --tag batch`,
  },
  {
    name: "Generator & Critic",
    description: "Iterative refinement loop between two agents",
    code: `# Generate with one provider, critique with another
GEN=$(zag spawn -p codex "Write API endpoint")
zag wait $GEN

# Pipe output to a reviewer
REVIEW=$(zag pipe $GEN -p claude \\
  "Review this code for security issues")
zag wait $REVIEW && zag collect $REVIEW`,
  },
];

const commands = [
  { cmd: "spawn", desc: "Launch background agent session" },
  { cmd: "wait", desc: "Block until session(s) complete" },
  { cmd: "collect", desc: "Gather results from sessions" },
  { cmd: "pipe", desc: "Chain output into new session" },
  { cmd: "status", desc: "Check session health" },
  { cmd: "input", desc: "Send message to running session" },
  { cmd: "broadcast", desc: "Message all sessions by tag" },
  { cmd: "events", desc: "Query structured event logs" },
  { cmd: "watch", desc: "React to log events" },
  { cmd: "subscribe", desc: "Multiplexed event stream" },
  { cmd: "cancel", desc: "Gracefully stop sessions" },
  { cmd: "retry", desc: "Re-run failed sessions" },
  { cmd: "summary", desc: "Log-based session summary" },
  { cmd: "gc", desc: "Clean up old session data" },
];

export default function Orchestration() {
  return (
    <section id="orchestration" className="border-t border-border py-20 md:py-28">
      <div className="mx-auto max-w-6xl px-6">
        <h2 className="text-center text-3xl font-bold text-text-primary md:text-4xl">
          Multi-agent orchestration from the shell
        </h2>
        <p className="mx-auto mt-4 max-w-2xl text-center text-text-secondary">
          Built-in primitives for spawning, coordinating, and collecting results from multiple agent sessions.
          Compose them into pipelines, fan-out patterns, and more.
        </p>

        {/* Patterns */}
        <div className="mt-14 grid gap-6 lg:grid-cols-3">
          {patterns.map((p) => (
            <div key={p.name} className="rounded-xl border border-border bg-surface-alt overflow-hidden">
              <div className="border-b border-border p-4">
                <h3 className="font-semibold text-text-primary">{p.name}</h3>
                <p className="mt-1 text-xs text-text-dim">{p.description}</p>
              </div>
              <pre className="overflow-x-auto p-4 text-xs leading-relaxed text-text-secondary">
                <code>{p.code}</code>
              </pre>
            </div>
          ))}
        </div>

        {/* Command reference */}
        <div className="mx-auto mt-16 max-w-3xl">
          <h3 className="mb-6 text-center text-xl font-semibold text-text-primary">Orchestration Commands</h3>
          <div className="grid grid-cols-2 gap-x-8 gap-y-3 sm:grid-cols-3 md:grid-cols-4">
            {commands.map((c) => (
              <div key={c.cmd}>
                <code className="text-sm font-semibold text-accent">{c.cmd}</code>
                <p className="text-xs text-text-dim">{c.desc}</p>
              </div>
            ))}
          </div>
        </div>
      </div>
    </section>
  );
}
