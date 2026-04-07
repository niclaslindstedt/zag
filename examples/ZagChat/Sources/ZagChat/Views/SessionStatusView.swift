import SwiftUI

struct SessionStatusView: View {
    let modelInfo: String
    let workingDirectory: String
    let sessionState: ChatViewModel.SessionState
    let onNewChat: () -> Void

    private var folderName: String {
        guard !workingDirectory.isEmpty else { return "" }
        return URL(fileURLWithPath: workingDirectory).lastPathComponent
    }

    var body: some View {
        HStack(spacing: 8) {
            Circle()
                .fill(statusColor)
                .frame(width: 8, height: 8)

            Text(statusLabel)
                .font(.caption)
                .foregroundStyle(.secondary)

            if !modelInfo.isEmpty {
                Text("·")
                    .foregroundStyle(.tertiary)
                Text(modelInfo)
                    .font(.caption)
                    .foregroundStyle(.tertiary)
            }

            if !folderName.isEmpty {
                Text("·")
                    .foregroundStyle(.tertiary)
                HStack(spacing: 3) {
                    Image(systemName: "folder")
                        .font(.caption2)
                    Text(folderName)
                        .lineLimit(1)
                }
                .font(.caption)
                .foregroundStyle(.tertiary)
                .help(workingDirectory)
            }

            Spacer()

            Button("New Chat", action: onNewChat)
                .buttonStyle(.borderless)
                .font(.caption)
                .foregroundColor(.accentColor)
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 8)
        .background(.bar)
    }

    private var statusColor: Color {
        switch sessionState {
        case .idle:        return Color.gray.opacity(0.5)
        case .starting:    return .orange
        case .running:     return .green
        case .agentTyping: return .blue
        case .interrupted: return .yellow
        case .finished:    return .gray
        case .error:       return .red
        }
    }

    private var statusLabel: String {
        switch sessionState {
        case .idle:        return "No session"
        case .starting:    return "Starting…"
        case .running:     return "Ready"
        case .agentTyping: return "Agent typing… (ESC to stop)"
        case .interrupted: return "Interrupted"
        case .finished:    return "Session ended"
        case .error(let m):
            let prefix = String(m.prefix(40))
            return "Error: \(prefix)\(m.count > 40 ? "…" : "")"
        }
    }
}
