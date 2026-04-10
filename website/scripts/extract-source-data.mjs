#!/usr/bin/env node
// Extract structured data from Rust source files and generate sourceData.ts.
//
// Usage: node scripts/extract-source-data.mjs
// Run from the website/ directory (or repo root — it auto-detects).
//
// This replaces hardcoded website data with values parsed from the actual
// Rust source, so the website stays in sync with the codebase.

import { readFileSync, writeFileSync, readdirSync, existsSync } from "fs";
import { execSync } from "child_process";
import { resolve, dirname, join } from "path";
import { fileURLToPath } from "url";

const __dirname = dirname(fileURLToPath(import.meta.url));

// Resolve repo root (works from website/ or repo root)
const REPO_ROOT = existsSync(resolve(__dirname, "../../zag-cli"))
  ? resolve(__dirname, "../..")
  : resolve(__dirname, "..");

// Resolve latest v* tag (empty string if none exist → fall back to working tree)
let LATEST_TAG = "";
try {
  LATEST_TAG = execSync("git tag -l 'v*' --sort=-version:refname", { cwd: REPO_ROOT, encoding: "utf-8" })
    .split("\n")
    .filter(Boolean)[0] || "";
} catch {
  // git not available or not a git repo
}

function read(relPath) {
  if (LATEST_TAG) {
    return execSync(`git show ${LATEST_TAG}:${relPath}`, { cwd: REPO_ROOT, encoding: "utf-8", stdio: ["pipe", "pipe", "pipe"] });
  }
  return readFileSync(join(REPO_ROOT, relPath), "utf-8");
}

// ---------------------------------------------------------------------------
// 1. Version (from zag-cli/Cargo.toml)
// ---------------------------------------------------------------------------

function extractVersion() {
  const cargo = read("zag-cli/Cargo.toml");
  const m = cargo.match(/^version\s*=\s*"([^"]+)"/m);
  if (!m) throw new Error("Could not extract version from zag-cli/Cargo.toml");
  return m[1];
}

// ---------------------------------------------------------------------------
// 2. Providers (from zag-agent/src/capability.rs + provider source files)
// ---------------------------------------------------------------------------

const PROVIDER_META = {
  claude:  { displayName: "Claude",  org: "Anthropic" },
  codex:   { displayName: "Codex",   org: "OpenAI" },
  gemini:  { displayName: "Gemini",  org: "Google" },
  copilot: { displayName: "Copilot", org: "GitHub" },
  ollama:  { displayName: "Ollama",  org: "Local" },
};

// Feature field name → human-readable label
const FEATURE_LABELS = {
  interactive:      "Interactive",
  non_interactive:  "Non-Interactive",
  resume:           "Resume",
  json_output:      "JSON Output",
  stream_json:      "Streaming",
  json_schema:      "JSON Schema",
  input_format:     "Input Format",
  streaming_input:  "Streaming Input",
  worktree:         "Worktree",
  sandbox:          "Sandbox",
  system_prompt:    "System Prompt",
  auto_approve:     "Auto-Approve",
  review:           "Review",
  add_dirs:         "Add Dirs",
  max_turns:        "Max Turns",
  mcp_config:       "MCP",
};

// Features to show on the website provider cards (user-facing highlights)
const CARD_FEATURES = [
  "interactive", "stream_json", "resume", "json_schema", "mcp_config",
];

function extractProviders() {
  const capSrc = read("zag-agent/src/capability.rs");
  const providers = [];

  for (const name of Object.keys(PROVIDER_META)) {
    const meta = PROVIDER_META[name];

    // Extract the provider block from get_capability()
    const blockRe = new RegExp(
      `"${name}"\\s*=>\\s*\\{[\\s\\S]*?Ok\\(ProviderCapability\\s*\\{([\\s\\S]*?)\\}\\)\\s*\\}`,
    );
    const blockMatch = capSrc.match(blockRe);
    if (!blockMatch) {
      console.warn(`Warning: could not parse capability block for ${name}`);
      continue;
    }
    const block = blockMatch[1];

    // default_model
    const defaultModelMatch = block.match(/default_model:\s*(\w+)::DEFAULT_MODEL/);
    let defaultModel = "";
    if (defaultModelMatch) {
      const constOwner = defaultModelMatch[1];
      // Find the actual value from the provider source
      defaultModel = extractConst(name, "DEFAULT_MODEL");
    } else {
      // Ollama style: direct reference
      const dm = block.match(/default_model:\s*(\w+)::DEFAULT_MODEL/);
      defaultModel = extractConst(name, "DEFAULT_MODEL");
    }

    // available_models
    let availableModels;
    if (name === "ollama") {
      availableModels = extractConstArray(name, "AVAILABLE_SIZES");
    } else {
      availableModels = extractConstArray(name, "AVAILABLE_MODELS");
    }

    // size_mappings
    const sizeMap = extractSizeMappings(block);

    // features — match "features: Features { ... },"
    const allFeatures = {};
    const featuresBlockMatch = block.match(/features:\s*Features\s*\{([\s\S]*?)\n\s{12,}\},/);
    if (featuresBlockMatch) {
      const featBlock = featuresBlockMatch[1];
      for (const [field, label] of Object.entries(FEATURE_LABELS)) {
        const re = new RegExp(`${field}:\\s*FeatureSupport::(\\w+)\\(\\)`);
        const m = featBlock.match(re);
        if (m) {
          allFeatures[field] = {
            supported: m[1] !== "unsupported",
            native: m[1] === "native",
          };
        }
      }
      // session_logs special case
      const slm = featBlock.match(/session_logs:\s*SessionLogSupport::(\w+)\(\)/);
      if (slm) {
        allFeatures["session_logs"] = {
          supported: slm[1] !== "unsupported",
          native: true,
          completeness: slm[1] === "unsupported" ? null : slm[1],
        };
      }
      // streaming_input special case: StreamingInputSupport with semantics
      const sim = featBlock.match(/streaming_input:\s*StreamingInputSupport::(\w+)\(\)/);
      if (sim) {
        const ctor = sim[1];
        // Constructor → semantics mapping: queue / interrupt / between_turns_only / unsupported
        const semanticsMap = {
          queue: "queue",
          interrupt: "interrupt",
          between_turns_only: "between-turns-only",
        };
        allFeatures["streaming_input"] = {
          supported: ctor !== "unsupported",
          native: ctor !== "unsupported",
          ...(semanticsMap[ctor] ? { semantics: semanticsMap[ctor] } : {}),
        };
      }
    }

    // MCP support: Claude has native mcp_config; others check if they support system_prompt
    // (MCP is only available for providers where zag can pass --mcp-config)
    // We derive it from the provider source. Claude has set_mcp_config; others don't.
    // For the card display, we check if `mcp_config` appears in the provider's CLI args
    if (!allFeatures["mcp_config"]) {
      allFeatures["mcp_config"] = { supported: hasMcpSupport(name), native: hasMcpSupport(name) };
    }

    // Card features: only supported features from the highlight set
    const cardFeatures = [];
    for (const key of CARD_FEATURES) {
      if (allFeatures[key]?.supported) {
        cardFeatures.push(FEATURE_LABELS[key] || key);
      }
    }
    // Special: Ollama "No API key needed"
    if (name === "ollama") {
      cardFeatures.push("No API key needed");
    }

    providers.push({
      name,
      displayName: meta.displayName,
      org: meta.org,
      defaultModel,
      availableModels,
      sizeMap,
      features: allFeatures,
      cardFeatures,
    });
  }

  return providers;
}

function extractConst(provider, constName) {
  const srcFile = providerSrcFile(provider);
  const src = read(srcFile);
  const m = src.match(new RegExp(`pub const ${constName}:\\s*&str\\s*=\\s*"([^"]+)"`));
  return m ? m[1] : "";
}

function extractConstArray(provider, constName) {
  const srcFile = providerSrcFile(provider);
  const src = read(srcFile);
  const re = new RegExp(`pub const ${constName}:[^=]*=\\s*&\\[([\\s\\S]*?)\\];`);
  const m = src.match(re);
  if (!m) return [];
  return [...m[1].matchAll(/"([^"]+)"/g)].map((x) => x[1]);
}

function extractSizeMappings(capBlock) {
  const sm = capBlock.match(/size_mappings:\s*SizeMappings\s*\{([\s\S]*?)\}/);
  if (!sm) return { small: "", medium: "", large: "" };
  const inner = sm[1];
  const get = (key) => {
    const m = inner.match(new RegExp(`${key}:\\s*(?:\\w+::model_for_size\\(ModelSize::\\w+\\)\\.to_string\\(\\)|"([^"]+)"\\.to_string\\(\\))`));
    if (m && m[1]) return m[1];
    // For references like Claude::model_for_size(...), we already extracted from provider src
    return "";
  };
  // Parse more robustly: extract all three fields
  const result = { small: "", medium: "", large: "" };
  for (const size of ["small", "medium", "large"]) {
    // Try literal string first
    const litMatch = inner.match(new RegExp(`${size}:\\s*"([^"]+)"\\.to_string`));
    if (litMatch) {
      result[size] = litMatch[1];
    }
    // Otherwise it's a model_for_size call — we parse from the provider source
  }
  return result;
}

function providerSrcFile(provider) {
  const files = {
    claude: "zag-agent/src/providers/claude/mod.rs",
    codex: "zag-agent/src/providers/codex.rs",
    gemini: "zag-agent/src/providers/gemini.rs",
    copilot: "zag-agent/src/providers/copilot.rs",
    ollama: "zag-agent/src/providers/ollama.rs",
  };
  return files[provider];
}

function hasMcpSupport(provider) {
  // Claude is the only provider with native MCP config support
  // But zag wraps MCP for other providers via system prompt injection
  // For the website, we show MCP as supported for providers where it works
  const src = read(providerSrcFile(provider));
  return src.includes("mcp_config") || src.includes("set_mcp_config");
}

// For providers using model_for_size calls, extract from source
function extractSizeMappingsFromSource(provider) {
  const src = read(providerSrcFile(provider));
  const result = { small: "", medium: "", large: "" };

  if (provider === "ollama") {
    // Ollama uses size_for_model_size
    const fn_match = src.match(/fn size_for_model_size[\s\S]*?match size\s*\{([\s\S]*?)\}/);
    if (fn_match) {
      for (const size of ["Small", "Medium", "Large"]) {
        const m = fn_match[1].match(new RegExp(`ModelSize::${size}\\s*=>\\s*"([^"]+)"`));
        if (m) result[size.toLowerCase()] = m[1];
      }
    }
    return result;
  }

  const fn_match = src.match(/fn model_for_size[\s\S]*?match size\s*\{([\s\S]*?)\}/);
  if (fn_match) {
    for (const size of ["Small", "Medium", "Large"]) {
      const m = fn_match[1].match(new RegExp(`ModelSize::${size}\\s*=>\\s*"([^"]+)"`));
      if (m) result[size.toLowerCase()] = m[1];
    }
  }
  return result;
}

// ---------------------------------------------------------------------------
// 3. CLI Commands (from zag-cli/src/cli.rs)
// ---------------------------------------------------------------------------

function extractCommands() {
  const src = read("zag-cli/src/cli.rs");

  // Match doc comments + enum variants in Commands enum
  const commandsBlock = src.match(/pub enum Commands\s*\{([\s\S]*?)^}/m);
  if (!commandsBlock) throw new Error("Could not find Commands enum in cli.rs");

  const commands = [];
  const re = /\/\/\/\s*(.+)\n\s+(\w+)\s*[\{,]/g;
  let m;
  while ((m = re.exec(commandsBlock[1])) !== null) {
    const desc = m[1].trim();
    const name = m[2];
    // Skip hidden commands
    if (desc.includes("Internal:") || desc.includes("#[command(hide")) continue;
    commands.push({ name: camelToKebab(name), description: desc });
  }
  return commands;
}

function camelToKebab(s) {
  return s.replace(/([a-z])([A-Z])/g, "$1-$2").toLowerCase();
}

// Categorize commands for the website
const COMMAND_CATEGORIES = {
  core: ["run", "exec", "review", "config", "capability", "man"],
  orchestration: [
    "spawn", "wait", "collect", "pipe", "cancel", "retry",
    "broadcast", "watch", "subscribe",
  ],
  session: [
    "session", "listen", "search", "input", "output",
    "status", "events", "summary", "log",
  ],
  process: ["ps", "gc", "env", "whoami"],
  extension: ["skills", "mcp"],
  remote: ["serve", "connect", "disconnect"],
};

function categorizeCommand(name) {
  for (const [cat, cmds] of Object.entries(COMMAND_CATEGORIES)) {
    if (cmds.includes(name)) return cat;
  }
  return "core";
}

// ---------------------------------------------------------------------------
// 4. Builder methods (from zag-agent/src/builder.rs)
// ---------------------------------------------------------------------------

function extractBuilderMethods() {
  const src = read("zag-agent/src/builder.rs");

  // Find the impl AgentBuilder block
  const implBlock = src.match(/impl AgentBuilder\s*\{([\s\S]*)/);
  if (!implBlock) return [];

  const methods = [];
  // Match pub fn declarations with doc comments
  const re = /\/\/\/\s*(.+)\n\s*pub fn (\w+)\(/g;
  let m;
  while ((m = re.exec(implBlock[1])) !== null) {
    const name = m[2];
    const desc = m[1].trim();
    // Skip internal helpers and constructor
    if (name === "new" || name === "default" || name.startsWith("resolve_") || name.startsWith("create_")) continue;
    methods.push({ name, description: desc });
  }
  return methods;
}

// ---------------------------------------------------------------------------
// 5. Bindings (from bindings/ directory)
// ---------------------------------------------------------------------------

function extractBindings() {
  // Ordered for website display (most popular languages first)
  const preferredOrder = ["typescript", "python", "csharp", "swift", "java", "kotlin"];
  let rawDirs;
  if (LATEST_TAG) {
    rawDirs = execSync(`git ls-tree --name-only ${LATEST_TAG} bindings/`, { cwd: REPO_ROOT, encoding: "utf-8" })
      .split("\n")
      .filter(Boolean)
      .map((p) => p.replace(/^bindings\//, ""))
      .filter((d) => !d.startsWith(".") && d !== "README.md" && d !== "rust");
  } else {
    const bindingsDir = join(REPO_ROOT, "bindings");
    rawDirs = readdirSync(bindingsDir).filter(
      (d) => !d.startsWith(".") && d !== "README.md" && d !== "rust",
    );
  }
  const dirs = [
    ...preferredOrder.filter((d) => rawDirs.includes(d)),
    ...rawDirs.filter((d) => !preferredOrder.includes(d)),
  ];

  const bindings = [];
  const meta = {
    typescript: { lang: "TypeScript", install: "", pkg: "package.json" },
    python:     { lang: "Python",     install: "", pkg: "pyproject.toml" },
    csharp:     { lang: "C#",         install: "", pkg: "src/Zag/Zag.csproj" },
    swift:      { lang: "Swift",      install: "", pkg: "Package.swift" },
    java:       { lang: "Java",       install: "", pkg: "pom.xml" },
    kotlin:     { lang: "Kotlin",     install: "", pkg: "pom.xml" },
  };

  for (const dir of dirs) {
    const info = meta[dir];
    if (!info) continue;

    // Extract install command from package metadata
    const pkgPath = join("bindings", dir, info.pkg);
    try {
      const pkgSrc = read(pkgPath);
      info.install = extractInstallCommand(dir, pkgSrc);
    } catch {
      info.install = `(see bindings/${dir}/)`;
    }

    bindings.push({
      language: info.lang,
      directory: dir,
      installCommand: info.install,
    });
  }

  return bindings;
}

function extractInstallCommand(dir, src) {
  switch (dir) {
    case "typescript": {
      const m = src.match(/"name":\s*"([^"]+)"/);
      return m ? `npm install ${m[1]}` : "";
    }
    case "python": {
      const m = src.match(/^name\s*=\s*"([^"]+)"/m);
      return m ? `pip install ${m[1]}` : "";
    }
    case "csharp": {
      const m = src.match(/<PackageId>([^<]+)<\/PackageId>/);
      return m ? `dotnet add package ${m[1]}` : "";
    }
    case "swift": {
      return `.package(url: "https://github.com/niclaslindstedt/zag", from: "${extractVersion()}")`;
    }
    case "java": {
      const gm = src.match(/<groupId>([^<]+)<\/groupId>/);
      const am = src.match(/<artifactId>([^<]+)<\/artifactId>/);
      if (gm && am) return `${gm[1]}:${am[1]}:${extractVersion()}`;
      return "";
    }
    case "kotlin": {
      // Gradle build — try group from build.gradle.kts
      const gm = src.match(/^group\s*=\s*"([^"]+)"/m);
      if (gm) return `${gm[1]}:zag:${extractVersion()}`;
      return `io.zag:zag:${extractVersion()}`;
    }
    default:
      return "";
  }
}

// ---------------------------------------------------------------------------
// 6. Agent prerequisites (install commands for upstream CLIs)
// ---------------------------------------------------------------------------

function extractPrereqs() {
  // These are external tools — not extractable from our Rust source.
  // We maintain them here, aligned with scripts/check-provider-status.sh.
  // The install commands for upstream CLIs are not in our codebase since
  // they're published by other organizations.
  return [
    { name: "Claude",  cmd: "curl -fsSL https://claude.ai/install.sh | bash" },
    { name: "Codex",   cmd: "npm i -g @openai/codex" },
    { name: "Gemini",  cmd: "npm i -g @anthropic-ai/gemini-cli" },
    { name: "Copilot", cmd: "npm i -g @github/copilot" },
    { name: "Ollama",  cmd: "# Download from ollama.com" },
  ];
}

// ---------------------------------------------------------------------------
// 7. Config keys (from zag-agent/src/config.rs)
// ---------------------------------------------------------------------------

function extractConfigSections() {
  const src = read("zag-agent/src/config.rs");
  const sections = [];

  // Match struct definitions with doc comments and fields
  const structRe = /\/\/\/\s*(.+)\n(?:.*\n)*?pub struct (\w+)\s*\{([\s\S]*?)\}/g;
  let m;
  while ((m = structRe.exec(src)) !== null) {
    const name = m[2];
    if (name === "Config") continue; // Skip the root wrapper
    const fields = [];
    const fieldRe = /(?:\/\/\/\s*(.+)\n\s*)?pub (\w+):\s*Option<([^>]+)>/g;
    let fm;
    while ((fm = fieldRe.exec(m[3])) !== null) {
      fields.push({
        name: fm[2],
        type: fm[3].trim(),
        description: fm[1]?.trim() || "",
      });
    }
    if (fields.length > 0) {
      sections.push({ name, description: m[1].trim(), fields });
    }
  }
  return sections;
}

// ---------------------------------------------------------------------------
// Generate output
// ---------------------------------------------------------------------------

function generate() {
  const version = extractVersion();
  const providers = extractProviders();
  const allCommands = extractCommands();
  const builderMethods = extractBuilderMethods();
  const bindings = extractBindings();
  const prereqs = extractPrereqs();
  const configSections = extractConfigSections();

  // Fill in size mappings from source for providers that use model_for_size calls
  for (const p of providers) {
    const srcSizes = extractSizeMappingsFromSource(p.name);
    if (!p.sizeMap.small && srcSizes.small) p.sizeMap.small = srcSizes.small;
    if (!p.sizeMap.medium && srcSizes.medium) p.sizeMap.medium = srcSizes.medium;
    if (!p.sizeMap.large && srcSizes.large) p.sizeMap.large = srcSizes.large;
  }

  // Categorize commands
  const commands = allCommands.map((c) => ({
    ...c,
    category: categorizeCommand(c.name),
  }));

  const orchestrationCommands = commands.filter((c) => c.category === "orchestration");

  const output = `// AUTO-GENERATED from Rust source — do not edit manually.
// To regenerate: npm run extract (from website/) or make extract-website-data
// Source files:
//   - zag-cli/Cargo.toml (version)
//   - zag-agent/src/capability.rs (providers, features)
//   - zag-agent/src/providers/*  (models, size mappings)
//   - zag-cli/src/cli.rs (commands)
//   - zag-agent/src/builder.rs (builder methods)
//   - zag-agent/src/config.rs (config sections)
//   - bindings/*/  (language bindings)

// --- Types ---

export interface ProviderFeatureSupport {
  supported: boolean;
  native: boolean;
}

export interface ProviderData {
  name: string;
  displayName: string;
  org: string;
  defaultModel: string;
  availableModels: string[];
  sizeMap: { small: string; medium: string; large: string };
  features: Record<string, ProviderFeatureSupport>;
  /** Human-readable feature labels for provider cards */
  cardFeatures: string[];
}

export interface CommandData {
  name: string;
  description: string;
  category: "core" | "orchestration" | "session" | "process" | "extension" | "remote";
}

export interface BuilderMethod {
  name: string;
  description: string;
}

export interface BindingData {
  language: string;
  directory: string;
  installCommand: string;
}

export interface Prereq {
  name: string;
  cmd: string;
}

export interface ConfigField {
  name: string;
  type: string;
  description: string;
}

export interface ConfigSection {
  name: string;
  description: string;
  fields: ConfigField[];
}

// --- Data ---

export const version = ${JSON.stringify(version)};

export const providerCount = ${providers.length};

export const providers: ProviderData[] = ${JSON.stringify(providers, null, 2)};

export const commands: CommandData[] = ${JSON.stringify(commands, null, 2)};

export const orchestrationCommands: CommandData[] = ${JSON.stringify(orchestrationCommands, null, 2)};

export const builderMethods: BuilderMethod[] = ${JSON.stringify(builderMethods, null, 2)};

export const bindings: BindingData[] = ${JSON.stringify(bindings, null, 2)};

export const prereqs: Prereq[] = ${JSON.stringify(prereqs, null, 2)};

export const configSections: ConfigSection[] = ${JSON.stringify(configSections, null, 2)};
`;

  const outPath = join(__dirname, "../src/data/sourceData.ts");
  writeFileSync(outPath, output, "utf-8");
  console.log(`Generated ${outPath}`);
  console.log(`  Source: ${LATEST_TAG || "working tree"}`);
  console.log(`  Version: ${version}`);
  console.log(`  Providers: ${providers.length} (${providers.map((p) => p.name).join(", ")})`);
  console.log(`  Commands: ${commands.length}`);
  console.log(`  Orchestration commands: ${orchestrationCommands.length}`);
  console.log(`  Builder methods: ${builderMethods.length}`);
  console.log(`  Bindings: ${bindings.length} (${bindings.map((b) => b.language).join(", ")})`);
  console.log(`  Config sections: ${configSections.length}`);
}

generate();
