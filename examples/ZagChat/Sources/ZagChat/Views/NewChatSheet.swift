import SwiftUI
import AppKit

struct NewChatSheet: View {
    /// Called with (prompt, provider, model?, root?) when user taps "Start Agent".
    let onStart: (String, String, String?, String?) -> Void

    @State private var prompt: String = ""
    @State private var provider: String = "claude"
    @State private var model: String = ""   // "" = provider default
    @State private var rootPath: String = FileManager.default.currentDirectoryPath
    @Environment(\.dismiss) private var dismiss

    private static let providers = ["claude", "codex", "gemini", "copilot", "ollama"]

    private static let modelsByProvider: [String: [String]] = [
        "claude": [
            "haiku", "haiku-4.5",
            "sonnet", "sonnet-4.6",
            "opus", "opus-4.6",
        ],
        "codex": [
            "gpt-5.4-mini", "gpt-5.4",
            "gpt-5.3-codex", "gpt-5.2",
            "o4-mini",
        ],
        "gemini": [
            "gemini-2.5-flash-lite", "gemini-2.5-flash",
            "gemini-2.5-pro",
            "gemini-3-flash-preview", "gemini-3-pro-preview",
            "gemini-3.1-pro-preview",
        ],
        "copilot": [
            "gpt-5.4-mini", "gpt-5.4", "gpt-5.3-codex",
            "claude-haiku-4.5", "claude-sonnet-4.6", "claude-opus-4.6",
            "gemini-3.1-pro-preview",
        ],
        "ollama": [
            "qwen3.5", "llama3", "llama3.2", "mistral", "codellama", "phi3",
        ],
    ]

    private var availableModels: [String] {
        Self.modelsByProvider[provider] ?? []
    }

    private var trimmedPrompt: String {
        prompt.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    /// Last path component for display, or full path if it's short.
    private var rootDisplayName: String {
        let url = URL(fileURLWithPath: rootPath)
        return url.lastPathComponent
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            Text("New Chat")
                .font(.title2)
                .fontWeight(.semibold)

            Divider()

            Grid(alignment: .leading, horizontalSpacing: 12, verticalSpacing: 10) {
                GridRow {
                    Text("Provider")
                        .foregroundStyle(.secondary)
                    Picker("", selection: $provider) {
                        ForEach(Self.providers, id: \.self) { p in
                            Text(p).tag(p)
                        }
                    }
                    .pickerStyle(.menu)
                    .labelsHidden()
                    .frame(maxWidth: 200, alignment: .leading)
                    .onChange(of: provider) { _ in
                        model = ""
                    }
                }
                GridRow {
                    Text("Model")
                        .foregroundStyle(.secondary)
                    Picker("", selection: $model) {
                        Text("Default").tag("")
                        Divider()
                        ForEach(availableModels, id: \.self) { m in
                            Text(m).tag(m)
                        }
                    }
                    .pickerStyle(.menu)
                    .labelsHidden()
                    .frame(maxWidth: 200, alignment: .leading)
                }
                GridRow {
                    Text("Folder")
                        .foregroundStyle(.secondary)
                    HStack(spacing: 6) {
                        Image(systemName: "folder.fill")
                            .foregroundStyle(.secondary)
                            .font(.caption)
                        Text(rootDisplayName)
                            .lineLimit(1)
                            .truncationMode(.middle)
                            .help(rootPath)
                        Spacer()
                        Button("Choose…") { pickFolder() }
                            .controlSize(.small)
                    }
                    .frame(maxWidth: 260, alignment: .leading)
                }
            }

            Divider()

            Text("Initial prompt")
                .foregroundStyle(.secondary)
                .font(.subheadline)

            TextEditor(text: $prompt)
                .font(.body)
                .frame(minHeight: 100, maxHeight: 200)
                .padding(8)
                .overlay(
                    RoundedRectangle(cornerRadius: 8)
                        .stroke(Color(NSColor.separatorColor))
                )

            HStack {
                Button("Cancel") { dismiss() }
                    .keyboardShortcut(.cancelAction)

                Spacer()

                Button("Start Agent") {
                    let m = model.isEmpty ? nil : model
                    onStart(trimmedPrompt, provider, m, rootPath)
                }
                .keyboardShortcut(.defaultAction)
                .disabled(trimmedPrompt.isEmpty)
                .buttonStyle(.borderedProminent)
            }
        }
        .padding(24)
        .frame(minWidth: 440, minHeight: 360)
    }

    private func pickFolder() {
        let panel = NSOpenPanel()
        panel.canChooseFiles = false
        panel.canChooseDirectories = true
        panel.canCreateDirectories = true
        panel.allowsMultipleSelection = false
        panel.directoryURL = URL(fileURLWithPath: rootPath)
        panel.prompt = "Select"
        if panel.runModal() == .OK, let url = panel.url {
            rootPath = url.path
        }
    }
}
