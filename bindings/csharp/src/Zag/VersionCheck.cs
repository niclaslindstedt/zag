using System.Collections.Concurrent;
using System.Diagnostics;

namespace Zag;

/// <summary>CLI version detection and compatibility checking for zag bindings.</summary>
public static class VersionCheck
{
    /// <summary>Parsed semver tuple.</summary>
    internal readonly record struct SemVer(int Major, int Minor, int Patch) : IComparable<SemVer>
    {
        public int CompareTo(SemVer other)
        {
            int c = Major.CompareTo(other.Major);
            if (c != 0) return c;
            c = Minor.CompareTo(other.Minor);
            if (c != 0) return c;
            return Patch.CompareTo(other.Patch);
        }
    }

    /// <summary>A feature requirement with method name, minimum version, and whether it is set.</summary>
    public readonly record struct Requirement(string Method, string Version, bool IsSet);

    private static readonly ConcurrentDictionary<string, string> VersionCache = new();

    /// <summary>Parse a semver string like "0.6.0" into a numeric tuple.</summary>
    internal static SemVer ParseSemver(string version)
    {
        var parts = version.Trim().Split('.');
        if (parts.Length != 3 ||
            !int.TryParse(parts[0], out var major) ||
            !int.TryParse(parts[1], out var minor) ||
            !int.TryParse(parts[2], out var patch))
        {
            throw new ZagException(
                $"Could not parse version \"{version}\": expected format \"X.Y.Z\"",
                null, "");
        }
        return new SemVer(major, minor, patch);
    }

    /// <summary>Detect the CLI version by running <c>{bin} --version</c>. Cached per binary path.</summary>
    public static async Task<string> DetectVersionAsync(string bin, CancellationToken ct = default)
    {
        if (VersionCache.TryGetValue(bin, out var cached))
            return cached;

        var psi = new ProcessStartInfo(bin, "--version")
        {
            RedirectStandardOutput = true,
            RedirectStandardError = true,
            UseShellExecute = false,
            CreateNoWindow = true,
        };

        Process process;
        try
        {
            process = Process.Start(psi)
                ?? throw new ZagException(
                    $"Could not detect zag CLI version: failed to start '{bin} --version'.",
                    null, "");
        }
        catch (Exception ex)
        {
            throw new ZagException(
                $"Could not detect zag CLI version: failed to run '{bin} --version'. " +
                $"Ensure zag is installed and on your PATH, or set ZAG_BIN. ({ex.Message})",
                null, "");
        }

        var stdout = await process.StandardOutput.ReadToEndAsync(ct);
        await process.WaitForExitAsync(ct);

        if (process.ExitCode != 0)
        {
            throw new ZagException(
                $"Could not detect zag CLI version: '{bin} --version' exited with code {process.ExitCode}",
                process.ExitCode, "");
        }

        var output = stdout.Trim();
        var parts = output.Split(' ', StringSplitOptions.RemoveEmptyEntries);
        var versionStr = parts.Length > 0 ? parts[^1] : "";

        // Validate it parses
        ParseSemver(versionStr);

        VersionCache[bin] = versionStr;
        return versionStr;
    }

    /// <summary>Check that the installed CLI version satisfies all configured requirements.</summary>
    public static async Task CheckAsync(string bin, IReadOnlyList<Requirement> requirements, CancellationToken ct = default)
    {
        var active = requirements.Where(r => r.IsSet).ToList();
        if (active.Count == 0) return;

        var detected = await DetectVersionAsync(bin, ct);
        var detectedSv = ParseSemver(detected);

        var failures = active
            .Where(r => detectedSv.CompareTo(ParseSemver(r.Version)) < 0)
            .ToList();

        if (failures.Count == 0) return;

        if (failures.Count == 1)
        {
            throw new ZagException(
                $"{failures[0].Method} requires zag CLI >= {failures[0].Version}, " +
                $"but the installed version is {detected}. Please update the zag binary.",
                null, "");
        }

        var lines = string.Join("\n", failures.Select(f => $"  - {f.Method} requires >= {f.Version}"));
        throw new ZagException(
            $"The following methods require a newer zag CLI version:\n{lines}\n" +
            $"Installed version: {detected}. Please update the zag binary.",
            null, "");
    }

    /// <summary>Inject a version into the cache for testing.</summary>
    internal static void SetVersionForTesting(string bin, string version)
    {
        VersionCache[bin] = version;
    }

    /// <summary>Clear the version cache for testing.</summary>
    internal static void ClearVersionCache()
    {
        VersionCache.Clear();
    }
}
