import SwiftUI

struct InputBarView: View {
    @Binding var text: String
    let onSend: () -> Void
    let canSend: Bool
    let isAgentTyping: Bool

    var body: some View {
        HStack(alignment: .bottom, spacing: 8) {
            TextField("Message…", text: $text, axis: .vertical)
                .lineLimit(1...6)
                .textFieldStyle(.plain)
                .padding(.horizontal, 12)
                .padding(.vertical, 8)
                .background(.quaternary, in: RoundedRectangle(cornerRadius: 20))
                .onSubmit {
                    if canSend { onSend() }
                }

            Button(action: { if canSend { onSend() } }) {
                if isAgentTyping {
                    ProgressView()
                        .controlSize(.small)
                        .frame(width: 30, height: 30)
                } else {
                    Image(systemName: "arrow.up.circle.fill")
                        .font(.title2)
                        .foregroundStyle(canSend ? Color.accentColor : Color.secondary)
                        .frame(width: 30, height: 30)
                }
            }
            .buttonStyle(.plain)
            .disabled(!canSend && !isAgentTyping)
        }
    }
}
