import {
  providerCount,
  providers,
  orchestrationCommands,
  bindings,
  commands,
} from "../data/sourceData";

const orchCmdNames = orchestrationCommands.map((c) => c.name);
const bindingLangs = bindings.map((b) => b.language);
const hasSkillsCmd = commands.some((c) => c.name === "skills");
const hasMcpCmd = commands.some((c) => c.name === "mcp");
const hasAutoProvider = providers.some((p) => p.availableModels.includes("auto"));

const features = [
  {
    title: `One CLI, ${providerCount} Agents`,
    description:
      `Switch between ${providers.map((p) => p.displayName).join(", ")} with a single -p flag. Same commands, same output format, any provider.`,
    icon: "\u{1F500}",
  },
  {
    title: "Multi-Agent Orchestration",
    description:
      `Built-in ${orchCmdNames.slice(0, 4).join(", ")}, and ${orchCmdNames[4]} primitives. Build sequential pipelines, fan-out/gather patterns, and coordinator workflows from the shell.`,
    icon: "\u{1F9E9}",
  },
  {
    title: "Structured JSON Output",
    description:
      "Request JSON responses with --json, validate against schemas with --json-schema (auto-retries on failure), or stream NDJSON events in real-time.",
    icon: "\u{1F4CB}",
  },
  {
    title: "Session Management",
    description:
      "Every session gets a UUID with name, description, and tags. Resume previous sessions, search history, and export structured event logs.",
    icon: "\u{1F4BE}",
  },
  {
    title: "Isolation Modes",
    description:
      "Run agents in isolated git worktrees (--worktree) or Docker sandboxes (--sandbox). Safe experimentation without touching your working tree.",
    icon: "\u{1F512}",
  },
  {
    title: "SDKs & Library",
    description:
      `Use as a Rust library with zero subprocess overhead, or from ${bindingLangs.join(", ")} via lightweight SDKs that wrap the CLI.`,
    icon: "\u{1F4E6}",
  },
  {
    title: "Portable Model Aliases",
    description:
      "Use small, medium, and large instead of provider-specific model names. The right model maps automatically for each provider.",
    icon: "\u{1F3F7}\u{FE0F}",
  },
  ...(hasSkillsCmd || hasMcpCmd
    ? [
        {
          title: "Skills & MCP Servers",
          description:
            `Manage agent skills and MCP servers across all providers from a single config. Add once, sync everywhere with ${hasSkillsCmd ? "zag skills" : ""}${hasSkillsCmd && hasMcpCmd ? " and " : ""}${hasMcpCmd ? "zag mcp" : ""}.`,
          icon: "\u{1F6E0}\u{FE0F}",
        },
      ]
    : []),
  ...(hasAutoProvider
    ? [
        {
          title: "Auto Provider Selection",
          description:
            "Use -p auto -m auto and let an LLM analyze your task to recommend the optimal provider and model size. Configurable selector agent.",
          icon: "\u{1F916}",
        },
      ]
    : []),
];

export default function Features() {
  return (
    <section id="features" className="border-t border-border py-20 md:py-28">
      <div className="mx-auto max-w-6xl px-6">
        <h2 className="text-center text-3xl font-bold text-text-primary md:text-4xl">
          Everything you need to work with AI agents
        </h2>
        <p className="mx-auto mt-4 max-w-2xl text-center text-text-secondary">
          A unified interface that gives you cross-provider features, orchestration primitives, and programmatic access — all from one tool.
        </p>

        <div className="mt-14 grid gap-6 sm:grid-cols-2 lg:grid-cols-3">
          {features.map((f) => (
            <div
              key={f.title}
              className="group rounded-xl border border-border bg-surface-alt p-6 transition-all hover:border-accent/40 hover:bg-surface-hover"
            >
              <div className="mb-4 text-2xl">{f.icon}</div>
              <h3 className="mb-2 text-lg font-semibold text-text-primary">{f.title}</h3>
              <p className="text-sm leading-relaxed text-text-secondary">{f.description}</p>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}
