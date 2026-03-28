import { useState } from "react";

interface Props {
  description?: string;
  prompt?: string;
  output?: string;
  error?: string;
  success?: boolean;
  timestamp?: string;
}

export function SubAgentBlock({
  description,
  prompt,
  output,
  error,
  success,
  timestamp,
}: Props) {
  const [expanded, setExpanded] = useState(false);
  const hasResult = success !== undefined || output || error;
  const isSuccess = success !== false;
  const summary = description || prompt?.slice(0, 80) || "Sub-agent task";

  return (
    <div className="my-2 rounded-lg border border-purple-800/40 bg-purple-950/10 overflow-hidden animate-fade-in">
      {/* Header */}
      <div
        className="flex items-center gap-2 px-3 py-2.5 cursor-pointer hover:bg-purple-900/10 transition-colors"
        onClick={() => setExpanded(!expanded)}
      >
        <span className={`text-zinc-600 text-[10px] w-3 transition-transform ${expanded ? "rotate-90" : ""}`}>
          {"\u25B6"}
        </span>
        <span className="text-purple-400 text-sm">{"\u2B21"}</span>
        <span className="font-mono text-xs font-medium text-purple-300">Agent</span>
        <span className="text-zinc-400 text-xs truncate flex-1 min-w-0">
          {summary}
        </span>
        {hasResult && (
          <span className={`text-sm font-semibold ${isSuccess ? "text-emerald-400" : "text-red-400"}`}>
            {isSuccess ? "\u2713" : "\u2717"}
          </span>
        )}
        {!hasResult && (
          <span className="flex items-center gap-1">
            <span className="w-1.5 h-1.5 rounded-full bg-purple-400 animate-pulse" />
          </span>
        )}
        {timestamp && (
          <span className="text-zinc-600 text-[11px] flex-shrink-0">
            {new Date(timestamp).toLocaleTimeString()}
          </span>
        )}
      </div>

      {/* Expanded content */}
      {expanded && (
        <div className="border-t border-purple-800/30">
          {/* Prompt / description */}
          {prompt && (
            <div className="px-4 py-2 border-b border-purple-800/20">
              <div className="text-[11px] font-medium text-purple-400/60 uppercase tracking-wider mb-1">Prompt</div>
              <div className="text-xs text-zinc-400 leading-relaxed whitespace-pre-wrap">
                {prompt}
              </div>
            </div>
          )}

          {/* Output */}
          {output && (
            <div className="ml-4 border-l-2 border-purple-800/30 pl-3 py-3 pr-4">
              <div className="text-[11px] font-medium text-purple-400/60 uppercase tracking-wider mb-2">Output</div>
              <pre className="font-mono text-xs text-zinc-300 leading-relaxed whitespace-pre-wrap break-words max-h-64 overflow-y-auto scrollbar-thin">
                {output.length > 3000 ? output.slice(0, 3000) + "\n... (truncated)" : output}
              </pre>
            </div>
          )}

          {/* Error */}
          {error && (
            <div className="px-4 py-2">
              <div className="text-[11px] font-medium text-red-400/60 uppercase tracking-wider mb-1">Error</div>
              <pre className="font-mono text-xs text-red-300 leading-relaxed whitespace-pre-wrap">
                {error}
              </pre>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
