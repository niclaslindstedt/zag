import Foundation
import Testing
@testable import Zag

// MARK: - Discover model deserialization tests

@Suite("DiscoverModels")
struct DiscoverModelsTests {

    static let capabilityJSON = """
    {
        "provider": "claude",
        "default_model": "sonnet",
        "available_models": ["haiku", "sonnet", "opus"],
        "size_mappings": {
            "small": "haiku",
            "medium": "sonnet",
            "large": "opus"
        },
        "features": {
            "interactive": {"supported": true, "native": true},
            "non_interactive": {"supported": true, "native": true},
            "resume": {"supported": true, "native": true},
            "resume_with_prompt": {"supported": true, "native": true},
            "session_logs": {"supported": true, "native": true, "completeness": "full"},
            "json_output": {"supported": true, "native": true},
            "stream_json": {"supported": true, "native": true},
            "json_schema": {"supported": true, "native": true},
            "input_format": {"supported": true, "native": true},
            "streaming_input": {"supported": true, "native": true, "semantics": "queue"},
            "worktree": {"supported": true, "native": false},
            "sandbox": {"supported": true, "native": false},
            "system_prompt": {"supported": true, "native": true},
            "auto_approve": {"supported": true, "native": true},
            "review": {"supported": false, "native": false},
            "add_dirs": {"supported": true, "native": true},
            "max_turns": {"supported": true, "native": true}
        }
    }
    """

    static let allCapabilitiesJSON = """
    [
        {
            "provider": "claude",
            "default_model": "sonnet",
            "available_models": ["haiku", "sonnet", "opus"],
            "size_mappings": {"small": "haiku", "medium": "sonnet", "large": "opus"},
            "features": {
                "interactive": {"supported": true, "native": true},
                "non_interactive": {"supported": true, "native": true},
                "resume": {"supported": true, "native": true},
                "resume_with_prompt": {"supported": true, "native": true},
                "session_logs": {"supported": true, "native": true, "completeness": "full"},
                "json_output": {"supported": true, "native": true},
                "stream_json": {"supported": true, "native": true},
                "json_schema": {"supported": true, "native": true},
                "input_format": {"supported": true, "native": true},
                "streaming_input": {"supported": true, "native": true, "semantics": "queue"},
                "worktree": {"supported": true, "native": false},
                "sandbox": {"supported": true, "native": false},
                "system_prompt": {"supported": true, "native": true},
                "auto_approve": {"supported": true, "native": true},
                "review": {"supported": false, "native": false},
                "add_dirs": {"supported": true, "native": true},
                "max_turns": {"supported": true, "native": true}
            }
        },
        {
            "provider": "codex",
            "default_model": "codex-mini",
            "available_models": ["codex-mini"],
            "size_mappings": {"small": "codex-mini", "medium": "codex-mini", "large": "codex-mini"},
            "features": {
                "interactive": {"supported": true, "native": true},
                "non_interactive": {"supported": true, "native": true},
                "resume": {"supported": false, "native": false},
                "resume_with_prompt": {"supported": false, "native": false},
                "session_logs": {"supported": false, "native": false},
                "json_output": {"supported": true, "native": true},
                "stream_json": {"supported": false, "native": false},
                "json_schema": {"supported": false, "native": false},
                "input_format": {"supported": false, "native": false},
                "streaming_input": {"supported": false, "native": false},
                "worktree": {"supported": false, "native": false},
                "sandbox": {"supported": false, "native": false},
                "system_prompt": {"supported": false, "native": false},
                "auto_approve": {"supported": true, "native": true},
                "review": {"supported": false, "native": false},
                "add_dirs": {"supported": false, "native": false},
                "max_turns": {"supported": false, "native": false}
            }
        }
    ]
    """

    static let resolvedModelJSON = """
    {
        "input": "small",
        "resolved": "haiku",
        "is_alias": true,
        "provider": "claude"
    }
    """

    @Test("ProviderCapability deserializes from JSON")
    func providerCapabilityDeserializes() throws {
        let data = DiscoverModelsTests.capabilityJSON.data(using: .utf8)!
        let cap = try JSONDecoder().decode(ProviderCapability.self, from: data)

        #expect(cap.provider == "claude")
        #expect(cap.defaultModel == "sonnet")
        #expect(cap.availableModels == ["haiku", "sonnet", "opus"])
        #expect(cap.sizeMappings.small == "haiku")
        #expect(cap.sizeMappings.medium == "sonnet")
        #expect(cap.sizeMappings.large == "opus")
    }

    @Test("Features deserialize all 17 fields")
    func featuresDeserializeAllFields() throws {
        let data = DiscoverModelsTests.capabilityJSON.data(using: .utf8)!
        let cap = try JSONDecoder().decode(ProviderCapability.self, from: data)
        let f = cap.features

        #expect(f.interactive.supported == true)
        #expect(f.interactive.native == true)
        #expect(f.nonInteractive.supported == true)
        #expect(f.resume.supported == true)
        #expect(f.resumeWithPrompt.supported == true)
        #expect(f.sessionLogs.supported == true)
        #expect(f.sessionLogs.completeness == "full")
        #expect(f.jsonOutput.supported == true)
        #expect(f.streamJson.supported == true)
        #expect(f.jsonSchema.supported == true)
        #expect(f.inputFormat.supported == true)
        #expect(f.streamingInput.supported == true)
        #expect(f.streamingInput.semantics == "queue")
        #expect(f.worktree.supported == true)
        #expect(f.worktree.native == false)
        #expect(f.sandbox.supported == true)
        #expect(f.systemPrompt.supported == true)
        #expect(f.autoApprove.supported == true)
        #expect(f.review.supported == false)
        #expect(f.addDirs.supported == true)
        #expect(f.maxTurns.supported == true)
    }

    @Test("StreamingInputSupport without semantics")
    func streamingInputSupportWithoutSemantics() throws {
        let json = """
        {"supported": false, "native": false}
        """
        let data = json.data(using: .utf8)!
        let sis = try JSONDecoder().decode(StreamingInputSupport.self, from: data)
        #expect(sis.supported == false)
        #expect(sis.native == false)
        #expect(sis.semantics == nil)
    }

    @Test("SessionLogSupport without completeness")
    func sessionLogSupportWithoutCompleteness() throws {
        let json = """
        {"supported": true, "native": false}
        """
        let data = json.data(using: .utf8)!
        let sls = try JSONDecoder().decode(SessionLogSupport.self, from: data)
        #expect(sls.supported == true)
        #expect(sls.native == false)
        #expect(sls.completeness == nil)
    }

    @Test("array of ProviderCapability deserializes")
    func allCapabilitiesDeserialize() throws {
        let data = DiscoverModelsTests.allCapabilitiesJSON.data(using: .utf8)!
        let caps = try JSONDecoder().decode([ProviderCapability].self, from: data)

        #expect(caps.count == 2)
        #expect(caps[0].provider == "claude")
        #expect(caps[1].provider == "codex")
    }

    @Test("ResolvedModel deserializes from JSON")
    func resolvedModelDeserializes() throws {
        let data = DiscoverModelsTests.resolvedModelJSON.data(using: .utf8)!
        let rm = try JSONDecoder().decode(ResolvedModel.self, from: data)

        #expect(rm.input == "small")
        #expect(rm.resolved == "haiku")
        #expect(rm.isAlias == true)
        #expect(rm.provider == "claude")
    }

    @Test("ResolvedModel non-alias")
    func resolvedModelNonAlias() throws {
        let json = """
        {"input": "opus", "resolved": "opus", "is_alias": false, "provider": "claude"}
        """
        let data = json.data(using: .utf8)!
        let rm = try JSONDecoder().decode(ResolvedModel.self, from: data)

        #expect(rm.input == "opus")
        #expect(rm.resolved == "opus")
        #expect(rm.isAlias == false)
    }

    @Test("SizeMappings deserializes")
    func sizeMappingsDeserializes() throws {
        let json = """
        {"small": "haiku", "medium": "sonnet", "large": "opus"}
        """
        let data = json.data(using: .utf8)!
        let sm = try JSONDecoder().decode(SizeMappings.self, from: data)

        #expect(sm.small == "haiku")
        #expect(sm.medium == "sonnet")
        #expect(sm.large == "opus")
    }

    @Test("FeatureSupport deserializes")
    func featureSupportDeserializes() throws {
        let json = """
        {"supported": true, "native": false}
        """
        let data = json.data(using: .utf8)!
        let fs = try JSONDecoder().decode(FeatureSupport.self, from: data)

        #expect(fs.supported == true)
        #expect(fs.native == false)
    }
}

// MARK: - Discover function tests (require zag binary)

#if os(macOS) || os(Linux)
@Suite("ZagDiscover")
struct ZagDiscoverTests {

    @Test("listProviders returns provider names")
    func listProvidersReturnsNames() async throws {
        let providers = try await ZagDiscover.listProviders()
        #expect(providers.count >= 5)
        #expect(providers.contains("claude"))
        #expect(providers.contains("codex"))
        #expect(providers.contains("gemini"))
        #expect(providers.contains("copilot"))
        #expect(providers.contains("ollama"))
    }

    @Test("getCapability returns single provider")
    func getCapabilityReturnsSingleProvider() async throws {
        let cap = try await ZagDiscover.getCapability(provider: "claude")
        #expect(cap.provider == "claude")
        #expect(cap.availableModels.count > 0)
        #expect(cap.features.interactive.supported == true)
    }

    @Test("getAllCapabilities returns all providers")
    func getAllCapabilitiesReturnsAll() async throws {
        let caps = try await ZagDiscover.getAllCapabilities()
        #expect(caps.count >= 5)
        let names = caps.map { $0.provider }
        #expect(names.contains("claude"))
    }

    @Test("resolveModel resolves alias")
    func resolveModelResolvesAlias() async throws {
        let rm = try await ZagDiscover.resolveModel(provider: "claude", model: "small")
        #expect(rm.input == "small")
        #expect(rm.resolved == "haiku")
        #expect(rm.isAlias == true)
        #expect(rm.provider == "claude")
    }

    @Test("resolveModel passes through non-alias")
    func resolveModelPassesThrough() async throws {
        let rm = try await ZagDiscover.resolveModel(provider: "claude", model: "opus")
        #expect(rm.input == "opus")
        #expect(rm.resolved == "opus")
        #expect(rm.isAlias == false)
    }
}
#endif
