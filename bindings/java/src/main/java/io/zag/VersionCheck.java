package io.zag;

import java.io.IOException;
import java.util.List;
import java.util.concurrent.ConcurrentHashMap;

/**
 * CLI version detection and compatibility checking for zag bindings.
 */
public final class VersionCheck {

    private VersionCheck() {}

    private static final ConcurrentHashMap<String, String> VERSION_CACHE = new ConcurrentHashMap<>();

    /** A parsed semver tuple. */
    record SemVer(int major, int minor, int patch) implements Comparable<SemVer> {
        @Override
        public int compareTo(SemVer other) {
            int c = Integer.compare(major, other.major);
            if (c != 0) return c;
            c = Integer.compare(minor, other.minor);
            if (c != 0) return c;
            return Integer.compare(patch, other.patch);
        }
    }

    /** A feature requirement with method name, minimum version, and whether it is set. */
    public record Requirement(String method, String version, boolean isSet) {}

    /** Parse a semver string like "0.6.0" into a numeric tuple. */
    static SemVer parseSemver(String version) throws ZagException {
        String[] parts = version.trim().split("\\.");
        if (parts.length != 3) {
            throw new ZagException(
                "Could not parse version \"" + version + "\": expected format \"X.Y.Z\"",
                null, "");
        }
        try {
            return new SemVer(
                Integer.parseInt(parts[0]),
                Integer.parseInt(parts[1]),
                Integer.parseInt(parts[2]));
        } catch (NumberFormatException e) {
            throw new ZagException(
                "Could not parse version \"" + version + "\": non-numeric components",
                null, "");
        }
    }

    /** Unchecked semver parse for use in stream lambdas. */
    private static SemVer parseSemverUnchecked(String version) {
        try {
            return parseSemver(version);
        } catch (ZagException e) {
            throw new RuntimeException(e);
        }
    }

    /**
     * Detect the CLI version by running {@code {bin} --version}.
     * Result is cached per binary path.
     */
    public static String detectVersion(String bin) throws ZagException {
        String cached = VERSION_CACHE.get(bin);
        if (cached != null) return cached;

        ProcessBuilder pb = new ProcessBuilder(bin, "--version")
            .redirectErrorStream(false);

        Process process;
        try {
            process = pb.start();
        } catch (IOException e) {
            throw new ZagException(
                "Could not detect zag CLI version: failed to run '" + bin + " --version'. " +
                "Ensure zag is installed and on your PATH, or set ZAG_BIN. (" + e.getMessage() + ")",
                null, "");
        }

        try {
            String stdout = new String(process.getInputStream().readAllBytes()).trim();
            int exitCode = process.waitFor();

            if (exitCode != 0) {
                throw new ZagException(
                    "Could not detect zag CLI version: '" + bin + " --version' exited with code " + exitCode,
                    exitCode, "");
            }

            String[] parts = stdout.split("\\s+");
            String versionStr = parts.length > 0 ? parts[parts.length - 1] : "";

            // Validate it parses
            parseSemver(versionStr);

            VERSION_CACHE.put(bin, versionStr);
            return versionStr;
        } catch (InterruptedException e) {
            Thread.currentThread().interrupt();
            throw new ZagException("Version detection interrupted", null, "");
        }
    }

    /**
     * Check that the installed CLI version satisfies all configured requirements.
     * Throws {@link ZagException} if any requirement is not met.
     */
    public static void check(String bin, List<Requirement> requirements) throws ZagException {
        List<Requirement> active = requirements.stream()
            .filter(Requirement::isSet)
            .toList();
        if (active.isEmpty()) return;

        String detected = detectVersion(bin);
        SemVer detectedSv = parseSemver(detected);

        List<Requirement> failures = active.stream()
            .filter(r -> detectedSv.compareTo(parseSemverUnchecked(r.version())) < 0)
            .toList();

        if (failures.isEmpty()) return;

        if (failures.size() == 1) {
            Requirement f = failures.get(0);
            throw new ZagException(
                f.method() + " requires zag CLI >= " + f.version() +
                ", but the installed version is " + detected + ". Please update the zag binary.",
                null, "");
        }

        StringBuilder sb = new StringBuilder("The following methods require a newer zag CLI version:\n");
        for (Requirement f : failures) {
            sb.append("  - ").append(f.method()).append(" requires >= ").append(f.version()).append("\n");
        }
        sb.append("Installed version: ").append(detected).append(". Please update the zag binary.");
        throw new ZagException(sb.toString(), null, "");
    }

    /** Inject a version into the cache for testing. */
    static void setVersionForTesting(String bin, String version) {
        VERSION_CACHE.put(bin, version);
    }

    /** Clear the version cache for testing. */
    static void clearVersionCache() {
        VERSION_CACHE.clear();
    }

}
