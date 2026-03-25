using System.Diagnostics;
using System.Runtime.CompilerServices;
using System.Text;
using System.Text.Json;

namespace Zag;

/// <summary>Subprocess helpers for invoking the zag CLI.</summary>
public static class ZagProcess
{
    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        PropertyNameCaseInsensitive = true,
    };

    /// <summary>Get the default zag binary path from ZAG_BIN env or "zag".</summary>
    public static string DefaultBin =>
        Environment.GetEnvironmentVariable("ZAG_BIN") ?? "zag";

    /// <summary>Run zag and return parsed AgentOutput.</summary>
    public static async Task<AgentOutput> ExecAsync(string bin, string[] args, CancellationToken ct = default)
    {
        using var process = StartProcess(bin, args, captureStdout: true);

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
            return JsonSerializer.Deserialize<AgentOutput>(stdout, JsonOptions)
                ?? throw new ZagException("Deserialized AgentOutput was null", process.ExitCode, stderr);
        }
        catch (JsonException ex)
        {
            throw new ZagException(
                $"Failed to parse zag JSON output: {stdout[..Math.Min(stdout.Length, 200)]}",
                process.ExitCode,
                stderr) { };
        }
    }

    /// <summary>Run zag in streaming mode, yielding Event objects from NDJSON.</summary>
    public static async IAsyncEnumerable<Event> StreamAsync(
        string bin,
        string[] args,
        [EnumeratorCancellation] CancellationToken ct = default)
    {
        using var process = StartProcess(bin, args, captureStdout: true);

        var stderrBuilder = new StringBuilder();
        process.ErrorDataReceived += (_, e) =>
        {
            if (e.Data != null) stderrBuilder.AppendLine(e.Data);
        };
        process.BeginErrorReadLine();

        while (!ct.IsCancellationRequested)
        {
            var line = await process.StandardOutput.ReadLineAsync(ct);
            if (line == null) break;

            var trimmed = line.Trim();
            if (string.IsNullOrEmpty(trimmed)) continue;

            Event? evt;
            try
            {
                evt = JsonSerializer.Deserialize<Event>(trimmed, JsonOptions);
            }
            catch (JsonException)
            {
                continue;
            }

            if (evt != null)
                yield return evt;
        }

        await process.WaitForExitAsync(ct);

        if (process.ExitCode != 0)
        {
            throw new ZagException(
                $"zag exited with code {process.ExitCode}",
                process.ExitCode,
                stderrBuilder.ToString());
        }
    }

    /// <summary>Run zag interactively with inherited stdio.</summary>
    public static async Task RunAsync(string bin, string[] args, CancellationToken ct = default)
    {
        var psi = new ProcessStartInfo(bin)
        {
            UseShellExecute = false,
        };
        foreach (var arg in args) psi.ArgumentList.Add(arg);

        using var process = Process.Start(psi)
            ?? throw new ZagException($"Failed to start '{bin}'", null, "");

        await process.WaitForExitAsync(ct);

        if (process.ExitCode != 0)
        {
            throw new ZagException(
                $"zag exited with code {process.ExitCode}",
                process.ExitCode,
                "");
        }
    }

    private static Process StartProcess(string bin, string[] args, bool captureStdout)
    {
        var psi = new ProcessStartInfo(bin)
        {
            UseShellExecute = false,
            RedirectStandardOutput = captureStdout,
            RedirectStandardError = true,
            RedirectStandardInput = false,
        };
        foreach (var arg in args) psi.ArgumentList.Add(arg);

        return Process.Start(psi)
            ?? throw new ZagException($"Failed to start '{bin}'", null, "");
    }
}
