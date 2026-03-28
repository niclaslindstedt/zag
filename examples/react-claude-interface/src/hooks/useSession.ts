import { useState, useCallback, useRef } from "react";
import type { AgentLogEvent } from "../types";

export type SessionStatus =
  | "idle"
  | "connecting"
  | "streaming"
  | "ready"
  | "error";

export function useSession() {
  const [events, setEvents] = useState<AgentLogEvent[]>([]);
  const [status, setStatus] = useState<SessionStatus>("idle");
  const [sessionId, setSessionId] = useState<string | null>(null);
  const [model, setModel] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const eventSourceRef = useRef<EventSource | null>(null);

  /** Parse an SSE stream from a fetch response, appending events to state. */
  const consumeSSEStream = useCallback(
    async (response: Response, opts?: { resetEvents?: boolean }) => {
      if (opts?.resetEvents) {
        setEvents([]);
      }

      const reader = response.body!.getReader();
      const decoder = new TextDecoder();
      let buffer = "";

      while (true) {
        const { done, value } = await reader.read();
        if (done) break;

        buffer += decoder.decode(value, { stream: true });
        const lines = buffer.split("\n");
        buffer = lines.pop() || "";

        for (const line of lines) {
          if (line.startsWith("event: session_id")) continue;
          if (line.startsWith("event: done")) continue;
          if (!line.startsWith("data: ")) continue;

          const data = line.slice(6);
          try {
            const parsed = JSON.parse(data);

            // Handle session_id event (only from initial session)
            if (parsed.session_id) {
              setSessionId(parsed.session_id);
              continue;
            }

            const event = parsed as AgentLogEvent;

            if (event.type === "session_started" && event.model) {
              setModel(event.model);
            }

            if (event.type === "session_ended") {
              if (event.success) {
                setStatus("ready");
              } else {
                setStatus("error");
                if (event.error) setError(event.error);
              }
            }

            setEvents((prev) => [...prev, event]);
          } catch {
            // Skip unparseable lines
          }
        }
      }

      // Stream finished — if we didn't get a session_ended, default to ready
      setStatus((prev) => (prev === "streaming" ? "ready" : prev));
    },
    [],
  );

  const startSession = useCallback(
    async (prompt: string, provider?: string, modelOverride?: string) => {
      setEvents([]);
      setStatus("connecting");
      setError(null);
      setModel(null);
      setSessionId(null);

      try {
        const response = await fetch("/api/session", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            prompt,
            provider,
            model: modelOverride,
          }),
        });

        if (!response.ok) {
          throw new Error(`Server error: ${response.status}`);
        }

        setStatus("streaming");
        await consumeSSEStream(response, { resetEvents: false });
      } catch (err) {
        setStatus("error");
        setError(err instanceof Error ? err.message : "Unknown error");
      }
    },
    [consumeSSEStream],
  );

  const sendMessage = useCallback(
    async (message: string) => {
      if (!sessionId) return;

      setStatus("streaming");
      setError(null);

      try {
        const response = await fetch("/api/input", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ sessionId, message }),
        });

        if (!response.ok) {
          throw new Error(`Server error: ${response.status}`);
        }

        await consumeSSEStream(response);
      } catch (err) {
        setStatus("error");
        setError(err instanceof Error ? err.message : "Unknown error");
      }
    },
    [sessionId, consumeSSEStream],
  );

  const listenToSession = useCallback((targetSessionId: string) => {
    setEvents([]);
    setStatus("connecting");
    setError(null);
    setSessionId(targetSessionId);

    const es = new EventSource(`/api/listen/${targetSessionId}`);
    eventSourceRef.current = es;

    es.onopen = () => setStatus("streaming");

    es.onmessage = (e) => {
      try {
        const event = JSON.parse(e.data) as AgentLogEvent;

        if (event.type === "session_started" && event.model) {
          setModel(event.model);
        }

        if (event.type === "session_ended") {
          setStatus(event.success ? "ready" : "error");
          if (event.error) setError(event.error);
          es.close();
        }

        setEvents((prev) => [...prev, event]);
      } catch {
        // Skip
      }
    };

    es.onerror = () => {
      setStatus("error");
      setError("Connection lost");
      es.close();
    };
  }, []);

  const disconnect = useCallback(() => {
    eventSourceRef.current?.close();
    eventSourceRef.current = null;
  }, []);

  return {
    events,
    status,
    sessionId,
    model,
    error,
    startSession,
    sendMessage,
    listenToSession,
    disconnect,
  };
}
