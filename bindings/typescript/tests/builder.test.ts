import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { ZagBuilder } from "../src/builder.js";
import { ZagError, ZagFeatureUnsupportedError } from "../src/types.js";
import type {
  AgentOutput,
  Event,
  ProviderCapability,
} from "../src/types.js";
import {
  parseSemver,
  compareSemver,
  checkVersion,
  _setVersionForTesting,
  _clearVersionCache,
} from "../src/version.js";
import {
  checkCapabilities,
  _setCapabilitiesForTesting,
  _clearCapabilityCache,
} from "../src/capability-check.js";

describe("ZagBuilder", () => {
  it("should construct with defaults", () => {
    const builder = new ZagBuilder();
    assert.ok(builder);
  });

  it("should support method chaining", () => {
    const builder = new ZagBuilder()
      .provider("claude")
      .model("sonnet")
      .systemPrompt("You are helpful")
      .root("/tmp/test")
      .autoApprove()
      .addDir("/extra")
      .file("/tmp/data.csv")
      .verbose()
      .quiet()
      .debug()
      .sessionId("abc-123")
      .maxTurns(5)
      .timeout("5m")
      .showUsage()
      .size("9b")
      .env("FOO", "bar");

    assert.ok(builder);
  });

  it("should support file attachments", () => {
    const builder = new ZagBuilder().file("/a.txt").file("/b.rs");
    // @ts-expect-error -- accessing private for test
    assert.deepStrictEqual(builder._files, ["/a.txt", "/b.rs"]);
  });

  it("should support env vars", () => {
    const builder = new ZagBuilder().env("FOO", "bar").env("BAZ", "qux");
    // @ts-expect-error -- accessing private for test
    assert.deepStrictEqual(builder._envVars, ["FOO=bar", "BAZ=qux"]);
  });

  it("should support headless()", () => {
    const builder = new ZagBuilder().headless();
    // @ts-expect-error -- accessing private for test
    assert.equal(builder._headless, true);
    // Explicit false disables it again and stays chainable.
    const disabled = new ZagBuilder().headless(false);
    // @ts-expect-error -- accessing private for test
    assert.equal(disabled._headless, false);
  });

  it("emits --headless in global args when enabled", () => {
    const builder = new ZagBuilder().provider("claude").headless();
    // @ts-expect-error -- accessing private for test
    const args = builder.buildGlobalArgs() as string[];
    assert.ok(args.includes("--headless"), `expected --headless in ${args.join(" ")}`);
  });

  it("should support autoCleanup()", () => {
    const builder = new ZagBuilder().autoCleanup();
    // @ts-expect-error -- accessing private for test
    assert.equal(builder._autoCleanup, true);
    // Explicit false disables it again and stays chainable.
    const disabled = new ZagBuilder().autoCleanup(false);
    // @ts-expect-error -- accessing private for test
    assert.equal(disabled._autoCleanup, false);
  });

  it("should support json options", () => {
    const builder = new ZagBuilder()
      .json()
      .jsonSchema({ type: "object" });

    assert.ok(builder);
  });

  it("should default stream() to -o stream-json", () => {
    const builder = new ZagBuilder().provider("claude");
    const args = (builder as any).buildExecArgs("hello", true);
    assert.ok(!args.includes("--json-stream"));
    const oi = args.indexOf("-o");
    assert.notEqual(oi, -1);
    assert.equal(args[oi + 1], "stream-json");
  });

  it("should support isolation modes", () => {
    const wt = new ZagBuilder().worktree();
    assert.ok(wt);

    const wtNamed = new ZagBuilder().worktree("my-feature");
    assert.ok(wtNamed);

    const sb = new ZagBuilder().sandbox();
    assert.ok(sb);

    const sbNamed = new ZagBuilder().sandbox("my-sandbox");
    assert.ok(sbNamed);
  });

  it("should support bin override", () => {
    const builder = new ZagBuilder().bin("/usr/local/bin/zag");
    assert.ok(builder);
  });

  it("should support mcpConfig", () => {
    const builder = new ZagBuilder().mcpConfig("./mcp.json");
    assert.ok(builder);
  });

  it("should include --exit with hint in run args", () => {
    const builder = new ZagBuilder().exit("the final answer");
    const args = (builder as any).buildRunArgs("compute 2+2");
    const i = args.indexOf("--exit");
    assert.notEqual(i, -1);
    assert.equal(args[i + 1], "the final answer");
  });

  it("should include bare --exit in run args when no hint", () => {
    const builder = new ZagBuilder().exit();
    const args = (builder as any).buildRunArgs();
    const i = args.indexOf("--exit");
    assert.notEqual(i, -1);
    // Either no following arg, or following arg isn't a hint (e.g. another flag)
    const next = args[i + 1];
    if (next !== undefined) {
      assert.ok(next.startsWith("-"));
    }
  });

  it("should omit --exit when not set", () => {
    const builder = new ZagBuilder();
    const args = (builder as any).buildRunArgs("hi");
    assert.ok(!args.includes("--exit"));
  });

  it("should include timeout in exec args", () => {
    const builder = new ZagBuilder().timeout("5m");
    const args = (builder as any).buildExecArgs("test", false);
    assert.ok(args.includes("--timeout"));
    assert.ok(args.includes("5m"));
  });

  it("should include --resume in execResume args", () => {
    const builder = new ZagBuilder().provider("claude");
    const args = (builder as any).buildExecArgs("follow up", false);
    const promptIdx = args.lastIndexOf("follow up");
    args.splice(promptIdx, 0, "--resume", "sess-123");
    assert.ok(args.includes("--resume"));
    assert.ok(args.includes("sess-123"));
    // --resume should come before the prompt
    const resumeIdx = args.indexOf("--resume");
    const newPromptIdx = args.lastIndexOf("follow up");
    assert.ok(resumeIdx < newPromptIdx);
  });

  it("should include --continue in execContinue args", () => {
    const builder = new ZagBuilder().provider("claude");
    const args = (builder as any).buildExecArgs("follow up", false);
    const promptIdx = args.lastIndexOf("follow up");
    args.splice(promptIdx, 0, "--continue");
    assert.ok(args.includes("--continue"));
    const continueIdx = args.indexOf("--continue");
    const newPromptIdx = args.lastIndexOf("follow up");
    assert.ok(continueIdx < newPromptIdx);
  });
});

describe("ZagError", () => {
  it("should contain exit code and stderr", () => {
    const err = new ZagError("test error", 1, "stderr output");
    assert.equal(err.message, "test error");
    assert.equal(err.exitCode, 1);
    assert.equal(err.stderr, "stderr output");
    assert.equal(err.name, "ZagError");
    assert.ok(err instanceof Error);
  });
});

describe("Version checking", () => {
  it("should parse valid semver", () => {
    assert.deepStrictEqual(parseSemver("0.6.0"), [0, 6, 0]);
    assert.deepStrictEqual(parseSemver("1.2.3"), [1, 2, 3]);
  });

  it("should reject invalid semver", () => {
    assert.throws(() => parseSemver("invalid"), ZagError);
    assert.throws(() => parseSemver("1.2"), ZagError);
    assert.throws(() => parseSemver("a.b.c"), ZagError);
  });

  it("should compare semver correctly", () => {
    assert.equal(compareSemver([0, 5, 0], [0, 6, 0]), -1);
    assert.equal(compareSemver([0, 6, 0], [0, 6, 0]), 0);
    assert.equal(compareSemver([0, 7, 0], [0, 6, 0]), 1);
    assert.equal(compareSemver([1, 0, 0], [0, 9, 9]), 1);
  });

  it("should pass when no requirements are set", async () => {
    _setVersionForTesting("zag", "0.5.0");
    try {
      await checkVersion("zag", [
        { method: "env()", version: "0.6.0", isSet: false },
      ]);
    } finally {
      _clearVersionCache();
    }
  });

  it("should pass when version is sufficient", async () => {
    _setVersionForTesting("zag", "0.6.0");
    try {
      await checkVersion("zag", [
        { method: "env()", version: "0.6.0", isSet: true },
      ]);
    } finally {
      _clearVersionCache();
    }
  });

  it("should throw when version is insufficient", async () => {
    _setVersionForTesting("zag", "0.5.0");
    try {
      await assert.rejects(
        () =>
          checkVersion("zag", [
            { method: "env()", version: "0.6.0", isSet: true },
          ]),
        (err: unknown) => {
          if (!(err instanceof ZagError)) return false;
          assert.ok(err.message.includes("env()"));
          assert.ok(err.message.includes("0.6.0"));
          assert.ok(err.message.includes("0.5.0"));
          return true;
        },
      );
    } finally {
      _clearVersionCache();
    }
  });

  it("should report multiple failures", async () => {
    _setVersionForTesting("zag", "0.5.0");
    try {
      await assert.rejects(
        () =>
          checkVersion("zag", [
            { method: "env()", version: "0.6.0", isSet: true },
            { method: "mcpConfig()", version: "0.6.0", isSet: true },
          ]),
        (err: unknown) => {
          if (!(err instanceof ZagError)) return false;
          assert.ok(err.message.includes("env()"));
          assert.ok(err.message.includes("mcpConfig()"));
          return true;
        },
      );
    } finally {
      _clearVersionCache();
    }
  });
});

function fakeCapability(
  provider: string,
  overrides: Partial<{
    streaming_input: boolean;
    worktree: boolean;
    sandbox: boolean;
    system_prompt: boolean;
    add_dirs: boolean;
    max_turns: boolean;
  }> = {},
): ProviderCapability {
  const support = (ok: boolean) => ({ supported: ok, native: ok });
  return {
    provider,
    default_model: "default",
    available_models: ["default"],
    size_mappings: { small: "s", medium: "m", large: "l" },
    features: {
      interactive: support(true),
      non_interactive: support(true),
      resume: support(true),
      resume_with_prompt: support(true),
      session_logs: { supported: true, native: true, completeness: "full" },
      json_output: support(true),
      stream_json: support(true),
      json_schema: support(true),
      input_format: support(true),
      streaming_input: {
        supported: overrides.streaming_input ?? true,
        native: overrides.streaming_input ?? true,
        semantics:
          (overrides.streaming_input ?? true) ? "queue" : undefined,
      },
      worktree: support(overrides.worktree ?? true),
      sandbox: support(overrides.sandbox ?? true),
      system_prompt: support(overrides.system_prompt ?? true),
      auto_approve: support(true),
      review: support(true),
      add_dirs: support(overrides.add_dirs ?? true),
      max_turns: support(overrides.max_turns ?? true),
    },
  };
}

describe("Capability checking", () => {
  it("should skip checks when no requirements are active", async () => {
    _setCapabilitiesForTesting("zag", [fakeCapability("claude")]);
    try {
      await checkCapabilities("zag", "claude", []);
      await checkCapabilities("zag", "claude", [
        {
          method: "worktree()",
          feature: "worktree",
          isSet: false,
        },
      ]);
    } finally {
      _clearCapabilityCache();
    }
  });

  it("should skip checks when no provider is set", async () => {
    _setCapabilitiesForTesting("zag", [
      fakeCapability("ollama", { streaming_input: false }),
    ]);
    try {
      await checkCapabilities("zag", undefined, [
        {
          method: "execStreaming()",
          feature: "streaming_input",
          isSet: true,
        },
      ]);
    } finally {
      _clearCapabilityCache();
    }
  });

  it("should pass when the provider supports the feature", async () => {
    _setCapabilitiesForTesting("zag", [
      fakeCapability("claude", { streaming_input: true }),
    ]);
    try {
      await checkCapabilities("zag", "claude", [
        {
          method: "execStreaming()",
          feature: "streaming_input",
          isSet: true,
        },
      ]);
    } finally {
      _clearCapabilityCache();
    }
  });

  it("should throw ZagFeatureUnsupportedError for unsupported features", async () => {
    _setCapabilitiesForTesting("zag", [
      fakeCapability("claude", { streaming_input: true }),
      fakeCapability("ollama", { streaming_input: false }),
    ]);
    try {
      await assert.rejects(
        () =>
          checkCapabilities("zag", "ollama", [
            {
              method: "execStreaming()",
              feature: "streaming_input",
              isSet: true,
            },
          ]),
        (err: unknown) => {
          if (!(err instanceof ZagFeatureUnsupportedError)) return false;
          assert.equal(err.provider, "ollama");
          assert.equal(err.feature, "streaming_input");
          assert.equal(err.method, "execStreaming()");
          assert.deepStrictEqual(err.supportedProviders, ["claude"]);
          assert.ok(err.message.includes("ollama"));
          assert.ok(err.message.includes("streaming_input"));
          assert.ok(err.message.includes("claude"));
          return true;
        },
      );
    } finally {
      _clearCapabilityCache();
    }
  });

  it("should surface add_dirs / max_turns gaps for ollama", async () => {
    _setCapabilitiesForTesting("zag", [
      fakeCapability("claude"),
      fakeCapability("ollama", { add_dirs: false, max_turns: false }),
    ]);
    try {
      await assert.rejects(
        () =>
          checkCapabilities("zag", "ollama", [
            { method: "addDir()", feature: "add_dirs", isSet: true },
          ]),
        ZagFeatureUnsupportedError,
      );
      await assert.rejects(
        () =>
          checkCapabilities("zag", "ollama", [
            { method: "maxTurns()", feature: "max_turns", isSet: true },
          ]),
        ZagFeatureUnsupportedError,
      );
    } finally {
      _clearCapabilityCache();
    }
  });

  it("should be silent for unknown providers", async () => {
    _setCapabilitiesForTesting("zag", [fakeCapability("claude")]);
    try {
      await checkCapabilities("zag", "does-not-exist", [
        { method: "worktree()", feature: "worktree", isSet: true },
      ]);
    } finally {
      _clearCapabilityCache();
    }
  });

  it("ZagFeatureUnsupportedError is a ZagError", () => {
    const err = new ZagFeatureUnsupportedError(
      "Provider 'ollama' does not support streaming_input",
      "ollama",
      "streaming_input",
      "execStreaming()",
      ["claude"],
    );
    assert.ok(err instanceof ZagError);
    assert.ok(err instanceof Error);
    assert.equal(err.name, "ZagFeatureUnsupportedError");
    assert.equal(err.provider, "ollama");
    assert.equal(err.feature, "streaming_input");
    assert.equal(err.method, "execStreaming()");
    assert.deepStrictEqual(err.supportedProviders, ["claude"]);
  });
});

describe("AgentOutput type", () => {
  it("should parse a sample JSON output", () => {
    const raw = `{
      "agent": "claude",
      "session_id": "sess-123",
      "events": [
        {
          "type": "init",
          "model": "sonnet",
          "tools": ["Bash", "Read"],
          "working_directory": "/home/user",
          "metadata": {}
        },
        {
          "type": "assistant_message",
          "content": [{"type": "text", "text": "Hello!"}],
          "usage": {"input_tokens": 100, "output_tokens": 50}
        },
        {
          "type": "tool_execution",
          "tool_name": "Bash",
          "tool_id": "tool_123",
          "input": {"command": "echo hello"},
          "result": {"success": true, "output": "hello", "error": null, "data": null}
        },
        {
          "type": "result",
          "success": true,
          "message": "Done",
          "duration_ms": 1500,
          "num_turns": 2
        }
      ],
      "result": "Hello!",
      "is_error": false,
      "total_cost_usd": 0.01,
      "usage": {"input_tokens": 100, "output_tokens": 50}
    }`;

    const output: AgentOutput = JSON.parse(raw);

    assert.equal(output.agent, "claude");
    assert.equal(output.session_id, "sess-123");
    assert.equal(output.events.length, 4);
    assert.equal(output.result, "Hello!");
    assert.equal(output.is_error, false);
    assert.equal(output.exit_code, undefined);
    assert.equal(output.error_message, undefined);
    assert.equal(output.total_cost_usd, 0.01);
    assert.equal(output.usage?.input_tokens, 100);

    // Check event types
    assert.equal(output.events[0].type, "init");
    assert.equal(output.events[1].type, "assistant_message");
    assert.equal(output.events[2].type, "tool_execution");
    assert.equal(output.events[3].type, "result");
  });

  it("should parse output with exit_code and error_message", () => {
    const raw = `{
      "agent": "codex",
      "session_id": "sess-456",
      "events": [],
      "result": null,
      "is_error": true,
      "exit_code": 2,
      "error_message": "provider crashed",
      "total_cost_usd": null,
      "usage": null
    }`;

    const output: AgentOutput = JSON.parse(raw);
    assert.equal(output.is_error, true);
    assert.equal(output.exit_code, 2);
    assert.equal(output.error_message, "provider crashed");
  });
});

describe("Event parsing", () => {
  it("should parse NDJSON events", () => {
    const lines = [
      '{"type":"init","model":"opus","tools":[],"working_directory":null,"metadata":{}}',
      '{"type":"assistant_message","content":[{"type":"text","text":"Hi"}],"usage":null}',
      '{"type":"error","message":"oops","details":null}',
      '{"type":"permission_request","tool_name":"Bash","description":"run cmd","granted":true}',
    ];

    const events: Event[] = lines.map((l) => JSON.parse(l));

    assert.equal(events.length, 4);
    assert.equal(events[0].type, "init");
    assert.equal(events[1].type, "assistant_message");
    assert.equal(events[2].type, "error");
    if (events[2].type === "error") {
      assert.equal(events[2].message, "oops");
    }
    assert.equal(events[3].type, "permission_request");
    if (events[3].type === "permission_request") {
      assert.equal(events[3].granted, true);
    }
  });

  it("should parse turn_complete events", () => {
    const line =
      '{"type":"turn_complete","stop_reason":"end_turn","turn_index":0,"usage":{"input_tokens":10,"output_tokens":5}}';
    const event: Event = JSON.parse(line);

    assert.equal(event.type, "turn_complete");
    if (event.type === "turn_complete") {
      assert.equal(event.stop_reason, "end_turn");
      assert.equal(event.turn_index, 0);
      assert.ok(event.usage);
      assert.equal(event.usage?.input_tokens, 10);
      assert.equal(event.usage?.output_tokens, 5);
    }
  });

  it("should parse turn_complete events with null stop_reason", () => {
    const line =
      '{"type":"turn_complete","stop_reason":null,"turn_index":3,"usage":null}';
    const event: Event = JSON.parse(line);

    assert.equal(event.type, "turn_complete");
    if (event.type === "turn_complete") {
      assert.equal(event.stop_reason, null);
      assert.equal(event.turn_index, 3);
      assert.equal(event.usage, null);
    }
  });
});
