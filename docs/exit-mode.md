# Exit mode (`--exit`)

`--exit` makes an interactive `zag run` session terminate by *capturing
a final result*, the way `zag exec` does — without paying the API-token
cost of Claude's non-interactive `--print` mode.

When you launch with `--exit`, zag appends instructions to the prompt
telling the agent: "when you're done, run `zag ps kill self
\"<result>\"` to terminate this session and submit the final result."
The kill call validates the result against the constraints captured at
launch (hint, JSON shape, JSON schema) and writes a `session_result`
event into the session log. `zag output <session>` then prints exactly
that string.

## Forms

```bash
zag -p claude run --exit "the result of the calculation" "what is 2+3?"
```

| Form                  | Meaning                                                          |
|-----------------------|------------------------------------------------------------------|
| `--exit`              | Bare — agent must call `zag ps kill self`, result unconstrained. |
| `--exit "<hint>"`     | Hint — `ps kill` rejects empty results; hint is shown to agent.  |

The hint is a short human-readable description of what the agent should
produce. It's both a prompt to the agent and an enforcement gate at
kill time.

## Combining with `--json` / `--json-schema`

| Flag                 | Effect at kill time                              |
|----------------------|--------------------------------------------------|
| `--json`             | Result must be valid JSON. Markdown fences are stripped automatically. |
| `--json-schema <S>`  | Result must validate against schema `S` (file path or inline JSON).    |

The schema is embedded verbatim in the agent's prompt so it knows the
exact shape to produce. Schema violations reject the kill with a
detailed stderr message listing each violation; the agent can read
that message, fix the result, and call kill again.

Example: require a specific shape.

```bash
zag -p claude run \
  --exit "the answer" --json --json-schema '{"type":"object","required":["answer"]}' \
  "Compute 6 * 7 and report it as JSON with key \"answer\"."
```

## Validation failures

If the submitted result fails any constraint, `zag ps kill self ...`
exits non-zero and writes a steering message to stderr. The session
keeps running — the agent reads stderr, corrects its result, and calls
kill again. Three error kinds:

| Kind                   | Trigger                                                        |
|------------------------|----------------------------------------------------------------|
| `EmptyResult`          | Hint was set but result is empty/whitespace.                   |
| `InvalidJson`          | `--json` was set but result doesn't parse as JSON.             |
| `SchemaViolations`     | `--json-schema` was set and one or more validations failed.    |

These are surfaced as the `Display` of
`zag_agent::exit_mode::ExitValidationError`.

## How the result reaches you

1. Agent calls `zag ps kill self "<result>"` (or `--file <path>` for
   large or multi-line results).
2. `zag ps kill` loads the session's `ExitConstraints`, validates the
   result. On failure: error, process keeps running.
3. On success: emits a `session_result` event into the session log,
   marks the process `killed`, sends `SIGTERM`.
4. `zag output <session-id>` reads back the `session_result` and prints
   it. `zag collect <session-id>` returns it as structured JSON.

## Interaction with usage-limit auto-resume

Exit constraints are captured at launch and persist across auto-resume.
If a session launched with `--exit "the answer"` hits a Claude weekly
limit, the auto-resume scheduler waits out the limit, injects the
configured resume message (default `Continue`), and the agent picks up
where it left off — *still bound to produce a non-empty answer at
`ps kill` time*. See [usage-limits.md](usage-limits.md) for the
auto-resume model.

## Not valid with `exec`

`--exit` is rejected at parse time when combined with `exec` — `exec`
already produces a structured result natively, so the two would be
ambiguous. The error suggests the right replacement:

```
--exit is only valid with `run` (interactive) mode.
Use `zag -p <provider> run --exit '<hint>' "<prompt>"` instead of `exec`.
```

## Library use

`AgentBuilder::exit(hint: Option<&str>)` enables exit mode for
programmatic callers. Internally a typed `ExitHint` enum tracks
`Bare` vs `Provided(s)`; the `ExitConstraints` struct bundles hint +
json_mode + schema and is persisted on `SessionEntry`. See
[`zag-agent/src/exit_mode.rs`](../zag-agent/src/exit_mode.rs).

```rust
let agent = AgentBuilder::new()
    .provider("claude")
    .json()
    .exit(Some("the result"))
    .build()?;
agent.run(Some("compute 2+2")).await?;
```
