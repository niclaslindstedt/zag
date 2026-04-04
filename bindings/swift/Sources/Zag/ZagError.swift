import Foundation

/// Error thrown when the zag process fails.
public struct ZagError: Error, CustomStringConvertible {
    /// Human-readable error message.
    public let message: String

    /// Process exit code, if available.
    public let exitCode: Int?

    /// Captured stderr output.
    public let stderr: String

    public var description: String { message }

    public init(message: String, exitCode: Int? = nil, stderr: String = "") {
        self.message = message
        self.exitCode = exitCode
        self.stderr = stderr
    }
}
