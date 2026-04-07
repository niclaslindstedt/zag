import SwiftUI

struct MessageBubbleView: View {
    let message: ChatMessage

    var body: some View {
        switch message.content {
        case .system(let text):
            Text(text)
                .font(.caption)
                .foregroundStyle(.secondary)
                .frame(maxWidth: .infinity, alignment: .center)
                .padding(.vertical, 4)

        case .toolUse(let toolName, let isComplete):
            ToolIndicatorView(toolName: toolName, isComplete: isComplete)
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(.vertical, 2)

        case .text(let text):
            HStack(alignment: .bottom, spacing: 0) {
                if message.role == .user {
                    Spacer(minLength: 60)
                }

                Text(text + (message.isStreaming ? "▍" : ""))
                    .textSelection(.enabled)
                    .padding(.horizontal, 12)
                    .padding(.vertical, 8)
                    .background(bubbleColor)
                    .foregroundStyle(foregroundColor)
                    .clipShape(RoundedRectangle(cornerRadius: 18))
                    .frame(
                        maxWidth: 480,
                        alignment: message.role == .user ? .trailing : .leading
                    )

                if message.role == .assistant {
                    Spacer(minLength: 60)
                }
            }
        }
    }

    private var bubbleColor: Color {
        switch message.role {
        case .user:      return .accentColor
        case .assistant: return Color(NSColor.controlBackgroundColor)
        case .system:    return .clear
        }
    }

    private var foregroundColor: Color {
        message.role == .user ? .white : .primary
    }
}
