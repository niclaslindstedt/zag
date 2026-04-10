package io.zag;

import java.util.List;

/**
 * Exception thrown when a builder option requires a provider feature that
 * the configured provider does not support.
 *
 * <p>The builder validates feature-gated options ({@link ZagBuilder#execStreaming},
 * {@link ZagBuilder#worktree}, {@link ZagBuilder#sandbox},
 * {@link ZagBuilder#systemPrompt}, {@link ZagBuilder#addDir},
 * {@link ZagBuilder#maxTurns}) against the capability declarations exposed by
 * {@code zag discover} before spawning the CLI, so callers receive a clear,
 * typed error instead of a cryptic runtime exit code.
 */
public class ZagFeatureUnsupportedException extends ZagException {

    private final String provider;
    private final String feature;
    private final String method;
    private final List<String> supportedProviders;

    public ZagFeatureUnsupportedException(
            String message,
            String provider,
            String feature,
            String method,
            List<String> supportedProviders) {
        super(message, null, "");
        this.provider = provider;
        this.feature = feature;
        this.method = method;
        this.supportedProviders = supportedProviders;
    }

    /** The provider that does not support the feature. */
    public String provider() {
        return provider;
    }

    /** The feature key (e.g. {@code "streaming_input"}). */
    public String feature() {
        return feature;
    }

    /** The builder method that requires the feature (e.g. {@code "execStreaming()"}). */
    public String method() {
        return method;
    }

    /** Providers that do support the feature. */
    public List<String> supportedProviders() {
        return supportedProviders;
    }
}
