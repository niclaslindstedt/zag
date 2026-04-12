// AUTO-GENERATED from Rust source — do not edit manually.
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

export const version = "0.11.0";

export const providerCount = 5;

export const providers: ProviderData[] = [
  {
    "name": "claude",
    "displayName": "Claude",
    "org": "Anthropic",
    "defaultModel": "default",
    "availableModels": [
      "default",
      "sonnet",
      "opus",
      "haiku"
    ],
    "sizeMap": {
      "small": "haiku",
      "medium": "sonnet",
      "large": "default"
    },
    "features": {
      "interactive": {
        "supported": true,
        "native": true
      },
      "non_interactive": {
        "supported": true,
        "native": true
      },
      "resume": {
        "supported": true,
        "native": true
      },
      "json_output": {
        "supported": true,
        "native": true
      },
      "stream_json": {
        "supported": true,
        "native": true
      },
      "json_schema": {
        "supported": true,
        "native": true
      },
      "input_format": {
        "supported": true,
        "native": true
      },
      "worktree": {
        "supported": true,
        "native": false
      },
      "sandbox": {
        "supported": true,
        "native": false
      },
      "system_prompt": {
        "supported": true,
        "native": true
      },
      "auto_approve": {
        "supported": true,
        "native": true
      },
      "review": {
        "supported": false,
        "native": false
      },
      "add_dirs": {
        "supported": true,
        "native": true
      },
      "max_turns": {
        "supported": true,
        "native": true
      },
      "session_logs": {
        "supported": true,
        "native": true,
        "completeness": "full"
      },
      "streaming_input": {
        "supported": true,
        "native": true,
        "semantics": "queue"
      },
      "mcp_config": {
        "supported": true,
        "native": true
      }
    },
    "cardFeatures": [
      "Interactive",
      "Streaming",
      "Resume",
      "JSON Schema",
      "MCP"
    ]
  },
  {
    "name": "codex",
    "displayName": "Codex",
    "org": "OpenAI",
    "defaultModel": "gpt-5.4",
    "availableModels": [
      "gpt-5.4",
      "gpt-5.4-mini",
      "gpt-5.3-codex-spark",
      "gpt-5.3-codex",
      "gpt-5-codex",
      "gpt-5.2-codex",
      "gpt-5.2",
      "o4-mini",
      "gpt-5.1-codex-max",
      "gpt-5.1-codex-mini"
    ],
    "sizeMap": {
      "small": "gpt-5.4-mini",
      "medium": "gpt-5.3-codex",
      "large": "gpt-5.4"
    },
    "features": {
      "interactive": {
        "supported": true,
        "native": true
      },
      "non_interactive": {
        "supported": true,
        "native": true
      },
      "resume": {
        "supported": true,
        "native": true
      },
      "json_output": {
        "supported": true,
        "native": true
      },
      "stream_json": {
        "supported": false,
        "native": false
      },
      "json_schema": {
        "supported": true,
        "native": false
      },
      "input_format": {
        "supported": false,
        "native": false
      },
      "worktree": {
        "supported": true,
        "native": false
      },
      "sandbox": {
        "supported": true,
        "native": false
      },
      "system_prompt": {
        "supported": true,
        "native": false
      },
      "auto_approve": {
        "supported": true,
        "native": true
      },
      "review": {
        "supported": true,
        "native": true
      },
      "add_dirs": {
        "supported": true,
        "native": true
      },
      "max_turns": {
        "supported": true,
        "native": true
      },
      "session_logs": {
        "supported": true,
        "native": true,
        "completeness": "partial"
      },
      "streaming_input": {
        "supported": false,
        "native": false
      },
      "mcp_config": {
        "supported": false,
        "native": false
      }
    },
    "cardFeatures": [
      "Interactive",
      "Resume",
      "JSON Schema"
    ]
  },
  {
    "name": "gemini",
    "displayName": "Gemini",
    "org": "Google",
    "defaultModel": "auto",
    "availableModels": [
      "auto",
      "gemini-3.1-pro-preview",
      "gemini-3.1-flash-lite-preview",
      "gemini-3-pro-preview",
      "gemini-3-flash-preview",
      "gemini-2.5-pro",
      "gemini-2.5-flash",
      "gemini-2.5-flash-lite"
    ],
    "sizeMap": {
      "small": "gemini-3.1-flash-lite-preview",
      "medium": "gemini-2.5-flash",
      "large": "gemini-3.1-pro-preview"
    },
    "features": {
      "interactive": {
        "supported": true,
        "native": true
      },
      "non_interactive": {
        "supported": true,
        "native": true
      },
      "resume": {
        "supported": true,
        "native": true
      },
      "json_output": {
        "supported": true,
        "native": false
      },
      "stream_json": {
        "supported": false,
        "native": false
      },
      "json_schema": {
        "supported": true,
        "native": false
      },
      "input_format": {
        "supported": false,
        "native": false
      },
      "worktree": {
        "supported": true,
        "native": false
      },
      "sandbox": {
        "supported": true,
        "native": false
      },
      "system_prompt": {
        "supported": true,
        "native": false
      },
      "auto_approve": {
        "supported": true,
        "native": true
      },
      "review": {
        "supported": false,
        "native": false
      },
      "add_dirs": {
        "supported": true,
        "native": true
      },
      "max_turns": {
        "supported": true,
        "native": true
      },
      "session_logs": {
        "supported": true,
        "native": true,
        "completeness": "full"
      },
      "streaming_input": {
        "supported": false,
        "native": false
      },
      "mcp_config": {
        "supported": false,
        "native": false
      }
    },
    "cardFeatures": [
      "Interactive",
      "Resume",
      "JSON Schema"
    ]
  },
  {
    "name": "copilot",
    "displayName": "Copilot",
    "org": "GitHub",
    "defaultModel": "claude-sonnet-4.6",
    "availableModels": [
      "claude-sonnet-4.6",
      "claude-haiku-4.5",
      "claude-opus-4.6",
      "claude-sonnet-4.5",
      "claude-opus-4.5",
      "gpt-5.4",
      "gpt-5.4-mini",
      "gpt-5.3-codex",
      "gpt-5.2-codex",
      "gpt-5.2",
      "gpt-5.1-codex-max",
      "gpt-5.1-codex",
      "gpt-5.1",
      "gpt-5",
      "gpt-5.1-codex-mini",
      "gpt-5-mini",
      "gpt-4.1",
      "gemini-3.1-pro-preview",
      "gemini-3-pro-preview"
    ],
    "sizeMap": {
      "small": "claude-haiku-4.5",
      "medium": "claude-sonnet-4.6",
      "large": "claude-opus-4.6"
    },
    "features": {
      "interactive": {
        "supported": true,
        "native": true
      },
      "non_interactive": {
        "supported": true,
        "native": true
      },
      "resume": {
        "supported": true,
        "native": true
      },
      "json_output": {
        "supported": false,
        "native": false
      },
      "stream_json": {
        "supported": false,
        "native": false
      },
      "json_schema": {
        "supported": false,
        "native": false
      },
      "input_format": {
        "supported": false,
        "native": false
      },
      "worktree": {
        "supported": true,
        "native": false
      },
      "sandbox": {
        "supported": true,
        "native": false
      },
      "system_prompt": {
        "supported": true,
        "native": false
      },
      "auto_approve": {
        "supported": true,
        "native": true
      },
      "review": {
        "supported": false,
        "native": false
      },
      "add_dirs": {
        "supported": true,
        "native": true
      },
      "max_turns": {
        "supported": true,
        "native": true
      },
      "session_logs": {
        "supported": true,
        "native": true,
        "completeness": "full"
      },
      "streaming_input": {
        "supported": false,
        "native": false
      },
      "mcp_config": {
        "supported": false,
        "native": false
      }
    },
    "cardFeatures": [
      "Interactive",
      "Resume"
    ]
  },
  {
    "name": "ollama",
    "displayName": "Ollama",
    "org": "Local",
    "defaultModel": "qwen3.5",
    "availableModels": [
      "0.8b",
      "2b",
      "4b",
      "9b",
      "27b",
      "35b",
      "122b"
    ],
    "sizeMap": {
      "small": "2b",
      "medium": "9b",
      "large": "35b"
    },
    "features": {
      "interactive": {
        "supported": true,
        "native": true
      },
      "non_interactive": {
        "supported": true,
        "native": true
      },
      "resume": {
        "supported": false,
        "native": false
      },
      "json_output": {
        "supported": true,
        "native": false
      },
      "stream_json": {
        "supported": false,
        "native": false
      },
      "json_schema": {
        "supported": true,
        "native": false
      },
      "input_format": {
        "supported": false,
        "native": false
      },
      "worktree": {
        "supported": true,
        "native": false
      },
      "sandbox": {
        "supported": true,
        "native": false
      },
      "system_prompt": {
        "supported": true,
        "native": false
      },
      "auto_approve": {
        "supported": true,
        "native": true
      },
      "review": {
        "supported": false,
        "native": false
      },
      "add_dirs": {
        "supported": false,
        "native": false
      },
      "max_turns": {
        "supported": false,
        "native": false
      },
      "session_logs": {
        "supported": false,
        "native": true,
        "completeness": null
      },
      "streaming_input": {
        "supported": false,
        "native": false
      },
      "mcp_config": {
        "supported": false,
        "native": false
      }
    },
    "cardFeatures": [
      "Interactive",
      "JSON Schema",
      "No API key needed"
    ]
  }
];

export const commands: CommandData[] = [
  {
    "name": "run",
    "description": "Start an interactive session",
    "category": "core"
  },
  {
    "name": "exec",
    "description": "Run non-interactively (print output and exit)",
    "category": "core"
  },
  {
    "name": "review",
    "description": "Review code changes",
    "category": "core"
  },
  {
    "name": "plan",
    "description": "Generate an implementation plan",
    "category": "core"
  },
  {
    "name": "config",
    "description": "View or set configuration values",
    "category": "core"
  },
  {
    "name": "session",
    "description": "List and inspect sessions",
    "category": "session"
  },
  {
    "name": "capability",
    "description": "Show capability declarations for a provider",
    "category": "core"
  },
  {
    "name": "discover",
    "description": "Discover available providers, models, and capabilities",
    "category": "core"
  },
  {
    "name": "listen",
    "description": "Listen to a session's log events in real-time",
    "category": "session"
  },
  {
    "name": "man",
    "description": "Show manual pages for commands",
    "category": "core"
  },
  {
    "name": "skills",
    "description": "Manage provider-agnostic skills stored in ~/.zag/skills/",
    "category": "extension"
  },
  {
    "name": "mcp",
    "description": "Manage MCP (Model Context Protocol) servers across providers",
    "category": "extension"
  },
  {
    "name": "ps",
    "description": "List, inspect, and kill agent processes started by zag",
    "category": "process"
  },
  {
    "name": "search",
    "description": "Search through session logs",
    "category": "session"
  },
  {
    "name": "whoami",
    "description": "Show identity of the current zag session (for agent introspection)",
    "category": "process"
  },
  {
    "name": "input",
    "description": "Send a user message to a single running or resumable session",
    "category": "session"
  },
  {
    "name": "env",
    "description": "Export session environment variables for nested agent invocations",
    "category": "process"
  },
  {
    "name": "collect",
    "description": "Gather results from multiple sessions",
    "category": "orchestration"
  },
  {
    "name": "status",
    "description": "Show session status (running, idle, completed, failed, dead, unknown)",
    "category": "session"
  },
  {
    "name": "spawn",
    "description": "Launch an agent session in the background, print session ID, and exit",
    "category": "orchestration"
  },
  {
    "name": "wait",
    "description": "Block until one or more sessions complete",
    "category": "orchestration"
  },
  {
    "name": "pipe",
    "description": "Chain results from completed sessions into a new agent session",
    "category": "orchestration"
  },
  {
    "name": "events",
    "description": "Query structured events from session logs",
    "category": "session"
  },
  {
    "name": "cancel",
    "description": "Gracefully cancel one or more running sessions",
    "category": "orchestration"
  },
  {
    "name": "summary",
    "description": "Show a log-based summary of one or more sessions",
    "category": "session"
  },
  {
    "name": "watch",
    "description": "Watch session logs and execute a command on matching events",
    "category": "orchestration"
  },
  {
    "name": "subscribe",
    "description": "Subscribe to a multiplexed event stream from all active sessions",
    "category": "orchestration"
  },
  {
    "name": "broadcast",
    "description": "Send a message to all sessions in the current project (optionally filtered by tag)",
    "category": "orchestration"
  },
  {
    "name": "log",
    "description": "Append a custom event to a session's log",
    "category": "session"
  },
  {
    "name": "output",
    "description": "Extract the final result text from a session",
    "category": "session"
  },
  {
    "name": "retry",
    "description": "Re-run a failed session with the same configuration",
    "category": "orchestration"
  },
  {
    "name": "gc",
    "description": "Clean up old session data, logs, and process entries",
    "category": "process"
  },
  {
    "name": "serve",
    "description": "Start the zag HTTPS/WebSocket server for remote access",
    "category": "remote"
  },
  {
    "name": "connect",
    "description": "Connect to a remote zag server (all subsequent commands proxy through it)",
    "category": "remote"
  },
  {
    "name": "disconnect",
    "description": "Disconnect from the remote zag server",
    "category": "remote"
  },
  {
    "name": "user",
    "description": "Manage user accounts on the server",
    "category": "core"
  }
];

export const orchestrationCommands: CommandData[] = [
  {
    "name": "collect",
    "description": "Gather results from multiple sessions",
    "category": "orchestration"
  },
  {
    "name": "spawn",
    "description": "Launch an agent session in the background, print session ID, and exit",
    "category": "orchestration"
  },
  {
    "name": "wait",
    "description": "Block until one or more sessions complete",
    "category": "orchestration"
  },
  {
    "name": "pipe",
    "description": "Chain results from completed sessions into a new agent session",
    "category": "orchestration"
  },
  {
    "name": "cancel",
    "description": "Gracefully cancel one or more running sessions",
    "category": "orchestration"
  },
  {
    "name": "watch",
    "description": "Watch session logs and execute a command on matching events",
    "category": "orchestration"
  },
  {
    "name": "subscribe",
    "description": "Subscribe to a multiplexed event stream from all active sessions",
    "category": "orchestration"
  },
  {
    "name": "broadcast",
    "description": "Send a message to all sessions in the current project (optionally filtered by tag)",
    "category": "orchestration"
  },
  {
    "name": "retry",
    "description": "Re-run a failed session with the same configuration",
    "category": "orchestration"
  }
];

export const builderMethods: BuilderMethod[] = [
  {
    "name": "provider",
    "description": "file) to allow automatic downgrading."
  },
  {
    "name": "model",
    "description": "Set the model (e.g., \"sonnet\", \"opus\", \"small\", \"large\")."
  },
  {
    "name": "system_prompt",
    "description": "Set a system prompt to configure agent behavior."
  },
  {
    "name": "root",
    "description": "Set the root directory for the agent to operate in."
  },
  {
    "name": "auto_approve",
    "description": "Enable auto-approve mode (skip permission prompts)."
  },
  {
    "name": "add_dir",
    "description": "Add an additional directory for the agent to include."
  },
  {
    "name": "file",
    "description": "Attach a file to the prompt (text files ≤50 KB inlined, others referenced)."
  },
  {
    "name": "env",
    "description": "Add an environment variable for the agent subprocess."
  },
  {
    "name": "worktree",
    "description": "Enable worktree mode with an optional name."
  },
  {
    "name": "sandbox",
    "description": "Enable sandbox mode with an optional name."
  },
  {
    "name": "size",
    "description": "Set the Ollama parameter size (e.g., \"2b\", \"9b\", \"35b\")."
  },
  {
    "name": "json",
    "description": "Request JSON output from the agent."
  },
  {
    "name": "json_schema",
    "description": "Implies `json()`."
  },
  {
    "name": "session_id",
    "description": "Set a specific session ID (UUID)."
  },
  {
    "name": "output_format",
    "description": "Set the output format (e.g., \"text\", \"json\", \"json-pretty\", \"stream-json\")."
  },
  {
    "name": "input_format",
    "description": "for the full per-provider support matrix."
  },
  {
    "name": "replay_user_messages",
    "description": "callers never need to set it manually. No-op for non-Claude providers."
  },
  {
    "name": "include_partial_messages",
    "description": "[`exec_streaming`](Self::exec_streaming). No-op for non-Claude providers."
  },
  {
    "name": "verbose",
    "description": "Enable verbose output."
  },
  {
    "name": "quiet",
    "description": "Enable quiet mode (suppress all non-essential output)."
  },
  {
    "name": "show_usage",
    "description": "Show token usage statistics."
  },
  {
    "name": "max_turns",
    "description": "Set the maximum number of agentic turns."
  },
  {
    "name": "timeout",
    "description": "duration, it will be killed and an error returned."
  },
  {
    "name": "mcp_config",
    "description": "`docs/providers.md` for the full per-provider support matrix."
  },
  {
    "name": "on_progress",
    "description": "Set a custom progress handler for status reporting."
  }
];

export const bindings: BindingData[] = [
  {
    "language": "TypeScript",
    "directory": "typescript",
    "installCommand": "npm install @nlindstedt/zag-agent"
  },
  {
    "language": "Python",
    "directory": "python",
    "installCommand": "pip install zag-agent"
  },
  {
    "language": "C#",
    "directory": "csharp",
    "installCommand": "dotnet add package Zag"
  },
  {
    "language": "Swift",
    "directory": "swift",
    "installCommand": ".package(url: \"https://github.com/niclaslindstedt/zag\", from: \"0.11.0\")"
  },
  {
    "language": "Java",
    "directory": "java",
    "installCommand": "io.zag:zag:0.11.0"
  },
  {
    "language": "Kotlin",
    "directory": "kotlin",
    "installCommand": "(see bindings/kotlin/)"
  }
];

export const prereqs: Prereq[] = [
  {
    "name": "Claude",
    "cmd": "curl -fsSL https://claude.ai/install.sh | bash"
  },
  {
    "name": "Codex",
    "cmd": "npm i -g @openai/codex"
  },
  {
    "name": "Gemini",
    "cmd": "npm i -g @anthropic-ai/gemini-cli"
  },
  {
    "name": "Copilot",
    "cmd": "npm i -g @github/copilot"
  },
  {
    "name": "Ollama",
    "cmd": "# Download from ollama.com"
  }
];

export const configSections: ConfigSection[] = [
  {
    "name": "AgentModels",
    "description": "Agent-specific model configuration.",
    "fields": [
      {
        "name": "claude",
        "type": "String",
        "description": ""
      },
      {
        "name": "codex",
        "type": "String",
        "description": ""
      },
      {
        "name": "gemini",
        "type": "String",
        "description": ""
      },
      {
        "name": "copilot",
        "type": "String",
        "description": ""
      },
      {
        "name": "ollama",
        "type": "String",
        "description": ""
      }
    ]
  },
  {
    "name": "OllamaConfig",
    "description": "Ollama-specific configuration.",
    "fields": [
      {
        "name": "model",
        "type": "String",
        "description": "Default model name (default: \"qwen3.5\")"
      },
      {
        "name": "size",
        "type": "String",
        "description": "Default parameter size (default: \"9b\")"
      },
      {
        "name": "size_small",
        "type": "String",
        "description": "Parameter size for small alias"
      },
      {
        "name": "size_medium",
        "type": "String",
        "description": "Parameter size for medium alias"
      },
      {
        "name": "size_large",
        "type": "String",
        "description": "Parameter size for large alias"
      }
    ]
  },
  {
    "name": "Defaults",
    "description": "Default settings applied when not overridden by CLI flags.",
    "fields": [
      {
        "name": "auto_approve",
        "type": "bool",
        "description": "Auto-approve all actions (skip permission prompts)"
      },
      {
        "name": "model",
        "type": "String",
        "description": "Default model size for all agents (small, medium, large)"
      },
      {
        "name": "provider",
        "type": "String",
        "description": "Default provider (claude, codex, gemini, copilot)"
      },
      {
        "name": "max_turns",
        "type": "u32",
        "description": "Default maximum number of agentic turns"
      },
      {
        "name": "system_prompt",
        "type": "String",
        "description": "Default system prompt for all agents"
      }
    ]
  },
  {
    "name": "AutoConfig",
    "description": "Auto-selection configuration.",
    "fields": [
      {
        "name": "provider",
        "type": "String",
        "description": "Provider used for auto-selection (default: \"claude\")"
      },
      {
        "name": "model",
        "type": "String",
        "description": "Model used for auto-selection (default: \"sonnet\")"
      }
    ]
  },
  {
    "name": "ListenConfig",
    "description": "Listen command configuration.",
    "fields": [
      {
        "name": "format",
        "type": "String",
        "description": "Default output format: \"text\", \"json\", or \"rich-text\""
      },
      {
        "name": "timestamp_format",
        "type": "String",
        "description": "strftime-style format for timestamps (default: \"%H:%M:%S\")"
      }
    ]
  }
];
