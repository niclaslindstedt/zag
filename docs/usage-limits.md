# Usage limits & auto-resume

zag detects when an upstream provider hits a usage / rate / weekly limit and,
when possible, **automatically resumes the session** once the limit resets.
This keeps unattended long-running batches (overnight orchestrations, agent
swarms, CI loops) from silently stalling at a rate-limit boundary.

It is a first-class zag feature. You don't need to opt in — it's on by default
for all four upstream providers.

## What gets detected

| Provider | Signal | Reset info |
|---|---|---|
| **Claude** | Assistant text `Claude AI usage limit reached\|<unix_epoch>` (also `weekly` / `global` variants). API-retry / result-error envelopes with `error: "rate_limit"` are also recognized. | **Unix epoch** — precise. |
| **Codex** | NDJSON `error` / `turn.failed` event containing `"You've hit your usage limit. … try again at <date>."` | **Local-TZ human date** — parsed via chrono. |
| **Copilot** | `events.jsonl` error event with `code` in {`rate_limited`, `user_weekly_rate_limited`, `user_global_rate_limited:*`}. Reset extracted from `"in N hours/minutes"` phrase. | **Relative phrase** — converted to absolute. |
| **Gemini** | Stderr / chat blob containing `[API Error: ... code:429 ... RESOURCE_EXHAUSTED ... RATE_LIMIT_EXCEEDED ...]`. Reset extracted from `retryDelay: "Ns"` when present. | **Usually none reliable** — falls back to the configured retry interval. |

When no reset time is available (e.g. Gemini Daily quotas), zag uses the
configurable **fallback wait** (default 1 hour). If the retry still hits the
limit, the cycle self-retriggers — eventually the window passes and the
session resumes.

## What gets emitted to the session log

Three new `LogEventKind` variants thread the lifecycle:

- `usage_limit_hit` — detection. Always emitted. Carries `provider`, `scope`
  (`session`/`weekly`/`global`/`daily`/`unknown`), `reset_at`,
  `scheduled_resume_at`, `fallback_used`, `incident_id`, and `raw` (the
  exact matched substring — invaluable when upstream formats drift).
- `usage_limit_resumed` — the timer fired, the resume message was delivered.
  Carries `incident_id` (joining back to the hit) and `attempt`.
- `usage_limit_resume_failed` — delivery failed. Carries `incident_id`,
  `error`, `attempt`.

You'll see all three in `zag listen <session>` and `zag events <session>`.

## Where auto-resume kicks in

Auto-resume works in **every long-running mode**:

| Invocation | How resume happens |
|---|---|
| `zag spawn --interactive --provider claude` | The relay holds a bidirectional `stream-json` session open. The timer fires inside the relay and live-injects the resume message via the FIFO — no respawn. |
| `zag exec ...` (foreground one-shot) | Wraps the agent invocation in a foreground auto-resume loop: when the agent exits with a `UsageLimitHit` in its output, the `exec` process sleeps until reset, then re-invokes via `--resume <provider_session_id>` with the resume message as the new prompt. The same `zag exec` process blocks across the reset window. Best for CI / scripted automation. |
| `zag spawn ...` (background, non-interactive) | `zag spawn` already invokes `zag exec` as its subprocess, so it picks up the same loop transparently. The background process now survives upstream rate-limit boundaries. |
| Historical replay (`zag listen` / backfill) | Detection only — no scheduling. Useful for after-the-fact diagnostics. |

In all cases the same configuration knobs apply (`[usage_limits]` in `zag.toml`).

## What gets resumed

When the timer fires, zag sends the configured **resume message** (default
literal `"Continue"`) into the session via the right channel for that provider:

- **Claude (interactive relay)** — live-injected into the existing interactive
  `stream-json` session via the relay's FIFO, so the running process picks it
  up like any human-typed message.
- **All providers in exec / background mode** — the upstream CLI has exited by
  the time the timer fires, so zag re-invokes the agent with
  `--resume <provider_session_id>` and the resume message as the new prompt.
  All four providers (Claude, Codex, Copilot, Gemini) implement
  `Agent::run_resume_with_prompt`, so auto-resume is fully end-to-end.

## Configuration

All knobs live under `[usage_limits]` in your `zag.toml`:

```toml
[usage_limits]
# Master switch. Detection still runs even if this is false (so the events
# still show in `zag listen`); only auto-resume scheduling is gated.
enabled = true

# Message injected when the timer fires.
resume_message = "Continue"

# Hard cap on any single wait. Past this, zag emits the UsageLimitHit but
# doesn't schedule. 24h.
max_wait_secs = 86400

# Used when the provider didn't tell us a reset time. 1h.
default_fallback_secs = 3600

# Added on top of the computed reset time to spread retries.
jitter_secs = 30

# Per-provider overrides.
[usage_limits.providers.copilot]
resume_message = "Please continue with the task."
fallback_secs = 1800
# User-supplied regex patterns OR'd into Copilot's default detection list.
# Useful when upstream changes a string and you want to patch it without
# upgrading zag.
extra_patterns = ['(?i)you have been rate-limited']

[usage_limits.providers.gemini]
enabled = false   # disable auto-resume for Gemini only
```

### Disabling auto-resume

Two levels:

```toml
[usage_limits]
enabled = false                    # all providers

[usage_limits.providers.copilot]
enabled = false                    # just one provider
```

Detection still runs in both cases — you'll see the `usage_limit_hit` event,
just without the timer.

## Updating detection patterns when upstream changes

Provider CLIs change wording occasionally. To minimize the blast radius:

1. **Default patterns** live in one file per provider:
   - `zag-agent/src/providers/claude/usage_limits.rs`
   - `zag-agent/src/providers/codex_usage_limits.rs`
   - `zag-agent/src/providers/copilot_usage_limits.rs`
   - `zag-agent/src/providers/gemini_usage_limits.rs`

   Each starts with a `DEFAULT_PATTERNS` constant listing the known regexes
   in order. Add a new pattern, drop a captured fixture under
   `zag-agent/tests/fixtures/usage_limits/<provider>/`, and ship a patch
   release.

2. **User-supplied patterns** via `extra_patterns` in `zag.toml` (above) let
   end users patch a drift overnight without rebuilding. Invalid regexes are
   logged at `WARN` and skipped.

3. **Inspect the raw match** via `zag events <session>` — every
   `usage_limit_hit` event carries the `raw` substring that fired detection.
   Copy that into a new fixture when filing a bug.

## How resume timing works

For every detection, `compute_resume_at` runs:

```
target = reset_at if known, else now + fallback_secs_for(provider)
scheduled_resume_at = clamp(target + jitter, max=now+max_wait_secs)
```

So:

- Known reset → wait exactly that long (+ jitter).
- Unknown reset → fall back to ~1h (+ jitter), and rely on self-retrigger.
- Pathological reset (years in the future) → capped to `max_wait_secs` so
  zag never pins a wait into next century.
- Past reset → clamped to `now + jitter`, giving upstream a beat to settle.

## Verification & manual testing

A relay-side smoke test for Claude:

```bash
# 1. Start an interactive Claude session
SID=$(zag spawn --interactive --provider claude --print-session-id)

# 2. Compute an epoch 10 seconds in the future
EPOCH=$(($(date +%s) + 10))

# 3. Inject a synthetic usage-limit message through the FIFO so the relay
#    parses it as if Claude had emitted it.
cat <<EOF > "$(zag spawn fifo-path $SID)"
{"type":"assistant","message":{"content":[{"type":"text","text":"Claude AI usage limit reached|$EPOCH"}]}}
EOF

# 4. Watch the log — expect UsageLimitHit, then ~10s later UsageLimitResumed
zag listen $SID --since 0
```

To rapid-test the fallback path (no reset time), drop `default_fallback_secs`
to a few seconds in your project's `zag.toml`:

```toml
[usage_limits]
default_fallback_secs = 5
```

## Limitations

1. **Process restart loses scheduled resumes.** If the `zag` process holding
   the timer (the relay, or the foreground `zag exec`) dies before the
   wake-up fires, the schedule is dropped. A
   `~/.zag/scheduled_resumes.json` persistence layer + a `zag resume --scan`
   rehydration command is on the roadmap. As a workaround for background
   sessions, `zag spawn` survives terminal disconnect on its own — only a
   reboot or explicit kill loses state.
2. **Gemini reset times.** Gemini's stderr 429 envelope rarely carries a
   reset timestamp. Auto-resume relies on the configurable fallback (default
   1h) until the upstream surfaces a usable `retryDelay`.
3. **Soft cap on attempts per exec invocation.** A single `zag exec`
   tolerates up to 12 consecutive resume cycles (so worst case ~12h with
   the default 1h fallback) before giving up. Background `zag spawn` runs
   inherit the same cap via the subprocess. The cap is a constant today;
   making it configurable is a small follow-up.
4. **Ollama is excluded.** No usage-limit concept on a self-hosted model.

## Why this matters

Without this feature, every overnight `/loop`-driven batch, every
multi-agent orchestration, and every long CI run silently dies the moment
the underlying account hits a 5-hour / weekly cap — and stays dead until a
human notices. With it, the run waits, resumes, and keeps going.

See also:

- `docs/events-and-logging.md` — full event-format reference
- `docs/configuration.md` — `zag.toml` reference
- `zag-agent/src/usage_limits.rs` — types
- `zag-orch/src/usage_resume.rs` — scheduler and strategies
