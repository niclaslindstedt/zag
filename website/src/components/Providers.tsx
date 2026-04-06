import { providers as sourceProviders, providerCount } from "../data/sourceData";

const colorMap: Record<string, string> = {
  claude:  "text-claude border-claude/30",
  codex:   "text-codex border-codex/30",
  gemini:  "text-gemini border-gemini/30",
  copilot: "text-copilot border-copilot/30",
  ollama:  "text-ollama border-ollama/30",
};

const providers = sourceProviders.map((p) => ({
  name: p.displayName,
  org: p.org,
  color: colorMap[p.name] ?? "text-text-primary border-border",
  models: p.sizeMap,
  features: p.cardFeatures,
}));

export default function Providers() {
  return (
    <section id="providers" className="border-t border-border bg-surface-alt py-20 md:py-28">
      <div className="mx-auto max-w-6xl px-6">
        <h2 className="text-center text-3xl font-bold text-text-primary md:text-4xl">
          {providerCount} providers, one interface
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
