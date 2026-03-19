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

After the session ends, agent resources are cleaned up. If a worktree was created (`--worktree`), you are prompted whether to keep or remove it.

## Examples

    agent run                                 Start with default provider (Claude)
    agent -p gemini run                       Interactive Gemini session
    agent run "refactor the auth module"      Start with an initial prompt
    agent -w my-feature run                   Run in a named worktree
    agent --model small run                   Use lightweight model for quick tasks

## See Also

    agent man exec      Non-interactive alternative
    agent man resume    Continue a previous session
