package zag

import kotlinx.coroutines.runBlocking
import kotlin.test.AfterTest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertFalse
import kotlin.test.assertIs
import kotlin.test.assertTrue

class CapabilityCheckTests {

    private fun fs(supported: Boolean) = FeatureSupport(supported = supported, isNative = false)

    private fun fakeFeatures(
        worktree: Boolean = false,
        sandbox: Boolean = false,
        systemPrompt: Boolean = false,
        addDirs: Boolean = false,
        streamingInput: Boolean = false,
    ) = Features(
        interactive = fs(true),
        nonInteractive = fs(true),
        resume = fs(false),
        resumeWithPrompt = fs(false),
        sessionLogs = SessionLogSupport(),
        jsonOutput = fs(true),
        streamJson = fs(true),
        jsonSchema = fs(false),
        inputFormat = fs(false),
        streamingInput = fs(streamingInput),
        worktree = fs(worktree),
        sandbox = fs(sandbox),
        systemPrompt = fs(systemPrompt),
        autoApprove = fs(true),
        review = fs(false),
        addDirs = fs(addDirs),
        maxTurns = fs(false),
    )

    private fun fakeCap(
        provider: String,
        worktree: Boolean = false,
        sandbox: Boolean = false,
        systemPrompt: Boolean = false,
        addDirs: Boolean = false,
        streamingInput: Boolean = false,
    ) = ProviderCapability(
        provider = provider,
        defaultModel = "default",
        availableModels = emptyList(),
        sizeMappings = SizeMappings(),
        features = fakeFeatures(worktree, sandbox, systemPrompt, addDirs, streamingInput),
    )

    private fun primeCaps(bin: String, caps: List<ProviderCapability>) {
        CapabilityCheck.clearCapabilityCache()
        CapabilityCheck.setAllCapabilitiesForTesting(bin, caps)
    }

    @AfterTest
    fun tearDown() {
        CapabilityCheck.clearCapabilityCache()
        VersionCheck.clearVersionCache()
    }

    // -- CapabilityCheck.check ------------------------------------------------

    @Test
    fun `no requirements returns silently`() = runBlocking {
        primeCaps("zag", listOf(fakeCap("ollama")))
        CapabilityCheck.check("zag", "ollama", emptyList())
    }

    @Test
    fun `inactive requirements return silently`() = runBlocking {
        primeCaps("zag", listOf(fakeCap("ollama")))
        CapabilityCheck.check(
            "zag",
            "ollama",
            listOf(CapabilityCheck.Requirement("addDir()", "add_dirs", false)),
        )
    }

    @Test
    fun `null provider skips check`() = runBlocking {
        // No cache primed; would raise if we tried to load.
        CapabilityCheck.check(
            "zag",
            null,
            listOf(CapabilityCheck.Requirement("addDir()", "add_dirs", true)),
        )
    }

    @Test
    fun `mock provider skips check`() = runBlocking {
        CapabilityCheck.check(
            "zag",
            "mock",
            listOf(CapabilityCheck.Requirement("addDir()", "add_dirs", true)),
        )
    }

    @Test
    fun `supported feature passes`() = runBlocking {
        primeCaps("zag", listOf(fakeCap("claude", streamingInput = true)))
        CapabilityCheck.check(
            "zag",
            "claude",
            listOf(CapabilityCheck.Requirement("execStreaming()", "streaming_input", true)),
        )
    }

    @Test
    fun `unsupported feature throws`() {
        primeCaps("zag", listOf(
            fakeCap("claude", streamingInput = true),
            fakeCap("ollama", streamingInput = false),
        ))

        val ex = assertFailsWith<ZagFeatureUnsupportedException> {
            runBlocking {
                CapabilityCheck.check(
                    "zag",
                    "ollama",
                    listOf(CapabilityCheck.Requirement("execStreaming()", "streaming_input", true)),
                )
            }
        }
        assertEquals("execStreaming()", ex.method)
        assertEquals("streaming_input", ex.feature)
        assertEquals("ollama", ex.provider)
        assertTrue("claude" in ex.supportedProviders)
        assertFalse("ollama" in ex.supportedProviders)
    }

    @Test
    fun `unsupported with no supporters shows none`() {
        primeCaps("zag", listOf(fakeCap("ollama")))

        val ex = assertFailsWith<ZagFeatureUnsupportedException> {
            runBlocking {
                CapabilityCheck.check(
                    "zag",
                    "ollama",
                    listOf(CapabilityCheck.Requirement("sandbox()", "sandbox", true)),
                )
            }
        }
        assertTrue(ex.message!!.contains("(none)"))
    }

    // -- ZagBuilder preflight -------------------------------------------------

    @Test
    fun `addDir on ollama throws before spawn`() {
        VersionCheck.setVersionForTesting("zag", "9.9.9")
        primeCaps("zag", listOf(
            fakeCap("claude", addDirs = true),
            fakeCap("ollama", addDirs = false),
        ))

        val ex = assertFailsWith<ZagFeatureUnsupportedException> {
            runBlocking {
                ZagBuilder().provider("ollama").addDir("/extra").exec("hello")
            }
        }
        assertEquals("addDir()", ex.method)
        assertEquals("ollama", ex.provider)
        assertTrue("claude" in ex.supportedProviders)
    }

    @Test
    fun `execStreaming on gemini throws before spawn`() {
        VersionCheck.setVersionForTesting("zag", "9.9.9")
        primeCaps("zag", listOf(
            fakeCap("claude", streamingInput = true),
            fakeCap("gemini", streamingInput = false),
        ))

        val ex = assertFailsWith<ZagFeatureUnsupportedException> {
            runBlocking {
                ZagBuilder().provider("gemini").execStreaming("hi")
            }
        }
        assertEquals("execStreaming()", ex.method)
        assertEquals("gemini", ex.provider)
        assertTrue("claude" in ex.supportedProviders)
    }

    // -- ZagFeatureUnsupportedException ---------------------------------------

    @Test
    fun `error message contains key parts`() {
        val ex = ZagFeatureUnsupportedException(
            method = "execStreaming()",
            feature = "streaming_input",
            provider = "ollama",
            supportedProviders = listOf("claude"),
        )
        assertTrue(ex.message!!.contains("execStreaming()"))
        assertTrue(ex.message!!.contains("ollama"))
        assertTrue(ex.message!!.contains("streaming_input"))
        assertTrue(ex.message!!.contains("claude"))
    }

    @Test
    fun `empty supported list shows none`() {
        val ex = ZagFeatureUnsupportedException(
            method = "sandbox()",
            feature = "sandbox",
            provider = "ollama",
            supportedProviders = emptyList(),
        )
        assertTrue(ex.message!!.contains("(none)"))
    }

    @Test
    fun `exception is a ZagException`() {
        val ex: ZagException = ZagFeatureUnsupportedException("worktree()", "worktree", "x", emptyList())
        assertIs<ZagFeatureUnsupportedException>(ex)
    }
}
