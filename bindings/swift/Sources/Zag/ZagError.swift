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

/// Error thrown by the capability preflight when a builder method is called
/// for a feature the configured provider does not support. Thrown before any
/// subprocess is spawned so callers can catch it distinctly from a runtime
/// ``ZagError``.
///
/// Swift cannot express subclass-style inheritance across struct types, so
/// this is a separate error type. Callers who want to handle both should
/// catch ``ZagFeatureUnsupportedError`` first, then fall through to
/// ``ZagError`` (or catch a generic `Error`). The error also exposes
/// ``asZagError`` if you need a `ZagError` value with the same message.
public struct ZagFeatureUnsupportedError: Error, CustomStringConvertible {
    public let method: String
    public let feature: String
    public let provider: String
    public let supportedProviders: [String]
    public let message: String

    public var description: String { message }

    /// A ``ZagError`` view with the same message, for callers that only
    /// branch on ``ZagError``.
    public var asZagError: ZagError {
        ZagError(message: message, exitCode: nil, stderr: "")
    }

    public init(
        method: String,
        feature: String,
        provider: String,
        supportedProviders: [String]
    ) {
        self.method = method
        self.feature = feature
        self.provider = provider
        self.supportedProviders = supportedProviders
        let supportedList = supportedProviders.isEmpty
            ? "(none)"
            : supportedProviders.joined(separator: ", ")
        self.message =
            "\(method) is not supported by provider '\(provider)' " +
            "(feature: \(feature)). Supported providers: \(supportedList)"
    }
}
