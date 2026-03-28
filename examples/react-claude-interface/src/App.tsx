import { useSession } from "./hooks/useSession";
import { StatusBar } from "./components/StatusBar";
import { MessageList } from "./components/MessageList";
import { PromptInput } from "./components/PromptInput";

export default function App() {
  const { events, status, sessionId, model, error, startSession, sendMessage } =
    useSession();

  const isStreaming = status === "streaming" || status === "connecting";

  const handleSubmit = (prompt: string) => {
    if (sessionId) {
      sendMessage(prompt);
    } else {
      startSession(prompt);
    }
  };

  return (
    <div className="flex flex-col h-screen max-w-4xl mx-auto">
      <StatusBar status={status} sessionId={sessionId} model={model} />

      <div className="flex-1 overflow-y-auto px-6 py-4 scrollbar-thin">
        {events.length === 0 && status === "idle" ? (
          <div className="flex flex-col items-center justify-center h-full gap-4 animate-fade-in">
            <div className="font-mono text-6xl font-semibold bg-gradient-to-r from-amber-400 to-orange-500 bg-clip-text text-transparent opacity-80">
              zag
            </div>
            <p className="text-zinc-500 text-sm">
              Send a prompt to start a session with Claude.
            </p>
          </div>
        ) : (
          <MessageList events={events} isStreaming={isStreaming} />
        )}
      </div>

      {error && status === "error" && (
        <div className="px-6 py-3 bg-red-950/50 text-red-400 text-sm border-t border-red-900/50">
          {error}
        </div>
      )}

      <PromptInput
        onSubmit={handleSubmit}
        disabled={isStreaming}
        placeholder={
          isStreaming
            ? "Waiting for response..."
            : sessionId
              ? "Send a follow-up message..."
              : "Send a message..."
        }
      />
    </div>
  );
}
