import SwiftUI

struct ToolIndicatorView: View {
    let toolName: String
    let isComplete: Bool

    @State private var rotation: Double = 0

    var body: some View {
        HStack(spacing: 6) {
            if isComplete {
                Image(systemName: "checkmark.circle.fill")
                    .foregroundStyle(.green)
                    .font(.caption)
            } else {
                Image(systemName: "gear")
                    .rotationEffect(.degrees(rotation))
                    .foregroundStyle(.secondary)
                    .font(.caption)
                    .onAppear {
                        withAnimation(.linear(duration: 1).repeatForever(autoreverses: false)) {
                            rotation = 360
                        }
                    }
            }

            Text(isComplete ? "Used \(toolName)" : "Using \(toolName)…")
                .font(.caption)
                .foregroundStyle(.secondary)
        }
        .padding(.horizontal, 10)
        .padding(.vertical, 5)
        .background(.quaternary, in: Capsule())
    }
}
