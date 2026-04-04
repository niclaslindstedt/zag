import Foundation
import Testing
@testable import Zag

// MARK: - Mock URL Protocol

/// A `URLProtocol` subclass that intercepts HTTP requests for testing.
final class MockURLProtocol: URLProtocol, @unchecked Sendable {
    nonisolated(unsafe) static var requestHandler: ((URLRequest) throws -> (Data, HTTPURLResponse))?

    override class func canInit(with request: URLRequest) -> Bool { true }

    override class func canonicalRequest(for request: URLRequest) -> URLRequest { request }

    override func startLoading() {
        guard let handler = MockURLProtocol.requestHandler else {
            client?.urlProtocolDidFinishLoading(self)
            return
        }

        do {
            let (data, response) = try handler(request)
            client?.urlProtocol(self, didReceive: response, cacheStoragePolicy: .notAllowed)
            client?.urlProtocol(self, didLoad: data)
            client?.urlProtocolDidFinishLoading(self)
        } catch {
            client?.urlProtocol(self, didFailWithError: error)
        }
    }

    override func stopLoading() {}
}

// MARK: - Helper

private func makeClient() throws -> (ZagRemoteClient, ZagConnection) {
    let config = URLSessionConfiguration.ephemeral
    config.protocolClasses = [MockURLProtocol.self]
    let session = URLSession(configuration: config)
    let conn = try ZagConnection(url: "https://test.local:2100", token: "test-token-abc")
    let client = ZagRemoteClient(connection: conn, session: session)
    return (client, conn)
}

private func jsonResponse(_ json: String, statusCode: Int = 200, url: URL? = nil) -> (Data, HTTPURLResponse) {
    let data = json.data(using: .utf8)!
    let response = HTTPURLResponse(
        url: url ?? URL(string: "https://test.local:2100")!,
        statusCode: statusCode,
        httpVersion: "HTTP/1.1",
        headerFields: ["Content-Type": "application/json"])!
    return (data, response)
}

// MARK: - Tests

@Suite("ZagRemoteClient")
struct ZagRemoteClientTests {

    @Test("spawn sends correct request")
    func spawnSendsCorrectRequest() async throws {
        let (client, _) = try makeClient()

        MockURLProtocol.requestHandler = { request in
            // Verify auth header
            #expect(request.value(forHTTPHeaderField: "Authorization") == "Bearer test-token-abc")
            #expect(request.httpMethod == "POST")
            #expect(request.url?.path.hasSuffix("/api/v1/sessions/spawn") == true)

            // Verify body
            if let body = request.httpBody {
                let decoded = try JSONDecoder.zag.decode(SpawnParams.self, from: body)
                #expect(decoded.prompt == "hello")
                #expect(decoded.provider == "claude")
            }

            return jsonResponse("""
                {"session_id": "sess-1", "pid": 123, "log_path": "/tmp/log"}
                """)
        }

        let response = try await client.spawn(SpawnParams(prompt: "hello", provider: "claude"))
        #expect(response.sessionId == "sess-1")
        #expect(response.pid == 123)
        #expect(response.logPath == "/tmp/log")
    }

    @Test("status returns parsed response")
    func statusReturnsParsedResponse() async throws {
        let (client, _) = try makeClient()

        MockURLProtocol.requestHandler = { request in
            #expect(request.httpMethod == "GET")
            #expect(request.url?.path.hasSuffix("/api/v1/sessions/sess-1/status") == true)

            return jsonResponse("""
                {"session_id": "sess-1", "status": "completed", "provider": "claude", "pid": 123}
                """)
        }

        let status = try await client.status("sess-1")
        #expect(status.sessionId == "sess-1")
        #expect(status.status == "completed")
        #expect(status.provider == "claude")
    }

    @Test("output returns parsed response")
    func outputReturnsParsedResponse() async throws {
        let (client, _) = try makeClient()

        MockURLProtocol.requestHandler = { _ in
            return jsonResponse("""
                {"session_id": "sess-1", "result": "Hello, world!"}
                """)
        }

        let output = try await client.output("sess-1")
        #expect(output.sessionId == "sess-1")
        #expect(output.result == "Hello, world!")
    }

    @Test("error response throws ZagError with status code")
    func errorResponseThrowsZagError() async throws {
        let (client, _) = try makeClient()

        MockURLProtocol.requestHandler = { _ in
            return jsonResponse("""
                {"error": "not found"}
                """, statusCode: 404)
        }

        do {
            _ = try await client.status("nonexistent")
            Issue.record("Expected ZagError to be thrown")
        } catch let error as ZagError {
            #expect(error.statusCode == 404)
            #expect(error.message.contains("404"))
        }
    }

    @Test("auth header is set on all requests")
    func authHeaderIsSet() async throws {
        let (client, _) = try makeClient()

        MockURLProtocol.requestHandler = { request in
            #expect(request.value(forHTTPHeaderField: "Authorization") == "Bearer test-token-abc")
            return jsonResponse("""
                {"session_id": "sess-1", "result": null}
                """)
        }

        _ = try await client.output("sess-1")
    }

    @Test("cancel sends correct request")
    func cancelSendsCorrectRequest() async throws {
        let (client, _) = try makeClient()

        MockURLProtocol.requestHandler = { request in
            #expect(request.httpMethod == "POST")
            #expect(request.url?.path.hasSuffix("/api/v1/sessions/sess-1/cancel") == true)
            return jsonResponse("{}")
        }

        try await client.cancel("sess-1", reason: "test cancel")
    }

    @Test("wait sends correct request")
    func waitSendsCorrectRequest() async throws {
        let (client, _) = try makeClient()

        MockURLProtocol.requestHandler = { request in
            #expect(request.httpMethod == "POST")
            #expect(request.url?.path.hasSuffix("/api/v1/sessions/wait") == true)

            if let body = request.httpBody {
                let json = try JSONSerialization.jsonObject(with: body) as! [String: Any]
                let ids = json["session_ids"] as! [String]
                #expect(ids == ["sess-1", "sess-2"])
            }

            return jsonResponse("[]")
        }

        _ = try await client.wait(sessionIds: ["sess-1", "sess-2"])
    }

    @Test("SpawnParams encodes with snake_case keys")
    func spawnParamsEncodesSnakeCase() throws {
        let params = SpawnParams(
            prompt: "test",
            autoApprove: true,
            systemPrompt: "Be helpful",
            addDirs: ["/a"],
            maxTurns: 5
        )
        let data = try JSONEncoder.zag.encode(params)
        let json = try JSONSerialization.jsonObject(with: data) as! [String: Any]

        #expect(json["auto_approve"] as? Bool == true)
        #expect(json["system_prompt"] as? String == "Be helpful")
        #expect(json["add_dirs"] as? [String] == ["/a"])
        #expect(json["max_turns"] as? Int == 5)
    }
}
