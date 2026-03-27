import type { SessionStatus } from "../hooks/useSession";

interface Props {
  status: SessionStatus;
  sessionId: string | null;
  model: string | null;
  provider?: string;
}

export function StatusBar({ status, sessionId, model }: Props) {
  const statusColor = {
    idle: "#6b7280",
    connecting: "#f59e0b",
    streaming: "#10b981",
    ended: "#6b7280",
    error: "#ef4444",
  }[status];

  const statusLabel = {
    idle: "Ready",
    connecting: "Connecting...",
    streaming: "Streaming",
    ended: "Session ended",
    error: "Error",
  }[status];

  return (
    <div className="status-bar">
      <div className="status-left">
        <span className="status-dot" style={{ background: statusColor }} />
        <span className="status-label">{statusLabel}</span>
        {model && <span className="status-model">{model}</span>}
      </div>
      <div className="status-right">
        {sessionId && (
          <span className="status-session" title={sessionId}>
            {sessionId.slice(0, 8)}
          </span>
        )}
      </div>
    </div>
  );
}
