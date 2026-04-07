import Foundation

enum MessageRole {
    case user
    case assistant
    case system
}

enum MessageContent {
    case text(String)
    case toolUse(toolName: String, isComplete: Bool)
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
