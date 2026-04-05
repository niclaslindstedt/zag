const bindings = [
  {
    lang: "TypeScript",
    install: "npm install @nlindstedt/zag-agent",
    code: `import { ZagBuilder } from "@nlindstedt/zag-agent";

const result = await new ZagBuilder()
  .provider("claude")
  .model("sonnet")
  .systemPrompt("You are a code reviewer")
  .json()
  .exec("Review this pull request");

console.log(result.output);`,
  },
  {
    lang: "Python",
    install: "pip install zag-agent",
    code: `from zag import ZagBuilder

result = await (
    ZagBuilder()
    .provider("claude")
    .model("sonnet")
    .system_prompt("You are a code reviewer")
    .json_mode()
    .exec("Review this pull request")
)

print(result.output)`,
  },
  {
    lang: "C#",
    install: "dotnet add package Zag",
    code: `using Zag;

var result = await new ZagBuilder()
    .Provider("claude")
    .Model("sonnet")
    .SystemPrompt("You are a code reviewer")
    .Json()
    .Exec("Review this pull request");

Console.WriteLine(result.Output);`,
  },
];

export default function Bindings() {
  return (
    <section id="sdks" className="border-t border-border py-20 md:py-28">
      <div className="mx-auto max-w-6xl px-6">
        <h2 className="text-center text-3xl font-bold text-text-primary md:text-4xl">
          Use from any language
        </h2>
        <p className="mx-auto mt-4 max-w-2xl text-center text-text-secondary">
          Lightweight SDKs with identical builder APIs. Zero heavy dependencies — each one
          simply wraps the zag CLI via subprocess.
        </p>

        <div className="mt-14 grid gap-6 lg:grid-cols-3">
          {bindings.map((b) => (
            <div key={b.lang} className="overflow-hidden rounded-xl border border-border bg-surface-alt">
              <div className="flex items-center justify-between border-b border-border px-5 py-3">
                <span className="text-sm font-semibold text-text-primary">{b.lang}</span>
                <code className="text-xs text-text-dim">{b.install}</code>
              </div>
              <pre className="overflow-x-auto p-5 text-xs leading-relaxed text-text-secondary">
                <code>{b.code}</code>
              </pre>
            </div>
          ))}
        </div>

        <p className="mt-8 text-center text-sm text-text-dim">
          Also available as a native Rust library:{" "}
          <code className="rounded bg-surface-alt px-1.5 py-0.5 text-accent">cargo add zag</code>
          {" "}— zero subprocess overhead.
        </p>
      </div>
    </section>
  );
}
