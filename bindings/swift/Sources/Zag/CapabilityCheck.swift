import Foundation

/// Provider capability validation for the `ZagBuilder`.
///
/// Before spawning the `zag` CLI, the builder validates feature-gated
/// options (`execStreaming`, `worktree`, `sandbox`, `systemPrompt`,
/// `addDir`, `maxTurns`) against the capability declarations exposed by
/// `zag discover`. When a caller configures an option that the selected
/// provider does not support, the preflight raises
/// `ZagFeatureUnsupportedError` with a message listing the providers
/// that do support the feature.
public enum CapabilityCheck {

    /// Capability feature keys the builder can gate on.
    public enum FeatureKey: String, Sendable {
        case streamingInput = "streaming_input"
        case worktree
        case sandbox
        case systemPrompt = "system_prompt"
        case addDirs = "add_dirs"
        case maxTurns = "max_turns"
    }

    /// A capability-gated builder option.
    public struct Requirement: Sendable {
        /// User-facing builder method name (e.g. `"execStreaming()"`).
        public let method: String

        /// Capability feature key.
        public let feature: FeatureKey

        /// Whether the option is active for this invocation.
        public let isSet: Bool

        public init(method: String, feature: FeatureKey, isSet: Bool) {
            self.method = method
            self.feature = feature
            self.isSet = isSet
        }
    }

    /// Cached provider capability matrices keyed by binary path.
    /// Capabilities are compiled into the binary, so the cache lives for
    /// the life of the process.
    private static var capabilityCache: [String: [ProviderCapability]] = [:]
    private static let cacheLock = NSLock()

    private static func isFeatureSupported(_ features: Features, _ key: FeatureKey) -> Bool {
        switch key {
        case .streamingInput: return features.streamingInput.supported
        case .worktree: return features.worktree.supported
        case .sandbox: return features.sandbox.supported
        case .systemPrompt: return features.systemPrompt.supported
        case .addDirs: return features.addDirs.supported
        case .maxTurns: return features.maxTurns.supported
        }
    }

    #if os(macOS) || os(Linux)
    /// Fetch and cache the full provider capability matrix for a given
    /// `zag` binary.
    private static func loadCapabilities(bin: String) async throws -> [ProviderCapability] {
        cacheLock.lock()
        if let cached = capabilityCache[bin] {
            cacheLock.unlock()
            return cached
        }
        cacheLock.unlock()

        let caps = try await ZagDiscover.getAllCapabilities(bin: bin)

        cacheLock.lock()
        capabilityCache[bin] = caps
        cacheLock.unlock()

        return caps
    }

    /// Validate that every active feature requirement is supported by
    /// the configured provider. No-op when `provider` is `nil` (so the
    /// CLI's default-provider behavior is preserved) or when no
    /// requirements are active. If the `zag discover` call itself fails,
    /// the preflight silently returns so the subsequent CLI invocation
    /// can surface the real error.
    ///
    /// - Throws: `ZagFeatureUnsupportedError` on the first unsupported feature.
    public static func check(
        bin: String,
        provider: String?,
        requirements: [Requirement]
    ) async throws {
        let active = requirements.filter(\.isSet)
        guard !active.isEmpty, let provider = provider else { return }

        let caps: [ProviderCapability]
        do {
            caps = try await loadCapabilities(bin: bin)
        } catch {
            // If `zag discover` can't be reached, skip the preflight —
            // the subsequent CLI invocation will surface the real error.
            return
        }

        guard let providerCap = caps.first(where: { $0.provider == provider }) else {
            return
        }

        for req in active {
            if isFeatureSupported(providerCap.features, req.feature) { continue }
            let supported = caps
                .filter { isFeatureSupported($0.features, req.feature) }
                .map(\.provider)
            let suffix = supported.isEmpty
                ? " No providers currently support this feature."
                : " Supported providers: \(supported.joined(separator: ", "))"
            throw ZagFeatureUnsupportedError(
                message: "Provider '\(provider)' does not support \(req.feature.rawValue) " +
                    "(required by \(req.method)).\(suffix)",
                provider: provider,
                feature: req.feature.rawValue,
                method: req.method,
                supportedProviders: supported)
        }
    }
    #endif

    /// Inject capabilities into the cache for testing.
    internal static func setCapabilitiesForTesting(bin: String, caps: [ProviderCapability]) {
        cacheLock.lock()
        capabilityCache[bin] = caps
        cacheLock.unlock()
    }

    /// Clear the capability cache for testing.
    internal static func clearCapabilityCache() {
        cacheLock.lock()
        capabilityCache.removeAll()
        cacheLock.unlock()
    }
}
