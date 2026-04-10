package zag

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNotNull
import kotlin.test.assertTrue
import kotlinx.coroutines.runBlocking

/**
 * Integration tests for ZagDiscover.
 *
 * These tests require the zag binary to be built and available in PATH.
 */
class ZagDiscoverTests {

    @Test
    fun `listProviders returns provider names`() = runBlocking {
        val providers = ZagDiscover.listProviders()
        assertTrue(providers.size >= 5)
        assertTrue(providers.contains("claude"))
        assertTrue(providers.contains("codex"))
        assertTrue(providers.contains("gemini"))
        assertTrue(providers.contains("copilot"))
        assertTrue(providers.contains("ollama"))
    }

    @Test
    fun `getCapability returns single provider`() = runBlocking {
        val cap = ZagDiscover.getCapability("claude")
        assertEquals("claude", cap.provider)
        assertTrue(cap.availableModels.isNotEmpty())
        assertTrue(cap.features.interactive.supported)
    }

    @Test
    fun `getAllCapabilities returns all providers`() = runBlocking {
        val caps = ZagDiscover.getAllCapabilities()
        assertTrue(caps.size >= 5)
        val names = caps.map { it.provider }
        assertTrue(names.contains("claude"))
    }

    @Test
    fun `resolveModel resolves alias`() = runBlocking {
        val rm = ZagDiscover.resolveModel("claude", "small")
        assertEquals("small", rm.input)
        assertEquals("haiku", rm.resolved)
        assertTrue(rm.isAlias)
        assertEquals("claude", rm.provider)
    }

    @Test
    fun `resolveModel passes through non-alias`() = runBlocking {
        val rm = ZagDiscover.resolveModel("claude", "opus")
        assertEquals("opus", rm.input)
        assertEquals("opus", rm.resolved)
        assertFalse(rm.isAlias)
    }
}

/**
 * Unit tests for discover model deserialization.
 */
class DiscoverModelsTests {

    @Test
    fun `FeatureSupport deserializes`() {
        val json = """{"supported":true,"native":false}"""
        val fs = ZagJson.decodeFromString<FeatureSupport>(json)
        assertTrue(fs.supported)
        assertFalse(fs.isNative)
    }

    @Test
    fun `SessionLogSupport deserializes`() {
        val json = """{"supported":true,"native":true,"completeness":"full"}"""
        val sls = ZagJson.decodeFromString<SessionLogSupport>(json)
        assertTrue(sls.supported)
        assertTrue(sls.isNative)
        assertEquals("full", sls.completeness)
    }

    @Test
    fun `SessionLogSupport with null completeness`() {
        val json = """{"supported":false,"native":false}"""
        val sls = ZagJson.decodeFromString<SessionLogSupport>(json)
        assertFalse(sls.supported)
        assertEquals(null, sls.completeness)
    }

    @Test
    fun `StreamingInputSupport deserializes with semantics`() {
        val json = """{"supported":true,"native":true,"semantics":"queue"}"""
        val sis = ZagJson.decodeFromString<StreamingInputSupport>(json)
        assertTrue(sis.supported)
        assertTrue(sis.isNative)
        assertEquals("queue", sis.semantics)
    }

    @Test
    fun `StreamingInputSupport with null semantics`() {
        val json = """{"supported":false,"native":false}"""
        val sis = ZagJson.decodeFromString<StreamingInputSupport>(json)
        assertFalse(sis.supported)
        assertEquals(null, sis.semantics)
    }

    @Test
    fun `SizeMappings deserializes`() {
        val json = """{"small":"haiku","medium":"sonnet","large":"opus"}"""
        val sm = ZagJson.decodeFromString<SizeMappings>(json)
        assertEquals("haiku", sm.small)
        assertEquals("sonnet", sm.medium)
        assertEquals("opus", sm.large)
    }

    @Test
    fun `ProviderCapability deserializes`() {
        val json = """
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
                "json_schema": {"supported": false, "native": false},
                "input_format": {"supported": true, "native": true},
                "streaming_input": {"supported": true, "native": true, "semantics": "queue"},
                "worktree": {"supported": true, "native": false},
                "sandbox": {"supported": true, "native": false},
                "system_prompt": {"supported": true, "native": true},
                "auto_approve": {"supported": true, "native": true},
                "review": {"supported": true, "native": true},
                "add_dirs": {"supported": true, "native": true},
                "max_turns": {"supported": true, "native": true}
            }
        }
        """.trimIndent()

        val cap = ZagJson.decodeFromString<ProviderCapability>(json)
        assertEquals("claude", cap.provider)
        assertEquals("sonnet", cap.defaultModel)
        assertEquals(3, cap.availableModels.size)
        assertEquals("haiku", cap.sizeMappings.small)
        assertTrue(cap.features.interactive.supported)
        assertTrue(cap.features.interactive.isNative)
        assertTrue(cap.features.sessionLogs.supported)
        assertEquals("full", cap.features.sessionLogs.completeness)
        assertFalse(cap.features.jsonSchema.supported)
        assertTrue(cap.features.worktree.supported)
        assertFalse(cap.features.worktree.isNative)
        assertTrue(cap.features.streamingInput.supported)
        assertEquals("queue", cap.features.streamingInput.semantics)
    }

    @Test
    fun `ResolvedModel deserializes`() {
        val json = """{"input":"small","resolved":"haiku","is_alias":true,"provider":"claude"}"""
        val rm = ZagJson.decodeFromString<ResolvedModel>(json)
        assertEquals("small", rm.input)
        assertEquals("haiku", rm.resolved)
        assertTrue(rm.isAlias)
        assertEquals("claude", rm.provider)
    }

    @Test
    fun `ResolvedModel non-alias`() {
        val json = """{"input":"opus","resolved":"opus","is_alias":false,"provider":"claude"}"""
        val rm = ZagJson.decodeFromString<ResolvedModel>(json)
        assertEquals("opus", rm.input)
        assertEquals("opus", rm.resolved)
        assertFalse(rm.isAlias)
    }
}
