using System.Collections.Concurrent;

namespace Zag;

/// <summary>
/// Provider-capability preflight helper for the zag C# bindings.
///
/// Mirrors the <see cref="VersionCheck"/> pattern: every terminal builder
/// method calls <see cref="CheckAsync"/> with the list of configured feature
/// requirements before spawning the real agent subprocess. When a configured
/// option is unsupported by the pinned provider, a
/// <see cref="ZagFeatureUnsupportedException"/> is thrown with an actionable
/// message — instead of the user seeing a cryptic "zag exited with code 1".
/// </summary>
public static class CapabilityCheck
{
    /// <summary>A single feature-support requirement evaluated at preflight time.</summary>
    public readonly record struct Requirement(string Method, string Feature, bool IsSet);

    private static readonly ConcurrentDictionary<(string Bin, string Provider), ProviderCapability> CapCache = new();
    private static readonly ConcurrentDictionary<string, ProviderCapability[]> AllCapsCache = new();

    private static async Task<ProviderCapability> LoadCapabilityAsync(
        string bin, string provider, CancellationToken ct)
    {
        if (CapCache.TryGetValue((bin, provider), out var cached))
            return cached;
        var cap = await ZagDiscover.GetCapabilityAsync(provider, bin, ct);
        CapCache[(bin, provider)] = cap;
        return cap;
    }

    private static async Task<ProviderCapability[]> LoadAllCapabilitiesAsync(
        string bin, CancellationToken ct)
    {
        if (AllCapsCache.TryGetValue(bin, out var cached))
            return cached;
        var caps = await ZagDiscover.GetAllCapabilitiesAsync(bin, ct);
        AllCapsCache[bin] = caps;
        foreach (var c in caps)
            CapCache[(bin, c.Provider)] = c;
        return caps;
    }

    private static FeatureSupport? FeatureFor(ProviderCapability cap, string feature)
    {
        return feature switch
        {
            "streaming_input" => cap.Features.StreamingInput,
            "worktree" => cap.Features.Worktree,
            "sandbox" => cap.Features.Sandbox,
            "system_prompt" => cap.Features.SystemPrompt,
            "add_dirs" => cap.Features.AddDirs,
            "json_output" => cap.Features.JsonOutput,
            "stream_json" => cap.Features.StreamJson,
            "json_schema" => cap.Features.JsonSchema,
            "input_format" => cap.Features.InputFormat,
            "interactive" => cap.Features.Interactive,
            "non_interactive" => cap.Features.NonInteractive,
            "resume" => cap.Features.Resume,
            "resume_with_prompt" => cap.Features.ResumeWithPrompt,
            "auto_approve" => cap.Features.AutoApprove,
            "review" => cap.Features.Review,
            "max_turns" => cap.Features.MaxTurns,
            _ => null,
        };
    }

    private static bool IsSupported(ProviderCapability cap, string feature)
    {
        var f = FeatureFor(cap, feature);
        return f != null && f.Supported;
    }

    /// <summary>
    /// Check that every active requirement is supported by <paramref name="provider"/>.
    /// Returns silently when no requirement is active, when <paramref name="provider"/>
    /// is null (auto-detect), or when <paramref name="provider"/> is "mock".
    /// </summary>
    public static async Task CheckAsync(
        string bin,
        string? provider,
        IReadOnlyList<Requirement> requirements,
        CancellationToken ct = default)
    {
        var active = requirements.Where(r => r.IsSet).ToList();
        if (active.Count == 0) return;
        if (string.IsNullOrEmpty(provider) || provider == "mock") return;

        var cap = await LoadCapabilityAsync(bin, provider, ct);

        foreach (var req in active)
        {
            if (IsSupported(cap, req.Feature)) continue;

            var caps = await LoadAllCapabilitiesAsync(bin, ct);
            var supported = caps.Where(c => IsSupported(c, req.Feature))
                .Select(c => c.Provider).ToList();
            throw new ZagFeatureUnsupportedException(
                req.Method, req.Feature, provider, supported);
        }
    }

    /// <summary>Inject a capability into the cache for testing.</summary>
    internal static void SetCapabilityForTesting(
        string bin, string provider, ProviderCapability cap)
    {
        CapCache[(bin, provider)] = cap;
    }

    /// <summary>Inject the full capability matrix into the cache for testing.</summary>
    internal static void SetAllCapabilitiesForTesting(
        string bin, ProviderCapability[] caps)
    {
        AllCapsCache[bin] = caps;
        foreach (var c in caps)
            CapCache[(bin, c.Provider)] = c;
    }

    /// <summary>Clear the capability caches for testing.</summary>
    internal static void ClearCapabilityCache()
    {
        CapCache.Clear();
        AllCapsCache.Clear();
    }
}
