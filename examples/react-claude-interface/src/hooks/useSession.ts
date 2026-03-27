import { useState, useCallback, useRef } from "react";
import type { AgentLogEvent, ToolCallEvent, ToolResultEvent } from "../types";

export type SessionStatus =
  | "idle"
  | "connecting"
  | "streaming"
  | "ended"
  | "error";

/** A paired tool call and its result */
export interface ToolPair {
  call: ToolCallEvent & { seq: number; ts: string };
  result?: ToolResultEvent & { seq: number; ts: string };
}

export function useSession() {
  const [events, setEvents] = useState<AgentLogEvent[]>([]);
  const [status, setStatus] = useState<SessionStatus>("idle");
  const [sessionId, setSessionId] = useState<string | null>(null);
  const [model, setModel] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const eventSourceRef = useRef<EventSource | null>(null);

  const startSession = useCallback(
    async (prompt: string, provider?: string, modelOverride?: string) => {
      // Reset state
      setEvents([]);
      setStatus("connecting");
      setError(null);
      setModel(null);

      try {
        // We use fetch + ReadableStream instead of EventSource because
        // EventSource only supports GET requests
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

        const sid = response.headers.get("X-Session-Id");

        setStatus("streaming");

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

              // Handle session_id event
              if (parsed.session_id) {
                setSessionId(parsed.session_id);
                continue;
              }

              const event = parsed as AgentLogEvent;

              if (event.type === "session_started" && event.model) {
                setModel(event.model);
              }

              if (event.type === "session_ended") {
                setStatus(event.success ? "ended" : "error");
                if (event.error) setError(event.error);
              }

              setEvents((prev) => [...prev, event]);
            } catch {
              // Skip unparseable lines
            }
          }
        }

        // Stream finished
        setStatus((prev) => (prev === "streaming" ? "ended" : prev));
      } catch (err) {
        setStatus("error");
        setError(err instanceof Error ? err.message : "Unknown error");
      }
    },
    [],
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
          setStatus(event.success ? "ended" : "error");
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
    listenToSession,
    disconnect,
  };
}
