import Foundation
import Testing
@testable import Zag

// MARK: - Helpers

private func fakeFeatures(
    worktree: Bool = false,
    sandbox: Bool = false,
    systemPrompt: Bool = false,
    addDirs: Bool = false,
    streamingInput: Bool = false
) -> Features {
    func fs(_ b: Bool) -> FeatureSupport { FeatureSupport(supported: b, native: false) }
    return Features(
        interactive: fs(true),
        nonInteractive: fs(true),
        resume: fs(false),
        resumeWithPrompt: fs(false),
        sessionLogs: SessionLogSupport(supported: false, native: false, completeness: nil),
        jsonOutput: fs(true),
        streamJson: fs(true),
        jsonSchema: fs(false),
        inputFormat: fs(false),
        streamingInput: fs(streamingInput),
        worktree: fs(worktree),
        sandbox: fs(sandbox),
        systemPrompt: fs(systemPrompt),
        autoApprove: fs(true),
        review: fs(false),
        addDirs: fs(addDirs),
        maxTurns: fs(false)
    )
}

private func fakeCap(
    _ provider: String,
    worktree: Bool = false,
    sandbox: Bool = false,
    systemPrompt: Bool = false,
    addDirs: Bool = false,
    streamingInput: Bool = false
) -> ProviderCapability {
    ProviderCapability(
        provider: provider,
        defaultModel: "default",
        availableModels: [],
        sizeMappings: SizeMappings(small: "", medium: "", large: ""),
        features: fakeFeatures(
            worktree: worktree,
            sandbox: sandbox,
            systemPrompt: systemPrompt,
            addDirs: addDirs,
            streamingInput: streamingInput
        )
    )
}

private func primeCaps(_ bin: String, _ caps: [ProviderCapability]) {
    CapabilityCheck.clearCapabilityCache()
    CapabilityCheck.setAllCapabilitiesForTesting(bin: bin, caps: caps)
}

// MARK: - CapabilityCheck

@Suite("CapabilityCheck")
struct CapabilityCheckTests {

    @Test("no requirements returns silently")
    func noRequirements() async throws {
        primeCaps("zag", [fakeCap("ollama")])
        defer { CapabilityCheck.clearCapabilityCache() }

        try await CapabilityCheck.check(
            bin: "zag",
            provider: "ollama",
            requirements: []
        )
    }

    @Test("inactive requirements return silently")
    func inactiveRequirements() async throws {
        primeCaps("zag", [fakeCap("ollama")])
        defer { CapabilityCheck.clearCapabilityCache() }

        try await CapabilityCheck.check(
            bin: "zag",
            provider: "ollama",
            requirements: [
                CapabilityCheck.Requirement(method: "addDir()", feature: "add_dirs", isSet: false)
            ]
        )
    }

    @Test("nil provider skips")
    func nilProviderSkips() async throws {
        // No cache primed; would raise if we tried to load.
        try await CapabilityCheck.check(
            bin: "zag",
            provider: nil,
            requirements: [
                CapabilityCheck.Requirement(method: "addDir()", feature: "add_dirs", isSet: true)
            ]
        )
    }

    @Test("mock provider skips")
    func mockProviderSkips() async throws {
        try await CapabilityCheck.check(
            bin: "zag",
            provider: "mock",
            requirements: [
                CapabilityCheck.Requirement(method: "addDir()", feature: "add_dirs", isSet: true)
            ]
        )
    }

    @Test("supported feature passes")
    func supportedFeature() async throws {
        primeCaps("zag", [fakeCap("claude", streamingInput: true)])
        defer { CapabilityCheck.clearCapabilityCache() }

        try await CapabilityCheck.check(
            bin: "zag",
            provider: "claude",
            requirements: [
                CapabilityCheck.Requirement(method: "execStreaming()", feature: "streaming_input", isSet: true)
            ]
        )
    }

    @Test("unsupported feature raises")
    func unsupportedFeature() async {
        primeCaps("zag", [
            fakeCap("claude", streamingInput: true),
            fakeCap("ollama", streamingInput: false),
        ])
        defer { CapabilityCheck.clearCapabilityCache() }

        do {
            try await CapabilityCheck.check(
                bin: "zag",
                provider: "ollama",
                requirements: [
                    CapabilityCheck.Requirement(method: "execStreaming()", feature: "streaming_input", isSet: true)
                ]
            )
            Issue.record("Expected ZagFeatureUnsupportedError")
        } catch let error as ZagFeatureUnsupportedError {
            #expect(error.method == "execStreaming()")
            #expect(error.feature == "streaming_input")
            #expect(error.provider == "ollama")
            #expect(error.supportedProviders.contains("claude"))
            #expect(!error.supportedProviders.contains("ollama"))
        } catch {
            Issue.record("Expected ZagFeatureUnsupportedError, got: \(error)")
        }
    }

    @Test("unsupported with no supporters shows (none)")
    func unsupportedNoSupporters() async {
        primeCaps("zag", [fakeCap("ollama")])
        defer { CapabilityCheck.clearCapabilityCache() }

        do {
            try await CapabilityCheck.check(
                bin: "zag",
                provider: "ollama",
                requirements: [
                    CapabilityCheck.Requirement(method: "sandbox()", feature: "sandbox", isSet: true)
                ]
            )
            Issue.record("Expected ZagFeatureUnsupportedError")
        } catch let error as ZagFeatureUnsupportedError {
            #expect(error.message.contains("(none)"))
        } catch {
            Issue.record("Expected ZagFeatureUnsupportedError, got: \(error)")
        }
    }
}

// MARK: - ZagBuilder capability preflight

@Suite("ZagBuilder capability preflight")
struct ZagBuilderCapabilityPreflightTests {

    @Test("addDir on ollama throws")
    func addDirOnOllamaThrows() async {
        VersionCheck.setVersionForTesting(bin: "zag", version: "9.9.9")
        primeCaps("zag", [
            fakeCap("claude", addDirs: true),
            fakeCap("ollama", addDirs: false),
        ])
        defer {
            CapabilityCheck.clearCapabilityCache()
            VersionCheck.clearVersionCache()
        }

        let builder = ZagBuilder().provider("ollama").addDir("/extra")
        do {
            _ = try await builder.exec("hello")
            Issue.record("Expected ZagFeatureUnsupportedError")
        } catch let error as ZagFeatureUnsupportedError {
            #expect(error.method == "addDir()")
            #expect(error.provider == "ollama")
        } catch {
            Issue.record("Expected ZagFeatureUnsupportedError, got: \(error)")
        }
    }

    @Test("execStreaming on gemini throws")
    func execStreamingOnGeminiThrows() async {
        VersionCheck.setVersionForTesting(bin: "zag", version: "9.9.9")
        primeCaps("zag", [
            fakeCap("claude", streamingInput: true),
            fakeCap("gemini", streamingInput: false),
        ])
        defer {
            CapabilityCheck.clearCapabilityCache()
            VersionCheck.clearVersionCache()
        }

        let builder = ZagBuilder().provider("gemini")
        do {
            _ = try await builder.execStreaming("hi")
            Issue.record("Expected ZagFeatureUnsupportedError")
        } catch let error as ZagFeatureUnsupportedError {
            #expect(error.method == "execStreaming()")
            #expect(error.provider == "gemini")
            #expect(error.supportedProviders.contains("claude"))
        } catch {
            Issue.record("Expected ZagFeatureUnsupportedError, got: \(error)")
        }
    }
}

// MARK: - ZagFeatureUnsupportedError

@Suite("ZagFeatureUnsupportedError")
struct ZagFeatureUnsupportedErrorTests {

    @Test("message format contains key parts")
    func messageFormat() {
        let err = ZagFeatureUnsupportedError(
            method: "execStreaming()",
            feature: "streaming_input",
            provider: "ollama",
            supportedProviders: ["claude"]
        )
        #expect(err.message.contains("execStreaming()"))
        #expect(err.message.contains("ollama"))
        #expect(err.message.contains("streaming_input"))
        #expect(err.message.contains("claude"))
    }

    @Test("empty supported list shows (none)")
    func emptySupportedList() {
        let err = ZagFeatureUnsupportedError(
            method: "sandbox()",
            feature: "sandbox",
            provider: "ollama",
            supportedProviders: []
        )
        #expect(err.message.contains("(none)"))
    }

    @Test("asZagError provides a ZagError view")
    func asZagError() {
        let err = ZagFeatureUnsupportedError(
            method: "worktree()",
            feature: "worktree",
            provider: "x",
            supportedProviders: []
        )
        let zErr = err.asZagError
        #expect(zErr.message == err.message)
        #expect(zErr.exitCode == nil)
    }
}
