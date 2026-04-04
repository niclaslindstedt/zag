import Foundation

/// Connection configuration for a remote zag-serve instance.
public struct ZagConnection: Sendable {
    /// Base URL of the zag server (e.g., `https://my-server:2100`).
    public let baseURL: URL

    /// Bearer token for authentication.
    public let token: String

    /// Create a connection from a `URL` and token.
    public init(baseURL: URL, token: String) {
        self.baseURL = baseURL
        self.token = token
    }

    /// Create a connection from a string URL and token.
    public init(url: String, token: String) throws {
        guard let parsed = URL(string: url) else {
            throw ZagError(message: "Invalid server URL: \(url)")
        }
        self.baseURL = parsed
        self.token = token
    }
}
