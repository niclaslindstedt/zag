import { describe, it } from "node:test";
import * as assert from "node:assert/strict";
import {
  listProviders,
  getCapability,
  getAllCapabilities,
  resolveModel,
} from "../src/index.js";
import type { ProviderCapability, ResolvedModel } from "../src/index.js";

// These tests require the zag binary to be built and available in PATH.
// Run with: npx tsx --test tests/discover.test.ts

describe("discover", () => {
  it("listProviders returns provider names", async () => {
    const providers = await listProviders();
    assert.ok(providers.length >= 5);
    assert.ok(providers.includes("claude"));
    assert.ok(providers.includes("codex"));
    assert.ok(providers.includes("gemini"));
    assert.ok(providers.includes("copilot"));
    assert.ok(providers.includes("ollama"));
  });

  it("getCapability returns single provider", async () => {
    const cap: ProviderCapability = await getCapability("claude");
    assert.equal(cap.provider, "claude");
    assert.ok(cap.available_models.length > 0);
    assert.ok(cap.features.interactive.supported);
  });

  it("getAllCapabilities returns all providers", async () => {
    const caps: ProviderCapability[] = await getAllCapabilities();
    assert.ok(caps.length >= 5);
    const names = caps.map((c) => c.provider);
    assert.ok(names.includes("claude"));
  });

  it("resolveModel resolves alias", async () => {
    const rm: ResolvedModel = await resolveModel("claude", "small");
    assert.equal(rm.input, "small");
    assert.equal(rm.resolved, "haiku");
    assert.equal(rm.is_alias, true);
    assert.equal(rm.provider, "claude");
  });

  it("resolveModel passes through non-alias", async () => {
    const rm: ResolvedModel = await resolveModel("claude", "opus");
    assert.equal(rm.input, "opus");
    assert.equal(rm.resolved, "opus");
    assert.equal(rm.is_alias, false);
  });
});
