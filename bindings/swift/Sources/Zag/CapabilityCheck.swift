import Foundation

/// Provider-capability preflight helper for the zag Swift bindings.
///
/// Mirrors the ``VersionCheck`` pattern: every terminal builder method calls
/// ``CapabilityCheck/check(bin:provider:requirements:)`` with the list of
/// configured feature requirements before spawning the real agent subprocess.
/// When a configured option is unsupported by the pinned provider, a
/// ``ZagFeatureUnsupportedError`` is thrown with an actionable message —
/// instead of the user seeing a cryptic "zag exited with code 1" after the
/// subprocess fails.
public enum CapabilityCheck {

    /// A single feature-support requirement evaluated at preflight time.
    public struct Requirement {
        public let method: String
        public let feature: String
        public let isSet: Bool

        public init(method: String, feature: String, isSet: Bool) {
            self.method = method
            self.feature = feature
            self.isSet = isSet
        }
    }

    /// Per-(bin, provider) capability cache.
    private static var capCache: [String: ProviderCapability] = [:]
    /// Per-bin full capability matrix cache.
    private static var allCapsCache: [String: [ProviderCapability]] = [:]
    private static let cacheLock = NSLock()

    private static func cacheKey(bin: String, provider: String) -> String {
        "\(bin)|||\(provider)"
    }

    private static func featureSupport(_ cap: ProviderCapability, feature: String) -> FeatureSupport? {
        switch feature {
        case "streaming_input": return cap.features.streamingInput
        case "worktree": return cap.features.worktree
        case "sandbox": return cap.features.sandbox
        case "system_prompt": return cap.features.systemPrompt
        case "add_dirs": return cap.features.addDirs
        case "json_output": return cap.features.jsonOutput
        case "stream_json": return cap.features.streamJson
        case "json_schema": return cap.features.jsonSchema
        case "input_format": return cap.features.inputFormat
        case "interactive": return cap.features.interactive
        case "non_interactive": return cap.features.nonInteractive
        case "resume": return cap.features.resume
        case "resume_with_prompt": return cap.features.resumeWithPrompt
        case "auto_approve": return cap.features.autoApprove
        case "review": return cap.features.review
        case "max_turns": return cap.features.maxTurns
        default: return nil
        }
    }

    private static func isSupported(_ cap: ProviderCapability, feature: String) -> Bool {
        guard let support = featureSupport(cap, feature: feature) else { return false }
        return support.supported
    }

    #if os(macOS) || os(Linux)
    private static func loadCapability(bin: String, provider: String) async throws -> ProviderCapability {
        let key = cacheKey(bin: bin, provider: provider)
        cacheLock.lock()
        if let cached = capCache[key] {
            cacheLock.unlock()
            return cached
        }
        cacheLock.unlock()

        let cap = try await ZagDiscover.getCapability(provider: provider, bin: bin)

        cacheLock.lock()
        capCache[key] = cap
        cacheLock.unlock()
        return cap
    }

    private static func loadAllCapabilities(bin: String) async throws -> [ProviderCapability] {
        cacheLock.lock()
        if let cached = allCapsCache[bin] {
            cacheLock.unlock()
            return cached
        }
        cacheLock.unlock()

        let caps = try await ZagDiscover.getAllCapabilities(bin: bin)

        cacheLock.lock()
        allCapsCache[bin] = caps
        for c in caps {
            capCache[cacheKey(bin: bin, provider: c.provider)] = c
        }
        cacheLock.unlock()
        return caps
    }

    /// Check that every active requirement is supported by `provider`.
    ///
    /// Throws ``ZagFeatureUnsupportedError`` on the first unsupported feature.
    /// Returns silently when no requirement is active, when `provider` is nil
    /// (auto-detect), or when `provider` is `"mock"`.
    public static func check(
        bin: String,
        provider: String?,
        requirements: [Requirement]
    ) async throws {
        let active = requirements.filter(\.isSet)
        guard !active.isEmpty else { return }
        guard let provider = provider, provider != "mock" else { return }

        let cap = try await loadCapability(bin: bin, provider: provider)

        for req in active {
            if isSupported(cap, feature: req.feature) { continue }

            let caps = try await loadAllCapabilities(bin: bin)
            let supported = caps
                .filter { isSupported($0, feature: req.feature) }
                .map(\.provider)
            throw ZagFeatureUnsupportedError(
                method: req.method,
                feature: req.feature,
                provider: provider,
                supportedProviders: supported
            )
        }
    }
    #endif

    // MARK: - Test helpers

    internal static func setCapabilityForTesting(
        bin: String, provider: String, cap: ProviderCapability
    ) {
        cacheLock.lock()
        capCache[cacheKey(bin: bin, provider: provider)] = cap
        cacheLock.unlock()
    }

    internal static func setAllCapabilitiesForTesting(
        bin: String, caps: [ProviderCapability]
    ) {
        cacheLock.lock()
        allCapsCache[bin] = caps
        for c in caps {
            capCache[cacheKey(bin: bin, provider: c.provider)] = c
        }
        cacheLock.unlock()
    }

    internal static func clearCapabilityCache() {
        cacheLock.lock()
        capCache.removeAll()
        allCapsCache.removeAll()
        cacheLock.unlock()
    }
}
