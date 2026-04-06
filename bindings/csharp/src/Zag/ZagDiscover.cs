using System.Diagnostics;
using System.Text.Json;

namespace Zag;

/// <summary>Discovery helpers for querying provider capabilities via the zag CLI.</summary>
public static class ZagDiscover
{
    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        PropertyNameCaseInsensitive = true,
    };

    /// <summary>List all available provider names.</summary>
    /// <param name="bin">Path to the zag binary (defaults to ZAG_BIN env or "zag").</param>
    /// <param name="ct">Cancellation token.</param>
    public static async Task<string[]> ListProvidersAsync(string? bin = null, CancellationToken ct = default)
    {
        var caps = await GetAllCapabilitiesAsync(bin, ct);
        return caps.Select(c => c.Provider).ToArray();
    }

    /// <summary>Get capability declarations for a specific provider.</summary>
    /// <param name="provider">Provider name (e.g. "claude", "codex", "gemini", "copilot", "ollama").</param>
    /// <param name="bin">Path to the zag binary (defaults to ZAG_BIN env or "zag").</param>
    /// <param name="ct">Cancellation token.</param>
    public static async Task<ProviderCapability> GetCapabilityAsync(string provider, string? bin = null, CancellationToken ct = default)
    {
        var b = bin ?? ZagProcess.DefaultBin;
        return await DiscoverExecAsync<ProviderCapability>(b, ["-p", provider], ct);
    }

    /// <summary>Get capability declarations for all providers.</summary>
    /// <param name="bin">Path to the zag binary (defaults to ZAG_BIN env or "zag").</param>
    /// <param name="ct">Cancellation token.</param>
    public static async Task<ProviderCapability[]> GetAllCapabilitiesAsync(string? bin = null, CancellationToken ct = default)
    {
        var b = bin ?? ZagProcess.DefaultBin;
        return await DiscoverExecAsync<ProviderCapability[]>(b, [], ct);
    }

    /// <summary>Resolve a model alias for a given provider.</summary>
    /// <param name="provider">Provider name.</param>
    /// <param name="model">Model name or alias to resolve.</param>
    /// <param name="bin">Path to the zag binary (defaults to ZAG_BIN env or "zag").</param>
    /// <param name="ct">Cancellation token.</param>
    public static async Task<ResolvedModel> ResolveModelAsync(string provider, string model, string? bin = null, CancellationToken ct = default)
    {
        var b = bin ?? ZagProcess.DefaultBin;
        return await DiscoverExecAsync<ResolvedModel>(b, ["-p", provider, "--resolve", model], ct);
    }

    /// <summary>Run a zag discover subcommand and parse JSON output.</summary>
    private static async Task<T> DiscoverExecAsync<T>(string bin, string[] args, CancellationToken ct)
    {
        var fullArgs = new List<string> { "discover" };
        fullArgs.AddRange(args);
        fullArgs.Add("--json");

        var psi = new ProcessStartInfo(bin)
        {
            UseShellExecute = false,
            RedirectStandardOutput = true,
            RedirectStandardError = true,
            RedirectStandardInput = false,
        };
        foreach (var arg in fullArgs) psi.ArgumentList.Add(arg);

        using var process = Process.Start(psi)
            ?? throw new ZagException($"Failed to start '{bin}'", null, "");

        var stdoutTask = process.StandardOutput.ReadToEndAsync(ct);
        var stderrTask = process.StandardError.ReadToEndAsync(ct);

        await process.WaitForExitAsync(ct);

        var stdout = await stdoutTask;
        var stderr = await stderrTask;

        if (process.ExitCode != 0)
        {
            throw new ZagException(
                $"zag exited with code {process.ExitCode}: {(string.IsNullOrEmpty(stderr) ? stdout : stderr)}",
                process.ExitCode,
                stderr);
        }

        try
        {
            return JsonSerializer.Deserialize<T>(stdout, JsonOptions)
                ?? throw new ZagException("Deserialized discover output was null", process.ExitCode, stderr);
        }
        catch (JsonException)
        {
            throw new ZagException(
                $"Failed to parse zag JSON output: {stdout[..Math.Min(stdout.Length, 200)]}",
                process.ExitCode,
                stderr);
        }
    }
}
