import { useSession } from "./hooks/useSession";
import { StatusBar } from "./components/StatusBar";
import { MessageList } from "./components/MessageList";
import { PromptInput } from "./components/PromptInput";

export default function App() {
  const { events, status, sessionId, model, error, startSession } =
    useSession();

  const isStreaming = status === "streaming" || status === "connecting";

  return (
    <div className="app">
      <StatusBar
        status={status}
        sessionId={sessionId}
        model={model}
      />

      <div className="main">
        {events.length === 0 && status === "idle" ? (
          <div className="empty-state">
            <div className="empty-logo">zag</div>
            <p className="empty-text">
              Send a prompt to start a session with Claude.
            </p>
          </div>
        ) : (
          <MessageList events={events} />
        )}
      </div>

      {error && status === "error" && (
        <div className="error-banner">{error}</div>
      )}

      <PromptInput
        onSubmit={(prompt) => startSession(prompt)}
        disabled={isStreaming}
        placeholder={
          isStreaming ? "Waiting for response..." : "Send a message..."
        }
      />
    </div>
  );
}
