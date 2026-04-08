import SwiftUI
import Zag

// MARK: - JSONValue convenience

extension JSONValue {
    fileprivate var stringValue: String? {
        if case .string(let s) = self { return s }
        return nil
    }

    fileprivate subscript(key: String) -> JSONValue? {
        if case .object(let dict) = self { return dict[key] }
        return nil
    }

    fileprivate var prettyText: String {
        switch self {
        case .null: return "null"
        case .bool(let b): return b ? "true" : "false"
        case .int(let i): return "\(i)"
        case .double(let d): return "\(d)"
        case .string(let s): return s
        case .array(let arr):
            return arr.map(\.prettyText).joined(separator: "\n")
        case .object(let dict):
            return dict.sorted(by: { $0.key < $1.key })
                .map { "\($0.key): \($0.value.prettyText)" }
                .joined(separator: "\n")
        }
    }
}

// MARK: - Main dispatcher

struct ToolDetailContentView: View {
    let detail: ToolDetail

    var body: some View {
        Group {
            switch detail.toolName {
            case "Bash":  BashDetailView(detail: detail)
            case "Read":  ReadDetailView(detail: detail)
            case "Write": WriteDetailView(detail: detail)
            case "Edit":  EditDetailView(detail: detail)
            case "Glob":  GlobDetailView(detail: detail)
            case "Grep":  GrepDetailView(detail: detail)
            case "Agent": AgentDetailView(detail: detail)
            default:      GenericDetailView(detail: detail)
            }
        }
        .padding(10)
        .frame(maxWidth: 440, alignment: .leading)
        .background(Color(NSColor.controlBackgroundColor))
        .clipShape(RoundedRectangle(cornerRadius: 10))
    }
}

// MARK: - Shared components

private struct SectionLabel: View {
    let text: String
    var body: some View {
        Text(text)
            .font(.caption2)
            .foregroundStyle(.tertiary)
            .textCase(.uppercase)
    }
}

private struct CodeBlockView: View {
    let text: String
    var maxHeight: CGFloat = 200
    var tint: Color? = nil

    var body: some View {
        ScrollView {
            Text(text)
                .font(.system(.caption, design: .monospaced))
                .textSelection(.enabled)
                .frame(maxWidth: .infinity, alignment: .leading)
        }
        .frame(maxHeight: maxHeight)
        .padding(8)
        .background((tint ?? Color(NSColor.textBackgroundColor)).opacity(tint != nil ? 0.15 : 1))
        .clipShape(RoundedRectangle(cornerRadius: 6))
    }
}

private struct ErrorTextView: View {
    let text: String
    var body: some View {
        Text(text)
            .font(.system(.caption, design: .monospaced))
            .foregroundStyle(.red)
            .textSelection(.enabled)
    }
}

// MARK: - Bash

private struct BashDetailView: View {
    let detail: ToolDetail

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            if let cmd = detail.input?["command"]?.stringValue {
                SectionLabel(text: "Command")
                CodeBlockView(text: cmd, maxHeight: 80)
            }
            if let output = detail.result?.output, !output.isEmpty {
                SectionLabel(text: "Output")
                CodeBlockView(text: output)
            }
            if let error = detail.result?.error, !error.isEmpty {
                SectionLabel(text: "Error")
                ErrorTextView(text: error)
            }
        }
    }
}

// MARK: - Read

private struct ReadDetailView: View {
    let detail: ToolDetail

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            if let path = detail.input?["file_path"]?.stringValue {
                SectionLabel(text: "File")
                Text(path)
                    .font(.system(.caption, design: .monospaced))
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)
            }
            if let output = detail.result?.output, !output.isEmpty {
                SectionLabel(text: "Content")
                CodeBlockView(text: output)
            }
            if let error = detail.result?.error, !error.isEmpty {
                ErrorTextView(text: error)
            }
        }
    }
}

// MARK: - Write

private struct WriteDetailView: View {
    let detail: ToolDetail

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            if let path = detail.input?["file_path"]?.stringValue {
                SectionLabel(text: "File")
                Text(path)
                    .font(.system(.caption, design: .monospaced))
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)
            }
            if let content = detail.input?["content"]?.stringValue, !content.isEmpty {
                SectionLabel(text: "Written")
                CodeBlockView(text: content, tint: .green)
            }
            if let error = detail.result?.error, !error.isEmpty {
                ErrorTextView(text: error)
            }
        }
    }
}

// MARK: - Edit

private struct EditDetailView: View {
    let detail: ToolDetail

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            if let path = detail.input?["file_path"]?.stringValue {
                SectionLabel(text: "File")
                Text(path)
                    .font(.system(.caption, design: .monospaced))
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)
            }
            if let old = detail.input?["old_string"]?.stringValue, !old.isEmpty {
                SectionLabel(text: "Removed")
                CodeBlockView(text: old, maxHeight: 120, tint: .red)
            }
            if let new = detail.input?["new_string"]?.stringValue, !new.isEmpty {
                SectionLabel(text: "Added")
                CodeBlockView(text: new, maxHeight: 120, tint: .green)
            }
            if let error = detail.result?.error, !error.isEmpty {
                ErrorTextView(text: error)
            }
        }
    }
}

// MARK: - Glob

private struct GlobDetailView: View {
    let detail: ToolDetail

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            if let pattern = detail.input?["pattern"]?.stringValue {
                SectionLabel(text: "Pattern")
                Text(pattern)
                    .font(.system(.caption, design: .monospaced))
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)
            }
            if let output = detail.result?.output, !output.isEmpty {
                SectionLabel(text: "Matches")
                let files = output.components(separatedBy: "\n").filter { !$0.isEmpty }
                ScrollView {
                    VStack(alignment: .leading, spacing: 2) {
                        ForEach(files, id: \.self) { file in
                            Text(file)
                                .font(.system(.caption, design: .monospaced))
                                .textSelection(.enabled)
                        }
                    }
                    .frame(maxWidth: .infinity, alignment: .leading)
                }
                .frame(maxHeight: 200)
                .padding(8)
                .background(Color(NSColor.textBackgroundColor))
                .clipShape(RoundedRectangle(cornerRadius: 6))
            }
            if let error = detail.result?.error, !error.isEmpty {
                ErrorTextView(text: error)
            }
        }
    }
}

// MARK: - Grep

private struct GrepDetailView: View {
    let detail: ToolDetail

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            if let pattern = detail.input?["pattern"]?.stringValue {
                SectionLabel(text: "Pattern")
                Text(pattern)
                    .font(.system(.caption, design: .monospaced))
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)
            }
            if let path = detail.input?["path"]?.stringValue {
                SectionLabel(text: "Path")
                Text(path)
                    .font(.system(.caption, design: .monospaced))
                    .foregroundStyle(.secondary)
                    .textSelection(.enabled)
            }
            if let output = detail.result?.output, !output.isEmpty {
                SectionLabel(text: "Results")
                CodeBlockView(text: output)
            }
            if let error = detail.result?.error, !error.isEmpty {
                ErrorTextView(text: error)
            }
        }
    }
}

// MARK: - Agent (sub-agent)

private struct AgentDetailView: View {
    let detail: ToolDetail

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            if let description = detail.input?["description"]?.stringValue {
                SectionLabel(text: "Task")
                Text(description)
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
            if !detail.children.isEmpty {
                SectionLabel(text: "Activity")
                VStack(alignment: .leading, spacing: 4) {
                    ForEach(Array(detail.children.enumerated()), id: \.offset) { _, child in
                        ChildToolRow(detail: child)
                    }
                }
            }
            if let output = detail.result?.output, !output.isEmpty {
                SectionLabel(text: "Result")
                CodeBlockView(text: output, maxHeight: 300)
            }
            if let error = detail.result?.error, !error.isEmpty {
                SectionLabel(text: "Error")
                ErrorTextView(text: error)
            }
        }
    }
}

/// A compact, expandable row for a child tool call within an Agent bubble.
private struct ChildToolRow: View {
    let detail: ToolDetail
    @State private var isExpanded = false

    var body: some View {
        VStack(alignment: .leading, spacing: 2) {
            Button {
                guard detail.isComplete else { return }
                withAnimation(.spring(duration: 0.2)) { isExpanded.toggle() }
            } label: {
                HStack(spacing: 4) {
                    statusIcon
                    Text(detail.toolName)
                        .font(.system(.caption2, design: .monospaced))
                        .foregroundStyle(.secondary)
                    if let summary = childSummary {
                        Text(summary)
                            .font(.caption2)
                            .foregroundStyle(.tertiary)
                            .lineLimit(1)
                    }
                    Spacer()
                    if detail.isComplete {
                        Image(systemName: isExpanded ? "chevron.up" : "chevron.down")
                            .font(.system(size: 8))
                            .foregroundStyle(.tertiary)
                    }
                }
            }
            .buttonStyle(.plain)

            if isExpanded, detail.isComplete {
                ToolDetailContentView(detail: detail)
                    .transition(.opacity)
            }
        }
    }

    @ViewBuilder
    private var statusIcon: some View {
        if detail.isComplete {
            Image(systemName: detail.result?.success == true ? "checkmark.circle.fill" : "xmark.circle.fill")
                .foregroundStyle(detail.result?.success == true ? .green : .red)
                .font(.system(size: 8))
        } else {
            ProgressView()
                .controlSize(.mini)
        }
    }

    /// One-line summary for the child (e.g. command, file path, pattern).
    private var childSummary: String? {
        switch detail.toolName {
        case "Bash":
            return detail.input?["command"]?.stringValue
        case "Read", "Write", "Edit":
            return detail.input?["file_path"]?.stringValue
        case "Glob":
            return detail.input?["pattern"]?.stringValue
        case "Grep":
            return detail.input?["pattern"]?.stringValue
        default:
            return detail.input?["description"]?.stringValue
        }
    }
}

// MARK: - Generic fallback

private struct GenericDetailView: View {
    let detail: ToolDetail

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            if let input = detail.input {
                SectionLabel(text: "Input")
                CodeBlockView(text: input.prettyText, maxHeight: 150)
            }
            if let output = detail.result?.output, !output.isEmpty {
                SectionLabel(text: "Output")
                CodeBlockView(text: output)
            }
            if let error = detail.result?.error, !error.isEmpty {
                SectionLabel(text: "Error")
                ErrorTextView(text: error)
            }
        }
    }
}
