# React Claude Interface

A single-page React app that provides a Claude Code-like interface with multi-turn conversation support using `zag exec` and `zag input`.

## How it works

1. You type a prompt in the input box
2. The Express backend spawns `zag exec -o stream-json --session <uuid> "<prompt>"`
3. NDJSON events stream back to the browser via Server-Sent Events (SSE)
4. Events render in real-time as a chat interface with collapsible tool calls and thinking blocks
5. After the first response completes, you can send follow-up messages
6. Follow-ups use `zag input <session-id> "<message>" -o stream-json` to continue the same session

## Prerequisites

- [Node.js](https://nodejs.org/) 18+
- [zag](../../) installed and on your PATH
- A configured provider (e.g. `ANTHROPIC_API_KEY` set for Claude)

## Setup

```bash
npm install
```

## Development

```bash
npm run dev
```

This starts both:
- **Vite dev server** on http://localhost:5173 (frontend)
- **Express SSE server** on http://localhost:3001 (backend, proxied by Vite)

Open http://localhost:5173 in your browser.

## Architecture

```
server.ts               Express backend — spawns zag, streams NDJSON as SSE
                        POST /api/session  — start a new session (zag exec)
                        POST /api/input    — send follow-up message (zag input)
                        GET  /api/listen   — attach to running session (zag listen)
src/
  App.tsx               Main layout: status bar + message list + prompt input
  App.css               Dark theme styles (Claude Code aesthetic)
  types.ts              AgentLogEvent types matching zag's session log format
  hooks/useSession.ts   SSE connection + event state management + multi-turn
  components/
    StatusBar.tsx        Connection status, model name, session ID
    MessageList.tsx      Routes events to the right component
    ChatMessage.tsx      User/assistant message rendering with basic markdown
    ToolBlock.tsx        Collapsible tool call + result with icons per tool kind
    ThinkingBlock.tsx    Collapsible reasoning/thinking block
    PromptInput.tsx      Textarea + send button
```

## Event Types

The app consumes `AgentLogEvent` objects from zag's session log format (NDJSON). Key event types:

| Type | Renders as |
|------|-----------|
| `session_started` | System message with model info |
| `user_message` | User chat bubble |
| `assistant_message` | Assistant chat bubble with markdown |
| `reasoning` | Collapsible thinking block |
| `tool_call` + `tool_result` | Collapsible tool block (paired by `tool_id`) |
| `permission` | Permission grant/deny indicator |
| `session_ended` | System message |
