import { useState } from "react";

interface Props {
  content: string;
  timestamp?: string;
}

export function ThinkingBlock({ content, timestamp }: Props) {
  const [expanded, setExpanded] = useState(false);

  return (
    <div className="my-1 rounded-lg border border-zinc-800/40 bg-zinc-900/30 opacity-60 hover:opacity-90 transition-opacity">
      <div
        className="flex items-center gap-2 px-3 py-2 cursor-pointer hover:bg-zinc-800/30 transition-colors"
        onClick={() => setExpanded(!expanded)}
      >
        <span className={`text-zinc-600 text-[10px] w-3 transition-transform ${expanded ? "rotate-90" : ""}`}>
          {"\u25B6"}
        </span>
        <span className="text-zinc-500 text-xs italic">Thinking...</span>
        {timestamp && (
          <span className="text-zinc-600 text-[11px] ml-auto">
            {new Date(timestamp).toLocaleTimeString()}
          </span>
        )}
      </div>
      {expanded && (
        <div className="border-t border-zinc-800/30 px-3 py-3">
          <pre className="font-mono text-xs text-zinc-500 italic leading-relaxed whitespace-pre-wrap break-words max-h-48 overflow-y-auto scrollbar-thin">
            {content}
          </pre>
        </div>
      )}
    </div>
  );
}
