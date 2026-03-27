/** Tool category — matches zag's ToolKind enum */
export type ToolKind =
  | "shell"
  | "file_read"
  | "file_write"
  | "file_edit"
  | "search"
  | "sub_agent"
  | "web"
  | "notebook"
  | "other";

/** A single event from `zag listen --json` or `zag exec -o stream-json` */
export type AgentLogEvent = {
  seq: number;
  ts: string;
  provider: string;
  wrapper_session_id: string;
  provider_session_id?: string;
  source_kind: string;
  completeness: string;
} & LogEventKind;

export type LogEventKind =
  | SessionStartedEvent
  | UserMessageEvent
  | AssistantMessageEvent
  | ReasoningEvent
  | ToolCallEvent
  | ToolResultEvent
  | PermissionEvent
  | ProviderStatusEvent
  | StderrEvent
  | ParseWarningEvent
  | SessionClearedEvent
  | SessionEndedEvent;

export interface SessionStartedEvent {
  type: "session_started";
  command: string;
  model?: string;
  cwd?: string;
  resumed: boolean;
  backfilled: boolean;
}

export interface UserMessageEvent {
  type: "user_message";
  role: string;
  content: string;
  message_id?: string;
}

export interface AssistantMessageEvent {
  type: "assistant_message";
  content: string;
  message_id?: string;
}

export interface ReasoningEvent {
  type: "reasoning";
  content: string;
  message_id?: string;
}

export interface ToolCallEvent {
  type: "tool_call";
  tool_name: string;
  tool_kind?: ToolKind;
  tool_id?: string;
  input?: Record<string, unknown>;
}

export interface ToolResultEvent {
  type: "tool_result";
  tool_name?: string;
  tool_kind?: ToolKind;
  tool_id?: string;
  success?: boolean;
  output?: string;
  error?: string;
  data?: unknown;
}

export interface PermissionEvent {
  type: "permission";
  tool_name: string;
  description: string;
  granted: boolean;
}

export interface ProviderStatusEvent {
  type: "provider_status";
  message: string;
  data?: unknown;
}

export interface StderrEvent {
  type: "stderr";
  message: string;
}

export interface ParseWarningEvent {
  type: "parse_warning";
  message: string;
  raw?: string;
}

export interface SessionClearedEvent {
  type: "session_cleared";
  old_session_id?: string;
  new_session_id?: string;
}

export interface SessionEndedEvent {
  type: "session_ended";
  success: boolean;
  error?: string;
}
