import { describe, it } from "node:test";
import assert from "node:assert/strict";
import {
  checkCapability,
  _setCapabilityForTesting,
  _setAllCapabilitiesForTesting,
  _clearCapabilityCache,
} from "../src/capability.js";
import { ZagBuilder } from "../src/builder.js";
import {
  ZagError,
  ZagFeatureUnsupportedError,
  type ProviderCapability,
} from "../src/types.js";
import {
  _setVersionForTesting,
  _clearVersionCache,
} from "../src/version.js";

/** Build a synthetic `ProviderCapability` with every feature marked supported. */
function fakeCap(
  provider: string,
  overrides: Partial<Record<string, boolean>> = {},
): ProviderCapability {
  const mk = (supported: boolean) => ({ supported, native: supported });
  const features = {
    interactive: mk(true),
    non_interactive: mk(true),
    resume: mk(true),
    resume_with_prompt: mk(true),
    session_logs: { supported: true, native: true, completeness: "full" },
    json_output: mk(true),
    stream_json: mk(true),
    json_schema: mk(true),
    input_format: mk(true),
    streaming_input: mk(true),
    worktree: mk(true),
    sandbox: mk(true),
    system_prompt: mk(true),
    auto_approve: mk(true),
    review: mk(true),
    add_dirs: mk(true),
    max_turns: mk(true),
  } as unknown as ProviderCapability["features"];
  for (const [key, value] of Object.entries(overrides)) {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (features as any)[key] = mk(value ?? false);
  }
  return {
    provider,
    default_model: "fake",
    available_models: [],
    size_mappings: { small: "s", medium: "m", large: "l" },
    features,
  };
}

/** Prime caches so the preflight never actually spawns a subprocess. */
function primeCaches(bin: string): void {
  _setVersionForTesting(bin, "0.6.0");
  _setAllCapabilitiesForTesting(bin, [
    fakeCap("claude"),
    fakeCap("codex", { streaming_input: false }),
    fakeCap("gemini", { streaming_input: false }),
    fakeCap("copilot", { streaming_input: false }),
    fakeCap("ollama", { streaming_input: false, add_dirs: false }),
  ]);
}

function resetCaches(): void {
  _clearVersionCache();
  _clearCapabilityCache();
}

describe("checkCapability", () => {
  it("returns silently when no requirements are active", async () => {
    primeCaches("zag");
    try {
      await checkCapability("zag", "ollama", [
        { method: "worktree()", feature: "worktree", isSet: false },
      ]);
    } finally {
      resetCaches();
    }
  });

  it("returns silently when the provider is undefined (auto)", async () => {
    primeCaches("zag");
    try {
      await checkCapability("zag", undefined, [
        { method: "addDir()", feature: "add_dirs", isSet: true },
      ]);
    } finally {
      resetCaches();
    }
  });

  it("returns silently for the 'mock' provider", async () => {
    primeCaches("zag");
    try {
      await checkCapability("zag", "mock", [
        { method: "execStreaming()", feature: "streaming_input", isSet: true },
      ]);
    } finally {
      resetCaches();
    }
  });

  it("passes when the provider supports the feature", async () => {
    primeCaches("zag");
    try {
      await checkCapability("zag", "claude", [
        { method: "execStreaming()", feature: "streaming_input", isSet: true },
        { method: "addDir()", feature: "add_dirs", isSet: true },
      ]);
    } finally {
      resetCaches();
    }
  });

  it("throws ZagFeatureUnsupportedError with supported providers", async () => {
    primeCaches("zag");
    try {
      await assert.rejects(
        () =>
          checkCapability("zag", "ollama", [
            {
              method: "execStreaming()",
              feature: "streaming_input",
              isSet: true,
            },
          ]),
        (err: unknown) => {
          if (!(err instanceof ZagFeatureUnsupportedError)) return false;
          assert.equal(err.method, "execStreaming()");
          assert.equal(err.feature, "streaming_input");
          assert.equal(err.provider, "ollama");
          assert.deepStrictEqual(err.supportedProviders, ["claude"]);
          assert.ok(err instanceof ZagError);
          assert.ok(err instanceof Error);
          assert.equal(err.name, "ZagFeatureUnsupportedError");
          assert.ok(err.message.includes("execStreaming()"));
          assert.ok(err.message.includes("ollama"));
          assert.ok(err.message.includes("claude"));
          return true;
        },
      );
    } finally {
      resetCaches();
    }
  });

  it("throws on the first unsupported feature in the list", async () => {
    primeCaches("zag");
    try {
      await assert.rejects(
        () =>
          checkCapability("zag", "ollama", [
            { method: "addDir()", feature: "add_dirs", isSet: true },
            {
              method: "execStreaming()",
              feature: "streaming_input",
              isSet: true,
            },
          ]),
        (err: unknown) => {
          if (!(err instanceof ZagFeatureUnsupportedError)) return false;
          assert.equal(err.method, "addDir()");
          assert.equal(err.feature, "add_dirs");
          return true;
        },
      );
    } finally {
      resetCaches();
    }
  });
});

describe("ZagBuilder capability preflight", () => {
  it("rejects addDir() on ollama with a typed error", async () => {
    primeCaches("zag");
    try {
      await assert.rejects(
        () =>
          new ZagBuilder()
            .bin("zag")
            .provider("ollama")
            .addDir("/extra")
            .exec("hi"),
        (err: unknown) => {
          if (!(err instanceof ZagFeatureUnsupportedError)) return false;
          assert.equal(err.method, "addDir()");
          assert.equal(err.feature, "add_dirs");
          assert.equal(err.provider, "ollama");
          return true;
        },
      );
    } finally {
      resetCaches();
    }
  });

  it("rejects execStreaming() on gemini with a typed error", async () => {
    primeCaches("zag");
    try {
      await assert.rejects(
        () =>
          new ZagBuilder()
            .bin("zag")
            .provider("gemini")
            .execStreaming("hi"),
        (err: unknown) => {
          if (!(err instanceof ZagFeatureUnsupportedError)) return false;
          assert.equal(err.method, "execStreaming()");
          assert.equal(err.feature, "streaming_input");
          assert.equal(err.provider, "gemini");
          assert.ok(err.supportedProviders.includes("claude"));
          return true;
        },
      );
    } finally {
      resetCaches();
    }
  });

  it("skips the check when no provider is set", async () => {
    // Without a provider, the preflight must not throw from capability data;
    // we don't set up a real subprocess, so reaching past preflight would
    // produce a different kind of error. Use a builder field that is *not*
    // set to keep the requirements list empty.
    primeCaches("zag");
    try {
      await checkCapability("zag", undefined, []);
    } finally {
      resetCaches();
    }
  });
});

describe("ZagFeatureUnsupportedError", () => {
  it("extends ZagError and formats a clear message", () => {
    const err = new ZagFeatureUnsupportedError(
      "execStreaming()",
      "streaming_input",
      "ollama",
      ["claude"],
    );
    assert.ok(err instanceof ZagError);
    assert.ok(err instanceof Error);
    assert.equal(err.name, "ZagFeatureUnsupportedError");
    assert.equal(err.method, "execStreaming()");
    assert.equal(err.feature, "streaming_input");
    assert.equal(err.provider, "ollama");
    assert.deepStrictEqual(err.supportedProviders, ["claude"]);
    assert.ok(err.message.includes("execStreaming()"));
    assert.ok(err.message.includes("ollama"));
    assert.ok(err.message.includes("streaming_input"));
    assert.ok(err.message.includes("claude"));
  });

  it("handles an empty supported-providers list", () => {
    const err = new ZagFeatureUnsupportedError(
      "foo()",
      "bar",
      "baz",
      [],
    );
    assert.ok(err.message.includes("(none)"));
  });
});
