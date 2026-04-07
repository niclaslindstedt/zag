import Foundation
import Zag

struct ToolDetail {
    let toolId: String
    let toolName: String
    let input: JSONValue?
    var result: ToolResult?
    var isComplete: Bool
    /// For Agent tools: child tool calls made by the sub-agent.
    var children: [ToolDetail]
}

enum MessageRole {
    case user
    case assistant
    case system
}

enum MessageContent {
    case text(String)
    case toolUse(ToolDetail)
    case system(String)
}

struct ChatMessage: Identifiable {
    let id: UUID
    let role: MessageRole
    var content: MessageContent
    let timestamp: Date
    /// True while an assistant bubble is still receiving streaming chunks.
    var isStreaming: Bool

    init(
        id: UUID = UUID(),
        role: MessageRole,
        content: MessageContent,
        timestamp: Date = Date(),
        isStreaming: Bool = false
    ) {
        self.id = id
        self.role = role
        self.content = content
        self.timestamp = timestamp
        self.isStreaming = isStreaming
    }
}
