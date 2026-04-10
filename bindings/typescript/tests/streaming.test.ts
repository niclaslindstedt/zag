import { describe, it } from "node:test";
import assert from "node:assert/strict";
import {
  streamWithInput,
  parseTimeoutMs,
  _getLiveSessionCount,
} from "../src/process.js";
import { ZagError } from "../src/types.js";

/**
 * These tests exercise the lifecycle helpers on `StreamingSession` —
 * `close()` and the opt-in `autoCleanup` registry — without needing a real
 * `zag` binary. We point `streamWithInput` at `node` itself and pass a short
 * inline script via `-e`, which gives us full control over how the fake
 * child handles stdin-close, SIGTERM, etc.
 */

const NODE = process.execPath;

describe("parseTimeoutMs", () => {
  it("accepts numeric milliseconds", () => {
    assert.equal(parseTimeoutMs(5000), 5000);
    assert.equal(parseTimeoutMs(0), 0);
  });

  it("parses humantime strings", () => {
    assert.equal(parseTimeoutMs("500ms"), 500);
    assert.equal(parseTimeoutMs("5s"), 5_000);
    assert.equal(parseTimeoutMs("1m"), 60_000);
    assert.equal(parseTimeoutMs("1h"), 3_600_000);
    assert.equal(parseTimeoutMs("2.5s"), 2_500);
    assert.equal(parseTimeoutMs("  250ms  "), 250);
    assert.equal(parseTimeoutMs("10S"), 10_000); // case-insensitive
  });

  it("throws on invalid input", () => {
    assert.throws(() => parseTimeoutMs("5"), ZagError);
    assert.throws(() => parseTimeoutMs("foo"), ZagError);
    assert.throws(() => parseTimeoutMs("5 seconds"), ZagError);
    assert.throws(() => parseTimeoutMs(-1), ZagError);
    assert.throws(() => parseTimeoutMs(Number.NaN), ZagError);
  });
});

describe("StreamingSession.close()", () => {
  it("resolves cleanly when the child exits on stdin close", async () => {
    // Child exits as soon as its stdin ends.
    const session = streamWithInput(NODE, [
      "-e",
      "process.stdin.on('end', () => process.exit(0)); process.stdin.resume();",
    ]);

    const start = Date.now();
    await session.close({ timeout: "5s" });
    const elapsed = Date.now() - start;

    assert.equal(session.isRunning, false);
    // Should be well under the 5s budget — child exits on EOF immediately.
    assert.ok(
      elapsed < 2_000,
      `expected fast graceful exit, took ${elapsed}ms`,
    );
  });

  it("SIGTERMs a child that ignores stdin close", async () => {
    // Child swallows stdin-end and just spins forever. SIGTERM will kill it
    // with Node's default handler.
    const session = streamWithInput(NODE, [
      "-e",
      "process.stdin.on('end', () => {}); setInterval(() => {}, 1_000);",
    ]);

    const start = Date.now();
    await session.close({ timeout: 400 });
    const elapsed = Date.now() - start;

    assert.equal(session.isRunning, false);
    // Graceful half (200ms) expires, SIGTERM fires, child exits. Should be
    // comfortably under ~1s on any reasonable CI.
    assert.ok(elapsed < 2_000, `close took ${elapsed}ms`);
  });

  it("SIGKILLs a child that traps SIGTERM", async () => {
    // Child installs an empty SIGTERM handler so the default-terminate
    // behavior is suppressed. Only SIGKILL will stop it.
    const session = streamWithInput(NODE, [
      "-e",
      "process.on('SIGTERM', () => {}); process.stdin.on('end', () => {}); setInterval(() => {}, 1_000);",
    ]);

    const start = Date.now();
    await session.close({ timeout: 300 });
    const elapsed = Date.now() - start;

    assert.equal(session.isRunning, false);
    assert.ok(elapsed < 2_000, `close took ${elapsed}ms`);
  });

  it("is idempotent — concurrent close() calls share one promise", async () => {
    const session = streamWithInput(NODE, [
      "-e",
      "process.stdin.on('end', () => process.exit(0)); process.stdin.resume();",
    ]);

    const p1 = session.close({ timeout: 500 });
    const p2 = session.close({ timeout: 500 });
    assert.equal(p1, p2, "expected same promise for concurrent close() calls");
    await p1;
    // And a call after the child has exited resolves immediately.
    await session.close();
    assert.equal(session.isRunning, false);
  });

  it("resolves immediately if the child is already gone", async () => {
    const session = streamWithInput(NODE, ["-e", "process.exit(0);"]);
    // Give the exit event a chance to fire.
    await new Promise((r) => setTimeout(r, 100));
    const start = Date.now();
    await session.close({ timeout: "5s" });
    assert.ok(Date.now() - start < 200);
  });
});

describe("autoCleanup registry", () => {
  it("tracks sessions created with autoCleanup and clears them on exit", async () => {
    const before = _getLiveSessionCount();
    const session = streamWithInput(
      NODE,
      [
        "-e",
        "process.stdin.on('end', () => process.exit(0)); process.stdin.resume();",
      ],
      { autoCleanup: true },
    );

    assert.equal(
      _getLiveSessionCount(),
      before + 1,
      "session should be registered",
    );

    await session.close({ timeout: "2s" });
    // The `exit` listener that removes the child from the registry may fire
    // on a microtask after close() resolves — wait one tick.
    await new Promise((r) => setTimeout(r, 50));

    assert.equal(
      _getLiveSessionCount(),
      before,
      "session should be removed from registry after exit",
    );
  });

  it("does not track sessions when autoCleanup is not set", async () => {
    const before = _getLiveSessionCount();
    const session = streamWithInput(NODE, [
      "-e",
      "process.stdin.on('end', () => process.exit(0)); process.stdin.resume();",
    ]);

    assert.equal(_getLiveSessionCount(), before);

    await session.close({ timeout: "2s" });
  });
});
