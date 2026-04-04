import Foundation

/// A live remote streaming session backed by WebSocket.
///
/// This is the remote equivalent of `StreamingSession` for bidirectional
/// communication with a running agent session via `zag serve`.
public final class ZagRemoteSession: @unchecked Sendable {
    private let webSocketTask: URLSessionWebSocketTask
    private let client: ZagRemoteClient
    private let sessionId: String

    init(webSocketTask: URLSessionWebSocketTask, client: ZagRemoteClient, sessionId: String) {
        self.webSocketTask = webSocketTask
        self.client = client
        self.sessionId = sessionId
        webSocketTask.resume()
    }

    /// Send a raw JSON string to the server via WebSocket.
    public func send(_ message: String) async throws {
        try await webSocketTask.send(.string(message))
    }

    /// Send a user message to the running agent session via HTTP.
    public func sendUserMessage(_ content: String) async throws {
        try await client.input(sessionId, message: content)
    }

    /// Async stream of parsed `Event` objects from the WebSocket.
    public var events: AsyncThrowingStream<Event, Error> {
        AsyncThrowingStream { continuation in
            let task = Task {
                do {
                    while !Task.isCancelled {
                        let message = try await self.webSocketTask.receive()
                        switch message {
                        case .string(let text):
                            guard let data = text.data(using: .utf8) else { continue }
                            do {
                                let event = try JSONDecoder.zag.decode(Event.self, from: data)
                                continuation.yield(event)
                            } catch {
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
                task.cancel()
            }
        }
    }

    /// Close the WebSocket connection.
    public func close() {
        webSocketTask.cancel(with: .goingAway, reason: nil)
    }
}
