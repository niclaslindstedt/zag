import { useState } from "react";
import type { ToolKind } from "../types";
import { TerminalOutput } from "./TerminalOutput";
import { DiffView } from "./DiffView";
import { SubAgentBlock } from "./SubAgentBlock";

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
    case "shell":      return "$";
    case "file_read":  return "\u2592";
    case "file_write": return "\u2591";
    case "file_edit":  return "\u00B1";
    case "search":     return "/";
    case "sub_agent":  return "\u2B21";
    case "web":        return "\u2197";
    case "notebook":   return "\u25A3";
    default:           return "\u26A1";
  }
}

function iconColor(kind?: ToolKind): string {
  switch (kind) {
    case "shell":      return "text-green-400";
    case "file_read":  return "text-blue-400";
    case "file_write": return "text-cyan-400";
    case "file_edit":  return "text-yellow-400";
    case "search":     return "text-yellow-300";
    case "sub_agent":  return "text-purple-400";
    case "web":        return "text-sky-400";
    default:           return "text-zinc-400";
  }
}

function borderColor(result?: Props["result"]): string {
  if (!result) return "border-l-yellow-500/60";
  return result.success !== false ? "border-l-emerald-500/60" : "border-l-red-500/60";
}

/** Render the always-visible inline preview based on tool kind */
function InlinePreview({ toolName, toolKind, input }: { toolName: string; toolKind?: ToolKind; input?: Record<string, unknown> }) {
  if (!input) return null;

  // Shell: terminal prompt
  if (toolKind === "shell" && input.command) {
    return (
      <div className="mt-1.5 rounded-md bg-zinc-950 border border-zinc-800 px-3 py-2">
        <code className="font-mono text-xs text-green-400/90">
          <span className="text-green-600 select-none">$ </span>
          {String(input.command)}
        </code>
      </div>
    );
  }

  // File read: path breadcrumb
  if (toolKind === "file_read" && input.file_path) {
    return (
      <div className="mt-1.5 flex items-center gap-1.5 text-blue-400/80 font-mono text-xs">
        <span className="text-zinc-600">{"\u2192"}</span>
        {String(input.file_path)}
      </div>
    );
  }

  // File edit: path + mini diff
  if (toolKind === "file_edit" && input.file_path) {
    const oldStr = input.old_string != null ? String(input.old_string) : null;
    const newStr = input.new_string != null ? String(input.new_string) : null;
    return (
      <div className="mt-1.5 space-y-1.5">
        <div className="flex items-center gap-2 font-mono text-xs">
          <span className="text-yellow-400/70">{String(input.file_path)}</span>
          <span className="px-1.5 py-0.5 rounded bg-yellow-900/30 text-yellow-400/80 text-[10px] font-medium">
            edited
          </span>
        </div>
        {oldStr && newStr && (
          <DiffView oldText={oldStr} newText={newStr} />
        )}
      </div>
    );
  }

  // File write: path + badge
  if (toolKind === "file_write" && (input.file_path || input.path)) {
    const path = String(input.file_path || input.path);
    const content = input.content ? String(input.content) : null;
    const preview = content ? content.split("\n").slice(0, 5).join("\n") : null;
    return (
      <div className="mt-1.5 space-y-1.5">
        <div className="flex items-center gap-2 font-mono text-xs">
          <span className="text-cyan-400/70">{path}</span>
          <span className="px-1.5 py-0.5 rounded bg-cyan-900/30 text-cyan-400/80 text-[10px] font-medium">
            created
          </span>
        </div>
        {preview && (
          <pre className="font-mono text-[11px] text-zinc-500 leading-relaxed pl-2 border-l-2 border-zinc-800">
            {preview}
            {content && content.split("\n").length > 5 && (
              <span className="text-zinc-600">{"\n..."}</span>
            )}
          </pre>
        )}
      </div>
    );
  }

  // Search/grep: search bar
  if (toolKind === "search") {
    const pattern = input.pattern ? String(input.pattern) : null;
    const path = input.path ? String(input.path) : null;
    return (
      <div className="mt-1.5 flex items-center gap-2 rounded-md bg-zinc-800/40 border border-zinc-700/40 px-3 py-1.5 font-mono text-xs">
        <span className="text-zinc-500 select-none">/</span>
        {pattern && <span className="text-yellow-300">{pattern}</span>}
        {path && <span className="text-zinc-500">{" in "}<span className="text-zinc-400">{path}</span></span>}
      </div>
    );
  }

  return null;
}

/** Render the expanded output based on tool kind */
function ToolOutput({ toolKind, result }: { toolKind?: ToolKind; result?: Props["result"] }) {
  if (!result) return null;

  if (result.error) {
    return (
      <div className="px-3 py-2">
        <div className="text-[10px] font-medium text-red-400/60 uppercase tracking-wider mb-1">Error</div>
        <pre className="font-mono text-xs text-red-300 leading-relaxed whitespace-pre-wrap break-words">
          {result.error}
        </pre>
      </div>
    );
  }

  if (!result.output) return null;

  // Shell output: terminal style
  if (toolKind === "shell") {
    return (
      <div className="px-3 py-2">
        <TerminalOutput content={result.output} />
      </div>
    );
  }

  // File read output: line numbers
  if (toolKind === "file_read") {
    const lines = result.output.split("\n");
    const display = lines.length > 50 ? lines.slice(0, 50) : lines;
    return (
      <div className="px-3 py-2">
        <div className="rounded-md bg-zinc-950 border border-zinc-800 overflow-hidden">
          <pre className="p-3 font-mono text-xs leading-relaxed max-h-64 overflow-y-auto scrollbar-thin">
            {display.map((line, i) => (
              <div key={i} className="flex">
                <span className="text-zinc-700 select-none w-8 text-right mr-3 flex-shrink-0">
                  {i + 1}
                </span>
                <span className="text-zinc-300">{line}</span>
              </div>
            ))}
            {lines.length > 50 && (
              <div className="text-zinc-600 mt-1">... {lines.length - 50} more lines</div>
            )}
          </pre>
        </div>
      </div>
    );
  }

  // Default: monospace output
  return (
    <div className="px-3 py-2">
      <pre className="font-mono text-xs text-zinc-400 leading-relaxed whitespace-pre-wrap break-words max-h-64 overflow-y-auto scrollbar-thin">
        {result.output.length > 2000
          ? result.output.slice(0, 2000) + "\n... (truncated)"
          : result.output}
      </pre>
    </div>
  );
}

export function ToolBlock({
  toolName,
  toolKind,
  input,
  result,
  timestamp,
}: Props) {
  const [expanded, setExpanded] = useState(false);

  // Sub-agents get their own special component
  if (toolKind === "sub_agent") {
    return (
      <SubAgentBlock
        description={input?.description ? String(input.description) : undefined}
        prompt={input?.prompt ? String(input.prompt) : undefined}
        output={result?.output}
        error={result?.error}
        success={result?.success}
        timestamp={timestamp}
      />
    );
  }

  const icon = toolIcon(toolKind);
  const color = iconColor(toolKind);
  const border = borderColor(result);
  const hasResult = result !== undefined;
  const isSuccess = result?.success !== false;
  const isPending = !hasResult;

  // Build summary text
  let summary = "";
  if (input) {
    if (input.command) summary = String(input.command);
    else if (input.file_path) summary = String(input.file_path);
    else if (input.path) summary = String(input.path);
    else if (input.pattern) summary = String(input.pattern);
    else if (input.query) summary = String(input.query);
  }

  // Whether the inline preview already shows enough context
  const hasInlinePreview = toolKind === "shell" || toolKind === "file_edit" || toolKind === "file_write" || toolKind === "search";
  const showExpandedInput = expanded && input && !hasInlinePreview;

  return (
    <div className={`my-1 rounded-lg border border-zinc-800/60 bg-zinc-900/40 overflow-hidden border-l-2 ${border} animate-fade-in ${isPending ? "animate-shimmer" : ""}`}>
      {/* Header */}
      <div
        className="flex items-center gap-2 px-3 py-2 cursor-pointer hover:bg-zinc-800/30 transition-colors"
        onClick={() => setExpanded(!expanded)}
      >
        <span className={`text-zinc-600 text-[10px] w-3 transition-transform flex-shrink-0 ${expanded ? "rotate-90" : ""}`}>
          {"\u25B6"}
        </span>
        <span className={`font-mono text-sm flex-shrink-0 ${color}`}>{icon}</span>
        <span className="font-mono text-xs font-medium text-purple-300">{toolName}</span>
        {summary && !hasInlinePreview && (
          <span className="text-zinc-500 font-mono text-xs truncate flex-1 min-w-0">
            {summary}
          </span>
        )}
        <span className="flex-1" />
        {hasResult && (
          <span className={`text-sm font-semibold flex-shrink-0 ${isSuccess ? "text-emerald-400" : "text-red-400"}`}>
            {isSuccess ? "\u2713" : "\u2717"}
          </span>
        )}
        {isPending && (
          <span className="w-1.5 h-1.5 rounded-full bg-yellow-400 animate-pulse flex-shrink-0" />
        )}
        {timestamp && (
          <span className="text-zinc-600 text-[11px] flex-shrink-0">
            {new Date(timestamp).toLocaleTimeString()}
          </span>
        )}
      </div>

      {/* Inline preview (always visible for supported tool kinds) */}
      {hasInlinePreview && (
        <div className="px-3 pb-2 -mt-0.5">
          <div className="pl-5">
            <InlinePreview toolName={toolName} toolKind={toolKind} input={input} />
          </div>
        </div>
      )}

      {/* Expanded details */}
      {expanded && (
        <div className="border-t border-zinc-800/40">
          {showExpandedInput && (
            <div className="px-3 py-2 border-b border-zinc-800/30">
              <div className="text-[10px] font-medium text-zinc-500 uppercase tracking-wider mb-1">Input</div>
              <pre className="font-mono text-xs text-zinc-400 leading-relaxed whitespace-pre-wrap break-words">
                {JSON.stringify(input, null, 2)}
              </pre>
            </div>
          )}
          <ToolOutput toolKind={toolKind} result={result} />
        </div>
      )}
    </div>
  );
}
