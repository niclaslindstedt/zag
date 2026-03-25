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
    | AssistantMessageEvent
    | ToolExecutionEvent
    | ResultEvent
    | ErrorEvent
    | PermissionRequestEvent
)

_EVENT_PARSERS: dict[str, type] = {
    "init": InitEvent,
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
            total_cost_usd=data.get("total_cost_usd"),
            usage=usage,
        )
