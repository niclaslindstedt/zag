using System.Collections.Concurrent;

namespace Zag;

/// <summary>
/// Provider capability validation for the <see cref="ZagBuilder"/>.
///
/// <para>Before spawning the <c>zag</c> CLI, the builder validates
/// feature-gated options (<c>ExecStreaming</c>, <c>Worktree</c>,
/// <c>Sandbox</c>, <c>SystemPrompt</c>, <c>AddDir</c>, <c>MaxTurns</c>)
/// against the capability declarations exposed by <c>zag discover</c>.
/// When a caller configures an option that the selected provider does not
/// support, the preflight raises
/// <see cref="ZagFeatureUnsupportedException"/> with a message listing the
/// providers that do support the feature.</para>
/// </summary>
public static class CapabilityCheck
{
    /// <summary>Capability feature keys the builder can gate on.</summary>
    public static class FeatureKeys
    {
        public const string StreamingInput = "streaming_input";
        public const string Worktree = "worktree";
        public const string Sandbox = "sandbox";
        public const string SystemPrompt = "system_prompt";
        public const string AddDirs = "add_dirs";
        public const string MaxTurns = "max_turns";
    }

    /// <summary>
    /// A capability-gated builder option.
    /// </summary>
    /// <param name="Method">User-facing builder method name (e.g.,
    /// <c>"ExecStreaming()"</c>).</param>
    /// <param name="Feature">Capability feature key (e.g.,
    /// <c>"streaming_input"</c>).</param>
    /// <param name="IsSet">Whether the option is active for this
    /// invocation.</param>
    public readonly record struct Requirement(string Method, string Feature, bool IsSet);

    private static readonly ConcurrentDictionary<string, ProviderCapability[]> CapabilityCache = new();

    /// <summary>
    /// Fetch and cache the full provider capability matrix for a given
    /// <c>zag</c> binary. Capabilities are compiled into the binary, so the
    /// cache lives for the life of the process.
    /// </summary>
    private static async Task<ProviderCapability[]> LoadCapabilitiesAsync(
        string bin, CancellationToken ct)
    {
        if (CapabilityCache.TryGetValue(bin, out var cached))
            return cached;
        var caps = await ZagDiscover.GetAllCapabilitiesAsync(bin, ct);
        CapabilityCache[bin] = caps;
        return caps;
    }

    private static bool IsFeatureSupported(Features features, string key) => key switch
    {
        FeatureKeys.StreamingInput => features.StreamingInput.Supported,
        FeatureKeys.Worktree => features.Worktree.Supported,
        FeatureKeys.Sandbox => features.Sandbox.Supported,
        FeatureKeys.SystemPrompt => features.SystemPrompt.Supported,
        FeatureKeys.AddDirs => features.AddDirs.Supported,
        FeatureKeys.MaxTurns => features.MaxTurns.Supported,
        // Unknown key — treat as supported so we never falsely block.
        _ => true,
    };

    /// <summary>
    /// Validate that every active feature requirement is supported by the
    /// configured provider. No-op when <paramref name="provider"/> is null
    /// (so the CLI's default-provider behavior is preserved) or when no
    /// requirements are active. If the <c>zag discover</c> call itself
    /// fails, the preflight silently returns so the subsequent CLI
    /// invocation can surface the real error.
    /// </summary>
    /// <exception cref="ZagFeatureUnsupportedException">
    /// Thrown on the first unsupported feature.
    /// </exception>
    public static async Task CheckAsync(
        string bin,
        string? provider,
        IReadOnlyList<Requirement> requirements,
        CancellationToken ct = default)
    {
        var active = requirements.Where(r => r.IsSet).ToList();
        if (active.Count == 0 || provider == null) return;

        ProviderCapability[] caps;
        try
        {
            caps = await LoadCapabilitiesAsync(bin, ct);
        }
        catch
        {
            // If `zag discover` can't be reached, skip the preflight — the
            // subsequent CLI invocation will surface the real error.
            return;
        }

        var providerCap = caps.FirstOrDefault(c => c.Provider == provider);
        if (providerCap == null) return;

        foreach (var req in active)
        {
            if (IsFeatureSupported(providerCap.Features, req.Feature)) continue;
            var supported = caps
                .Where(c => IsFeatureSupported(c.Features, req.Feature))
                .Select(c => c.Provider)
                .ToList();
            var suffix = supported.Count > 0
                ? $" Supported providers: {string.Join(", ", supported)}"
                : " No providers currently support this feature.";
            throw new ZagFeatureUnsupportedException(
                $"Provider '{provider}' does not support {req.Feature} " +
                $"(required by {req.Method}).{suffix}",
                provider,
                req.Feature,
                req.Method,
                supported);
        }
    }

    /// <summary>Inject capabilities into the cache for testing.</summary>
    internal static void SetCapabilitiesForTesting(string bin, ProviderCapability[] caps)
    {
        CapabilityCache[bin] = caps;
    }

    /// <summary>Clear the capability cache for testing.</summary>
    internal static void ClearCapabilityCache()
    {
        CapabilityCache.Clear();
    }
}
