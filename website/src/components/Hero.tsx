export default function Hero() {
  return (
    <section className="relative overflow-hidden pt-32 pb-20 md:pt-44 md:pb-32">
      {/* Background glow */}
      <div className="pointer-events-none absolute top-0 left-1/2 -translate-x-1/2 h-[600px] w-[800px] rounded-full bg-accent/5 blur-3xl" />

      <div className="relative mx-auto max-w-6xl px-6 text-center">
        <div className="mb-6 inline-block rounded-full border border-border bg-surface-alt px-4 py-1.5 text-xs text-text-secondary">
          v0.2.0 — Now available on crates.io
        </div>

        <h1 className="mx-auto max-w-4xl text-4xl leading-tight font-extrabold tracking-tight text-text-primary md:text-6xl md:leading-tight">
          One CLI for{" "}
          <span className="bg-gradient-to-r from-accent to-accent-light bg-clip-text text-transparent">
            all your AI coding agents
          </span>
        </h1>

        <p className="mx-auto mt-6 max-w-2xl text-lg text-text-secondary md:text-xl">
          Switch between Claude, Codex, Gemini, Copilot, and Ollama with a single command.
          Orchestrate multi-agent workflows. Use from Rust, TypeScript, Python, or C#.
        </p>

        {/* Provider pills */}
        <div className="mt-8 flex flex-wrap items-center justify-center gap-3">
          {[
            { name: "Claude", color: "text-claude" },
            { name: "Codex", color: "text-codex" },
            { name: "Gemini", color: "text-gemini" },
            { name: "Copilot", color: "text-copilot" },
            { name: "Ollama", color: "text-ollama" },
          ].map((p) => (
            <span key={p.name} className={`rounded-full border border-border bg-surface-alt px-3 py-1 text-sm font-medium ${p.color}`}>
              {p.name}
            </span>
          ))}
        </div>

        {/* Terminal mockup */}
        <div className="mx-auto mt-12 max-w-2xl overflow-hidden rounded-xl border border-border bg-surface-alt shadow-2xl">
          <div className="flex items-center gap-2 border-b border-border px-4 py-3">
            <div className="h-3 w-3 rounded-full bg-[#ff5f57]" />
            <div className="h-3 w-3 rounded-full bg-[#febc2e]" />
            <div className="h-3 w-3 rounded-full bg-[#28c840]" />
            <span className="ml-2 text-xs text-text-dim">terminal</span>
          </div>
          <div className="p-5 text-left text-sm leading-relaxed">
            <div className="text-text-dim">{"# Run with any provider"}</div>
            <div>
              <span className="text-accent">$</span>{" "}
              <span className="text-text-primary">zag exec -p claude </span>
              <span className="text-text-secondary">"Add error handling to src/api.rs"</span>
            </div>
            <div className="mt-3 text-text-dim">{"# Or let zag pick the best one"}</div>
            <div>
              <span className="text-accent">$</span>{" "}
              <span className="text-text-primary">zag exec -p auto -m auto </span>
              <span className="text-text-secondary">"Refactor the auth module"</span>
            </div>
            <div className="mt-3 text-text-dim">{"# Orchestrate multi-agent workflows"}</div>
            <div>
              <span className="text-accent">$</span>{" "}
              <span className="text-text-primary">SID=$(zag spawn -p codex </span>
              <span className="text-text-secondary">"Write tests"</span>
              <span className="text-text-primary">)</span>
            </div>
            <div>
              <span className="text-accent">$</span>{" "}
              <span className="text-text-primary">zag wait $SID && zag collect $SID</span>
            </div>
          </div>
        </div>

        {/* Install CTA */}
        <div className="mt-10 flex flex-col items-center gap-4 sm:flex-row sm:justify-center">
          <a
            href="#get-started"
            className="rounded-lg bg-accent px-6 py-3 text-sm font-semibold text-white shadow-lg shadow-accent/20 hover:bg-accent-light transition-colors"
          >
            Get Started
          </a>
          <code className="rounded-lg border border-border bg-surface-alt px-5 py-3 text-sm text-text-secondary">
            cargo install zag-cli
          </code>
        </div>
      </div>
    </section>
  );
}
