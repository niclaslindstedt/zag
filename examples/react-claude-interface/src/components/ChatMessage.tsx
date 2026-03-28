import { useState } from "react";

interface Props {
  role: "user" | "assistant";
  content: string;
  timestamp?: string;
}

function CopyButton({ text }: { text: string }) {
  const [copied, setCopied] = useState(false);

  const handleCopy = () => {
    navigator.clipboard.writeText(text);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <button
      onClick={handleCopy}
      className="text-xs text-zinc-500 hover:text-zinc-300 transition-colors px-2 py-1"
      title="Copy code"
    >
      {copied ? "Copied!" : "Copy"}
    </button>
  );
}

function renderContent(text: string, role: "user" | "assistant") {
  if (role === "user") return <span>{text}</span>;

  const parts = text.split(/(```[\s\S]*?```)/g);

  return (
    <>
      {parts.map((part, i) => {
        if (part.startsWith("```")) {
          const match = part.match(/^```(\w*)\n?([\s\S]*?)```$/);
          const lang = match?.[1] || "";
          const code = match?.[2] || part.slice(3, -3);
          return (
            <div key={i} className="my-3 rounded-lg border border-zinc-800 overflow-hidden bg-zinc-900">
              <div className="flex items-center justify-between px-3 py-1.5 bg-zinc-800/60 border-b border-zinc-800">
                <span className="font-mono text-[11px] text-zinc-500">{lang || "code"}</span>
                <CopyButton text={code.trim()} />
              </div>
              <pre className="p-3 overflow-x-auto font-mono text-[13px] leading-relaxed text-zinc-300">
                <code>{code.trim()}</code>
              </pre>
            </div>
          );
        }

        // Inline formatting: bold, inline code, links
        const segments = part.split(/(\*\*[^*]+\*\*|`[^`]+`|\[[^\]]+\]\([^)]+\))/g);
        return (
          <span key={i}>
            {segments.map((seg, j) => {
              if (seg.startsWith("**") && seg.endsWith("**")) {
                return (
                  <strong key={j} className="font-semibold text-zinc-100">
                    {seg.slice(2, -2)}
                  </strong>
                );
              }
              if (seg.startsWith("`") && seg.endsWith("`")) {
                return (
                  <code
                    key={j}
                    className="font-mono text-xs px-1.5 py-0.5 bg-zinc-800 text-amber-300 border border-zinc-700 rounded"
                  >
                    {seg.slice(1, -1)}
                  </code>
                );
              }
              const linkMatch = seg.match(/^\[([^\]]+)\]\(([^)]+)\)$/);
              if (linkMatch) {
                return (
                  <a
                    key={j}
                    href={linkMatch[2]}
                    className="text-blue-400 hover:text-blue-300 underline underline-offset-2"
                    target="_blank"
                    rel="noopener noreferrer"
                  >
                    {linkMatch[1]}
                  </a>
                );
              }
              return <span key={j}>{seg}</span>;
            })}
          </span>
        );
      })}
    </>
  );
}

export function ChatMessage({ role, content, timestamp }: Props) {
  const isUser = role === "user";

  return (
    <div className="py-2 animate-fade-in">
      <div className="flex items-center gap-2 mb-1">
        <span className={`font-mono font-semibold text-sm ${isUser ? "text-blue-400" : "text-amber-400"}`}>
          {isUser ? ">" : "\u25CB"}
        </span>
        <span className="font-semibold text-xs text-zinc-400">
          {isUser ? "You" : "Assistant"}
        </span>
        {timestamp && (
          <span className="text-zinc-600 text-[11px] ml-auto">
            {new Date(timestamp).toLocaleTimeString()}
          </span>
        )}
      </div>
      <div className={`pl-6 whitespace-pre-wrap break-words ${isUser ? "text-zinc-200" : "text-zinc-100 leading-7"}`}>
        {renderContent(content, role)}
      </div>
    </div>
  );
}
