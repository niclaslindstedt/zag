import type { SessionStatus } from "../hooks/useSession";

interface Props {
  status: SessionStatus;
  sessionId: string | null;
  model: string | null;
}

export function StatusBar({ status, sessionId, model }: Props) {
  const dotColor = {
    idle: "bg-zinc-500",
    connecting: "bg-yellow-400 animate-pulse",
    streaming: "bg-emerald-400 animate-pulse",
    ready: "bg-emerald-400",
    error: "bg-red-400",
  }[status];

  const statusLabel = {
    idle: "Ready",
    connecting: "Connecting",
    streaming: "Streaming",
    ready: "Ready",
    error: "Error",
  }[status];

  return (
    <div className="flex items-center justify-between px-6 py-2.5 border-b border-zinc-800/80 bg-zinc-900/60 backdrop-blur-sm">
      <div className="flex items-center gap-3">
        <div className="flex items-center gap-2 px-2.5 py-1 rounded-full bg-zinc-800/60 border border-zinc-700/50">
          <span className={`w-2 h-2 rounded-full ${dotColor}`} />
          <span className="text-xs font-medium text-zinc-400">{statusLabel}</span>
        </div>
        {model && (
          <span className="px-2.5 py-1 rounded-full bg-amber-950/40 text-amber-400 border border-amber-800/30 font-mono text-xs font-medium">
            {model}
          </span>
        )}
      </div>
      {sessionId && (
        <span className="font-mono text-xs text-zinc-600" title={sessionId}>
          {sessionId.slice(0, 8)}
        </span>
      )}
    </div>
  );
}
