import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { streamZag, streamWithInput } from "../src/process.js";
import { ZagError } from "../src/types.js";

// Regression tests for issue #106: streaming variants must include stderr
// in the thrown ZagError's `message`, not only on the `stderr` property.

describe("process error handling", () => {
  it("streamZag includes stderr in error message on non-zero exit", async () => {
    let caught: unknown;
    try {
      for await (const _event of streamZag("sh", [
        "-c",
        "echo boom 1>&2; exit 7",
      ])) {
        // no events expected
      }
    } catch (err) {
      caught = err;
    }

    assert.ok(caught instanceof ZagError, "expected ZagError");
    const err = caught as ZagError;
    assert.equal(err.exitCode, 7);
    assert.match(err.stderr, /boom/);
    assert.match(err.message, /exited with code 7/);
    assert.match(err.message, /boom/);
  });

  it("streamWithInput().wait() includes stderr in error message on non-zero exit", async () => {
    const session = streamWithInput("sh", ["-c", "echo boom 1>&2; exit 7"]);
    session.closeInput();

    // Drain events so the stdout pipe closes cleanly.
    for await (const _event of session.events()) {
      // no events expected
    }

    let caught: unknown;
    try {
      await session.wait();
    } catch (err) {
      caught = err;
    }

    assert.ok(caught instanceof ZagError, "expected ZagError");
    const err = caught as ZagError;
    assert.equal(err.exitCode, 7);
    assert.match(err.stderr, /boom/);
    assert.match(err.message, /exited with code 7/);
    assert.match(err.message, /boom/);
  });
});
