using Xunit;
using Zag;

namespace Zag.Tests;

/// <summary>
/// These tests require the zag binary to be built and available in PATH.
/// Run with: dotnet test
/// </summary>
public class ZagDiscoverTests
{
    [Fact]
    public async Task ListProviders_ReturnsAtLeastFiveProviders()
    {
        var providers = await ZagDiscover.ListProvidersAsync();
        Assert.True(providers.Length >= 5, $"Expected at least 5 providers, got {providers.Length}");
        Assert.Contains("claude", providers);
        Assert.Contains("codex", providers);
        Assert.Contains("gemini", providers);
        Assert.Contains("copilot", providers);
        Assert.Contains("ollama", providers);
    }

    [Fact]
    public async Task GetCapability_ReturnsCorrectProvider()
    {
        var cap = await ZagDiscover.GetCapabilityAsync("claude");
        Assert.Equal("claude", cap.Provider);
        Assert.NotEmpty(cap.AvailableModels);
        Assert.True(cap.Features.Interactive.Supported);
    }

    [Fact]
    public async Task GetAllCapabilities_ReturnsAllProviders()
    {
        var caps = await ZagDiscover.GetAllCapabilitiesAsync();
        Assert.True(caps.Length >= 5, $"Expected at least 5 capabilities, got {caps.Length}");
        var names = caps.Select(c => c.Provider).ToArray();
        Assert.Contains("claude", names);
    }

    [Fact]
    public async Task ResolveModel_ResolvesAlias()
    {
        var rm = await ZagDiscover.ResolveModelAsync("claude", "small");
        Assert.Equal("small", rm.Input);
        Assert.Equal("haiku", rm.Resolved);
        Assert.True(rm.IsAlias);
        Assert.Equal("claude", rm.Provider);
    }

    [Fact]
    public async Task ResolveModel_PassesThroughNonAlias()
    {
        var rm = await ZagDiscover.ResolveModelAsync("claude", "opus");
        Assert.Equal("opus", rm.Input);
        Assert.Equal("opus", rm.Resolved);
        Assert.False(rm.IsAlias);
    }
}
