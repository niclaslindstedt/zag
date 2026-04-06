import { bindings as sourceBindings } from "../data/sourceData";

// Code examples are editorial content — they demonstrate idiomatic usage per language.
// Install commands come from sourceData (extracted from binding package manifests).
const codeExamples: Record<string, string> = {
  TypeScript: `import { ZagBuilder } from "@nlindstedt/zag-agent";

const result = await new ZagBuilder()
  .provider("claude")
  .model("sonnet")
  .systemPrompt("You are a code reviewer")
  .json()
  .exec("Review this pull request");

console.log(result.output);`,
  Python: `from zag import ZagBuilder

result = await (
    ZagBuilder()
    .provider("claude")
    .model("sonnet")
    .system_prompt("You are a code reviewer")
    .json_mode()
    .exec("Review this pull request")
)

print(result.output)`,
  "C#": `using Zag;

var result = await new ZagBuilder()
    .Provider("claude")
    .Model("sonnet")
    .SystemPrompt("You are a code reviewer")
    .Json()
    .Exec("Review this pull request");

Console.WriteLine(result.Output);`,
  Swift: `import Zag

let result = try await ZagBuilder()
    .provider("claude")
    .model("sonnet")
    .systemPrompt("You are a code reviewer")
    .json()
    .exec("Review this pull request")

print(result.output ?? "")`,
  Java: `import io.zag.ZagBuilder;

var result = new ZagBuilder()
    .provider("claude")
    .model("sonnet")
    .systemPrompt("You are a code reviewer")
    .json()
    .exec("Review this pull request");

System.out.println(result.getOutput());`,
  Kotlin: `import zag.ZagBuilder

val result = ZagBuilder()
    .provider("claude")
    .model("sonnet")
    .systemPrompt("You are a code reviewer")
    .json()
    .exec("Review this pull request")

println(result.output)`,
};

const bindings = sourceBindings.map((b) => ({
  lang: b.language,
  install: b.installCommand,
  code: codeExamples[b.language] ?? `// See bindings/${b.directory}/`,
}));

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

        <div className="mt-14 grid gap-6 md:grid-cols-2 lg:grid-cols-3">
          {bindings.map((b) => (
            <div key={b.lang} className="overflow-hidden rounded-xl border border-border bg-surface-alt">
              <div className="flex flex-col gap-1 sm:flex-row sm:items-center sm:justify-between border-b border-border px-5 py-3">
                <span className="text-sm font-semibold text-text-primary">{b.lang}</span>
                <code className="truncate text-xs text-text-dim">{b.install}</code>
              </div>
              <pre className="overflow-x-auto p-5 text-xs leading-relaxed text-text-secondary">
                <code>{b.code}</code>
              </pre>
            </div>
          ))}
        </div>

        <p className="mt-8 text-center text-sm text-text-dim">
          Also available as a native Rust crate:{" "}
          <code className="rounded bg-surface-alt px-1.5 py-0.5 text-accent">cargo add zag</code>
          {" "}— zero subprocess overhead.
        </p>
      </div>
    </section>
  );
}
