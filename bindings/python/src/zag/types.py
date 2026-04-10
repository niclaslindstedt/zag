"""Type definitions for zag agent output."""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any


class ZagError(Exception):
    """Error raised when the zag process fails."""

    def __init__(self, message: str, exit_code: int | None, stderr: str) -> None:
        super().__init__(message)
        self.exit_code = exit_code
        self.stderr = stderr


class ZagFeatureUnsupportedError(ZagError):
    """Error raised when a builder option requires a provider feature that the
    configured provider does not support.

    The builder validates feature-gated options (``exec_streaming``,
    ``worktree``, ``sandbox``, ``system_prompt``, ``add_dir``, ``max_turns``)
    against the capability declarations exposed by ``zag discover`` before
    spawning the CLI, so callers receive a clear, typed error instead of a
    cryptic runtime exit code.
    """

    def __init__(
        self,
        message: str,
        provider: str,
        feature: str,
        method: str,
        supported_providers: list[str],
    ) -> None:
        super().__init__(message, None, "")
        self.provider = provider
        self.feature = feature
        self.method = method
        self.supported_providers = supported_providers


# ---------------------------------------------------------------------------
# Usage
# ---------------------------------------------------------------------------


@dataclass
class Usage:
    """Token usage statistics for an agent session."""

    input_tokens: int = 0
    output_tokens: int = 0
    cache_read_tokens: int | None = None
    cache_creation_tokens: int | None = None
    web_search_requests: int | None = None
    web_fetch_requests: int | None = None

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> Usage:
        return cls(
            input_tokens=data.get("input_tokens", 0),
            output_tokens=data.get("output_tokens", 0),
            cache_read_tokens=data.get("cache_read_tokens"),
            cache_creation_tokens=data.get("cache_creation_tokens"),
            web_search_requests=data.get("web_search_requests"),
            web_fetch_requests=data.get("web_fetch_requests"),
        )


# ---------------------------------------------------------------------------
# Tool Result
# ---------------------------------------------------------------------------


@dataclass
class ToolResult:
    """Result from a tool execution."""

    success: bool
    output: str | None = None
    error: str | None = None
    data: Any = None

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> ToolResult:
        return cls(
            success=data.get("success", False),
            output=data.get("output"),
            error=data.get("error"),
            data=data.get("data"),
        )


# ---------------------------------------------------------------------------
# Content Blocks
# ---------------------------------------------------------------------------


@dataclass
class TextBlock:
    """Plain text content block."""

    type: str = "text"
    text: str = ""

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> TextBlock:
        return cls(text=data.get("text", ""))


@dataclass
class ToolUseBlock:
    """Tool invocation content block."""

    type: str = "tool_use"
    id: str = ""
    name: str = ""
    input: Any = None

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> ToolUseBlock:
        return cls(
            id=data.get("id", ""),
            name=data.get("name", ""),
            input=data.get("input"),
        )


ContentBlock = TextBlock | ToolUseBlock


def _parse_content_block(data: dict[str, Any]) -> ContentBlock:
    if data.get("type") == "tool_use":
        return ToolUseBlock.from_dict(data)
    return TextBlock.from_dict(data)


# ---------------------------------------------------------------------------
# Events (tagged union on "type")
# ---------------------------------------------------------------------------


@dataclass
class InitEvent:
    """Session initialization event."""

    type: str = "init"
    model: str = ""
    tools: list[str] = field(default_factory=list)
    working_directory: str | None = None
    metadata: dict[str, Any] = field(default_factory=dict)

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> InitEvent:
        return cls(
            model=data.get("model", ""),
            tools=data.get("tools", []),
            working_directory=data.get("working_directory"),
            metadata=data.get("metadata", {}),
        )


@dataclass
class UserMessageEvent:
    """User message (replayed via --replay-user-messages)."""

    type: str = "user_message"
    content: list[ContentBlock] = field(default_factory=list)

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> UserMessageEvent:
        content = [_parse_content_block(b) for b in data.get("content", [])]
        return cls(content=content)


@dataclass
class AssistantMessageEvent:
    """Message from the assistant."""

    type: str = "assistant_message"
    content: list[ContentBlock] = field(default_factory=list)
    usage: Usage | None = None

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> AssistantMessageEvent:
        content = [_parse_content_block(b) for b in data.get("content", [])]
        usage_data = data.get("usage")
        usage = Usage.from_dict(usage_data) if usage_data else None
        return cls(content=content, usage=usage)


@dataclass
class ToolExecutionEvent:
    """Tool execution event."""

    type: str = "tool_execution"
    tool_name: str = ""
    tool_id: str = ""
    input: Any = None
    result: ToolResult = field(default_factory=lambda: ToolResult(success=False))

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> ToolExecutionEvent:
        return cls(
            tool_name=data.get("tool_name", ""),
            tool_id=data.get("tool_id", ""),
            input=data.get("input"),
            result=ToolResult.from_dict(data.get("result", {})),
        )


@dataclass
class ResultEvent:
    """Final session result event."""

    type: str = "result"
    success: bool = False
    message: str | None = None
    duration_ms: int | None = None
    num_turns: int | None = None

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> ResultEvent:
        return cls(
            success=data.get("success", False),
            message=data.get("message"),
            duration_ms=data.get("duration_ms"),
            num_turns=data.get("num_turns"),
        )


@dataclass
class ErrorEvent:
    """Error event."""

    type: str = "error"
    message: str = ""
    details: Any = None

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> ErrorEvent:
        return cls(
            message=data.get("message", ""),
            details=data.get("details"),
        )


@dataclass
class PermissionRequestEvent:
    """Permission request event."""

    type: str = "permission_request"
    tool_name: str = ""
    description: str = ""
    granted: bool = False

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> PermissionRequestEvent:
        return cls(
            tool_name=data.get("tool_name", ""),
            description=data.get("description", ""),
            granted=data.get("granted", False),
        )


Event = (
    InitEvent
    | UserMessageEvent
    | AssistantMessageEvent
    | ToolExecutionEvent
    | ResultEvent
    | ErrorEvent
    | PermissionRequestEvent
)

_EVENT_PARSERS: dict[str, type] = {
    "init": InitEvent,
    "user_message": UserMessageEvent,
    "assistant_message": AssistantMessageEvent,
    "tool_execution": ToolExecutionEvent,
    "result": ResultEvent,
    "error": ErrorEvent,
    "permission_request": PermissionRequestEvent,
}


def parse_event(data: dict[str, Any]) -> Event:
    """Parse a raw dict into the appropriate Event subtype."""
    event_type = data.get("type", "")
    parser = _EVENT_PARSERS.get(event_type)
    if parser is None:
        raise ValueError(f"Unknown event type: {event_type}")
    return parser.from_dict(data)  # type: ignore[union-attr]


# ---------------------------------------------------------------------------
# AgentOutput
# ---------------------------------------------------------------------------


@dataclass
class AgentOutput:
    """Unified output from an agent session."""

    agent: str = ""
    session_id: str = ""
    events: list[Event] = field(default_factory=list)
    result: str | None = None
    is_error: bool = False
    exit_code: int | None = None
    error_message: str | None = None
    total_cost_usd: float | None = None
    usage: Usage | None = None

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> AgentOutput:
        events = [parse_event(e) for e in data.get("events", [])]
        usage_data = data.get("usage")
        usage = Usage.from_dict(usage_data) if usage_data else None
        return cls(
            agent=data.get("agent", ""),
            session_id=data.get("session_id", ""),
            events=events,
            result=data.get("result"),
            is_error=data.get("is_error", False),
            exit_code=data.get("exit_code"),
            error_message=data.get("error_message"),
            total_cost_usd=data.get("total_cost_usd"),
            usage=usage,
        )


# ---------------------------------------------------------------------------
# Discovery Types
# ---------------------------------------------------------------------------


@dataclass
class FeatureSupport:
    """Feature support declaration for a provider capability."""

    supported: bool = False
    native: bool = False

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> FeatureSupport:
        return cls(
            supported=data.get("supported", False),
            native=data.get("native", False),
        )


@dataclass
class SessionLogSupport:
    """Session log support with completeness level."""

    supported: bool = False
    native: bool = False
    completeness: str | None = None

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> SessionLogSupport:
        return cls(
            supported=data.get("supported", False),
            native=data.get("native", False),
            completeness=data.get("completeness"),
        )


@dataclass
class StreamingInputSupport:
    """Streaming input support with mid-turn injection semantics.

    ``semantics`` describes what happens when
    :meth:`StreamingSession.send_user_message` is called while the agent is
    producing a response on the current turn. One of:

    - ``"queue"`` — buffered and delivered at the next turn boundary (the
      current turn is not interrupted).
    - ``"interrupt"`` — cancels the current turn and starts a new one.
    - ``"between-turns-only"`` — mid-turn sends are an error or no-op.

    ``None`` when ``supported`` is ``False``.
    """

    supported: bool = False
    native: bool = False
    semantics: str | None = None

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> StreamingInputSupport:
        return cls(
            supported=data.get("supported", False),
            native=data.get("native", False),
            semantics=data.get("semantics"),
        )


@dataclass
class SizeMappings:
    """Size alias mappings (small/medium/large to actual model names)."""

    small: str = ""
    medium: str = ""
    large: str = ""

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> SizeMappings:
        return cls(
            small=data.get("small", ""),
            medium=data.get("medium", ""),
            large=data.get("large", ""),
        )


@dataclass
class Features:
    """All feature flags for a provider."""

    interactive: FeatureSupport = field(default_factory=FeatureSupport)
    non_interactive: FeatureSupport = field(default_factory=FeatureSupport)
    resume: FeatureSupport = field(default_factory=FeatureSupport)
    resume_with_prompt: FeatureSupport = field(default_factory=FeatureSupport)
    session_logs: SessionLogSupport = field(default_factory=SessionLogSupport)
    json_output: FeatureSupport = field(default_factory=FeatureSupport)
    stream_json: FeatureSupport = field(default_factory=FeatureSupport)
    json_schema: FeatureSupport = field(default_factory=FeatureSupport)
    input_format: FeatureSupport = field(default_factory=FeatureSupport)
    streaming_input: StreamingInputSupport = field(default_factory=StreamingInputSupport)
    worktree: FeatureSupport = field(default_factory=FeatureSupport)
    sandbox: FeatureSupport = field(default_factory=FeatureSupport)
    system_prompt: FeatureSupport = field(default_factory=FeatureSupport)
    auto_approve: FeatureSupport = field(default_factory=FeatureSupport)
    review: FeatureSupport = field(default_factory=FeatureSupport)
    add_dirs: FeatureSupport = field(default_factory=FeatureSupport)
    max_turns: FeatureSupport = field(default_factory=FeatureSupport)

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> Features:
        def fs(key: str) -> FeatureSupport:
            return FeatureSupport.from_dict(data.get(key, {}))

        return cls(
            interactive=fs("interactive"),
            non_interactive=fs("non_interactive"),
            resume=fs("resume"),
            resume_with_prompt=fs("resume_with_prompt"),
            session_logs=SessionLogSupport.from_dict(data.get("session_logs", {})),
            json_output=fs("json_output"),
            stream_json=fs("stream_json"),
            json_schema=fs("json_schema"),
            input_format=fs("input_format"),
            streaming_input=StreamingInputSupport.from_dict(
                data.get("streaming_input", {})
            ),
            worktree=fs("worktree"),
            sandbox=fs("sandbox"),
            system_prompt=fs("system_prompt"),
            auto_approve=fs("auto_approve"),
            review=fs("review"),
            add_dirs=fs("add_dirs"),
            max_turns=fs("max_turns"),
        )


@dataclass
class ProviderCapability:
    """Full capability declaration for a provider."""

    provider: str = ""
    default_model: str = ""
    available_models: list[str] = field(default_factory=list)
    size_mappings: SizeMappings = field(default_factory=SizeMappings)
    features: Features = field(default_factory=Features)

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> ProviderCapability:
        return cls(
            provider=data.get("provider", ""),
            default_model=data.get("default_model", ""),
            available_models=data.get("available_models", []),
            size_mappings=SizeMappings.from_dict(data.get("size_mappings", {})),
            features=Features.from_dict(data.get("features", {})),
        )


@dataclass
class ResolvedModel:
    """Result of resolving a model alias."""

    input: str = ""
    resolved: str = ""
    is_alias: bool = False
    provider: str = ""

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> ResolvedModel:
        return cls(
            input=data.get("input", ""),
            resolved=data.get("resolved", ""),
            is_alias=data.get("is_alias", False),
            provider=data.get("provider", ""),
        )
