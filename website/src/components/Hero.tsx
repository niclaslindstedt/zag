import Terminal from "./Terminal";
import { terminalDemos } from "../data/terminalDemos";

export default function Hero() {
  return (
    <section className="relative overflow-hidden pt-32 pb-20 md:pt-44 md:pb-32">
      {/* Background glow */}
      <div className="pointer-events-none absolute top-0 left-1/2 -translate-x-1/2 h-[600px] w-[800px] rounded-full bg-accent/5 blur-3xl" />

      <div className="relative mx-auto max-w-6xl px-6 text-center">
        <div className="mb-6 inline-block rounded-full border border-border bg-surface-alt px-4 py-1.5 text-xs text-text-secondary">
          v0.5.0 — Now available on crates.io
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

        {/* Animated terminal */}
        <Terminal tabs={terminalDemos} className="mx-auto mt-12 max-w-2xl" />

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
