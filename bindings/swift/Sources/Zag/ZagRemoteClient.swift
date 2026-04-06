import Foundation

// MARK: - Request / Response types

/// Parameters for spawning a remote session (mirrors SpawnRequest in zag-serve).
public struct SpawnParams: Codable, Sendable {
    public var prompt: String
    public var provider: String?
    public var model: String?
    public var root: String?
    public var autoApprove: Bool?
    public var systemPrompt: String?
    public var addDirs: [String]?
    public var size: String?
    public var maxTurns: Int?
    public var timeout: String?
    public var name: String?
    public var description: String?
    public var tags: [String]?
    public var dependsOn: [String]?
    public var injectContext: Bool?

    public init(
        prompt: String,
        provider: String? = nil,
        model: String? = nil,
        root: String? = nil,
        autoApprove: Bool? = nil,
        systemPrompt: String? = nil,
        addDirs: [String]? = nil,
        size: String? = nil,
        maxTurns: Int? = nil,
        timeout: String? = nil,
        name: String? = nil,
        description: String? = nil,
        tags: [String]? = nil,
        dependsOn: [String]? = nil,
        injectContext: Bool? = nil
    ) {
        self.prompt = prompt
        self.provider = provider
        self.model = model
        self.root = root
        self.autoApprove = autoApprove
        self.systemPrompt = systemPrompt
        self.addDirs = addDirs
        self.size = size
        self.maxTurns = maxTurns
        self.timeout = timeout
        self.name = name
        self.description = description
        self.tags = tags
        self.dependsOn = dependsOn
        self.injectContext = injectContext
    }
}

/// Response from POST /api/v1/sessions/spawn.
public struct SpawnResponse: Codable, Sendable {
    public let sessionId: String
    public let pid: Int
    public let logPath: String
}

/// Response from GET /api/v1/sessions/{id}/output.
public struct OutputResponse: Codable, Sendable {
    public let sessionId: String
    public let result: String?
}

/// Response from GET /api/v1/sessions/{id}/status.
public struct StatusResponse: Codable, Sendable {
    public let sessionId: String
    public let status: String
    public let provider: String?
    public let pid: Int?
}

/// A single result entry from wait/collect responses.
public struct SessionResult: Codable, Sendable {
    public let sessionId: String
    public let result: String?
    public let success: Bool?
    public let error: String?
}

// MARK: - Remote Client

/// HTTP and WebSocket client for communicating with a `zag serve` instance.
///
/// Uses `URLSession` for all network communication (no external dependencies).
///
/// ```swift
/// let client = ZagRemoteClient(
///     connection: ZagConnection(url: "https://server:2100", token: "my-token")
/// )
/// let spawn = try await client.spawn(SpawnParams(prompt: "analyze code"))
/// for try await event in client.stream(spawn.sessionId) {
///     // handle streaming events
/// }
/// ```
public final class ZagRemoteClient: @unchecked Sendable {
    private let connection: ZagConnection
    private let session: URLSession

    public init(connection: ZagConnection, session: URLSession = .shared) {
        self.connection = connection
        self.session = session
    }

    // MARK: - Session management

    /// Spawn a new background agent session.
    public func spawn(_ params: SpawnParams) async throws -> SpawnResponse {
        try await post("/api/v1/sessions/spawn", body: params)
    }

    /// List sessions, optionally filtered by tag, provider, or limit.
    public func listSessions(
        tag: String? = nil,
        provider: String? = nil,
        limit: Int? = nil,
        global: Bool? = nil
    ) async throws -> [SessionResult] {
        var query: [(String, String)] = []
        if let tag { query.append(("tag", tag)) }
        if let provider { query.append(("provider", provider)) }
        if let limit { query.append(("limit", String(limit))) }
        if let global { query.append(("global", String(global))) }
        return try await get("/api/v1/sessions", query: query)
    }

    /// Get the status of a session.
    public func status(_ sessionId: String) async throws -> StatusResponse {
        try await get("/api/v1/sessions/\(sessionId)/status")
    }

    /// Get events for a session.
    public func events(
        _ sessionId: String,
        type: String? = nil,
        last: Int? = nil,
        afterSeq: Int? = nil,
        beforeSeq: Int? = nil
    ) async throws -> [Event] {
        var query: [(String, String)] = []
        if let type { query.append(("type", type)) }
        if let last { query.append(("last", String(last))) }
        if let afterSeq { query.append(("after_seq", String(afterSeq))) }
        if let beforeSeq { query.append(("before_seq", String(beforeSeq))) }
        return try await get("/api/v1/sessions/\(sessionId)/events", query: query)
    }

    /// Get the final output of a completed session.
    public func output(_ sessionId: String) async throws -> OutputResponse {
        try await get("/api/v1/sessions/\(sessionId)/output")
    }

    /// Send a message to a running session.
    public func input(_ sessionId: String, message: String) async throws {
        struct InputRequest: Codable { let message: String }
        let _: EmptyResponse = try await post(
            "/api/v1/sessions/\(sessionId)/input",
            body: InputRequest(message: message))
    }

    /// Cancel a running session.
    public func cancel(_ sessionId: String, reason: String? = nil) async throws {
        struct CancelRequest: Codable { let reason: String? }
        let _: EmptyResponse = try await post(
            "/api/v1/sessions/\(sessionId)/cancel",
            body: CancelRequest(reason: reason))
    }

    /// Collect results from multiple sessions.
    public func collect(
        sessionIds: [String] = [],
        tag: String? = nil
    ) async throws -> [SessionResult] {
        struct CollectRequest: Codable { let sessionIds: [String]; let tag: String? }
        return try await post(
            "/api/v1/sessions/collect",
            body: CollectRequest(sessionIds: sessionIds, tag: tag))
    }

    /// Wait for sessions to complete.
    public func wait(
        sessionIds: [String] = [],
        tag: String? = nil,
        timeout: String? = nil,
        any: Bool? = nil
    ) async throws -> [SessionResult] {
        struct WaitRequest: Codable {
            let sessionIds: [String]
            let tag: String?
            let timeout: String?
            let any: Bool?
        }
        return try await post(
            "/api/v1/sessions/wait",
            body: WaitRequest(sessionIds: sessionIds, tag: tag, timeout: timeout, any: any))
    }

    // MARK: - WebSocket streaming

    /// Stream events from a session via WebSocket.
    public func stream(_ sessionId: String, filter: String? = nil) -> AsyncThrowingStream<Event, Error> {
        var query: [(String, String)] = []
        if let filter { query.append(("filter", filter)) }
        return websocketStream(path: "/api/v1/sessions/\(sessionId)/stream", query: query)
    }

    /// Subscribe to events across all sessions via WebSocket.
    public func subscribe(tag: String? = nil, type: String? = nil) -> AsyncThrowingStream<Event, Error> {
        var query: [(String, String)] = []
        if let tag { query.append(("tag", tag)) }
        if let type { query.append(("type", type)) }
        return websocketStream(path: "/api/v1/subscribe", query: query)
    }

    // MARK: - Composite operations

    /// Spawn a session, wait for it to complete, and return its output.
    /// This mirrors the behavior of the local `exec()` terminal method.
    public func exec(_ params: SpawnParams) async throws -> OutputResponse {
        let spawned = try await spawn(params)
        _ = try await wait(sessionIds: [spawned.sessionId])
        return try await output(spawned.sessionId)
    }

    // MARK: - Private HTTP helpers

    private struct EmptyResponse: Codable {}

    private func buildURL(path: String, query: [(String, String)] = []) -> URL {
        var components = URLComponents(url: connection.baseURL.appendingPathComponent(path), resolvingAgainstBaseURL: false)!
        if !query.isEmpty {
            components.queryItems = query.map { URLQueryItem(name: $0.0, value: $0.1) }
        }
        return components.url!
    }

    private func authorizedRequest(url: URL, method: String = "GET") -> URLRequest {
        var request = URLRequest(url: url)
        request.httpMethod = method
        request.setValue("Bearer \(connection.token)", forHTTPHeaderField: "Authorization")
        return request
    }

    private func get<T: Decodable>(_ path: String, query: [(String, String)] = []) async throws -> T {
        let url = buildURL(path: path, query: query)
        let request = authorizedRequest(url: url)
        let (data, response) = try await session.data(for: request)
        try checkResponse(response, data: data)
        return try JSONDecoder.zag.decode(T.self, from: data)
    }

    private func post<B: Encodable, T: Decodable>(_ path: String, body: B) async throws -> T {
        let url = buildURL(path: path)
        var request = authorizedRequest(url: url, method: "POST")
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.httpBody = try JSONEncoder.zag.encode(body)
        let (data, response) = try await session.data(for: request)
        try checkResponse(response, data: data)
        return try JSONDecoder.zag.decode(T.self, from: data)
    }

    private func checkResponse(_ response: URLResponse, data: Data) throws {
        guard let http = response as? HTTPURLResponse else {
            throw ZagError(message: "Invalid response from server")
        }
        guard (200...299).contains(http.statusCode) else {
            let body = String(data: data, encoding: .utf8) ?? ""
            throw ZagError(
                message: "Server returned HTTP \(http.statusCode): \(body)",
                statusCode: http.statusCode)
        }
    }

    private func websocketStream(path: String, query: [(String, String)] = []) -> AsyncThrowingStream<Event, Error> {
        let httpURL = buildURL(path: path, query: query)

        // Convert http(s) to ws(s)
        var components = URLComponents(url: httpURL, resolvingAgainstBaseURL: false)!
        if components.scheme == "https" {
            components.scheme = "wss"
        } else {
            components.scheme = "ws"
        }
        let wsURL = components.url!

        var request = URLRequest(url: wsURL)
        request.setValue("Bearer \(connection.token)", forHTTPHeaderField: "Authorization")

        let webSocketTask = session.webSocketTask(with: request)

        return AsyncThrowingStream { continuation in
            webSocketTask.resume()

            let readTask = Task {
                do {
                    while !Task.isCancelled {
                        let message = try await webSocketTask.receive()
                        switch message {
                        case .string(let text):
                            guard let data = text.data(using: .utf8) else { continue }
                            do {
                                let event = try JSONDecoder.zag.decode(Event.self, from: data)
                                continuation.yield(event)
                            } catch {
                                // Skip unparseable messages
                                continue
                            }
                        case .data(let data):
                            do {
                                let event = try JSONDecoder.zag.decode(Event.self, from: data)
                                continuation.yield(event)
                            } catch {
                                continue
                            }
                        @unknown default:
                            continue
                        }
                    }
                    continuation.finish()
                } catch {
                    continuation.finish(throwing: error)
                }
            }

            continuation.onTermination = { _ in
                readTask.cancel()
                webSocketTask.cancel(with: .goingAway, reason: nil)
            }
        }
    }
}
