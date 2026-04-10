import { spawn, type ChildProcess } from "node:child_process";
import { createInterface } from "node:readline";
import type { AgentOutput, Event } from "./types.js";
import { ZagError } from "./types.js";

/**
 * Parse a timeout value into milliseconds.
 *
 * Accepts a number (already in ms) or a humantime string with a unit
 * suffix: `ms`, `s`, `m`, or `h` (e.g. `"500ms"`, `"5s"`, `"1m"`, `"1h"`).
 * Throws `ZagError` on unparseable input.
 */
export function parseTimeoutMs(input: number | string): number {
  if (typeof input === "number") {
    if (!Number.isFinite(input) || input < 0) {
      throw new ZagError(`Invalid timeout: ${input}`, null, "");
    }
    return Math.floor(input);
  }
  const match = /^\s*(\d+(?:\.\d+)?)\s*(ms|s|m|h)\s*$/i.exec(input);
  if (!match) {
    throw new ZagError(
      `Invalid timeout string: "${input}" (expected e.g. "500ms", "5s", "1m", "1h")`,
      null,
      "",
    );
  }
  const value = Number.parseFloat(match[1]);
  const unit = match[2].toLowerCase();
  const multipliers: Record<string, number> = {
    ms: 1,
    s: 1_000,
    m: 60_000,
    h: 3_600_000,
  };
  return Math.floor(value * multipliers[unit]);
}

// ---------------------------------------------------------------------------
// Module-level orphan-cleanup registry.
//
// When a caller opts in with `ZagBuilder.autoCleanup()`, the resulting
// StreamingSession is added to `liveSessions` and process-wide shutdown
// handlers are installed (idempotently) to SIGTERM every tracked child if
// the parent Node process dies unexpectedly.
// ---------------------------------------------------------------------------

const liveSessions = new Set<ChildProcess>();
let handlersInstalled = false;

function killAllLiveSessions(): void {
  for (const child of liveSessions) {
    try {
      child.kill("SIGTERM");
    } catch {
      // Child may already be dead or unreachable; ignore.
    }
  }
}

function ensureCleanupHandlersInstalled(): void {
  if (handlersInstalled) return;
  handlersInstalled = true;

  // `exit` is synchronous-only in Node — we can only fire-and-forget SIGTERM.
  process.on("exit", killAllLiveSessions);

  const signals: Array<{ name: NodeJS.Signals; code: number }> = [
    { name: "SIGINT", code: 2 },
    { name: "SIGTERM", code: 15 },
    { name: "SIGHUP", code: 1 },
  ];
  for (const { name, code } of signals) {
    process.on(name, () => {
      killAllLiveSessions();
      // Preserve standard shell exit semantics (128 + signal number).
      process.exit(128 + code);
    });
  }

  process.on("uncaughtException", (err) => {
    killAllLiveSessions();
    // Re-throw so Node's default crash behavior still kicks in.
    throw err;
  });
}

/** Test-only: current number of sessions tracked for auto-cleanup. */
export function _getLiveSessionCount(): number {
  return liveSessions.size;
}

/** Default binary name — override with `ZAG_BIN` env var or builder option. */
export function defaultBin(): string {
  return process.env.ZAG_BIN ?? "zag";
}

/**
 * Run `zag` and collect stdout as a parsed `AgentOutput`.
 * Throws `ZagError` on non-zero exit.
 */
export async function execZag(
  bin: string,
  args: string[],
): Promise<AgentOutput> {
  return new Promise((resolve, reject) => {
    const child = spawn(bin, args, { stdio: ["ignore", "pipe", "pipe"] });

    const stdoutChunks: Buffer[] = [];
    const stderrChunks: Buffer[] = [];

    child.stdout.on("data", (chunk: Buffer) => stdoutChunks.push(chunk));
    child.stderr.on("data", (chunk: Buffer) => stderrChunks.push(chunk));

    child.on("error", (err) => {
      reject(
        new ZagError(
          `Failed to spawn '${bin}': ${err.message}`,
          null,
          Buffer.concat(stderrChunks).toString(),
        ),
      );
    });

    child.on("close", (code) => {
      const stdout = Buffer.concat(stdoutChunks).toString();
      const stderr = Buffer.concat(stderrChunks).toString();

      if (code !== 0) {
        reject(
          new ZagError(
            `zag exited with code ${code}: ${stderr || stdout}`,
            code,
            stderr,
          ),
        );
        return;
      }

      try {
        const output: AgentOutput = JSON.parse(stdout);
        resolve(output);
      } catch {
        reject(
          new ZagError(
            `Failed to parse zag JSON output: ${stdout.slice(0, 200)}`,
            code,
            stderr,
          ),
        );
      }
    });
  });
}

/**
 * Run `zag` in streaming mode and yield parsed `Event` objects (NDJSON).
 */
export async function* streamZag(
  bin: string,
  args: string[],
): AsyncGenerator<Event> {
  const child = spawn(bin, args, { stdio: ["ignore", "pipe", "pipe"] });

  const stderrChunks: Buffer[] = [];
  child.stderr.on("data", (chunk: Buffer) => stderrChunks.push(chunk));

  const rl = createInterface({ input: child.stdout });

  for await (const line of rl) {
    const trimmed = line.trim();
    if (!trimmed) continue;
    try {
      const event: Event = JSON.parse(trimmed);
      yield event;
    } catch {
      // Skip unparseable lines
    }
  }

  const exitCode = await new Promise<number | null>((resolve) => {
    child.on("close", resolve);
  });

  if (exitCode !== 0) {
    const stderr = Buffer.concat(stderrChunks).toString();
    throw new ZagError(
      `zag exited with code ${exitCode}${stderr ? `: ${stderr}` : ""}`,
      exitCode,
      stderr,
    );
  }
}

/**
 * A live streaming session with piped stdin and stdout.
 *
 * Send NDJSON messages via `send()`, read events via `events()`,
 * then call `wait()` when done.
 */
export interface StreamingSession {
  /** Send a raw NDJSON line to the agent's stdin. */
  send(message: string): void;

  /** Send a user message to the agent. */
  sendUserMessage(content: string): void;

  /** Close stdin to signal no more input. */
  closeInput(): void;

  /** Async iterator over parsed Event objects from stdout. */
  events(): AsyncGenerator<Event>;

  /** Whether the child process is still running. */
  readonly isRunning: boolean;

  /** Send SIGTERM to the child process. No-op if already exited. */
  terminate(): void;

  /** Wait for the process to exit. Throws ZagError on non-zero exit. */
  wait(): Promise<void>;

  /**
   * Gracefully stop the session.
   *
   * Performs the full shutdown dance so consumers don't have to:
   *
   *   1. Closes stdin to signal no more input.
   *   2. Waits up to half of `timeout` for the child to exit on its own.
   *   3. Sends SIGTERM and waits the remaining half.
   *   4. Sends SIGKILL as a last resort.
   *
   * The returned promise resolves once the child has exited, regardless of
   * exit code — `close()` is a cleanup helper and will not reject for
   * non-zero exit. Idempotent: concurrent calls share the same promise.
   *
   * @param options.timeout Total budget. Number (ms) or humantime string
   *   like `"5s"`, `"500ms"`, `"1m"`. Defaults to 5000ms.
   */
  close(options?: { timeout?: number | string }): Promise<void>;
}

/** Options for {@link streamWithInput}. */
export interface StreamWithInputOptions {
  /**
   * When `true`, the resulting session is tracked in a module-level registry
   * and process-wide shutdown handlers (`exit`, `SIGINT`, `SIGTERM`, `SIGHUP`,
   * `uncaughtException`) are installed once. On parent exit the child is
   * SIGTERM'd so orphan zag/claude subprocesses are not left behind.
   */
  autoCleanup?: boolean;
}

/**
 * Spawn `zag` with piped stdin and stdout for bidirectional streaming.
 */
export function streamWithInput(
  bin: string,
  args: string[],
  options: StreamWithInputOptions = {},
): StreamingSession {
  const child = spawn(bin, args, { stdio: ["pipe", "pipe", "pipe"] });

  const stderrChunks: Buffer[] = [];
  child.stderr.on("data", (chunk: Buffer) => stderrChunks.push(chunk));

  let running = true;

  if (options.autoCleanup) {
    ensureCleanupHandlersInstalled();
    liveSessions.add(child);
  }

  child.on("exit", () => {
    running = false;
    liveSessions.delete(child);
  });

  // Promise that resolves whenever the child exits, regardless of exit code.
  // Used by `close()` so it never rejects for a SIGTERM/SIGKILL exit.
  const exited: Promise<void> = running
    ? new Promise((resolve) => {
        child.once("exit", () => resolve());
      })
    : Promise.resolve();

  let closingPromise: Promise<void> | null = null;

  return {
    get isRunning() {
      return running;
    },

    terminate() {
      if (running) {
        child.kill("SIGTERM");
      }
    },

    send(message: string) {
      child.stdin.write(message + "\n");
    },

    sendUserMessage(content: string) {
      const msg = JSON.stringify({ type: "user_message", content });
      child.stdin.write(msg + "\n");
    },

    closeInput() {
      child.stdin.end();
    },

    async *events(): AsyncGenerator<Event> {
      const rl = createInterface({ input: child.stdout });
      for await (const line of rl) {
        const trimmed = line.trim();
        if (!trimmed) continue;
        try {
          const event: Event = JSON.parse(trimmed);
          yield event;
        } catch {
          // Skip unparseable lines
        }
      }
    },

    wait(): Promise<void> {
      return new Promise((resolve, reject) => {
        child.on("close", (code) => {
          if (code !== 0) {
            const stderr = Buffer.concat(stderrChunks).toString();
            reject(
              new ZagError(
                `zag exited with code ${code}${stderr ? `: ${stderr}` : ""}`,
                code,
                stderr,
              ),
            );
          } else {
            resolve();
          }
        });
      });
    },

    close(opts: { timeout?: number | string } = {}): Promise<void> {
      if (closingPromise) return closingPromise;
      if (!running) return Promise.resolve();

      const totalMs = parseTimeoutMs(opts.timeout ?? 5000);
      const half = Math.max(50, Math.floor(totalMs / 2));

      closingPromise = (async () => {
        // Step 1: close stdin to signal no more input.
        try {
          child.stdin.end();
        } catch {
          // stdin may already be closed; that's fine.
        }

        // Step 2: wait up to `half` ms for graceful exit.
        if (!running) return;
        await raceWithTimeout(exited, half);

        // Step 3: still running → SIGTERM, wait up to `half` ms.
        if (!running) return;
        try {
          child.kill("SIGTERM");
        } catch {
          // ignore
        }
        await raceWithTimeout(exited, half);

        // Step 4: still running → SIGKILL and wait.
        if (!running) return;
        try {
          child.kill("SIGKILL");
        } catch {
          // ignore
        }
        await exited;
      })();

      return closingPromise;
    },
  };
}

/** Resolve when `promise` settles or `ms` elapses, whichever comes first. */
function raceWithTimeout(promise: Promise<void>, ms: number): Promise<void> {
  return new Promise((resolve) => {
    let settled = false;
    const timer = setTimeout(() => {
      if (settled) return;
      settled = true;
      resolve();
    }, ms);
    // Don't hold the event loop open for this timer.
    if (typeof timer.unref === "function") timer.unref();
    promise.then(() => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      resolve();
    });
  });
}

/**
 * Run `zag` interactively with inherited stdio.
 * Returns when the process exits.
 */
export async function runZag(bin: string, args: string[]): Promise<void> {
  return new Promise((resolve, reject) => {
    const child = spawn(bin, args, { stdio: "inherit" });

    child.on("error", (err) => {
      reject(new ZagError(`Failed to spawn '${bin}': ${err.message}`, null, ""));
    });

    child.on("close", (code) => {
      if (code !== 0) {
        reject(new ZagError(`zag exited with code ${code}`, code, ""));
      } else {
        resolve();
      }
    });
  });
}
