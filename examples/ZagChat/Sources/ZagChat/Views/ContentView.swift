import SwiftUI
import AppKit

struct ContentView: View {
    @StateObject private var viewModel = ChatViewModel()
    @State private var showingNewChatSheet = false

    var body: some View {
        VStack(spacing: 0) {
            SessionStatusView(
                modelInfo: viewModel.modelInfo,
                workingDirectory: viewModel.workingDirectory,
                sessionState: viewModel.sessionState,
                onNewChat: {
                    guard viewModel.canStartNewChat else { return }
                    showingNewChatSheet = true
                }
            )

            Divider()

            if viewModel.messages.isEmpty {
                WelcomeView(onStart: { showingNewChatSheet = true })
            } else {
                MessageListView(messages: viewModel.messages)
            }

            Divider()

            InputBarView(
                text: $viewModel.inputText,
                onSend: viewModel.sendMessage,
                canSend: viewModel.canSendMessage,
                isAgentTyping: viewModel.sessionState == .agentTyping
                    || viewModel.sessionState == .starting
            )
            .padding(.horizontal, 12)
            .padding(.vertical, 8)
        }
        .frame(minWidth: 520, minHeight: 420)
        .background(EscKeyHandler { viewModel.interrupt() })
        .sheet(isPresented: $showingNewChatSheet) {
            NewChatSheet { prompt, provider, model, root in
                showingNewChatSheet = false
                viewModel.startNewChat(initialPrompt: prompt, provider: provider, model: model, root: root)
            }
        }
    }
}

/// Invisible view that installs an NSEvent local monitor for the ESC key.
private struct EscKeyHandler: NSViewRepresentable {
    let onEscape: () -> Void

    func makeNSView(context: Context) -> NSView {
        let view = KeyCatchView()
        view.onEscape = onEscape
        return view
    }

    func updateNSView(_ nsView: NSView, context: Context) {
        (nsView as? KeyCatchView)?.onEscape = onEscape
    }

    private class KeyCatchView: NSView {
        var onEscape: (() -> Void)?
        private var monitor: Any?

        override func viewDidMoveToWindow() {
            super.viewDidMoveToWindow()
            if window != nil, monitor == nil {
                monitor = NSEvent.addLocalMonitorForEvents(matching: .keyDown) { [weak self] event in
                    if event.keyCode == 53 { // ESC
                        self?.onEscape?()
                        return nil // consume the event
                    }
                    return event
                }
            }
        }

        override func removeFromSuperview() {
            if let monitor { NSEvent.removeMonitor(monitor) }
            monitor = nil
            super.removeFromSuperview()
        }

        deinit {
            if let monitor { NSEvent.removeMonitor(monitor) }
        }
    }
}
