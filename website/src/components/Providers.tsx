const providers = [
  {
    name: "Claude",
    org: "Anthropic",
    color: "text-claude border-claude/30",
    models: { small: "haiku", medium: "sonnet", large: "opus / sonnet" },
    features: ["Interactive", "Streaming", "Resume", "JSON Schema", "MCP"],
  },
  {
    name: "Codex",
    org: "OpenAI",
    color: "text-codex border-codex/30",
    models: { small: "gpt-5.4-mini", medium: "gpt-5.3-codex", large: "gpt-5.4" },
    features: ["Interactive", "Streaming", "Resume", "MCP"],
  },
  {
    name: "Gemini",
    org: "Google",
    color: "text-gemini border-gemini/30",
    models: { small: "flash-lite", medium: "flash", large: "pro" },
    features: ["Interactive", "Streaming", "MCP"],
  },
  {
    name: "Copilot",
    org: "GitHub",
    color: "text-copilot border-copilot/30",
    models: { small: "—", medium: "—", large: "default" },
    features: ["Interactive", "MCP"],
  },
  {
    name: "Ollama",
    org: "Local",
    color: "text-ollama border-ollama/30",
    models: { small: "auto (by size)", medium: "auto (by size)", large: "auto (by size)" },
    features: ["Interactive", "No API key needed"],
  },
];

export default function Providers() {
  return (
    <section id="providers" className="border-t border-border bg-surface-alt py-20 md:py-28">
      <div className="mx-auto max-w-6xl px-6">
        <h2 className="text-center text-3xl font-bold text-text-primary md:text-4xl">
          Five providers, one interface
        </h2>
        <p className="mx-auto mt-4 max-w-2xl text-center text-text-secondary">
          Use portable size aliases — <code className="rounded bg-surface px-1.5 py-0.5 text-xs text-accent">small</code>,{" "}
          <code className="rounded bg-surface px-1.5 py-0.5 text-xs text-accent">medium</code>,{" "}
          <code className="rounded bg-surface px-1.5 py-0.5 text-xs text-accent">large</code> — that
          map to the right model for each provider automatically.
        </p>

        <div className="mt-14 grid gap-6 sm:grid-cols-2 lg:grid-cols-3">
          {providers.map((p) => (
            <div key={p.name} className={`rounded-xl border ${p.color} bg-surface p-6`}>
              <div className="mb-1 text-xs font-medium uppercase tracking-wider text-text-dim">{p.org}</div>
              <h3 className={`mb-4 text-xl font-bold ${p.color.split(" ")[0]}`}>{p.name}</h3>
              <div className="mb-4 space-y-2 text-sm">
                {(["small", "medium", "large"] as const).map((size) => (
                  <div key={size} className="flex justify-between">
                    <span className="text-text-dim">{size}</span>
                    <span className="text-text-secondary">{p.models[size]}</span>
                  </div>
                ))}
              </div>
              <div className="flex flex-wrap gap-1.5">
                {p.features.map((f) => (
                  <span key={f} className="rounded-full bg-surface-alt px-2 py-0.5 text-xs text-text-dim">
                    {f}
                  </span>
                ))}
              </div>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}
