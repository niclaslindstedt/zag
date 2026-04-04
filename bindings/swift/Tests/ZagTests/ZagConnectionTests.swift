import Foundation
import Testing
@testable import Zag

@Suite("ZagConnection")
struct ZagConnectionTests {

    @Test("init from URL and token")
    func initFromURL() {
        let url = URL(string: "https://example.com:2100")!
        let conn = ZagConnection(baseURL: url, token: "abc123")
        #expect(conn.baseURL == url)
        #expect(conn.token == "abc123")
    }

    @Test("init from string URL and token")
    func initFromString() throws {
        let conn = try ZagConnection(url: "https://example.com:2100", token: "tok")
        #expect(conn.baseURL.absoluteString == "https://example.com:2100")
        #expect(conn.token == "tok")
    }

    @Test("invalid URL throws")
    func invalidURL() {
        #expect(throws: ZagError.self) {
            try ZagConnection(url: "", token: "tok")
        }
    }
}
