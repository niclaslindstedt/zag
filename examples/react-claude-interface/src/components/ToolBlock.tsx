import { useState } from "react";
import type { ToolKind } from "../types";

interface Props {
  toolName: string;
  toolKind?: ToolKind;
  toolId?: string;
  input?: Record<string, unknown>;
  result?: {
    success?: boolean;
    output?: string;
    error?: string;
  };
  timestamp?: string;
}

function toolIcon(kind?: ToolKind): string {
  switch (kind) {
    case "shell":
      return "$";
    case "file_read":
      return "\u{1F4C4}";
    case "file_write":
      return "\u{1F4DD}";
    case "file_edit":
      return "\u270F";
    case "search":
      return "\u{1F50D}";
    case "sub_agent":
      return "\u{1F916}";
    case "web":
      return "\u{1F310}";
    default:
      return "\u26A1";
  }
}

function summarizeInput(
  toolName: string,
  input?: Record<string, unknown>,
): string {
  if (!input) return "";
  // Common patterns
  if (input.command) return String(input.command);
  if (input.file_path) return String(input.file_path);
  if (input.path) return String(input.path);
  if (input.pattern) return String(input.pattern);
  if (input.query) return String(input.query);
  if (input.description) return String(input.description);
  return "";
}

export function ToolBlock({
  toolName,
  toolKind,
  input,
  result,
  timestamp,
}: Props) {
  const [expanded, setExpanded] = useState(false);

  const icon = toolIcon(toolKind);
  const summary = summarizeInput(toolName, input);
  const hasResult = result !== undefined;
  const isSuccess = result?.success !== false;

  return (
    <div
      className={`tool-block ${hasResult ? (isSuccess ? "tool-block--success" : "tool-block--error") : "tool-block--pending"}`}
    >
      <div className="tool-header" onClick={() => setExpanded(!expanded)}>
        <span className="tool-chevron">{expanded ? "\u25BC" : "\u25B6"}</span>
        <span className="tool-icon">{icon}</span>
        <span className="tool-name">{toolName}</span>
        {summary && <span className="tool-summary">{summary}</span>}
        {hasResult && (
          <span className={`tool-status ${isSuccess ? "success" : "error"}`}>
            {isSuccess ? "\u2713" : "\u2717"}
          </span>
        )}
        {timestamp && (
          <span className="tool-time">
            {new Date(timestamp).toLocaleTimeString()}
          </span>
        )}
      </div>

      {expanded && (
        <div className="tool-details">
          {input && (
            <div className="tool-section">
              <div className="tool-section-label">Input</div>
              <pre className="tool-pre">
                {typeof input === "object"
                  ? JSON.stringify(input, null, 2)
                  : String(input)}
              </pre>
            </div>
          )}
          {result?.output && (
            <div className="tool-section">
              <div className="tool-section-label">
                Output{" "}
                {result.output.length > 500 &&
                  `(${result.output.length} chars)`}
              </div>
              <pre className="tool-pre">
                {result.output.length > 2000
                  ? result.output.slice(0, 2000) + "\n... (truncated)"
                  : result.output}
              </pre>
            </div>
          )}
          {result?.error && (
            <div className="tool-section">
              <div className="tool-section-label tool-section-label--error">
                Error
              </div>
              <pre className="tool-pre tool-pre--error">{result.error}</pre>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
