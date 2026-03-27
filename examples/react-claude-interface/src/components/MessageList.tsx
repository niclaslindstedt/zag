import { useEffect, useRef } from "react";
import type { AgentLogEvent, ToolCallEvent, ToolResultEvent } from "../types";
import { ChatMessage } from "./ChatMessage";
import { ToolBlock } from "./ToolBlock";
import { ThinkingBlock } from "./ThinkingBlock";

interface Props {
  events: AgentLogEvent[];
}

/**
 * Build a map of tool_id -> ToolResult for pairing calls with results.
 */
function buildToolResultMap(events: AgentLogEvent[]) {
  const map = new Map<
    string,
    ToolResultEvent & { seq: number; ts: string }
  >();
  for (const e of events) {
    if (e.type === "tool_result" && e.tool_id) {
      map.set(e.tool_id, e as ToolResultEvent & { seq: number; ts: string });
    }
  }
  return map;
}

export function MessageList({ events }: Props) {
  const bottomRef = useRef<HTMLDivElement>(null);
  const toolResults = buildToolResultMap(events);

  // Auto-scroll to bottom on new events
  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [events.length]);

  // Track which tool_ids we've already rendered (via their call)
  const renderedToolResults = new Set<string>();

  return (
    <div className="message-list">
      {events.map((event) => {
        switch (event.type) {
          case "session_started":
            return (
              <div key={event.seq} className="system-message">
                <span className="system-icon">{"\u25CF"}</span>
                Session started
                {event.model && (
                  <span className="system-detail"> — {event.model}</span>
                )}
                {event.cwd && (
                  <span className="system-detail"> in {event.cwd}</span>
                )}
              </div>
            );

          case "user_message":
            return (
              <ChatMessage
                key={event.seq}
                role="user"
                content={event.content}
                timestamp={event.ts}
              />
            );

          case "assistant_message":
            return (
              <ChatMessage
                key={event.seq}
                role="assistant"
                content={event.content}
                timestamp={event.ts}
              />
            );

          case "reasoning":
            return (
              <ThinkingBlock
                key={event.seq}
                content={event.content}
                timestamp={event.ts}
              />
            );

          case "tool_call": {
            const call = event as ToolCallEvent & {
              seq: number;
              ts: string;
            };
            const result = call.tool_id
              ? toolResults.get(call.tool_id)
              : undefined;
            if (call.tool_id) renderedToolResults.add(call.tool_id);

            return (
              <ToolBlock
                key={event.seq}
                toolName={call.tool_name}
                toolKind={call.tool_kind}
                toolId={call.tool_id}
                input={call.input}
                result={
                  result
                    ? {
                        success: result.success,
                        output: result.output,
                        error: result.error,
                      }
                    : undefined
                }
                timestamp={event.ts}
              />
            );
          }

          case "tool_result": {
            // Skip if already rendered with its call
            const tr = event as ToolResultEvent & { seq: number };
            if (tr.tool_id && renderedToolResults.has(tr.tool_id)) return null;

            // Orphaned result (no matching call)
            return (
              <ToolBlock
                key={event.seq}
                toolName={tr.tool_name || "Unknown tool"}
                toolKind={tr.tool_kind}
                result={{
                  success: tr.success,
                  output: tr.output,
                  error: tr.error,
                }}
                timestamp={event.ts}
              />
            );
          }

          case "permission":
            return (
              <div key={event.seq} className="system-message permission">
                <span className="system-icon">
                  {event.granted ? "\u{1F513}" : "\u{1F512}"}
                </span>
                <span>
                  {event.granted ? "Allowed" : "Denied"}: {event.tool_name}
                </span>
                <span className="system-detail"> — {event.description}</span>
              </div>
            );

          case "session_ended":
            return (
              <div
                key={event.seq}
                className={`system-message ${event.success ? "" : "system-message--error"}`}
              >
                <span className="system-icon">{"\u25CF"}</span>
                Session ended
                {event.error && (
                  <span className="system-detail"> — {event.error}</span>
                )}
              </div>
            );

          case "provider_status":
            return (
              <div key={event.seq} className="system-message status">
                <span className="system-icon">{">"}</span>
                {event.message}
              </div>
            );

          case "stderr":
            return (
              <div key={event.seq} className="system-message stderr">
                <span className="system-icon">!</span>
                {event.message}
              </div>
            );

          default:
            return null;
        }
      })}
      <div ref={bottomRef} />
    </div>
  );
}
