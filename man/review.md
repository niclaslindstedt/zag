# zag review

Review code changes.

## Synopsis

    zag [flags] review [options]

## Description

Runs an automated code review using Codex under the hood. You specify what to review — uncommitted changes, a diff against a base branch, or a specific commit — and the agent analyzes the code and provides feedback.

At least one of `--uncommitted`, `--base`, or `--commit` must be provided.

## Flags

    --uncommitted          Review staged, unstaged, and untracked changes
    --base <BRANCH>        Review changes compared to a base branch (e.g., main)
    --commit <SHA>         Review changes from a specific commit
    --title <TEXT>         Optional title for the review summary

Global flags that apply: `--model`, `--system-prompt`, `--root`, `--auto-approve`, `--add-dir`, `--debug`, `--quiet`.

Flags that cannot be used with review: `--worktree`, `--sandbox`, `--json`, `--json-schema`, `--json-stream`, `-p auto`, `-m auto`.

## Notes

- The provider flag (`-p`) is ignored — review always uses Codex
- The `--model` flag selects the Codex model (e.g., `--model large` for gpt-5.4)
- Output is interactive (streamed to terminal), not captured

## Examples

    zag review --uncommitted                          Review working changes
    zag review --base main                            Review against main branch
    zag review --commit abc123                        Review a specific commit
    zag review --uncommitted --title "Auth refactor"  Review with a title
    zag review --uncommitted --model large -a         Use max model, auto-approve

## See Also

    zag man exec    Non-interactive execution for custom review prompts
