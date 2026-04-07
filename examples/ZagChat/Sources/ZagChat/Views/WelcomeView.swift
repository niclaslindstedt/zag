import SwiftUI

struct WelcomeView: View {
    let onStart: () -> Void

    var body: some View {
        VStack(spacing: 16) {
            Image(systemName: "bubble.left.and.bubble.right")
                .font(.system(size: 52))
                .foregroundStyle(.tertiary)

            Text("ZagChat")
                .font(.largeTitle)
                .fontWeight(.semibold)

            Text("Start a conversation with an AI agent.")
                .foregroundStyle(.secondary)

            Button("New Chat") { onStart() }
                .buttonStyle(.borderedProminent)
                .controlSize(.large)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}
