import { useEffect, useRef } from "react";
import type { AgentLogEvent, ToolCallEvent, ToolResultEvent } from "../types";
import { ChatMessage } from "./ChatMessage";
import { ToolBlock } from "./ToolBlock";
import { ThinkingBlock } from "./ThinkingBlock";

interface Props {
  events: AgentLogEvent[];
  isStreaming?: boolean;
}

function buildToolResultMap(events: AgentLogEvent[]) {
  const map = new Map<string, ToolResultEvent & { seq: number; ts: string }>();
  for (const e of events) {
    if (e.type === "tool_result" && e.tool_id) {
      map.set(e.tool_id, e as ToolResultEvent & { seq: number; ts: string });
    }
  }
  return map;
}

function StreamingDots() {
  return (
    <div className="flex items-center gap-1.5 py-3 pl-6">
      <span className="w-1.5 h-1.5 rounded-full bg-amber-400 animate-bounce" style={{ animationDelay: "0ms" }} />
      <span className="w-1.5 h-1.5 rounded-full bg-amber-400 animate-bounce" style={{ animationDelay: "150ms" }} />
      <span className="w-1.5 h-1.5 rounded-full bg-amber-400 animate-bounce" style={{ animationDelay: "300ms" }} />
    </div>
  );
}

export function MessageList({ events, isStreaming }: Props) {
  const bottomRef = useRef<HTMLDivElement>(null);
  const toolResults = buildToolResultMap(events);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [events.length]);

  const renderedToolResults = new Set<string>();

  // Check if the last event suggests we're waiting for more
  const lastEvent = events[events.length - 1];
  const showStreamingDots =
    isStreaming &&
    lastEvent &&
    lastEvent.type !== "assistant_message" &&
    lastEvent.type !== "session_ended";

  return (
    <div className="flex flex-col gap-0.5">
      {events.map((event) => {
        switch (event.type) {
          case "session_started":
            return (
              <div key={event.seq} className="flex items-center gap-3 py-2 my-2">
                <div className="flex-1 h-px bg-zinc-800" />
                <span className="text-xs text-zinc-600 flex items-center gap-2">
                  <span className="w-1.5 h-1.5 rounded-full bg-emerald-500" />
                  Session started
                  {event.model && (
                    <span className="font-mono text-zinc-500">{event.model}</span>
                  )}
                </span>
                <div className="flex-1 h-px bg-zinc-800" />
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
            const call = event as ToolCallEvent & { seq: number; ts: string };
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
            const tr = event as ToolResultEvent & { seq: number };
            if (tr.tool_id && renderedToolResults.has(tr.tool_id)) return null;
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
              <div key={event.seq} className="flex items-center gap-2 py-1 text-xs text-yellow-500/80">
                <span>{event.granted ? "\u{1F513}" : "\u{1F512}"}</span>
                <span className="font-medium">
                  {event.granted ? "Allowed" : "Denied"}:
                </span>
                <span className="font-mono text-yellow-400/60">{event.tool_name}</span>
                <span className="text-zinc-600">{event.description}</span>
              </div>
            );

          case "session_ended":
            return (
              <div key={event.seq} className="flex items-center gap-3 py-2 my-2">
                <div className="flex-1 h-px bg-zinc-800" />
                <span className={`text-xs flex items-center gap-2 ${event.success ? "text-zinc-600" : "text-red-400"}`}>
                  <span className={`w-1.5 h-1.5 rounded-full ${event.success ? "bg-zinc-600" : "bg-red-500"}`} />
                  Session ended
                  {event.error && (
                    <span className="text-red-400/70">{event.error}</span>
                  )}
                </span>
                <div className="flex-1 h-px bg-zinc-800" />
              </div>
            );

          case "provider_status":
            return (
              <div key={event.seq} className="flex items-center gap-2 py-0.5 text-xs text-zinc-600 italic">
                <span className="text-zinc-700">{">"}</span>
                {event.message}
              </div>
            );

          case "stderr":
            return (
              <div key={event.seq} className="flex items-center gap-2 py-0.5 text-xs text-red-400/60">
                <span className="text-red-500/50">!</span>
                <span className="font-mono">{event.message}</span>
              </div>
            );

          default:
            return null;
        }
      })}

      {showStreamingDots && <StreamingDots />}
      <div ref={bottomRef} />
    </div>
  );
}
