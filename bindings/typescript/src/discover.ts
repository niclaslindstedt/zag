import { spawn } from "node:child_process";
import type { ProviderCapability, ResolvedModel } from "./types.js";
import { ZagError } from "./types.js";
import { defaultBin } from "./process.js";

/**
 * Run a zag discover subcommand and parse JSON output.
 */
async function discoverExec<T>(bin: string, args: string[]): Promise<T> {
  return new Promise((resolve, reject) => {
    const child = spawn(bin, ["discover", ...args, "--json"], {
      stdio: ["ignore", "pipe", "pipe"],
    });

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
        resolve(JSON.parse(stdout) as T);
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
 * List all available provider names.
 *
 * @param bin - Path to the zag binary (defaults to `ZAG_BIN` env or `"zag"`)
 */
export async function listProviders(bin?: string): Promise<string[]> {
  const caps = await getAllCapabilities(bin);
  return caps.map((c) => c.provider);
}

/**
 * Get capability declarations for a specific provider.
 *
 * @param provider - Provider name (e.g. "claude", "codex", "gemini", "copilot", "ollama")
 * @param bin - Path to the zag binary (defaults to `ZAG_BIN` env or `"zag"`)
 */
export async function getCapability(
  provider: string,
  bin?: string,
): Promise<ProviderCapability> {
  const b = bin ?? defaultBin();
  return discoverExec<ProviderCapability>(b, ["-p", provider]);
}

/**
 * Get capability declarations for all providers.
 *
 * @param bin - Path to the zag binary (defaults to `ZAG_BIN` env or `"zag"`)
 */
export async function getAllCapabilities(
  bin?: string,
): Promise<ProviderCapability[]> {
  const b = bin ?? defaultBin();
  return discoverExec<ProviderCapability[]>(b, []);
}

/**
 * Resolve a model alias for a given provider.
 *
 * Size aliases (`small`/`s`, `medium`/`m`/`default`, `large`/`l`/`max`) are
 * resolved to the provider-specific model. Non-alias names pass through unchanged.
 *
 * @param provider - Provider name
 * @param model - Model name or alias to resolve
 * @param bin - Path to the zag binary (defaults to `ZAG_BIN` env or `"zag"`)
 */
export async function resolveModel(
  provider: string,
  model: string,
  bin?: string,
): Promise<ResolvedModel> {
  const b = bin ?? defaultBin();
  return discoverExec<ResolvedModel>(b, ["-p", provider, "--resolve", model]);
}
