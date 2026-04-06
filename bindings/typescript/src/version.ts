import { spawn } from "node:child_process";
import { ZagError } from "./types.js";

/** Minimum CLI version required for each feature (only post-initial-release features). */
const MIN_VERSIONS: Record<string, string> = {
  env: "0.6.0",
  mcpConfig: "0.6.0",
};

/** Parsed semver tuple. */
type SemVer = [number, number, number];

/** Parse a semver string like "0.6.0" into a numeric tuple. */
export function parseSemver(version: string): SemVer {
  const parts = version.trim().split(".");
  if (parts.length !== 3) {
    throw new ZagError(
      `Could not parse version "${version}": expected format "X.Y.Z"`,
      null,
      "",
    );
  }
  const nums = parts.map(Number);
  if (nums.some(isNaN)) {
    throw new ZagError(
      `Could not parse version "${version}": non-numeric components`,
      null,
      "",
    );
  }
  return nums as unknown as SemVer;
}

/** Compare two semver tuples. Returns -1 if a < b, 0 if equal, 1 if a > b. */
export function compareSemver(a: SemVer, b: SemVer): number {
  for (let i = 0; i < 3; i++) {
    if (a[i] < b[i]) return -1;
    if (a[i] > b[i]) return 1;
  }
  return 0;
}

/** Cached detected versions keyed by binary path. */
const versionCache = new Map<string, string>();

/**
 * Detect the CLI version by running `{bin} --version`.
 * Result is cached per binary path.
 */
export async function detectVersion(bin: string): Promise<string> {
  const cached = versionCache.get(bin);
  if (cached) return cached;

  const version = await new Promise<string>((resolve, reject) => {
    const child = spawn(bin, ["--version"], {
      stdio: ["ignore", "pipe", "pipe"],
    });

    const stdoutChunks: Buffer[] = [];

    child.stdout.on("data", (chunk: Buffer) => stdoutChunks.push(chunk));

    child.on("error", (err) => {
      reject(
        new ZagError(
          `Could not detect zag CLI version: failed to run '${bin} --version'. ` +
            `Ensure zag is installed and on your PATH, or set ZAG_BIN. ` +
            `(${err.message})`,
          null,
          "",
        ),
      );
    });

    child.on("close", (code) => {
      if (code !== 0) {
        reject(
          new ZagError(
            `Could not detect zag CLI version: '${bin} --version' exited with code ${code}`,
            code,
            "",
          ),
        );
        return;
      }

      const output = Buffer.concat(stdoutChunks).toString().trim();
      // Expected format: "zag-cli 0.6.0" or just "0.6.0"
      const parts = output.split(/\s+/);
      const versionStr = parts[parts.length - 1];

      try {
        parseSemver(versionStr);
      } catch {
        reject(
          new ZagError(
            `Could not parse zag CLI version from output: "${output}". ` +
              `Expected format: "zag-cli X.Y.Z"`,
            null,
            "",
          ),
        );
        return;
      }

      resolve(versionStr);
    });
  });

  versionCache.set(bin, version);
  return version;
}

/** Feature requirement passed to checkVersion. */
export interface VersionRequirement {
  method: string;
  version: string;
  isSet: boolean;
}

/**
 * Check that the installed CLI version satisfies all configured feature requirements.
 * Throws ZagError if any requirement is not met.
 */
export async function checkVersion(
  bin: string,
  requirements: VersionRequirement[],
): Promise<void> {
  const active = requirements.filter((r) => r.isSet);
  if (active.length === 0) return;

  const detected = await detectVersion(bin);
  const detectedSemver = parseSemver(detected);

  const failures = active.filter(
    (r) => compareSemver(detectedSemver, parseSemver(r.version)) < 0,
  );

  if (failures.length === 0) return;

  if (failures.length === 1) {
    throw new ZagError(
      `${failures[0].method} requires zag CLI >= ${failures[0].version}, ` +
        `but the installed version is ${detected}. Please update the zag binary.`,
      null,
      "",
    );
  }

  const lines = failures.map(
    (f) => `  - ${f.method} requires >= ${f.version}`,
  );
  throw new ZagError(
    `The following methods require a newer zag CLI version:\n` +
      `${lines.join("\n")}\n` +
      `Installed version: ${detected}. Please update the zag binary.`,
    null,
    "",
  );
}

/** @internal Inject a version into the cache for testing. */
export function _setVersionForTesting(bin: string, version: string): void {
  versionCache.set(bin, version);
}

/** @internal Clear the version cache for testing. */
export function _clearVersionCache(): void {
  versionCache.clear();
}
