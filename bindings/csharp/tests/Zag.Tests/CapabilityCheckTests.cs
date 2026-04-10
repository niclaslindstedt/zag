using Xunit;
using Zag;

namespace Zag.Tests;

public class CapabilityCheckTests
{
    private static ProviderCapability FakeCap(
        string provider,
        bool worktree = false,
        bool sandbox = false,
        bool systemPrompt = false,
        bool addDirs = false,
        bool streamingInput = false)
    {
        return new ProviderCapability
        {
            Provider = provider,
            DefaultModel = "default",
            AvailableModels = [],
            SizeMappings = new SizeMappings(),
            Features = new Features
            {
                Interactive = new FeatureSupport { Supported = true },
                NonInteractive = new FeatureSupport { Supported = true },
                JsonOutput = new FeatureSupport { Supported = true },
                StreamJson = new FeatureSupport { Supported = true },
                AutoApprove = new FeatureSupport { Supported = true },
                Worktree = new FeatureSupport { Supported = worktree },
                Sandbox = new FeatureSupport { Supported = sandbox },
                SystemPrompt = new FeatureSupport { Supported = systemPrompt },
                AddDirs = new FeatureSupport { Supported = addDirs },
                StreamingInput = new FeatureSupport { Supported = streamingInput },
            },
        };
    }

    private static void PrimeCaps(string bin, ProviderCapability[] caps)
    {
        CapabilityCheck.ClearCapabilityCache();
        CapabilityCheck.SetAllCapabilitiesForTesting(bin, caps);
    }

    [Fact]
    public async Task Check_NoRequirements_Passes()
    {
        PrimeCaps("zag", [FakeCap("ollama")]);
        try
        {
            await CapabilityCheck.CheckAsync(
                "zag", "ollama", Array.Empty<CapabilityCheck.Requirement>());
        }
        finally
        {
            CapabilityCheck.ClearCapabilityCache();
        }
    }

    [Fact]
    public async Task Check_InactiveRequirements_Passes()
    {
        PrimeCaps("zag", [FakeCap("ollama")]);
        try
        {
            await CapabilityCheck.CheckAsync(
                "zag",
                "ollama",
                new[]
                {
                    new CapabilityCheck.Requirement("AddDir()", "add_dirs", false),
                });
        }
        finally
        {
            CapabilityCheck.ClearCapabilityCache();
        }
    }

    [Fact]
    public async Task Check_NullProvider_Skips()
    {
        // No cache primed — would raise if we tried to load.
        await CapabilityCheck.CheckAsync(
            "zag",
            null,
            new[]
            {
                new CapabilityCheck.Requirement("AddDir()", "add_dirs", true),
            });
    }

    [Fact]
    public async Task Check_MockProvider_Skips()
    {
        await CapabilityCheck.CheckAsync(
            "zag",
            "mock",
            new[]
            {
                new CapabilityCheck.Requirement("AddDir()", "add_dirs", true),
            });
    }

    [Fact]
    public async Task Check_SupportedFeature_Passes()
    {
        PrimeCaps("zag", [FakeCap("claude", streamingInput: true)]);
        try
        {
            await CapabilityCheck.CheckAsync(
                "zag",
                "claude",
                new[]
                {
                    new CapabilityCheck.Requirement(
                        "ExecStreaming()", "streaming_input", true),
                });
        }
        finally
        {
            CapabilityCheck.ClearCapabilityCache();
        }
    }

    [Fact]
    public async Task Check_UnsupportedFeature_Throws()
    {
        PrimeCaps(
            "zag",
            [
                FakeCap("claude", streamingInput: true),
                FakeCap("ollama", streamingInput: false),
            ]);
        try
        {
            var ex = await Assert.ThrowsAsync<ZagFeatureUnsupportedException>(() =>
                CapabilityCheck.CheckAsync(
                    "zag",
                    "ollama",
                    new[]
                    {
                        new CapabilityCheck.Requirement(
                            "ExecStreaming()", "streaming_input", true),
                    }));

            Assert.Equal("ExecStreaming()", ex.Method);
            Assert.Equal("streaming_input", ex.Feature);
            Assert.Equal("ollama", ex.Provider);
            Assert.Contains("claude", ex.SupportedProviders);
            Assert.DoesNotContain("ollama", ex.SupportedProviders);
            Assert.IsAssignableFrom<ZagException>(ex);
        }
        finally
        {
            CapabilityCheck.ClearCapabilityCache();
        }
    }

    [Fact]
    public async Task Check_UnsupportedWithNoSupporters_ShowsNone()
    {
        PrimeCaps("zag", [FakeCap("ollama")]);
        try
        {
            var ex = await Assert.ThrowsAsync<ZagFeatureUnsupportedException>(() =>
                CapabilityCheck.CheckAsync(
                    "zag",
                    "ollama",
                    new[]
                    {
                        new CapabilityCheck.Requirement("Sandbox()", "sandbox", true),
                    }));

            Assert.Contains("(none)", ex.Message);
        }
        finally
        {
            CapabilityCheck.ClearCapabilityCache();
        }
    }
}

public class ZagBuilderCapabilityPreflightTests
{
    private static ProviderCapability FakeCap(
        string provider,
        bool worktree = false,
        bool sandbox = false,
        bool systemPrompt = false,
        bool addDirs = false,
        bool streamingInput = false)
    {
        return new ProviderCapability
        {
            Provider = provider,
            DefaultModel = "default",
            AvailableModels = [],
            SizeMappings = new SizeMappings(),
            Features = new Features
            {
                Interactive = new FeatureSupport { Supported = true },
                NonInteractive = new FeatureSupport { Supported = true },
                JsonOutput = new FeatureSupport { Supported = true },
                StreamJson = new FeatureSupport { Supported = true },
                AutoApprove = new FeatureSupport { Supported = true },
                Worktree = new FeatureSupport { Supported = worktree },
                Sandbox = new FeatureSupport { Supported = sandbox },
                SystemPrompt = new FeatureSupport { Supported = systemPrompt },
                AddDirs = new FeatureSupport { Supported = addDirs },
                StreamingInput = new FeatureSupport { Supported = streamingInput },
            },
        };
    }

    [Fact]
    public async Task AddDir_OnOllama_Throws()
    {
        VersionCheck.SetVersionForTesting("zag", "9.9.9");
        CapabilityCheck.ClearCapabilityCache();
        CapabilityCheck.SetAllCapabilitiesForTesting("zag",
            [
                FakeCap("claude", addDirs: true),
                FakeCap("ollama", addDirs: false),
            ]);
        try
        {
            var builder = new ZagBuilder().Provider("ollama").AddDir("/extra");
            var ex = await Assert.ThrowsAsync<ZagFeatureUnsupportedException>(() =>
                builder.ExecAsync("hello"));
            Assert.Equal("AddDir()", ex.Method);
            Assert.Equal("ollama", ex.Provider);
        }
        finally
        {
            CapabilityCheck.ClearCapabilityCache();
            VersionCheck.ClearVersionCache();
        }
    }

    [Fact]
    public async Task ExecStreaming_OnGemini_Throws()
    {
        VersionCheck.SetVersionForTesting("zag", "9.9.9");
        CapabilityCheck.ClearCapabilityCache();
        CapabilityCheck.SetAllCapabilitiesForTesting("zag",
            [
                FakeCap("claude", streamingInput: true),
                FakeCap("gemini", streamingInput: false),
            ]);
        try
        {
            var builder = new ZagBuilder().Provider("gemini");
            var ex = await Assert.ThrowsAsync<ZagFeatureUnsupportedException>(() =>
                builder.ExecStreaming("hi"));
            Assert.Equal("ExecStreaming()", ex.Method);
            Assert.Equal("gemini", ex.Provider);
            Assert.Contains("claude", ex.SupportedProviders);
        }
        finally
        {
            CapabilityCheck.ClearCapabilityCache();
            VersionCheck.ClearVersionCache();
        }
    }
}

public class ZagFeatureUnsupportedExceptionTests
{
    [Fact]
    public void MessageFormat_ContainsKeyParts()
    {
        var ex = new ZagFeatureUnsupportedException(
            "ExecStreaming()",
            "streaming_input",
            "ollama",
            new[] { "claude" });

        Assert.Contains("ExecStreaming()", ex.Message);
        Assert.Contains("ollama", ex.Message);
        Assert.Contains("streaming_input", ex.Message);
        Assert.Contains("claude", ex.Message);
    }

    [Fact]
    public void EmptySupportedList_ShowsNone()
    {
        var ex = new ZagFeatureUnsupportedException(
            "Sandbox()", "sandbox", "ollama", Array.Empty<string>());
        Assert.Contains("(none)", ex.Message);
    }

    [Fact]
    public void IsZagException()
    {
        var ex = new ZagFeatureUnsupportedException(
            "Worktree()", "worktree", "x", Array.Empty<string>());
        Assert.IsAssignableFrom<ZagException>(ex);
    }
}
