package zag

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFails
import kotlin.test.assertIs
import kotlin.test.assertNotNull
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue

class ZagBuilderTests {

    @Test
    fun `builder defaults produce minimal args`() {
        val builder = ZagBuilder()
        val args = builder.buildGlobalArgs()
        assertEquals(emptyList(), args)
    }

    @Test
    fun `builder method chaining sets all options`() {
        val builder = ZagBuilder()
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

        val args = builder.buildGlobalArgs()

        assertEquals(
            listOf(
                "-p", "gemini",
                "--model", "large",
                "--root", "/project",
                "--auto-approve",
                "--add-dir", "/docs",
                "--file", "/tmp/data.csv",
                "--verbose",
                "--debug",
                "--session", "sess-1",
            ),
            args,
        )
    }

    @Test
    fun `headless emits --headless flag`() {
        val enabled = ZagBuilder().provider("claude").headless().buildGlobalArgs()
        assertTrue(enabled.contains("--headless"))

        val disabled = ZagBuilder().provider("claude").headless(false).buildGlobalArgs()
        assertTrue(!disabled.contains("--headless"))
    }

    @Test
    fun `env vars produce correct args`() {
        val builder = ZagBuilder()
            .env("FOO", "bar")
            .env("BAZ", "qux")

        val args = builder.buildGlobalArgs()

        assertTrue(args.contains("--env"))
        val idx = args.indexOf("--env")
        assertEquals("FOO=bar", args[idx + 1])
        assertEquals("--env", args[idx + 2])
        assertEquals("BAZ=qux", args[idx + 3])
    }

    @Test
    fun `exec args default to json`() {
        val builder = ZagBuilder().provider("claude")
        val args = builder.buildExecArgs("hello")

        assertTrue(args.contains("exec"))
        assertTrue(args.contains("-o"))
        assertTrue(args.contains("json"))
        assertTrue(args.contains("hello"))
    }

    @Test
    fun `exec args streaming`() {
        val builder = ZagBuilder()
        val args = builder.buildExecArgs("hello", streaming = true)
        assertFalse(args.contains("--json-stream"))
        val oi = args.indexOf("-o")
        assertTrue(oi >= 0)
        assertEquals("stream-json", args[oi + 1])
    }

    @Test
    fun `worktree no name`() {
        val builder = ZagBuilder().worktree()
        val args = builder.buildGlobalArgs()
        assertTrue(args.contains("-w"))
        assertEquals(1, args.size)
    }

    @Test
    fun `worktree named`() {
        val builder = ZagBuilder().worktree("feat")
        val args = builder.buildGlobalArgs()
        assertEquals(listOf("-w", "feat"), args)
    }

    @Test
    fun `sandbox no name`() {
        val builder = ZagBuilder().sandbox()
        val args = builder.buildGlobalArgs()
        assertTrue(args.contains("--sandbox"))
        assertEquals(1, args.size)
    }

    @Test
    fun `sandbox named`() {
        val builder = ZagBuilder().sandbox("box1")
        val args = builder.buildGlobalArgs()
        assertEquals(listOf("--sandbox", "box1"), args)
    }

    @Test
    fun `exit with hint`() {
        val args = ZagBuilder().exit("the answer").buildRunArgs("compute")
        val i = args.indexOf("--exit")
        assertTrue(i >= 0)
        assertEquals("the answer", args[i + 1])
    }

    @Test
    fun `exit bare`() {
        val args = ZagBuilder().exit().buildRunArgs()
        assertTrue(args.contains("--exit"))
    }

    @Test
    fun `exit omitted when not set`() {
        val args = ZagBuilder().buildRunArgs("hi")
        assertFalse(args.contains("--exit"))
    }

    @Test
    fun `max turns`() {
        val builder = ZagBuilder().maxTurns(10)
        val args = builder.buildGlobalArgs()
        assertEquals(listOf("--max-turns", "10"), args)
    }

    @Test
    fun `mcp config`() {
        val builder = ZagBuilder().mcpConfig("./mcp.json")
        val args = builder.buildGlobalArgs()
        assertEquals(listOf("--mcp-config", "./mcp.json"), args)
    }

    @Test
    fun `show usage`() {
        val builder = ZagBuilder().showUsage()
        val args = builder.buildGlobalArgs()
        assertEquals(listOf("--show-usage"), args)
    }

    @Test
    fun `size option`() {
        val builder = ZagBuilder().size("35b")
        val args = builder.buildGlobalArgs()
        assertEquals(listOf("--size", "35b"), args)
    }

    @Test
    fun `timeout included in exec args`() {
        val args = ZagBuilder().timeout("5m").buildExecArgs("test")
        assertTrue(args.contains("--timeout"))
        assertTrue(args.contains("5m"))
    }

    @Test
    fun `resume included in exec args`() {
        val args = ZagBuilder().provider("claude").buildExecArgs("follow up").toMutableList()
        val promptIdx = args.size - 1
        args.add(promptIdx, "--resume")
        args.add(promptIdx + 1, "sess-123")
        assertTrue(args.contains("--resume"))
        assertTrue(args.contains("sess-123"))
        assertTrue(args.indexOf("--resume") < args.indexOf("follow up"))
    }

    @Test
    fun `continue included in exec args`() {
        val args = ZagBuilder().provider("claude").buildExecArgs("follow up").toMutableList()
        val promptIdx = args.size - 1
        args.add(promptIdx, "--continue")
        assertTrue(args.contains("--continue"))
        assertTrue(args.indexOf("--continue") < args.indexOf("follow up"))
    }
}

class VersionCheckTests {

    @Test
    fun `parse valid semver`() {
        val v = VersionCheck.parseSemver("0.6.0")
        assertEquals(0, v.major)
        assertEquals(6, v.minor)
        assertEquals(0, v.patch)
    }

    @Test
    fun `parse invalid semver throws`() {
        assertFails { VersionCheck.parseSemver("invalid") }
        assertFails { VersionCheck.parseSemver("1.2") }
        assertFails { VersionCheck.parseSemver("a.b.c") }
    }

    @Test
    fun `semver comparison`() {
        val v050 = VersionCheck.parseSemver("0.5.0")
        val v060 = VersionCheck.parseSemver("0.6.0")
        val v070 = VersionCheck.parseSemver("0.7.0")

        assertTrue(v050 < v060)
        assertEquals(v060, v060)
        assertTrue(v070 > v060)
    }

    @Test
    fun `check with no active requirements passes`() = kotlinx.coroutines.test.runTest {
        VersionCheck.setVersionForTesting("zag", "0.5.0")
        try {
            VersionCheck.check("zag", listOf(
                VersionCheck.Requirement("env()", "0.6.0", false),
            ))
        } finally {
            VersionCheck.clearVersionCache()
        }
    }

    @Test
    fun `check with sufficient version passes`() = kotlinx.coroutines.test.runTest {
        VersionCheck.setVersionForTesting("zag", "0.6.0")
        try {
            VersionCheck.check("zag", listOf(
                VersionCheck.Requirement("env()", "0.6.0", true),
            ))
        } finally {
            VersionCheck.clearVersionCache()
        }
    }

    @Test
    fun `check with insufficient version throws`() = kotlinx.coroutines.test.runTest {
        VersionCheck.setVersionForTesting("zag", "0.5.0")
        try {
            val ex = assertFails {
                VersionCheck.check("zag", listOf(
                    VersionCheck.Requirement("env()", "0.6.0", true),
                ))
            }
            assertTrue(ex.message!!.contains("env()"))
            assertTrue(ex.message!!.contains("0.6.0"))
            assertTrue(ex.message!!.contains("0.5.0"))
        } finally {
            VersionCheck.clearVersionCache()
        }
    }

    @Test
    fun `check with multiple failures reports all`() = kotlinx.coroutines.test.runTest {
        VersionCheck.setVersionForTesting("zag", "0.5.0")
        try {
            val ex = assertFails {
                VersionCheck.check("zag", listOf(
                    VersionCheck.Requirement("env()", "0.6.0", true),
                    VersionCheck.Requirement("mcpConfig()", "0.6.0", true),
                ))
            }
            assertTrue(ex.message!!.contains("env()"))
            assertTrue(ex.message!!.contains("mcpConfig()"))
        } finally {
            VersionCheck.clearVersionCache()
        }
    }
}

class CapabilityCheckTests {

    private fun fakeCapability(
        provider: String,
        streamingInput: Boolean = false,
        addDirs: Boolean = true,
    ): ProviderCapability {
        val yes = FeatureSupport(supported = true, isNative = true)
        return ProviderCapability(
            provider = provider,
            defaultModel = "default",
            availableModels = emptyList(),
            sizeMappings = SizeMappings("s", "m", "l"),
            features = Features(
                interactive = yes,
                nonInteractive = yes,
                resume = yes,
                resumeWithPrompt = yes,
                sessionLogs = SessionLogSupport(supported = true, isNative = true),
                jsonOutput = yes,
                streamJson = yes,
                jsonSchema = yes,
                inputFormat = yes,
                streamingInput = StreamingInputSupport(
                    supported = streamingInput,
                    isNative = streamingInput,
                    semantics = if (streamingInput) "queue" else null),
                worktree = yes,
                sandbox = yes,
                systemPrompt = yes,
                autoApprove = yes,
                review = yes,
                addDirs = FeatureSupport(supported = addDirs, isNative = false),
                maxTurns = yes,
            ),
        )
    }

    @Test
    fun `no active requirements passes`() = kotlinx.coroutines.test.runTest {
        CapabilityCheck.setCapabilitiesForTesting("zag", listOf(
            fakeCapability("ollama", streamingInput = false),
        ))
        try {
            CapabilityCheck.check("zag", "ollama", listOf(
                CapabilityCheck.Requirement(
                    "execStreaming()", CapabilityCheck.FeatureKeys.STREAMING_INPUT, false),
            ))
        } finally {
            CapabilityCheck.clearCapabilityCache()
        }
    }

    @Test
    fun `null provider skips check`() = kotlinx.coroutines.test.runTest {
        CapabilityCheck.setCapabilitiesForTesting("zag", listOf(
            fakeCapability("ollama", streamingInput = false),
        ))
        try {
            CapabilityCheck.check("zag", null, listOf(
                CapabilityCheck.Requirement(
                    "execStreaming()", CapabilityCheck.FeatureKeys.STREAMING_INPUT, true),
            ))
        } finally {
            CapabilityCheck.clearCapabilityCache()
        }
    }

    @Test
    fun `supported feature passes`() = kotlinx.coroutines.test.runTest {
        CapabilityCheck.setCapabilitiesForTesting("zag", listOf(
            fakeCapability("claude", streamingInput = true),
        ))
        try {
            CapabilityCheck.check("zag", "claude", listOf(
                CapabilityCheck.Requirement(
                    "execStreaming()", CapabilityCheck.FeatureKeys.STREAMING_INPUT, true),
            ))
        } finally {
            CapabilityCheck.clearCapabilityCache()
        }
    }

    @Test
    fun `unsupported feature throws`() = kotlinx.coroutines.test.runTest {
        CapabilityCheck.setCapabilitiesForTesting("zag", listOf(
            fakeCapability("claude", streamingInput = true),
            fakeCapability("ollama", streamingInput = false),
        ))
        try {
            val ex = assertFails {
                CapabilityCheck.check("zag", "ollama", listOf(
                    CapabilityCheck.Requirement(
                        "execStreaming()", CapabilityCheck.FeatureKeys.STREAMING_INPUT, true),
                ))
            }
            assertIs<ZagFeatureUnsupportedException>(ex)
            assertEquals("ollama", ex.provider)
            assertEquals("streaming_input", ex.feature)
            assertEquals("execStreaming()", ex.method)
            assertEquals(listOf("claude"), ex.supportedProviders)
            assertTrue(ex.message!!.contains("ollama"))
            assertTrue(ex.message!!.contains("streaming_input"))
            assertTrue(ex.message!!.contains("execStreaming()"))
            assertTrue(ex.message!!.contains("Supported providers: claude"))
        } finally {
            CapabilityCheck.clearCapabilityCache()
        }
    }

    @Test
    fun `unknown provider skips check`() = kotlinx.coroutines.test.runTest {
        CapabilityCheck.setCapabilitiesForTesting("zag", listOf(
            fakeCapability("claude", streamingInput = true),
        ))
        try {
            CapabilityCheck.check("zag", "imaginary", listOf(
                CapabilityCheck.Requirement(
                    "execStreaming()", CapabilityCheck.FeatureKeys.STREAMING_INPUT, true),
            ))
        } finally {
            CapabilityCheck.clearCapabilityCache()
        }
    }

    @Test
    fun `addDirs unsupported on ollama throws`() = kotlinx.coroutines.test.runTest {
        CapabilityCheck.setCapabilitiesForTesting("zag", listOf(
            fakeCapability("claude", addDirs = true),
            fakeCapability("ollama", addDirs = false),
        ))
        try {
            val ex = assertFails {
                CapabilityCheck.check("zag", "ollama", listOf(
                    CapabilityCheck.Requirement(
                        "addDir()", CapabilityCheck.FeatureKeys.ADD_DIRS, true),
                ))
            }
            assertIs<ZagFeatureUnsupportedException>(ex)
            assertEquals("add_dirs", ex.feature)
            assertEquals(listOf("claude"), ex.supportedProviders)
        } finally {
            CapabilityCheck.clearCapabilityCache()
        }
    }

    @Test
    fun `no providers support feature throws`() = kotlinx.coroutines.test.runTest {
        CapabilityCheck.setCapabilitiesForTesting("zag", listOf(
            fakeCapability("ollama", streamingInput = false),
        ))
        try {
            val ex = assertFails {
                CapabilityCheck.check("zag", "ollama", listOf(
                    CapabilityCheck.Requirement(
                        "execStreaming()", CapabilityCheck.FeatureKeys.STREAMING_INPUT, true),
                ))
            }
            assertIs<ZagFeatureUnsupportedException>(ex)
            assertTrue(ex.supportedProviders.isEmpty())
            assertTrue(ex.message!!.contains("No providers currently support this feature"))
        } finally {
            CapabilityCheck.clearCapabilityCache()
        }
    }
}

class ZagExceptionTests {

    @Test
    fun `exception contains fields`() {
        val ex = ZagException("test error", 1, "stderr output")
        assertEquals("test error", ex.message)
        assertEquals(1, ex.exitCode)
        assertEquals("stderr output", ex.stderr)
        assertIs<RuntimeException>(ex)
    }
}

class ModelsTests {

    private val sampleJson = """
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
            }
        ],
        "result": "Hello!",
        "is_error": false,
        "total_cost_usd": 0.01,
        "usage": {"input_tokens": 100, "output_tokens": 50}
    }
    """.trimIndent()

    @Test
    fun `agent output deserializes`() {
        val output = ZagJson.decodeFromString<AgentOutput>(sampleJson)
        assertEquals("claude", output.agent)
        assertEquals("sess-123", output.sessionId)
        assertEquals(7, output.events.size)
        assertEquals("Hello!", output.result)
        assertFalse(output.isError)
        assertNull(output.exitCode)
        assertNull(output.errorMessage)
        assertEquals(0.01, output.totalCostUsd)
        assertNotNull(output.usage)
        assertEquals(100, output.usage!!.inputTokens)
    }

    @Test
    fun `events deserialize correct types`() {
        val output = ZagJson.decodeFromString<AgentOutput>(sampleJson)

        assertIs<InitEvent>(output.events[0])
        assertIs<AssistantMessageEvent>(output.events[1])
        assertIs<ToolExecutionEvent>(output.events[2])
        assertIs<TurnCompleteEvent>(output.events[3])
        assertIs<ResultEvent>(output.events[4])
        assertIs<ErrorEvent>(output.events[5])
        assertIs<PermissionRequestEvent>(output.events[6])
    }

    @Test
    fun `turn complete fields`() {
        val output = ZagJson.decodeFromString<AgentOutput>(sampleJson)
        val turn = output.events[3] as TurnCompleteEvent
        assertEquals("end_turn", turn.stopReason)
        assertEquals(0L, turn.turnIndex)
        assertNotNull(turn.usage)
        assertEquals(80, turn.usage!!.inputTokens)
        assertEquals(40, turn.usage!!.outputTokens)
    }

    @Test
    fun `turn complete null stop reason round-trips`() {
        val line = """{"type":"turn_complete","stop_reason":null,"turn_index":2,"usage":null}"""
        val event = ZagJson.decodeFromString<Event>(line)
        assertIs<TurnCompleteEvent>(event)
        val turn = event as TurnCompleteEvent
        assertNull(turn.stopReason)
        assertEquals(2L, turn.turnIndex)
        assertNull(turn.usage)
    }

    @Test
    fun `init event fields`() {
        val output = ZagJson.decodeFromString<AgentOutput>(sampleJson)
        val init = output.events[0] as InitEvent
        assertEquals("sonnet", init.model)
        assertEquals(listOf("Bash", "Read"), init.tools)
        assertEquals("/home/user", init.workingDirectory)
    }

    @Test
    fun `assistant message content blocks`() {
        val output = ZagJson.decodeFromString<AgentOutput>(sampleJson)
        val msg = output.events[1] as AssistantMessageEvent
        assertEquals(1, msg.content.size)
        val text = msg.content[0] as TextBlock
        assertEquals("Hello!", text.text)
        assertNotNull(msg.usage)
        assertEquals(100, msg.usage!!.inputTokens)
    }

    @Test
    fun `tool execution fields`() {
        val output = ZagJson.decodeFromString<AgentOutput>(sampleJson)
        val tool = output.events[2] as ToolExecutionEvent
        assertEquals("Bash", tool.toolName)
        assertEquals("tool_123", tool.toolId)
        assertTrue(tool.result.success)
        assertEquals("hello", tool.result.output)
    }

    @Test
    fun `error event fields`() {
        val output = ZagJson.decodeFromString<AgentOutput>(sampleJson)
        val err = output.events[5] as ErrorEvent
        assertEquals("oops", err.message)
    }

    @Test
    fun `permission request fields`() {
        val output = ZagJson.decodeFromString<AgentOutput>(sampleJson)
        val perm = output.events[6] as PermissionRequestEvent
        assertEquals("Bash", perm.toolName)
        assertTrue(perm.granted)
    }

    @Test
    fun `ndjson event parses`() {
        val line = """{"type":"init","model":"opus","tools":[],"working_directory":null,"metadata":{}}"""
        val event = ZagJson.decodeFromString<Event>(line)
        assertIs<InitEvent>(event)
        assertEquals("opus", (event as InitEvent).model)
    }

    @Test
    fun `user message event deserializes`() {
        val json = """{"type":"user_message","content":[{"type":"text","text":"hi"}]}"""
        val event = ZagJson.decodeFromString<Event>(json)
        assertIs<UserMessageEvent>(event)
        val msg = event as UserMessageEvent
        assertEquals(1, msg.content.size)
        assertIs<TextBlock>(msg.content[0])
    }

    @Test
    fun `tool use block deserializes`() {
        val json = """{"type":"assistant_message","content":[{"type":"tool_use","id":"t1","name":"Bash","input":{"cmd":"ls"}}],"usage":null}"""
        val event = ZagJson.decodeFromString<Event>(json)
        assertIs<AssistantMessageEvent>(event)
        val msg = event as AssistantMessageEvent
        val block = msg.content[0] as ToolUseBlock
        assertEquals("t1", block.id)
        assertEquals("Bash", block.name)
    }

    @Test
    fun `usage with optional fields`() {
        val json = """{"input_tokens":500,"output_tokens":200,"cache_read_tokens":50,"cache_creation_tokens":null,"web_search_requests":3,"web_fetch_requests":null}"""
        val usage = ZagJson.decodeFromString<Usage>(json)
        assertEquals(500, usage.inputTokens)
        assertEquals(200, usage.outputTokens)
        assertEquals(50, usage.cacheReadTokens)
        assertNull(usage.cacheCreationTokens)
        assertEquals(3, usage.webSearchRequests)
        assertNull(usage.webFetchRequests)
    }
}
