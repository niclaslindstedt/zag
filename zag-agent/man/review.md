# zag review

Review code changes.

## Synopsis

    zag [flags] review [options] [prompt]

## Description

Runs an automated code review. You specify what to review — uncommitted changes, a diff against a base branch, or a specific commit — and the agent analyzes the code and provides feedback.

At least one of `--uncommitted`, `--base`, or `--commit` must be provided.

When using Codex (`-p codex`), the native `codex review` command is used. For all other providers, the diff is gathered by the CLI and passed to the agent as a structured review prompt.

An optional positional `prompt` argument lets you append additional instructions (e.g., "Focus on security issues" or "Ignore formatting changes").

## Flags

    --uncommitted          Review staged, unstaged, and untracked changes
    --base <BRANCH>        Review changes compared to a base branch (e.g., main)
    --commit <SHA>         Review changes from a specific commit
    --title <TEXT>         Optional title for the review summary

Global flags that apply: `-p`, `--model`, `--system-prompt`, `--root`, `--auto-approve`, `--add-dir`, `--file`, `--env`, `--mcp-config`, `--max-turns`, `--debug`, `--quiet`.

Flags that cannot be used with review: `--worktree`, `--sandbox`, `--json`, `--json-schema`, `-p auto`, `-m auto`.

## Notes

- The `-p` flag selects the provider (e.g., `-p claude`, `-p codex`, `-p gemini`)
- Codex uses its native `codex review` command; other providers receive a prompt-based review
- The `--model` flag selects the model for the chosen provider
- Output is streamed to the terminal (non-interactive for non-Codex providers)

## Examples

    zag review --uncommitted                                Review working changes
    zag review --base main                                  Review against main branch
    zag review --commit abc123                              Review a specific commit
    zag review --uncommitted --title "Auth refactor"        Review with a title
    zag review --uncommitted -p claude                      Review with Claude
    zag review --base main -p gemini "Focus on security"    Review with Gemini + custom prompt
    zag review --uncommitted --model large -a               Use max model, auto-approve

## See Also

    zag man exec    Non-interactive execution for custom review prompts
