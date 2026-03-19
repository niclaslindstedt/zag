# agent resume

Resume a previous agent session.

## Synopsis

    agent [flags] resume [session-id] [--last]

## Description

Continues a previous session where it left off. The agent reloads conversation history and you can keep working from the same point.

If a session ID is provided, that specific session is resumed. If `--last` is used, the most recent session is resumed. If neither is given, the agent shows a session picker or resumes the most recent (behavior depends on the provider).

When resuming a worktree session, the CLI automatically restores the correct worktree directory from the session mapping in `.agent/sessions.json`. If the worktree no longer exists, the stale mapping is removed and the session resumes without it.

## Arguments

    session-id    Optional session ID to resume

## Flags

    --last    Resume the most recent session

All global flags apply (see `agent man agent`), with these notes:

- `--worktree` is ignored (the worktree comes from the session mapping)
- `--json`, `--json-schema`, `--json-stream` cannot be used with resume
- `-p auto` / `-m auto` cannot be used with resume

## Provider Behavior

    claude    Uses --resume <id> or --continue (most recent)
    codex     Uses resume <id> or --last
    gemini    Uses --resume <id> or --resume latest
    copilot   Uses --resume (always resumes most recent; ignores session ID)

## Post-Session

After an interactive resume of a worktree session, you are prompted whether to keep or remove the worktree — same as after `agent run` with `--worktree`.

## Examples

    agent resume                        Resume most recent / show picker
    agent resume abc-123-def-456        Resume specific session
    agent resume --last                 Resume most recent session
    agent -p codex resume --last        Resume most recent Codex session

## See Also

    agent man run     Start a new session
    agent man exec    Non-interactive execution
