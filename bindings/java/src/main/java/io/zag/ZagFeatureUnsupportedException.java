package io.zag;

import java.util.List;

/**
 * Raised by the capability preflight when a builder method is called for a
 * feature the configured provider does not support. Thrown before any
 * subprocess is spawned so callers can catch it distinctly from a runtime
 * {@link ZagException}.
 */
public class ZagFeatureUnsupportedException extends ZagException {

    private final String method;
    private final String feature;
    private final String provider;
    private final List<String> supportedProviders;

    public ZagFeatureUnsupportedException(
            String method,
            String feature,
            String provider,
            List<String> supportedProviders) {
        super(buildMessage(method, feature, provider, supportedProviders), null, "");
        this.method = method;
        this.feature = feature;
        this.provider = provider;
        this.supportedProviders = List.copyOf(supportedProviders);
    }

    private static String buildMessage(
            String method, String feature, String provider, List<String> supportedProviders) {
        String supportedList = supportedProviders.isEmpty()
                ? "(none)"
                : String.join(", ", supportedProviders);
        return method + " is not supported by provider '" + provider + "' "
                + "(feature: " + feature + "). Supported providers: " + supportedList;
    }

    public String method() {
        return method;
    }

    public String feature() {
        return feature;
    }

    public String provider() {
        return provider;
    }

    public List<String> supportedProviders() {
        return supportedProviders;
    }
}
