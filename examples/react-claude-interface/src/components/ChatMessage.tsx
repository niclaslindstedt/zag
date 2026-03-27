interface Props {
  role: "user" | "assistant";
  content: string;
  timestamp?: string;
}

/** Basic markdown-ish rendering for assistant messages */
function renderContent(text: string, role: "user" | "assistant") {
  if (role === "user") return <span>{text}</span>;

  // Split into code blocks and text
  const parts = text.split(/(```[\s\S]*?```)/g);

  return (
    <>
      {parts.map((part, i) => {
        if (part.startsWith("```")) {
          const match = part.match(/^```(\w*)\n?([\s\S]*?)```$/);
          const lang = match?.[1] || "";
          const code = match?.[2] || part.slice(3, -3);
          return (
            <div key={i} className="code-block">
              {lang && <div className="code-lang">{lang}</div>}
              <pre>
                <code>{code.trim()}</code>
              </pre>
            </div>
          );
        }

        // Inline formatting
        const formatted = part
          .split(/(`[^`]+`)/g)
          .map((seg, j) =>
            seg.startsWith("`") && seg.endsWith("`") ? (
              <code key={j} className="inline-code">
                {seg.slice(1, -1)}
              </code>
            ) : (
              <span key={j}>{seg}</span>
            ),
          );

        return <span key={i}>{formatted}</span>;
      })}
    </>
  );
}

export function ChatMessage({ role, content, timestamp }: Props) {
  return (
    <div className={`chat-message chat-message--${role}`}>
      <div className="chat-message-header">
        <span className="chat-message-icon">
          {role === "user" ? ">" : "\u23FA"}
        </span>
        <span className="chat-message-role">
          {role === "user" ? "You" : "Assistant"}
        </span>
        {timestamp && (
          <span className="chat-message-time">
            {new Date(timestamp).toLocaleTimeString()}
          </span>
        )}
      </div>
      <div className="chat-message-content">{renderContent(content, role)}</div>
    </div>
  );
}
