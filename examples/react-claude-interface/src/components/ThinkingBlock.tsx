import { useState } from "react";

interface Props {
  content: string;
  timestamp?: string;
}

export function ThinkingBlock({ content, timestamp }: Props) {
  const [expanded, setExpanded] = useState(false);

  return (
    <div className="thinking-block">
      <div className="thinking-header" onClick={() => setExpanded(!expanded)}>
        <span className="tool-chevron">{expanded ? "\u25BC" : "\u25B6"}</span>
        <span className="thinking-icon">\u2026</span>
        <span className="thinking-label">Thinking</span>
        {timestamp && (
          <span className="thinking-time">
            {new Date(timestamp).toLocaleTimeString()}
          </span>
        )}
      </div>
      {expanded && (
        <div className="thinking-content">
          <pre>{content}</pre>
        </div>
      )}
    </div>
  );
}
