import SwiftUI

struct MessageListView: View {
    let messages: [ChatMessage]

    private let bottomAnchorID = "list-bottom"

    var body: some View {
        ScrollViewReader { proxy in
            ScrollView {
                LazyVStack(alignment: .leading, spacing: 6) {
                    ForEach(messages) { message in
                        MessageBubbleView(message: message)
                    }
                }
                .padding(.horizontal, 16)
                .padding(.top, 12)
                .padding(.bottom, 8)

                // Invisible anchor at the bottom for scrolling
                Color.clear
                    .frame(height: 1)
                    .id(bottomAnchorID)
            }
            .onChange(of: messages.count) { _ in
                withAnimation(.easeOut(duration: 0.15)) {
                    proxy.scrollTo(bottomAnchorID)
                }
            }
        }
    }
}
