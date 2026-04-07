import SwiftUI

struct ToolIndicatorView: View {
    let detail: ToolDetail

    @State private var isExpanded = false
    @State private var userToggled = false
    @State private var rotation: Double = 0

    /// Agent tools can be expanded while still running (to show live child activity).
    private var canExpand: Bool {
        detail.isComplete || !detail.children.isEmpty
    }

    /// Auto-expand Agent bubbles when children arrive (unless user collapsed it).
    private var effectiveExpanded: Bool {
        if userToggled { return isExpanded }
        return !detail.children.isEmpty
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Button {
                guard canExpand else { return }
                withAnimation(.spring(duration: 0.25)) {
                    isExpanded.toggle()
                    userToggled = true
                }
            } label: {
                HStack(spacing: 6) {
                    statusIcon
                    Text(statusText)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                    if canExpand {
                        Image(systemName: effectiveExpanded ? "chevron.up" : "chevron.down")
                            .font(.caption2)
                            .foregroundStyle(.tertiary)
                    }
                }
                .padding(.horizontal, 10)
                .padding(.vertical, 5)
                .background(.quaternary, in: Capsule())
            }
            .buttonStyle(.plain)

            if effectiveExpanded, canExpand {
                ToolDetailContentView(detail: detail)
                    .transition(.opacity.combined(with: .move(edge: .top)))
            }
        }
    }

    @ViewBuilder
    private var statusIcon: some View {
        if detail.isComplete {
            if detail.result?.success == true {
                Image(systemName: "checkmark.circle.fill")
                    .foregroundStyle(.green)
                    .font(.caption)
            } else {
                Image(systemName: "xmark.circle.fill")
                    .foregroundStyle(.red)
                    .font(.caption)
            }
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
    }

    private var statusText: String {
        detail.isComplete ? "Used \(detail.toolName)" : "Using \(detail.toolName)…"
    }
}
