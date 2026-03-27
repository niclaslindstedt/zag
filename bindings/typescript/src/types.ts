/** Unified output from an agent session. */
export interface AgentOutput {
  agent: string;
  session_id: string;
  events: Event[];
  result: string | null;
  is_error: boolean;
  total_cost_usd: number | null;
  usage: Usage | null;
}

/** Usage statistics for an agent session. */
export interface Usage {
  input_tokens: number;
  output_tokens: number;
  cache_read_tokens?: number;
  cache_creation_tokens?: number;
  web_search_requests?: number;
  web_fetch_requests?: number;
}

/** A single event in an agent session (tagged union on `type`). */
export type Event =
  | InitEvent
  | UserMessageEvent
  | AssistantMessageEvent
  | ToolExecutionEvent
  | ResultEvent
  | ErrorEvent
  | PermissionRequestEvent;

export interface InitEvent {
  type: "init";
  model: string;
  tools: string[];
  working_directory: string | null;
  metadata: Record<string, unknown>;
}

export interface UserMessageEvent {
  type: "user_message";
  content: ContentBlock[];
}

export interface AssistantMessageEvent {
  type: "assistant_message";
  content: ContentBlock[];
  usage: Usage | null;
}

export interface ToolExecutionEvent {
  type: "tool_execution";
  tool_name: string;
  tool_id: string;
  input: unknown;
  result: ToolResult;
}

export interface ResultEvent {
  type: "result";
  success: boolean;
  message: string | null;
  duration_ms: number | null;
  num_turns: number | null;
}

export interface ErrorEvent {
  type: "error";
  message: string;
  details: unknown | null;
}

export interface PermissionRequestEvent {
  type: "permission_request";
  tool_name: string;
  description: string;
  granted: boolean;
}

/** A block of content in an assistant message. */
export type ContentBlock = TextBlock | ToolUseBlock;

export interface TextBlock {
  type: "text";
  text: string;
}

export interface ToolUseBlock {
  type: "tool_use";
  id: string;
  name: string;
  input: unknown;
}

/** Result from a tool execution. */
export interface ToolResult {
  success: boolean;
  output: string | null;
  error: string | null;
  data: unknown | null;
}

/** Error thrown when the zag process fails. */
export class ZagError extends Error {
  constructor(
    message: string,
    public readonly exitCode: number | null,
    public readonly stderr: string,
  ) {
    super(message);
    this.name = "ZagError";
  }
}
