import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { ZagBuilder } from "../src/builder.js";
import { ZagError } from "../src/types.js";
import type { AgentOutput, Event } from "../src/types.js";
import {
  parseSemver,
  compareSemver,
  checkVersion,
  _setVersionForTesting,
  _clearVersionCache,
} from "../src/version.js";

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

  it("should support json options", () => {
    const builder = new ZagBuilder()
      .json()
      .jsonSchema({ type: "object" })
      .jsonStream();

    assert.ok(builder);
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

  it("should include timeout in exec args", () => {
    const builder = new ZagBuilder().timeout("5m");
    const args = (builder as any).buildExecArgs("test", false);
    assert.ok(args.includes("--timeout"));
    assert.ok(args.includes("5m"));
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
});
