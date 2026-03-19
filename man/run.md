# agent run

Start an interactive agent session.

## Synopsis

    agent [flags] run [prompt]

## Description

Starts a full interactive session with the selected agent. The agent's CLI takes over the terminal — you can type prompts, approve tool use, and have a back-and-forth conversation.

If a prompt is provided, it is sent as the first message. Otherwise the agent starts with an empty conversation.

When `--json` or `--json-schema` is combined with a prompt, the session runs non-interactively instead (equivalent to `exec`) to capture and validate the output.

## Arguments

    prompt    Optional initial prompt for the session

## Flags

All global flags apply (see `agent man agent`). No command-specific flags.

## Behavior

The agent subprocess inherits stdin, stdout, and stderr, giving you full interactive control. The wrapper displays initialization messages (model name, auto-approve status) unless `--quiet` is set.

After the session ends, agent resources are cleaned up. If a worktree was created (`--worktree`), you are prompted whether to keep or remove it. If a sandbox was created (`--sandbox`), you are similarly prompted.

## Sandbox Mode

The `--sandbox` flag runs the agent inside a Docker sandbox microVM for stronger isolation than git worktrees. Docker sandboxes provide: microVM isolation, bidirectional workspace file sync, network policy enforcement, and transparent credential injection from host env vars.

    agent --sandbox run                       Auto-named sandbox
    agent --sandbox my-name run               Named sandbox

Each provider maps to a Docker sandbox template (e.g., `docker/sandbox-templates:claude-code` for Claude). The agent binary and flags are passed through to the sandbox via `docker sandbox run ... -- <agent-flags>`.

`--sandbox` and `--worktree` are mutually exclusive. `--sandbox` cannot be used with `review`, `config`, or `man`.

After an interactive sandbox session, you are prompted whether to keep or remove the sandbox. Sandboxes can be resumed with `agent resume <session-id>`.

## Examples

    agent run                                 Start with default provider (Claude)
    agent -p gemini run                       Interactive Gemini session
    agent run "refactor the auth module"      Start with an initial prompt
    agent -w my-feature run                   Run in a named worktree
    agent --sandbox run                       Run in a Docker sandbox
    agent --sandbox my-sandbox run            Named sandbox session
    agent --model small run                   Use lightweight model for quick tasks

## See Also

    agent man exec      Non-interactive alternative
    agent man resume    Continue a previous session
