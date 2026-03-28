import { useState } from "react";

interface Props {
  content: string;
  maxLines?: number;
}

export function TerminalOutput({ content, maxLines = 20 }: Props) {
  const [showAll, setShowAll] = useState(false);
  const lines = content.split("\n");
  const truncated = !showAll && lines.length > maxLines;
  const displayLines = truncated ? lines.slice(0, maxLines) : lines;

  return (
    <div className="rounded-md bg-zinc-950 border border-zinc-800 overflow-hidden">
      <pre className="p-3 font-mono text-xs text-green-300/90 leading-relaxed whitespace-pre-wrap break-words max-h-64 overflow-y-auto scrollbar-thin">
        {displayLines.join("\n")}
        {truncated && (
          <span className="text-zinc-600">{"\n... truncated"}</span>
        )}
      </pre>
      {truncated && (
        <button
          onClick={() => setShowAll(true)}
          className="w-full px-3 py-1.5 text-xs text-zinc-500 hover:text-zinc-300 bg-zinc-900/50 border-t border-zinc-800 transition-colors"
        >
          Show all {lines.length} lines
        </button>
      )}
    </div>
  );
}
