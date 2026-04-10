package io.zag;

import static org.junit.jupiter.api.Assertions.*;

import java.util.List;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.Test;

class CapabilityCheckTests {

    private static ProviderCapability.FeatureSupport fs(boolean supported) {
        return new ProviderCapability.FeatureSupport(supported, false);
    }

    private static ProviderCapability.Features fakeFeatures(
            boolean worktree,
            boolean sandbox,
            boolean systemPrompt,
            boolean addDirs,
            boolean streamingInput) {
        return new ProviderCapability.Features(
                fs(true),                                                   // interactive
                fs(true),                                                   // nonInteractive
                fs(false),                                                  // resume
                fs(false),                                                  // resumeWithPrompt
                new ProviderCapability.SessionLogSupport(false, false, null), // sessionLogs
                fs(true),                                                   // jsonOutput
                fs(true),                                                   // streamJson
                fs(false),                                                  // jsonSchema
                fs(false),                                                  // inputFormat
                fs(streamingInput),                                         // streamingInput
                fs(worktree),                                               // worktree
                fs(sandbox),                                                // sandbox
                fs(systemPrompt),                                           // systemPrompt
                fs(true),                                                   // autoApprove
                fs(false),                                                  // review
                fs(addDirs),                                                // addDirs
                fs(false));                                                 // maxTurns
    }

    private static ProviderCapability fakeCap(
            String provider,
            boolean worktree,
            boolean sandbox,
            boolean systemPrompt,
            boolean addDirs,
            boolean streamingInput) {
        return new ProviderCapability(
                provider,
                "default",
                List.of(),
                new ProviderCapability.SizeMappings("", "", ""),
                fakeFeatures(worktree, sandbox, systemPrompt, addDirs, streamingInput));
    }

    private static ProviderCapability fakeCap(String provider) {
        return fakeCap(provider, false, false, false, false, false);
    }

    private static void primeCaps(String bin, List<ProviderCapability> caps) {
        CapabilityCheck.clearCapabilityCache();
        CapabilityCheck.setAllCapabilitiesForTesting(bin, caps);
    }

    @AfterEach
    void tearDown() {
        CapabilityCheck.clearCapabilityCache();
        VersionCheck.clearVersionCache();
    }

    // -- CapabilityCheck.check -----------------------------------------------

    @Test
    void noRequirements_returnsSilently() throws Exception {
        primeCaps("zag", List.of(fakeCap("ollama")));
        CapabilityCheck.check("zag", "ollama", List.of());
    }

    @Test
    void inactiveRequirements_returnSilently() throws Exception {
        primeCaps("zag", List.of(fakeCap("ollama")));
        CapabilityCheck.check("zag", "ollama", List.of(
                new CapabilityCheck.Requirement("addDir()", "add_dirs", false)));
    }

    @Test
    void nullProvider_skipsCheck() throws Exception {
        // No cache primed — would raise if we tried to load.
        CapabilityCheck.check("zag", null, List.of(
                new CapabilityCheck.Requirement("addDir()", "add_dirs", true)));
    }

    @Test
    void mockProvider_skipsCheck() throws Exception {
        CapabilityCheck.check("zag", "mock", List.of(
                new CapabilityCheck.Requirement("addDir()", "add_dirs", true)));
    }

    @Test
    void supportedFeature_passes() throws Exception {
        primeCaps("zag", List.of(
                fakeCap("claude", false, false, false, false, true)));
        CapabilityCheck.check("zag", "claude", List.of(
                new CapabilityCheck.Requirement("execStreaming()", "streaming_input", true)));
    }

    @Test
    void unsupportedFeature_throws() {
        primeCaps("zag", List.of(
                fakeCap("claude", false, false, false, false, true),
                fakeCap("ollama", false, false, false, false, false)));

        ZagFeatureUnsupportedException ex = assertThrows(
                ZagFeatureUnsupportedException.class,
                () -> CapabilityCheck.check("zag", "ollama", List.of(
                        new CapabilityCheck.Requirement("execStreaming()", "streaming_input", true))));

        assertEquals("execStreaming()", ex.method());
        assertEquals("streaming_input", ex.feature());
        assertEquals("ollama", ex.provider());
        assertTrue(ex.supportedProviders().contains("claude"));
        assertFalse(ex.supportedProviders().contains("ollama"));
    }

    @Test
    void unsupportedWithNoSupporters_showsNone() {
        primeCaps("zag", List.of(fakeCap("ollama")));

        ZagFeatureUnsupportedException ex = assertThrows(
                ZagFeatureUnsupportedException.class,
                () -> CapabilityCheck.check("zag", "ollama", List.of(
                        new CapabilityCheck.Requirement("sandbox()", "sandbox", true))));

        assertTrue(ex.getMessage().contains("(none)"));
    }

    // -- ZagBuilder preflight ------------------------------------------------

    @Test
    void addDirOnOllama_throwsBeforeSpawn() {
        VersionCheck.setVersionForTesting("zag", "9.9.9");
        primeCaps("zag", List.of(
                fakeCap("claude", false, false, false, true, false),
                fakeCap("ollama", false, false, false, false, false)));

        ZagFeatureUnsupportedException ex = assertThrows(
                ZagFeatureUnsupportedException.class,
                () -> new ZagBuilder()
                        .provider("ollama")
                        .addDir("/extra")
                        .exec("hello"));

        assertEquals("addDir()", ex.method());
        assertEquals("ollama", ex.provider());
        assertTrue(ex.supportedProviders().contains("claude"));
    }

    @Test
    void execStreamingOnGemini_throwsBeforeSpawn() {
        VersionCheck.setVersionForTesting("zag", "9.9.9");
        primeCaps("zag", List.of(
                fakeCap("claude", false, false, false, false, true),
                fakeCap("gemini", false, false, false, false, false)));

        ZagFeatureUnsupportedException ex = assertThrows(
                ZagFeatureUnsupportedException.class,
                () -> new ZagBuilder()
                        .provider("gemini")
                        .execStreaming("hi"));

        assertEquals("execStreaming()", ex.method());
        assertEquals("gemini", ex.provider());
        assertTrue(ex.supportedProviders().contains("claude"));
    }

    // -- ZagFeatureUnsupportedException --------------------------------------

    @Test
    void errorMessage_containsKeyParts() {
        var ex = new ZagFeatureUnsupportedException(
                "execStreaming()",
                "streaming_input",
                "ollama",
                List.of("claude"));
        assertTrue(ex.getMessage().contains("execStreaming()"));
        assertTrue(ex.getMessage().contains("ollama"));
        assertTrue(ex.getMessage().contains("streaming_input"));
        assertTrue(ex.getMessage().contains("claude"));
    }

    @Test
    void emptySupportedList_showsNone() {
        var ex = new ZagFeatureUnsupportedException(
                "sandbox()",
                "sandbox",
                "ollama",
                List.of());
        assertTrue(ex.getMessage().contains("(none)"));
    }

    @Test
    void exceptionIsZagException() {
        var ex = new ZagFeatureUnsupportedException("worktree()", "worktree", "x", List.of());
        assertTrue(ex instanceof ZagException);
    }
}
