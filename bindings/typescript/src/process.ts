import { spawn } from "node:child_process";
import { createInterface } from "node:readline";
import type { AgentOutput, Event } from "./types.js";
import { ZagError } from "./types.js";

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
    throw new ZagError(`zag exited with code ${exitCode}`, exitCode, stderr);
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

  /** Wait for the process to exit. Throws ZagError on non-zero exit. */
  wait(): Promise<void>;
}

/**
 * Spawn `zag` with piped stdin and stdout for bidirectional streaming.
 */
export function streamWithInput(
  bin: string,
  args: string[],
): StreamingSession {
  const child = spawn(bin, args, { stdio: ["pipe", "pipe", "pipe"] });

  const stderrChunks: Buffer[] = [];
  child.stderr.on("data", (chunk: Buffer) => stderrChunks.push(chunk));

  return {
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
              new ZagError(`zag exited with code ${code}`, code, stderr),
            );
          } else {
            resolve();
          }
        });
      });
    },
  };
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
