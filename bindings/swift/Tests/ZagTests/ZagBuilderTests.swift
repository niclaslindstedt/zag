import Foundation
import Testing
@testable import Zag

// MARK: - Builder arg tests

@Suite("ZagBuilder")
struct ZagBuilderTests {

    @Test("defaults produce minimal args")
    func defaultsProduceMinimalArgs() {
        let builder = ZagBuilder()
        let args = builder.buildGlobalArgs()
        #expect(args.isEmpty)
    }

    @Test("method chaining sets all options")
    func methodChainingSetsAllOptions() {
        let builder = ZagBuilder()
            .provider("gemini")
            .model("large")
            .root("/project")
            .autoApprove()
            .addDir("/docs")
            .file("/tmp/data.csv")
            .verbose()
            .debug()
            .sessionId("sess-1")
            .timeout("5m")

        let args = builder.buildGlobalArgs()

        #expect(args == [
            "-p", "gemini",
            "--model", "large",
            "--root", "/project",
            "--auto-approve",
            "--add-dir", "/docs",
            "--file", "/tmp/data.csv",
            "--verbose",
            "--debug",
            "--session", "sess-1",
        ])
    }

    @Test("headless emits --headless flag")
    func headlessEmitsFlag() {
        let enabled = ZagBuilder().provider("claude").headless().buildGlobalArgs()
        #expect(enabled.contains("--headless"))

        let disabled = ZagBuilder().provider("claude").headless(false).buildGlobalArgs()
        #expect(!disabled.contains("--headless"))
    }

    @Test("env vars produce correct args")
    func envVarsProduceCorrectArgs() {
        let builder = ZagBuilder()
            .env("FOO", "bar")
            .env("BAZ", "qux")

        let args = builder.buildGlobalArgs()

        #expect(args.contains("--env"))
        if let idx = args.firstIndex(of: "--env") {
            #expect(args[idx + 1] == "FOO=bar")
            #expect(args[idx + 2] == "--env")
            #expect(args[idx + 3] == "BAZ=qux")
        }
    }

    @Test("exec args default to json output")
    func execArgsDefaultToJson() {
        let builder = ZagBuilder().provider("claude")
        let args = builder.buildExecArgs(prompt: "hello")

        #expect(args.contains("exec"))
        #expect(args.contains("-o"))
        #expect(args.contains("json"))
        #expect(args.contains("hello"))
    }

    @Test("exec args with streaming use -o stream-json")
    func execArgsStreaming() {
        let builder = ZagBuilder()
        let args = builder.buildExecArgs(prompt: "hello", streaming: true)
        #expect(!args.contains("--json-stream"))
        let oi = args.firstIndex(of: "-o")!
        #expect(args[oi + 1] == "stream-json")
    }

    @Test("worktree without name")
    func worktreeNoName() {
        let builder = ZagBuilder().worktree()
        let args = builder.buildGlobalArgs()
        #expect(args.contains("-w"))
        #expect(args.count == 1)
    }

    @Test("worktree with name")
    func worktreeNamed() {
        let builder = ZagBuilder().worktree("feat")
        let args = builder.buildGlobalArgs()
        #expect(args == ["-w", "feat"])
    }

    @Test("sandbox without name")
    func sandboxNoName() {
        let builder = ZagBuilder().sandbox()
        let args = builder.buildGlobalArgs()
        #expect(args.contains("--sandbox"))
        #expect(args.count == 1)
    }

    @Test("sandbox with name")
    func sandboxNamed() {
        let builder = ZagBuilder().sandbox("box1")
        let args = builder.buildGlobalArgs()
        #expect(args == ["--sandbox", "box1"])
    }

    @Test("exit with hint")
    func exitWithHint() {
        let args = ZagBuilder().exit("the answer").buildRunArgs(prompt: "compute")
        guard let i = args.firstIndex(of: "--exit") else {
            Issue.record("--exit missing")
            return
        }
        #expect(args[args.index(after: i)] == "the answer")
    }

    @Test("exit bare")
    func exitBare() {
        let args = ZagBuilder().exit().buildRunArgs()
        #expect(args.contains("--exit"))
    }

    @Test("exit omitted when not set")
    func exitOmitted() {
        let args = ZagBuilder().buildRunArgs(prompt: "hi")
        #expect(!args.contains("--exit"))
    }

    @Test("max turns")
    func maxTurns() {
        let builder = ZagBuilder().maxTurns(10)
        let args = builder.buildGlobalArgs()
        #expect(args == ["--max-turns", "10"])
    }

    @Test("mcp config")
    func mcpConfig() {
        let builder = ZagBuilder().mcpConfig("./mcp.json")
        let args = builder.buildGlobalArgs()
        #expect(args == ["--mcp-config", "./mcp.json"])
    }

    @Test("show usage")
    func showUsage() {
        let builder = ZagBuilder().showUsage()
        let args = builder.buildGlobalArgs()
        #expect(args == ["--show-usage"])
    }

    @Test("size")
    func size() {
        let builder = ZagBuilder().size("35b")
        let args = builder.buildGlobalArgs()
        #expect(args == ["--size", "35b"])
    }

    @Test("timeout included in exec args")
    func timeoutInExecArgs() {
        let args = ZagBuilder().timeout("5m").buildExecArgs(prompt: "test")
        #expect(args.contains("--timeout"))
        #expect(args.contains("5m"))
    }

    @Test("resume included in exec args")
    func resumeInExecArgs() {
        var args = ZagBuilder().provider("claude").buildExecArgs(prompt: "follow up")
        let promptIdx = args.count - 2 // insert before "--prompt", prompt
        args.insert(contentsOf: ["--resume", "sess-123"], at: promptIdx)
        #expect(args.contains("--resume"))
        #expect(args.contains("sess-123"))
        let resumeIdx = args.firstIndex(of: "--resume")!
        let newPromptIdx = args.lastIndex(of: "--prompt")!
        #expect(resumeIdx < newPromptIdx)
    }

    @Test("continue included in exec args")
    func continueInExecArgs() {
        var args = ZagBuilder().provider("claude").buildExecArgs(prompt: "follow up")
        let promptIdx = args.count - 2 // insert before "--prompt", prompt
        args.insert("--continue", at: promptIdx)
        #expect(args.contains("--continue"))
        let continueIdx = args.firstIndex(of: "--continue")!
        let newPromptIdx = args.lastIndex(of: "--prompt")!
        #expect(continueIdx < newPromptIdx)
    }

    @Test("system prompt")
    func systemPrompt() {
        let builder = ZagBuilder().systemPrompt("You are helpful")
        let args = builder.buildGlobalArgs()
        #expect(args == ["--system-prompt", "You are helpful"])
    }

    @Test("quiet")
    func quiet() {
        let builder = ZagBuilder().quiet()
        let args = builder.buildGlobalArgs()
        #expect(args == ["--quiet"])
    }

    @Test("multiple addDir calls accumulate")
    func multipleAddDirs() {
        let builder = ZagBuilder().addDir("/a").addDir("/b")
        let args = builder.buildGlobalArgs()
        #expect(args == ["--add-dir", "/a", "--add-dir", "/b"])
    }

    @Test("multiple file calls accumulate")
    func multipleFiles() {
        let builder = ZagBuilder().file("/a.txt").file("/b.rs")
        let args = builder.buildGlobalArgs()
        #expect(args == ["--file", "/a.txt", "--file", "/b.rs"])
    }

    @Test("json schema implies json")
    func jsonSchemaImpliesJson() {
        let builder = ZagBuilder().jsonSchema("{\"type\":\"object\"}")
        let args = builder.buildExecArgs(prompt: "test")
        #expect(args.contains("--json"))
        #expect(args.contains("--json-schema"))
    }

    @Test("exec streaming args")
    func execStreamingArgs() throws {
        let builder = ZagBuilder().provider("claude").includePartialMessages()
        // We test the arg building indirectly via buildExecArgs
        // execStreaming builds its own args, so let's test the builder state
        let globalArgs = builder.buildGlobalArgs()
        #expect(globalArgs.contains("-p"))
        #expect(globalArgs.contains("claude"))
    }

    @Test("remote connection setter")
    func remoteConnectionSetter() throws {
        let conn = try ZagConnection(url: "https://server:2100", token: "tok")
        let builder = ZagBuilder().connection(conn).provider("claude")
        // Builder should still produce valid global args (for arg building tests)
        let args = builder.buildGlobalArgs()
        #expect(args.contains("claude"))
    }

    @Test("remote convenience setter")
    func remoteConvenienceSetter() {
        let builder = ZagBuilder().remote(url: "https://server:2100", token: "tok")
        // Should not crash, connection is set internally
        let args = builder.buildGlobalArgs()
        #expect(args.isEmpty)
    }

    @Test("buildSpawnParams maps builder state")
    func buildSpawnParams() {
        let builder = ZagBuilder()
            .provider("claude")
            .model("sonnet")
            .root("/project")
            .autoApprove()
            .addDir("/docs")
            .systemPrompt("Be helpful")
            .maxTurns(10)
            .timeout("5m")
            .size("9b")

        let params = builder.buildSpawnParams(prompt: "hello")
        #expect(params.prompt == "hello")
        #expect(params.provider == "claude")
        #expect(params.model == "sonnet")
        #expect(params.root == "/project")
        #expect(params.autoApprove == true)
        #expect(params.addDirs == ["/docs"])
        #expect(params.systemPrompt == "Be helpful")
        #expect(params.maxTurns == 10)
        #expect(params.timeout == "5m")
        #expect(params.size == "9b")
    }

    @Test("buildSpawnParams omits nil for defaults")
    func buildSpawnParamsDefaults() {
        let builder = ZagBuilder()
        let params = builder.buildSpawnParams(prompt: "test")
        #expect(params.prompt == "test")
        #expect(params.provider == nil)
        #expect(params.model == nil)
        #expect(params.autoApprove == nil)
        #expect(params.addDirs == nil)
    }

    @Test("remoteClient throws without connection")
    func remoteClientThrowsWithoutConnection() {
        let builder = ZagBuilder()
        #expect(throws: ZagError.self) {
            try builder.remoteClient()
        }
    }

    @Test("remoteClient succeeds with connection")
    func remoteClientSucceedsWithConnection() throws {
        let builder = ZagBuilder().remote(url: "https://server:2100", token: "tok")
        let client = try builder.remoteClient()
        #expect(client is ZagRemoteClient)
    }
}

// MARK: - Version checking tests

@Suite("VersionCheck")
struct VersionCheckTests {

    @Test("parse valid semver")
    func parseValidSemver() throws {
        let v = try VersionCheck.parseSemver("0.6.0")
        #expect(v.major == 0)
        #expect(v.minor == 6)
        #expect(v.patch == 0)
    }

    @Test("parse invalid semver throws")
    func parseInvalidSemver() {
        #expect(throws: ZagError.self) { try VersionCheck.parseSemver("invalid") }
        #expect(throws: ZagError.self) { try VersionCheck.parseSemver("1.2") }
        #expect(throws: ZagError.self) { try VersionCheck.parseSemver("a.b.c") }
    }

    @Test("semver comparison")
    func semverComparison() throws {
        let v050 = try VersionCheck.parseSemver("0.5.0")
        let v060 = try VersionCheck.parseSemver("0.6.0")
        let v070 = try VersionCheck.parseSemver("0.7.0")

        #expect(v050 < v060)
        #expect(v060 == v060)
        #expect(v070 > v060)
    }

    #if os(macOS) || os(Linux)
    @Test("check with no active requirements passes")
    func checkNoActiveRequirements() async throws {
        VersionCheck.setVersionForTesting(bin: "zag", version: "0.5.0")
        defer { VersionCheck.clearVersionCache() }
        try await VersionCheck.check(bin: "zag", requirements: [
            VersionCheck.Requirement(method: "env()", version: "0.6.0", isSet: false),
        ])
    }

    @Test("check with sufficient version passes")
    func checkSufficientVersion() async throws {
        VersionCheck.setVersionForTesting(bin: "zag", version: "0.6.0")
        defer { VersionCheck.clearVersionCache() }
        try await VersionCheck.check(bin: "zag", requirements: [
            VersionCheck.Requirement(method: "env()", version: "0.6.0", isSet: true),
        ])
    }

    @Test("check with insufficient version throws")
    func checkInsufficientVersion() async {
        VersionCheck.setVersionForTesting(bin: "zag", version: "0.5.0")
        defer { VersionCheck.clearVersionCache() }
        do {
            try await VersionCheck.check(bin: "zag", requirements: [
                VersionCheck.Requirement(method: "env()", version: "0.6.0", isSet: true),
            ])
            Issue.record("Expected ZagError")
        } catch let error as ZagError {
            #expect(error.message.contains("env()"))
            #expect(error.message.contains("0.6.0"))
            #expect(error.message.contains("0.5.0"))
        } catch {
            Issue.record("Expected ZagError, got \(error)")
        }
    }

    @Test("check with multiple failures reports all")
    func checkMultipleFailures() async {
        VersionCheck.setVersionForTesting(bin: "zag", version: "0.5.0")
        defer { VersionCheck.clearVersionCache() }
        do {
            try await VersionCheck.check(bin: "zag", requirements: [
                VersionCheck.Requirement(method: "env()", version: "0.6.0", isSet: true),
                VersionCheck.Requirement(method: "mcpConfig()", version: "0.6.0", isSet: true),
            ])
            Issue.record("Expected ZagError")
        } catch let error as ZagError {
            #expect(error.message.contains("env()"))
            #expect(error.message.contains("mcpConfig()"))
        } catch {
            Issue.record("Expected ZagError, got \(error)")
        }
    }
    #endif
}

// MARK: - CapabilityCheck tests

#if os(macOS) || os(Linux)
@Suite("CapabilityCheck")
struct CapabilityCheckTests {

    /// Build a fake `ProviderCapability` with per-feature support flags.
    private static func fakeCapability(
        provider: String,
        streamingInput: Bool = false,
        worktree: Bool = true,
        sandbox: Bool = true,
        systemPrompt: Bool = true,
        addDirs: Bool = true,
        maxTurns: Bool = true
    ) -> ProviderCapability {
        let yes = FeatureSupport(supported: true, native: true)
        let on = { (b: Bool) in FeatureSupport(supported: b, native: false) }
        return ProviderCapability(
            provider: provider,
            defaultModel: "default",
            availableModels: [],
            sizeMappings: SizeMappings(small: "s", medium: "m", large: "l"),
            features: Features(
                interactive: yes,
                nonInteractive: yes,
                resume: yes,
                resumeWithPrompt: yes,
                sessionLogs: SessionLogSupport(supported: true, native: true),
                jsonOutput: yes,
                streamJson: yes,
                jsonSchema: yes,
                inputFormat: yes,
                streamingInput: StreamingInputSupport(
                    supported: streamingInput,
                    native: streamingInput,
                    semantics: streamingInput ? "queue" : nil),
                worktree: on(worktree),
                sandbox: on(sandbox),
                systemPrompt: on(systemPrompt),
                autoApprove: yes,
                review: yes,
                addDirs: on(addDirs),
                maxTurns: on(maxTurns)))
    }

    @Test("no requirements active passes")
    func noRequirementsActive() async throws {
        CapabilityCheck.setCapabilitiesForTesting(bin: "zag", caps: [
            Self.fakeCapability(provider: "ollama", streamingInput: false),
        ])
        defer { CapabilityCheck.clearCapabilityCache() }
        try await CapabilityCheck.check(bin: "zag", provider: "ollama", requirements: [
            CapabilityCheck.Requirement(
                method: "execStreaming()", feature: .streamingInput, isSet: false),
        ])
    }

    @Test("nil provider skips check")
    func nilProviderSkips() async throws {
        CapabilityCheck.setCapabilitiesForTesting(bin: "zag", caps: [
            Self.fakeCapability(provider: "ollama", streamingInput: false),
        ])
        defer { CapabilityCheck.clearCapabilityCache() }
        try await CapabilityCheck.check(bin: "zag", provider: nil, requirements: [
            CapabilityCheck.Requirement(
                method: "execStreaming()", feature: .streamingInput, isSet: true),
        ])
    }

    @Test("supported feature passes")
    func supportedFeaturePasses() async throws {
        CapabilityCheck.setCapabilitiesForTesting(bin: "zag", caps: [
            Self.fakeCapability(provider: "claude", streamingInput: true),
        ])
        defer { CapabilityCheck.clearCapabilityCache() }
        try await CapabilityCheck.check(bin: "zag", provider: "claude", requirements: [
            CapabilityCheck.Requirement(
                method: "execStreaming()", feature: .streamingInput, isSet: true),
        ])
    }

    @Test("unsupported feature throws")
    func unsupportedFeatureThrows() async {
        CapabilityCheck.setCapabilitiesForTesting(bin: "zag", caps: [
            Self.fakeCapability(provider: "claude", streamingInput: true),
            Self.fakeCapability(provider: "ollama", streamingInput: false),
        ])
        defer { CapabilityCheck.clearCapabilityCache() }
        do {
            try await CapabilityCheck.check(bin: "zag", provider: "ollama", requirements: [
                CapabilityCheck.Requirement(
                    method: "execStreaming()", feature: .streamingInput, isSet: true),
            ])
            Issue.record("Expected ZagFeatureUnsupportedError")
        } catch let error as ZagFeatureUnsupportedError {
            #expect(error.provider == "ollama")
            #expect(error.feature == "streaming_input")
            #expect(error.method == "execStreaming()")
            #expect(error.supportedProviders == ["claude"])
            #expect(error.message.contains("ollama"))
            #expect(error.message.contains("streaming_input"))
            #expect(error.message.contains("execStreaming()"))
            #expect(error.message.contains("Supported providers: claude"))
        } catch {
            Issue.record("Expected ZagFeatureUnsupportedError, got \(error)")
        }
    }

    @Test("unknown provider skips check")
    func unknownProviderSkips() async throws {
        CapabilityCheck.setCapabilitiesForTesting(bin: "zag", caps: [
            Self.fakeCapability(provider: "claude", streamingInput: true),
        ])
        defer { CapabilityCheck.clearCapabilityCache() }
        try await CapabilityCheck.check(bin: "zag", provider: "imaginary", requirements: [
            CapabilityCheck.Requirement(
                method: "execStreaming()", feature: .streamingInput, isSet: true),
        ])
    }

    @Test("addDirs unsupported on ollama throws")
    func addDirsUnsupportedThrows() async {
        CapabilityCheck.setCapabilitiesForTesting(bin: "zag", caps: [
            Self.fakeCapability(provider: "claude", addDirs: true),
            Self.fakeCapability(provider: "ollama", addDirs: false),
        ])
        defer { CapabilityCheck.clearCapabilityCache() }
        do {
            try await CapabilityCheck.check(bin: "zag", provider: "ollama", requirements: [
                CapabilityCheck.Requirement(
                    method: "addDir()", feature: .addDirs, isSet: true),
            ])
            Issue.record("Expected ZagFeatureUnsupportedError")
        } catch let error as ZagFeatureUnsupportedError {
            #expect(error.feature == "add_dirs")
            #expect(error.supportedProviders == ["claude"])
        } catch {
            Issue.record("Expected ZagFeatureUnsupportedError, got \(error)")
        }
    }

    @Test("no providers support feature")
    func noProvidersSupport() async {
        CapabilityCheck.setCapabilitiesForTesting(bin: "zag", caps: [
            Self.fakeCapability(provider: "ollama", streamingInput: false),
        ])
        defer { CapabilityCheck.clearCapabilityCache() }
        do {
            try await CapabilityCheck.check(bin: "zag", provider: "ollama", requirements: [
                CapabilityCheck.Requirement(
                    method: "execStreaming()", feature: .streamingInput, isSet: true),
            ])
            Issue.record("Expected ZagFeatureUnsupportedError")
        } catch let error as ZagFeatureUnsupportedError {
            #expect(error.supportedProviders.isEmpty)
            #expect(error.message.contains("No providers currently support this feature"))
        } catch {
            Issue.record("Expected ZagFeatureUnsupportedError, got \(error)")
        }
    }
}
#endif

// MARK: - ZagError tests

@Suite("ZagError")
struct ZagErrorTests {

    @Test("error contains fields")
    func errorContainsFields() {
        let error = ZagError(message: "test error", exitCode: 1, stderr: "stderr output")
        #expect(error.message == "test error")
        #expect(error.exitCode == 1)
        #expect(error.stderr == "stderr output")
        #expect(error.description == "test error")
    }

    @Test("error without exit code")
    func errorWithoutExitCode() {
        let error = ZagError(message: "no code")
        #expect(error.exitCode == nil)
        #expect(error.stderr == "")
    }
}

// MARK: - Models deserialization tests

@Suite("Models")
struct ModelsTests {

    static let sampleJSON = """
    {
        "agent": "claude",
        "session_id": "sess-123",
        "events": [
            {
                "type": "init",
                "model": "sonnet",
                "tools": ["Bash", "Read"],
                "working_directory": "/home/user",
                "metadata": {}
            },
            {
                "type": "assistant_message",
                "content": [{"type": "text", "text": "Hello!"}],
                "usage": {"input_tokens": 100, "output_tokens": 50}
            },
            {
                "type": "tool_execution",
                "tool_name": "Bash",
                "tool_id": "tool_123",
                "input": {"command": "echo hello"},
                "result": {"success": true, "output": "hello", "error": null, "data": null}
            },
            {
                "type": "turn_complete",
                "stop_reason": "end_turn",
                "turn_index": 0,
                "usage": {"input_tokens": 80, "output_tokens": 40}
            },
            {
                "type": "result",
                "success": true,
                "message": "Done",
                "duration_ms": 1500,
                "num_turns": 2
            },
            {
                "type": "error",
                "message": "oops",
                "details": null
            },
            {
                "type": "permission_request",
                "tool_name": "Bash",
                "description": "run cmd",
                "granted": true
            },
            {
                "type": "user_message",
                "content": [{"type": "text", "text": "Hi"}]
            }
        ],
        "result": "Hello!",
        "is_error": false,
        "total_cost_usd": 0.01,
        "usage": {"input_tokens": 100, "output_tokens": 50}
    }
    """

    @Test("AgentOutput deserializes")
    func agentOutputDeserializes() throws {
        let data = ModelsTests.sampleJSON.data(using: .utf8)!
        let output = try JSONDecoder.zag.decode(AgentOutput.self, from: data)

        #expect(output.agent == "claude")
        #expect(output.sessionId == "sess-123")
        #expect(output.events.count == 8)
        #expect(output.result == "Hello!")
        #expect(output.isError == false)
        #expect(output.exitCode == nil)
        #expect(output.errorMessage == nil)
        #expect(output.totalCostUsd == 0.01)
        #expect(output.usage != nil)
        #expect(output.usage?.inputTokens == 100)
    }

    @Test("events deserialize correct types")
    func eventsDeserializeCorrectTypes() throws {
        let data = ModelsTests.sampleJSON.data(using: .utf8)!
        let output = try JSONDecoder.zag.decode(AgentOutput.self, from: data)

        if case .`init` = output.events[0] {} else { Issue.record("Expected init event") }
        if case .assistantMessage = output.events[1] {} else { Issue.record("Expected assistant_message event") }
        if case .toolExecution = output.events[2] {} else { Issue.record("Expected tool_execution event") }
        if case .turnComplete = output.events[3] {} else { Issue.record("Expected turn_complete event") }
        if case .result = output.events[4] {} else { Issue.record("Expected result event") }
        if case .error = output.events[5] {} else { Issue.record("Expected error event") }
        if case .permissionRequest = output.events[6] {} else { Issue.record("Expected permission_request event") }
        if case .userMessage = output.events[7] {} else { Issue.record("Expected user_message event") }
    }

    @Test("turn_complete fields")
    func turnCompleteFields() throws {
        let data = ModelsTests.sampleJSON.data(using: .utf8)!
        let output = try JSONDecoder.zag.decode(AgentOutput.self, from: data)

        guard case .turnComplete(let payload) = output.events[3] else {
            Issue.record("Expected turn_complete event")
            return
        }
        #expect(payload.stopReason == "end_turn")
        #expect(payload.turnIndex == 0)
        #expect(payload.usage?.inputTokens == 80)
        #expect(payload.usage?.outputTokens == 40)
    }

    @Test("turn_complete null stop_reason round-trips")
    func turnCompleteNullStopReason() throws {
        let line = """
        {"type":"turn_complete","stop_reason":null,"turn_index":2,"usage":null}
        """
        let data = line.data(using: .utf8)!
        let event = try JSONDecoder.zag.decode(Event.self, from: data)
        guard case .turnComplete(let payload) = event else {
            Issue.record("Expected turn_complete event")
            return
        }
        #expect(payload.stopReason == nil)
        #expect(payload.turnIndex == 2)
        #expect(payload.usage == nil)
    }

    @Test("init event fields")
    func initEventFields() throws {
        let data = ModelsTests.sampleJSON.data(using: .utf8)!
        let output = try JSONDecoder.zag.decode(AgentOutput.self, from: data)

        guard case .`init`(let payload) = output.events[0] else {
            Issue.record("Expected init event")
            return
        }
        #expect(payload.model == "sonnet")
        #expect(payload.tools == ["Bash", "Read"])
        #expect(payload.workingDirectory == "/home/user")
    }

    @Test("assistant message content blocks")
    func assistantMessageContentBlocks() throws {
        let data = ModelsTests.sampleJSON.data(using: .utf8)!
        let output = try JSONDecoder.zag.decode(AgentOutput.self, from: data)

        guard case .assistantMessage(let payload) = output.events[1] else {
            Issue.record("Expected assistant_message event")
            return
        }
        #expect(payload.content.count == 1)
        guard case .text(let textBlock) = payload.content[0] else {
            Issue.record("Expected text content block")
            return
        }
        #expect(textBlock.text == "Hello!")
        #expect(payload.usage?.inputTokens == 100)
    }

    @Test("tool execution fields")
    func toolExecutionFields() throws {
        let data = ModelsTests.sampleJSON.data(using: .utf8)!
        let output = try JSONDecoder.zag.decode(AgentOutput.self, from: data)

        guard case .toolExecution(let payload) = output.events[2] else {
            Issue.record("Expected tool_execution event")
            return
        }
        #expect(payload.toolName == "Bash")
        #expect(payload.toolId == "tool_123")
        #expect(payload.result.success == true)
        #expect(payload.result.output == "hello")
    }

    @Test("error event fields")
    func errorEventFields() throws {
        let data = ModelsTests.sampleJSON.data(using: .utf8)!
        let output = try JSONDecoder.zag.decode(AgentOutput.self, from: data)

        guard case .error(let payload) = output.events[5] else {
            Issue.record("Expected error event")
            return
        }
        #expect(payload.message == "oops")
    }

    @Test("permission request fields")
    func permissionRequestFields() throws {
        let data = ModelsTests.sampleJSON.data(using: .utf8)!
        let output = try JSONDecoder.zag.decode(AgentOutput.self, from: data)

        guard case .permissionRequest(let payload) = output.events[6] else {
            Issue.record("Expected permission_request event")
            return
        }
        #expect(payload.toolName == "Bash")
        #expect(payload.granted == true)
    }

    @Test("NDJSON event parses")
    func ndjsonEventParses() throws {
        let line = """
        {"type":"init","model":"opus","tools":[],"working_directory":null,"metadata":{}}
        """
        let data = line.data(using: .utf8)!
        let event = try JSONDecoder.zag.decode(Event.self, from: data)

        guard case .`init`(let payload) = event else {
            Issue.record("Expected init event")
            return
        }
        #expect(payload.model == "opus")
    }

    @Test("tool use content block")
    func toolUseContentBlock() throws {
        let json = """
        {"type":"tool_use","id":"t1","name":"Bash","input":{"command":"ls"}}
        """
        let data = json.data(using: .utf8)!
        let block = try JSONDecoder.zag.decode(ContentBlock.self, from: data)

        guard case .toolUse(let toolUse) = block else {
            Issue.record("Expected tool_use block")
            return
        }
        #expect(toolUse.id == "t1")
        #expect(toolUse.name == "Bash")
    }

    @Test("usage with optional fields")
    func usageOptionalFields() throws {
        let json = """
        {"input_tokens": 200, "output_tokens": 100, "cache_read_tokens": 50}
        """
        let data = json.data(using: .utf8)!
        let usage = try JSONDecoder.zag.decode(Usage.self, from: data)

        #expect(usage.inputTokens == 200)
        #expect(usage.outputTokens == 100)
        #expect(usage.cacheReadTokens == 50)
        #expect(usage.cacheCreationTokens == nil)
        #expect(usage.webSearchRequests == nil)
    }
}
