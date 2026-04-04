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
