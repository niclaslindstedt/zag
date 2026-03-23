# logs

Historical session log utilities.

## Synopsis

    agent logs import

## Description

`agent logs import` imports historical provider logs into the unified session log store under `~/.agent/projects/<sanitized-path>/logs/`.

The import is idempotent:

- Previously imported sessions are skipped
- Existing unified session logs are preserved
- Providers with no historical logs are ignored

## Commands

### import

Import historical logs from supported providers:

- Claude
- Codex
- Gemini
- Copilot
- Ollama (no-op today)

## Examples

    agent logs import
    agent --root /tmp/project logs import

## Notes

- This command is manual by design and does not run automatically during `agent run`
- Imported session logs are tracked with backfill state so repeated imports only add new history
