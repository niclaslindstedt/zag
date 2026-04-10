import Foundation

/// Error thrown when the zag process or remote server fails.
public struct ZagError: Error, CustomStringConvertible {
    /// Human-readable error message.
    public let message: String

    /// Process exit code, if available (local execution only).
    public let exitCode: Int?

    /// Captured stderr output (local execution only).
    public let stderr: String

    /// HTTP status code, if available (remote execution only).
    public let statusCode: Int?

    public var description: String { message }

    public init(message: String, exitCode: Int? = nil, stderr: String = "", statusCode: Int? = nil) {
        self.message = message
        self.exitCode = exitCode
        self.stderr = stderr
        self.statusCode = statusCode
    }
}

/// Error thrown when a builder option requires a provider feature that
/// the configured provider does not support.
///
/// The builder validates feature-gated options (`execStreaming`,
/// `worktree`, `sandbox`, `systemPrompt`, `addDir`, `maxTurns`) against
/// the capability declarations exposed by `zag discover` before spawning
/// the CLI, so callers receive a clear, typed error instead of a cryptic
/// runtime exit code.
public struct ZagFeatureUnsupportedError: Error, CustomStringConvertible {
    /// Human-readable error message.
    public let message: String

    /// The provider that does not support the feature.
    public let provider: String

    /// The feature key (e.g. `"streaming_input"`).
    public let feature: String

    /// The builder method that requires the feature (e.g. `"execStreaming()"`).
    public let method: String

    /// Providers that do support the feature.
    public let supportedProviders: [String]

    public var description: String { message }

    public init(
        message: String,
        provider: String,
        feature: String,
        method: String,
        supportedProviders: [String]
    ) {
        self.message = message
        self.provider = provider
        self.feature = feature
        self.method = method
        self.supportedProviders = supportedProviders
    }
}
