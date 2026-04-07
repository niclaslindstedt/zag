import Foundation

// MARK: - JSONValue (untyped JSON)

/// A type-safe representation of arbitrary JSON values.
public enum JSONValue: Codable, Equatable, Sendable {
    case null
    case bool(Bool)
    case int(Int)
    case double(Double)
    case string(String)
    case array([JSONValue])
    case object([String: JSONValue])

    public init(from decoder: Decoder) throws {
        let container = try decoder.singleValueContainer()
        if container.decodeNil() {
            self = .null
        } else if let v = try? container.decode(Bool.self) {
            self = .bool(v)
        } else if let v = try? container.decode(Int.self) {
            self = .int(v)
        } else if let v = try? container.decode(Double.self) {
            self = .double(v)
        } else if let v = try? container.decode(String.self) {
            self = .string(v)
        } else if let v = try? container.decode([JSONValue].self) {
            self = .array(v)
        } else if let v = try? container.decode([String: JSONValue].self) {
            self = .object(v)
        } else {
            throw DecodingError.dataCorruptedError(
                in: container, debugDescription: "Cannot decode JSONValue")
        }
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.singleValueContainer()
        switch self {
        case .null: try container.encodeNil()
        case .bool(let v): try container.encode(v)
        case .int(let v): try container.encode(v)
        case .double(let v): try container.encode(v)
        case .string(let v): try container.encode(v)
        case .array(let v): try container.encode(v)
        case .object(let v): try container.encode(v)
        }
    }
}

// MARK: - Usage

/// Token usage statistics.
public struct Usage: Codable, Equatable, Sendable {
    public let inputTokens: Int
    public let outputTokens: Int
    public let cacheReadTokens: Int?
    public let cacheCreationTokens: Int?
    public let webSearchRequests: Int?
    public let webFetchRequests: Int?

    public init(
        inputTokens: Int,
        outputTokens: Int,
        cacheReadTokens: Int? = nil,
        cacheCreationTokens: Int? = nil,
        webSearchRequests: Int? = nil,
        webFetchRequests: Int? = nil
    ) {
        self.inputTokens = inputTokens
        self.outputTokens = outputTokens
        self.cacheReadTokens = cacheReadTokens
        self.cacheCreationTokens = cacheCreationTokens
        self.webSearchRequests = webSearchRequests
        self.webFetchRequests = webFetchRequests
    }
}

// MARK: - ToolResult

/// Result from a tool execution.
public struct ToolResult: Codable, Equatable, Sendable {
    public let success: Bool
    public let output: String?
    public let error: String?
    public let data: JSONValue?

    public init(success: Bool, output: String? = nil, error: String? = nil, data: JSONValue? = nil) {
        self.success = success
        self.output = output
        self.error = error
        self.data = data
    }
}

// MARK: - ContentBlock

/// A block of content in a message.
public enum ContentBlock: Codable, Equatable, Sendable {
    case text(TextBlock)
    case toolUse(ToolUseBlock)

    public struct TextBlock: Codable, Equatable, Sendable {
        public let text: String

        public init(text: String) {
            self.text = text
        }
    }

    public struct ToolUseBlock: Codable, Equatable, Sendable {
        public let id: String
        public let name: String
        public let input: JSONValue?

        public init(id: String, name: String, input: JSONValue? = nil) {
            self.id = id
            self.name = name
            self.input = input
        }
    }

    private enum CodingKeys: String, CodingKey {
        case type
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        let type = try container.decode(String.self, forKey: .type)
        switch type {
        case "text":
            self = .text(try TextBlock(from: decoder))
        case "tool_use":
            self = .toolUse(try ToolUseBlock(from: decoder))
        default:
            throw DecodingError.dataCorruptedError(
                forKey: .type, in: container,
                debugDescription: "Unknown content block type: \(type)")
        }
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        switch self {
        case .text(let block):
            try container.encode("text", forKey: .type)
            try block.encode(to: encoder)
        case .toolUse(let block):
            try container.encode("tool_use", forKey: .type)
            try block.encode(to: encoder)
        }
    }
}

// MARK: - Event

/// An agent session event (tagged union on "type" field).
public enum Event: Codable, Equatable, Sendable {
    case `init`(InitPayload)
    case userMessage(UserMessagePayload)
    case assistantMessage(AssistantMessagePayload)
    case toolExecution(ToolExecutionPayload)
    case result(ResultPayload)
    case error(ErrorPayload)
    case permissionRequest(PermissionRequestPayload)

    // MARK: Payload types

    public struct InitPayload: Codable, Equatable, Sendable {
        public let model: String
        public let tools: [String]
        public let workingDirectory: String?
        public let metadata: [String: JSONValue]

        public init(
            model: String, tools: [String],
            workingDirectory: String? = nil,
            metadata: [String: JSONValue] = [:]
        ) {
            self.model = model
            self.tools = tools
            self.workingDirectory = workingDirectory
            self.metadata = metadata
        }
    }

    public struct UserMessagePayload: Codable, Equatable, Sendable {
        public let content: [ContentBlock]

        public init(content: [ContentBlock]) {
            self.content = content
        }
    }

    public struct AssistantMessagePayload: Codable, Equatable, Sendable {
        public let content: [ContentBlock]
        public let usage: Usage?

        public init(content: [ContentBlock], usage: Usage? = nil) {
            self.content = content
            self.usage = usage
        }
    }

    public struct ToolExecutionPayload: Codable, Equatable, Sendable {
        public let toolName: String
        public let toolId: String
        public let input: JSONValue?
        public let result: ToolResult

        public init(toolName: String, toolId: String, input: JSONValue? = nil, result: ToolResult) {
            self.toolName = toolName
            self.toolId = toolId
            self.input = input
            self.result = result
        }
    }

    public struct ResultPayload: Codable, Equatable, Sendable {
        public let success: Bool
        public let message: String?
        public let durationMs: Int?
        public let numTurns: Int?

        public init(success: Bool, message: String? = nil, durationMs: Int? = nil, numTurns: Int? = nil) {
            self.success = success
            self.message = message
            self.durationMs = durationMs
            self.numTurns = numTurns
        }
    }

    public struct ErrorPayload: Codable, Equatable, Sendable {
        public let message: String
        public let details: JSONValue?

        public init(message: String, details: JSONValue? = nil) {
            self.message = message
            self.details = details
        }
    }

    public struct PermissionRequestPayload: Codable, Equatable, Sendable {
        public let toolName: String
        public let description: String
        public let granted: Bool

        public init(toolName: String, description: String, granted: Bool) {
            self.toolName = toolName
            self.description = description
            self.granted = granted
        }
    }

    // MARK: Codable

    private enum CodingKeys: String, CodingKey {
        case type
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        let type = try container.decode(String.self, forKey: .type)
        switch type {
        case "init":
            self = .`init`(try InitPayload(from: decoder))
        case "user_message":
            self = .userMessage(try UserMessagePayload(from: decoder))
        case "assistant_message":
            self = .assistantMessage(try AssistantMessagePayload(from: decoder))
        case "tool_execution":
            self = .toolExecution(try ToolExecutionPayload(from: decoder))
        case "result":
            self = .result(try ResultPayload(from: decoder))
        case "error":
            self = .error(try ErrorPayload(from: decoder))
        case "permission_request":
            self = .permissionRequest(try PermissionRequestPayload(from: decoder))
        default:
            throw DecodingError.dataCorruptedError(
                forKey: .type, in: container,
                debugDescription: "Unknown event type: \(type)")
        }
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        switch self {
        case .`init`(let p):
            try container.encode("init", forKey: .type)
            try p.encode(to: encoder)
        case .userMessage(let p):
            try container.encode("user_message", forKey: .type)
            try p.encode(to: encoder)
        case .assistantMessage(let p):
            try container.encode("assistant_message", forKey: .type)
            try p.encode(to: encoder)
        case .toolExecution(let p):
            try container.encode("tool_execution", forKey: .type)
            try p.encode(to: encoder)
        case .result(let p):
            try container.encode("result", forKey: .type)
            try p.encode(to: encoder)
        case .error(let p):
            try container.encode("error", forKey: .type)
            try p.encode(to: encoder)
        case .permissionRequest(let p):
            try container.encode("permission_request", forKey: .type)
            try p.encode(to: encoder)
        }
    }
}

// MARK: - AgentOutput

/// Unified output from an agent session.
public struct AgentOutput: Codable, Equatable, Sendable {
    public let agent: String
    public let sessionId: String
    public let events: [Event]
    public let result: String?
    public let isError: Bool
    public let exitCode: Int?
    public let errorMessage: String?
    public let totalCostUsd: Double?
    public let usage: Usage?

    public init(
        agent: String,
        sessionId: String,
        events: [Event] = [],
        result: String? = nil,
        isError: Bool = false,
        exitCode: Int? = nil,
        errorMessage: String? = nil,
        totalCostUsd: Double? = nil,
        usage: Usage? = nil
    ) {
        self.agent = agent
        self.sessionId = sessionId
        self.events = events
        self.result = result
        self.isError = isError
        self.exitCode = exitCode
        self.errorMessage = errorMessage
        self.totalCostUsd = totalCostUsd
        self.usage = usage
    }
}

// MARK: - Shared decoder / encoder

extension JSONDecoder {
    /// A pre-configured decoder for zag JSON output (snake_case keys).
    static let zag: JSONDecoder = {
        let decoder = JSONDecoder()
        decoder.keyDecodingStrategy = .convertFromSnakeCase
        return decoder
    }()
}

extension JSONEncoder {
    /// A pre-configured encoder for zag JSON input (snake_case keys).
    static let zag: JSONEncoder = {
        let encoder = JSONEncoder()
        encoder.keyEncodingStrategy = .convertToSnakeCase
        return encoder
    }()
}
