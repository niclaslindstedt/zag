/** Unified output from an agent session. */
export interface AgentOutput {
  agent: string;
  session_id: string;
  events: Event[];
  result: string | null;
  is_error: boolean;
  exit_code?: number | null;
  error_message?: string | null;
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

/** Feature support declaration for a provider capability. */
export interface FeatureSupport {
  supported: boolean;
  native: boolean;
}

/** Session log support with completeness level. */
export interface SessionLogSupport {
  supported: boolean;
  native: boolean;
  completeness?: string;
}

/** Size alias mappings (small/medium/large to actual model names). */
export interface SizeMappings {
  small: string;
  medium: string;
  large: string;
}

/** All feature flags for a provider. */
export interface Features {
  interactive: FeatureSupport;
  non_interactive: FeatureSupport;
  resume: FeatureSupport;
  resume_with_prompt: FeatureSupport;
  session_logs: SessionLogSupport;
  json_output: FeatureSupport;
  stream_json: FeatureSupport;
  json_schema: FeatureSupport;
  input_format: FeatureSupport;
  streaming_input: FeatureSupport;
  worktree: FeatureSupport;
  sandbox: FeatureSupport;
  system_prompt: FeatureSupport;
  auto_approve: FeatureSupport;
  review: FeatureSupport;
  add_dirs: FeatureSupport;
  max_turns: FeatureSupport;
}

/** Full capability declaration for a provider. */
export interface ProviderCapability {
  provider: string;
  default_model: string;
  available_models: string[];
  size_mappings: SizeMappings;
  features: Features;
}

/** Result of resolving a model alias. */
export interface ResolvedModel {
  input: string;
  resolved: string;
  is_alias: boolean;
  provider: string;
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
