import Foundation
import Zag

@MainActor
final class ChatViewModel: ObservableObject {

    enum SessionState: Equatable {
        case idle
        case starting
        case running
        case agentTyping
        case interrupted     // user pressed ESC — session killed, ready to resume
        case finished
        case error(String)

        static func == (lhs: SessionState, rhs: SessionState) -> Bool {
            switch (lhs, rhs) {
            case (.idle, .idle), (.starting, .starting), (.running, .running),
                 (.agentTyping, .agentTyping), (.interrupted, .interrupted),
                 (.finished, .finished):
                return true
            case (.error(let a), .error(let b)):
                return a == b
            default:
                return false
            }
        }
    }

    @Published var messages: [ChatMessage] = []
    @Published var inputText: String = ""
    @Published var sessionState: SessionState = .idle
    @Published var modelInfo: String = ""
    @Published var workingDirectory: String = ""

    private var session: StreamingSession?
    private var eventTask: Task<Void, Never>?
    private var currentAssistantMessageId: UUID?

    // Builder config preserved across turns.
    private var provider: String = "claude"
    private var model: String?
    private var root: String?

    /// Session ID captured from the last init event, used for --resume after interrupt.
    private var lastSessionId: String?

    var canSendMessage: Bool {
        guard !inputText.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else { return false }
        switch sessionState {
        case .running, .finished, .interrupted: return true
        default: return false
        }
    }

    var canStartNewChat: Bool {
        sessionState != .starting
    }

    /// True when there is an active agent that can be interrupted with ESC.
    var canInterrupt: Bool {
        switch sessionState {
        case .starting, .agentTyping, .running: return session?.isRunning == true
        default: return false
        }
    }

    // MARK: - Session lifecycle

    func startNewChat(initialPrompt: String, provider: String, model: String?, root: String?) {
        killSession()
        messages = []
        currentAssistantMessageId = nil
        modelInfo = ""
        lastSessionId = nil
        self.provider = provider
        self.model = model
        self.root = root
        workingDirectory = root ?? FileManager.default.currentDirectoryPath

        execPrompt(initialPrompt, resume: false)
    }

    func sendMessage() {
        let text = inputText.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !text.isEmpty else { return }
        inputText = ""

        // If the previous turn was interrupted, resume that session with the new prompt.
        let shouldResume = (sessionState == .interrupted && lastSessionId != nil)
        execPrompt(text, resume: shouldResume)
    }

    /// Interrupt the running agent (ESC).
    func interrupt() {
        guard canInterrupt else { return }
        eventTask?.cancel()
        eventTask = nil
        session?.terminate()
        session = nil
        finalizeCurrentAssistantBubble()
        messages.append(ChatMessage(role: .system, content: .system("Interrupted")))
        sessionState = .interrupted
    }

    // MARK: - Internal

    /// Start a zag exec process for a single prompt.
    private func execPrompt(_ prompt: String, resume: Bool) {
        killSession()
        sessionState = .starting
        messages.append(ChatMessage(role: .user, content: .text(prompt)))
        finalizeCurrentAssistantBubble()

        var args: [String] = ["exec"]
        args += makeBuilder().buildGlobalArgs()
        args.append("--json-stream")
        // Resume the interrupted session if applicable.
        if resume, let sid = lastSessionId {
            args += ["--context", sid]
        }
        args.append(prompt)

        do {
            let newSession = try ZagProcess.startStreamingProcess(
                bin: ZagProcess.defaultBin, args: args)
            // Close stdin immediately — we use one-shot exec, not bidirectional streaming.
            newSession.closeInput()
            session = newSession
        } catch {
            handleStreamError(error)
            return
        }

        guard let session else { return }
        let stream = session.events

        eventTask = Task { [weak self] in
            do {
                for try await event in stream {
                    guard let self, !Task.isCancelled else { return }
                    self.handle(event: event)
                }
                self?.handleTurnEnded()
            } catch {
                guard let self, !Task.isCancelled else { return }
                self.handleStreamError(error)
            }
        }
    }

    private func makeBuilder() -> ZagBuilder {
        var b = ZagBuilder()
            .provider(provider)
            .autoApprove()
        if let m = model, !m.isEmpty { b = b.model(m) }
        if let r = root { b = b.root(r) }
        return b
    }

    private func killSession() {
        eventTask?.cancel()
        eventTask = nil
        session?.terminate()
        session = nil
    }

    // MARK: - Event handling

    private func handle(event: Event) {
        switch event {

        case .`init`(let payload):
            modelInfo = payload.model
            // Capture session ID from metadata for potential --context resume.
            if case .string(let sid) = payload.metadata["session_id"] {
                lastSessionId = sid
            }
            if sessionState == .starting {
                sessionState = .running
            }

        case .userMessage:
            break

        case .assistantMessage(let payload):
            sessionState = .agentTyping

            // If this message belongs to a sub-agent, nest its tool calls
            // under the parent Agent tool bubble instead of adding top-level.
            if let parentId = payload.parentToolUseId {
                for block in payload.content {
                    if case .toolUse(let tu) = block {
                        let child = ToolDetail(
                            toolId: tu.id,
                            toolName: tu.name,
                            input: tu.input,
                            result: nil,
                            isComplete: false,
                            children: []
                        )
                        appendChildTool(child, parentId: parentId)
                    }
                }
                break
            }

            var pendingText: [String] = []

            for block in payload.content {
                switch block {
                case .text(let tb):
                    pendingText.append(tb.text)
                case .toolUse(let tu):
                    let partial = pendingText.joined()
                    if !partial.isEmpty {
                        appendOrUpdateAssistantText(partial)
                        pendingText = []
                    }
                    finalizeCurrentAssistantBubble()
                    let detail = ToolDetail(
                        toolId: tu.id,
                        toolName: tu.name,
                        input: tu.input,
                        result: nil,
                        isComplete: false,
                        children: []
                    )
                    messages.append(ChatMessage(
                        role: .assistant,
                        content: .toolUse(detail)
                    ))
                }
            }

            let text = pendingText.joined()
            if !text.isEmpty {
                appendOrUpdateAssistantText(text)
            }

        case .toolExecution(let payload):
            // If this execution belongs to a sub-agent, update the child
            // within the parent Agent tool bubble.
            if let parentId = payload.parentToolUseId {
                updateChildTool(toolId: payload.toolId, result: payload.result, parentId: parentId)
                break
            }

            for i in messages.indices.reversed() {
                if case .toolUse(var detail) = messages[i].content,
                   detail.toolId == payload.toolId {
                    detail.result = payload.result
                    detail.isComplete = true
                    messages[i].content = .toolUse(detail)
                    break
                }
            }

        case .result(let payload):
            finalizeCurrentAssistantBubble()
            if let ms = payload.durationMs, let turns = payload.numTurns {
                messages.append(ChatMessage(
                    role: .system,
                    content: .system("\(turns) turn\(turns == 1 ? "" : "s") · \(ms)ms")
                ))
            }
            sessionState = .running

        case .error(let payload):
            finalizeCurrentAssistantBubble()
            messages.append(ChatMessage(
                role: .system,
                content: .system("Error: \(payload.message)")
            ))
            sessionState = .error(payload.message)

        case .permissionRequest(let payload):
            messages.append(ChatMessage(
                role: .system,
                content: .system("Permission: \(payload.toolName) (\(payload.granted ? "granted" : "denied"))")
            ))
        }
    }

    // MARK: - Helpers

    private func appendOrUpdateAssistantText(_ text: String) {
        if let existingId = currentAssistantMessageId,
           let idx = messages.firstIndex(where: { $0.id == existingId }),
           case .text(let existing) = messages[idx].content
        {
            messages[idx].content = .text(existing + text)
        } else {
            let msg = ChatMessage(
                role: .assistant,
                content: .text(text),
                isStreaming: true
            )
            currentAssistantMessageId = msg.id
            messages.append(msg)
        }
    }

    private func finalizeCurrentAssistantBubble() {
        if let id = currentAssistantMessageId,
           let idx = messages.firstIndex(where: { $0.id == id })
        {
            messages[idx].isStreaming = false
        }
        currentAssistantMessageId = nil
    }

    /// Append a child tool detail to the parent Agent tool bubble.
    private func appendChildTool(_ child: ToolDetail, parentId: String) {
        for i in messages.indices.reversed() {
            if case .toolUse(var parent) = messages[i].content,
               parent.toolId == parentId {
                parent.children.append(child)
                messages[i].content = .toolUse(parent)
                return
            }
        }
    }

    /// Update a child tool's result within the parent Agent tool bubble.
    private func updateChildTool(toolId: String, result: ToolResult, parentId: String) {
        for i in messages.indices.reversed() {
            if case .toolUse(var parent) = messages[i].content,
               parent.toolId == parentId {
                for j in parent.children.indices.reversed() {
                    if parent.children[j].toolId == toolId {
                        parent.children[j].result = result
                        parent.children[j].isComplete = true
                        break
                    }
                }
                messages[i].content = .toolUse(parent)
                return
            }
        }
    }

    private func handleTurnEnded() {
        finalizeCurrentAssistantBubble()
        switch sessionState {
        case .error, .interrupted: break
        default: sessionState = .running
        }
    }

    private func handleStreamError(_ error: Error) {
        finalizeCurrentAssistantBubble()
        // Ignore errors from cancelled/interrupted tasks.
        if Task.isCancelled { return }
        let msg = zagErrorDescription(error)
        messages.append(ChatMessage(role: .system, content: .system("Error: \(msg)")))
        sessionState = .error(msg)
    }

    private func zagErrorDescription(_ error: Error) -> String {
        guard let ze = error as? ZagError else { return error.localizedDescription }
        let stderr = ze.stderr.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !stderr.isEmpty else { return ze.message }
        return "\(ze.message)\n\(stderr)"
    }
}
