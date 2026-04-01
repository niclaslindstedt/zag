# zag orchestration

Agentic orchestration patterns and how to implement them with zag.

## Synopsis

This guide documents the foundational multi-agent orchestration patterns
identified in recent research (Google ADK, Gulli's *Agentic Design Patterns*,
and Google Research's scaling principles for multi-agent coordination) and shows
how to implement each pattern using zag's built-in primitives.

## Key Primitives

Before diving into patterns, here are the zag commands that form the
orchestration toolkit:

    spawn       Launch a background agent session, return session ID
    wait        Block until session(s) complete
    status      Machine-readable session health check
    collect     Gather results from multiple sessions
    pipe        Chain session results into a new agent session
    input       Send a message to a running session
    broadcast   Send a message to all sessions (optionally filtered by tag)
    watch       Execute a command when a session event matches
    subscribe   Multiplexed event stream from all active sessions
    output      Extract final result text from a session
    cancel      Gracefully cancel a running session
    retry       Re-run a failed session with the same config
    log         Append custom events to a session log
    summary     Log-based session summaries and statistics
    events      Query structured events from session logs
    listen      Tail a session's log events in real-time
    env         Export session environment variables
    whoami      Session identity introspection (for agent self-discovery)

## Pattern 1: Sequential Pipeline

**What**: Agents execute in a fixed order. Each agent's output feeds into the
next agent's input, forming an assembly line.

**When to use**: Data processing pipelines, multi-stage document workflows,
parse-then-extract-then-summarize chains, or any workflow requiring a
deterministic, debuggable sequence of steps.

### Using `--depends-on` (DAG-style)

```sh
sid_a=$(zag spawn "parse the raw API logs into structured JSON")
sid_b=$(zag spawn --depends-on $sid_a --inject-context "extract error patterns from the parsed logs")
sid_c=$(zag spawn --depends-on $sid_b --inject-context "write a summary report of the error patterns")

zag wait $sid_c
zag output $sid_c
```

The `--depends-on` flag makes each stage wait for the previous one to complete
before starting. The `--inject-context` flag automatically feeds the dependency's
result into the new session's prompt.

### Using `pipe` (explicit chaining)

```sh
sid_a=$(zag spawn "analyze the authentication module")
zag wait $sid_a

sid_b=$(zag spawn "review the test coverage")
zag wait $sid_b

zag pipe $sid_a $sid_b -- "synthesize both analyses into an action plan"
```

### Using shell sequential execution

```sh
step1=$(zag -q exec "parse config files into a summary")
step2=$(zag -q exec "Given this summary: $step1 — identify security issues")
zag -q exec "Given these issues: $step2 — write fixes"
```

## Pattern 2: Parallel Fan-Out / Gather

**What**: Multiple independent agents run concurrently on different sub-tasks.
A final step aggregates their outputs.

**When to use**: Speed-critical scenarios like multi-perspective code review,
parallel analysis of independent modules, or batch processing where sub-tasks
don't depend on each other.

### Basic fan-out/gather

```sh
# Fan out: spawn parallel agents with a shared tag
sid1=$(zag spawn --tag review "review auth module for security issues")
sid2=$(zag spawn --tag review "review database queries for performance")
sid3=$(zag spawn --tag review "review API endpoints for correctness")

# Wait for all to complete
zag wait --tag review --timeout 10m

# Gather: collect all results
zag collect --tag review --json

# Or synthesize into a single report
zag pipe --tag review -- "create a unified code review report"
```

### Fan-out with mixed providers

```sh
sid1=$(zag spawn -p claude --tag analysis "deep architectural analysis")
sid2=$(zag spawn -p gemini --tag analysis "dependency graph analysis")
sid3=$(zag spawn -p codex --tag analysis "test coverage gaps")

zag wait --tag analysis --timeout 15m
zag pipe --tag analysis -- "merge these analyses into recommendations"
```

### Early exit (race pattern)

```sh
# Spawn multiple approaches, take whichever finishes first
sid1=$(zag spawn --tag race "solve with approach A")
sid2=$(zag spawn --tag race "solve with approach B")

# Exit as soon as one completes
zag wait --tag race --any
winner=$(zag collect --tag race --json | jq -r '.[] | select(.status=="completed") | .session_id' | head -1)
zag output $winner

# Cancel the remaining session(s)
zag cancel --tag race
```

## Pattern 3: Coordinator / Dispatcher

**What**: A central intelligent agent analyzes the user's request and routes
it to specialized agents best suited for each sub-task.

**When to use**: Complex multi-domain applications, customer service systems,
or scenarios requiring intelligent request classification before delegation.

### Orchestrator agent with spawn

```sh
# The orchestrator agent is given spawn capability via system prompt
zag exec --auto-approve --system-prompt '
You are an orchestrator. Analyze the user request and delegate to specialists.
Use these shell commands to delegate:
  zag spawn --name <specialist> --tag job "<specialist prompt>"
  zag wait --tag job
  zag collect --tag job --json
Return the collected results.
' "Refactor the auth system: update the API, fix the tests, and update the docs"
```

### Script-based dispatcher

```sh
#!/bin/bash
# Dispatcher: classify the task, then route to the right provider/model
classification=$(zag -q exec -p claude -m small --json \
  "Classify this task as 'simple', 'complex', or 'creative': $1")

case $(echo "$classification" | jq -r '.category') in
  simple)   zag exec -p claude -m small "$1" ;;
  complex)  zag exec -p claude -m large "$1" ;;
  creative) zag exec -p gemini -m large "$1" ;;
esac
```

## Pattern 4: Hierarchical Decomposition

**What**: A parent agent breaks a complex goal into sub-tasks and delegates
them to child agents. Children may further decompose into grandchildren.
The parent retains context and synthesizes results.

**When to use**: Tasks that exceed a single agent's context window, research-
heavy workflows, or problems requiring progressive refinement through levels
of abstraction.

### Nested spawn with parent tracking

```sh
# Parent spawns children — ZAG_SESSION_ID is inherited automatically
parent=$(zag spawn --name coordinator --tag project "plan the migration")
zag wait $parent

# Children reference parent
child1=$(zag spawn --depends-on $parent --inject-context \
  --name db-migration --tag project "execute database schema migration")
child2=$(zag spawn --depends-on $parent --inject-context \
  --name api-update --tag project "update API endpoints for new schema")

zag wait --tag project --timeout 20m

# Query the hierarchy
zag session list --parent $parent
zag ps list --children $parent

# Synthesize
zag pipe --tag project -- "summarize the migration status"
```

### Agent-driven decomposition

```sh
# Give the agent tools to spawn sub-agents
zag exec --auto-approve --system-prompt '
You are a lead engineer. Break the task into sub-tasks and delegate:
  zag spawn --name <name> --tag subtask "<prompt>"
Wait for all: zag wait --tag subtask --timeout 10m
Collect results: zag collect --tag subtask --json
Then synthesize a final answer.
' "Implement a complete user authentication system with OAuth, JWT, and MFA"
```

## Pattern 5: Generator & Critic

**What**: One agent generates output, another validates it against criteria.
If validation fails, feedback loops back to the generator. The cycle repeats
until the output passes.

**When to use**: Code generation requiring syntax/test validation, compliance-
heavy content creation, or any scenario needing iterative quality gates.

### Shell loop with retry

```sh
#!/bin/bash
MAX_RETRIES=3
attempt=0

while [ $attempt -lt $MAX_RETRIES ]; do
  # Generate
  gen_sid=$(zag spawn --name generator --tag gen-crit "write a REST API handler for /users")
  zag wait $gen_sid

  # Critique
  critic_result=$(zag -q exec --context $gen_sid --json \
    "Review this code. Output {\"pass\": true/false, \"feedback\": \"...\"}")

  if echo "$critic_result" | jq -e '.pass == true' > /dev/null 2>&1; then
    echo "Passed on attempt $((attempt + 1))"
    zag output $gen_sid
    break
  fi

  feedback=$(echo "$critic_result" | jq -r '.feedback')
  echo "Attempt $((attempt + 1)) failed: $feedback"

  # Re-generate with feedback
  gen_sid=$(zag spawn --name generator --tag gen-crit \
    "Revise: $feedback. Write a REST API handler for /users")
  zag wait $gen_sid

  attempt=$((attempt + 1))
done
```

### Watch-based critic loop

```sh
# Start generator
gen_sid=$(zag spawn --name generator "write unit tests for auth.rs")

# When generator completes, run the critic
zag watch $gen_sid --on session_ended --once -- \
  zag spawn --name critic --depends-on {session_id} --inject-context \
    "review these tests for correctness and coverage"
```

## Pattern 6: Iterative Refinement

**What**: An agent produces output that is progressively improved through
multiple cycles of critique and refinement until a quality threshold is
reached or a maximum iteration count is hit.

**When to use**: Creative writing, optimization problems, performance tuning,
or any task benefiting from progressive improvement rather than binary
pass/fail.

### Pipe chain refinement

```sh
# Draft
draft=$(zag spawn --name drafter "write a technical blog post about WebAssembly")
zag wait $draft

# Refine: each stage improves on the previous
r1=$(zag spawn --depends-on $draft --inject-context \
  --name refiner-1 "improve clarity and fix technical inaccuracies")
zag wait $r1

r2=$(zag spawn --depends-on $r1 --inject-context \
  --name refiner-2 "polish prose, add code examples, ensure consistent tone")
zag wait $r2

zag output $r2
```

### Automated refinement with quality scoring

```sh
#!/bin/bash
MAX_ITERS=5
current_sid=$(zag spawn "write a high-performance sorting algorithm in Rust")
zag wait $current_sid

for i in $(seq 1 $MAX_ITERS); do
  score=$(zag -q exec --context $current_sid --json \
    "Score this code 1-10 on: correctness, performance, readability. Output {\"score\": N, \"suggestions\": \"...\"}")

  numeric=$(echo "$score" | jq '.score')
  if [ "$numeric" -ge 8 ]; then
    echo "Quality threshold reached at iteration $i (score: $numeric)"
    break
  fi

  suggestions=$(echo "$score" | jq -r '.suggestions')
  current_sid=$(zag spawn --depends-on $current_sid --inject-context \
    "Improve based on this feedback: $suggestions")
  zag wait $current_sid
done

zag output $current_sid
```

## Pattern 7: Human-in-the-Loop

**What**: Agents handle routine processing autonomously but pause for human
authorization on high-stakes, irreversible, or ambiguous decisions.

**When to use**: Financial transactions, production deployments, sensitive
data handling, or anywhere requiring human oversight and accountability.

### Status polling with manual intervention

```sh
sid=$(zag spawn --name deployer "prepare the production deployment")

# Poll until ready for review
while true; do
  state=$(zag status $sid --json | jq -r '.status')
  case $state in
    completed)
      echo "Agent completed. Review the output:"
      zag output $sid
      read -p "Approve deployment? [y/n] " answer
      if [ "$answer" = "y" ]; then
        zag exec --context $sid "execute the deployment plan"
      fi
      break
      ;;
    failed|dead)
      echo "Agent failed:"
      zag summary $sid
      break
      ;;
    *)
      sleep 5
      ;;
  esac
done
```

### Interactive input injection

```sh
# Start an agent session
sid=$(zag spawn --name analyst "analyze security vulnerabilities in the codebase")

# Monitor progress
zag listen $sid &

# Inject guidance at any time
zag input --session $sid "focus on SQL injection vectors specifically"

# When done, review and approve next steps
zag wait $sid
zag output $sid
read -p "Proceed with fixes? [y/n] " answer
if [ "$answer" = "y" ]; then
  zag exec --context $sid "implement the security fixes you identified"
fi
```

## Pattern 8: Inter-Agent Communication (A2A)

**What**: Agents communicate directly with each other, enabling collaborative
problem-solving and real-time information sharing.

**When to use**: Peer review, collaborative debugging, multi-agent negotiation,
or any scenario where agents need to exchange information without going through
a central orchestrator.

### Named agent messaging

```sh
# Start two named agents
sid1=$(zag spawn --name frontend "implement the React login form")
sid2=$(zag spawn --name backend "implement the /auth API endpoint")

# Frontend asks backend about the API contract
zag input --name backend "What will the /auth endpoint request/response format be?"

# Backend replies to frontend
zag input --name frontend "The /auth endpoint accepts POST {email, password} and returns {token, expires_at}"
```

### Broadcast coordination

```sh
# Start a team of agents
zag spawn --name lead --tag team "coordinate the feature implementation"
zag spawn --name dev-1 --tag team "implement the data model"
zag spawn --name dev-2 --tag team "implement the API layer"
zag spawn --name dev-3 --tag team "implement the UI components"

# Lead broadcasts a status request
zag broadcast --tag team "report your current progress and any blockers"

# Each agent receives the message wrapped in an <agent-message> envelope
# with sender identity and reply instructions
```

### Agent-message envelope format

When `zag input` or `zag broadcast` is called from within a zag session
(detected via `ZAG_SESSION_ID`), messages are automatically wrapped:

```xml
<agent-message>
<from session="abc123" name="frontend" provider="claude" model="opus"/>
<reply-with>zag input --name frontend "your reply here"</reply-with>
<body>
What will the /auth endpoint format be?
</body>
</agent-message>
```

Use `--raw` to send without the envelope.

## Pattern 9: Composite Patterns

Real-world applications combine multiple patterns. Here are common compositions.

### Fan-out with generator-critic per branch

```sh
#!/bin/bash
# Fan out three analysis streams, each with its own quality gate
for module in auth payments notifications; do
  # Generator
  gen=$(zag spawn --tag "$module" --name "${module}-gen" "analyze $module for bugs")
  # Critic depends on generator
  zag spawn --tag "$module" --name "${module}-critic" \
    --depends-on $gen --inject-context \
    "verify the analysis is thorough and accurate"
done

# Wait for all critics
zag wait --tag auth --tag payments --tag notifications --timeout 15m

# Synthesize
zag pipe --tag auth --tag payments --tag notifications \
  -- "create a unified bug report across all modules"
```

### Hierarchical fan-out with human gate

```sh
# Phase 1: Plan (single agent)
plan=$(zag spawn --name planner "create a migration plan for the database")
zag wait $plan
zag output $plan

read -p "Approve migration plan? [y/n] " answer
[ "$answer" != "y" ] && exit 1

# Phase 2: Execute (parallel agents, gated by human approval)
m1=$(zag spawn --depends-on $plan --inject-context --tag migrate "migrate users table")
m2=$(zag spawn --depends-on $plan --inject-context --tag migrate "migrate orders table")
m3=$(zag spawn --depends-on $plan --inject-context --tag migrate "migrate products table")

zag wait --tag migrate --timeout 20m

# Phase 3: Verify (sequential)
zag pipe --tag migrate -- "verify all migrations completed successfully and data is consistent"
```

### Event-driven reactive pipeline

```sh
# Start a long-running analysis
sid=$(zag spawn --name analyzer "comprehensive security audit of the entire codebase")

# React to specific events
zag watch $sid --on tool_call --filter "tool=bash" -- \
  echo "Security audit ran shell command at {ts}"

# When analysis completes, automatically start the fix phase
zag watch $sid --on session_ended --once -- \
  zag spawn --depends-on {session_id} --inject-context \
    --name fixer "implement fixes for the issues found"
```

## Common Use Cases

### CI/CD: Automated code review pipeline

```sh
#!/bin/bash
# Stage 1: Parallel review from multiple perspectives
sec=$(zag spawn --tag pr-review "review this PR for security vulnerabilities")
perf=$(zag spawn --tag pr-review "review this PR for performance regressions")
style=$(zag spawn --tag pr-review "review this PR for code style and best practices")

zag wait --tag pr-review --timeout 5m

# Stage 2: Synthesize into a single review
review=$(zag -q pipe --tag pr-review -- "combine into a single PR review with severity ratings")
echo "$review"
```

### Research: Multi-source analysis

```sh
# Analyze from multiple angles simultaneously
a=$(zag spawn --tag research -p claude "analyze the competitive landscape")
b=$(zag spawn --tag research -p gemini "analyze market trends and forecasts")
c=$(zag spawn --tag research -p claude -m large "analyze technical feasibility")

zag wait --tag research --timeout 15m
zag pipe --tag research -m large -- "synthesize into an executive briefing"
```

### Batch processing: Process files in parallel

```sh
#!/bin/bash
for file in src/*.rs; do
  zag spawn --tag batch --name "$(basename $file)" \
    "analyze $file for potential improvements"
done

zag wait --tag batch --timeout 10m
zag collect --tag batch --json > analysis.json
```

### Self-healing: Retry with escalation

```sh
#!/bin/bash
# Try with small model first
sid=$(zag spawn -m small "fix the failing test in auth_test.rs")
zag wait $sid --timeout 3m

status=$(zag status $sid --json | jq -r '.status')
if [ "$status" = "failed" ]; then
  echo "Small model failed, escalating to large model"
  zag retry $sid --model large
fi
```

### DAG workflow: Multi-stage deployment

```sh
# Stage 1: Build (no dependencies)
build=$(zag spawn --name build --tag deploy "build the release artifacts")

# Stage 2: Test (depends on build)
test=$(zag spawn --name test --tag deploy --depends-on $build --inject-context "run the full test suite")

# Stage 3: Staging (depends on test)
staging=$(zag spawn --name staging --tag deploy --depends-on $test --inject-context "deploy to staging environment")

# Stage 4: Smoke test (depends on staging)
smoke=$(zag spawn --name smoke --tag deploy --depends-on $staging --inject-context "run smoke tests against staging")

# Wait for the full pipeline
zag wait $smoke --timeout 30m
zag summary --tag deploy --stats
```

## Topology Selection Guide

Based on Google Research's scaling principles, choose the right topology:

| Topology | When to use | zag implementation |
|----------|------------|-------------------|
| **Single agent** | Simple tasks, small context | `zag exec "..."` |
| **Independent** | Embarrassingly parallel sub-tasks | `spawn` (no deps) + `wait` + `collect` |
| **Centralized** | Tasks needing coordination, error-sensitive | Orchestrator `exec` that `spawn`s workers |
| **Decentralized** | Peer-to-peer collaboration, exploration | Named agents with `input`/`broadcast` |
| **Hybrid** | Complex projects mixing the above | Combine patterns as needed |

**Key insight from Google Research**: Centralized coordination reduces error
propagation (~4.4x) compared to decentralized (~17x), but decentralized
topologies perform better for web navigation and exploration tasks. Choose
based on your task's error tolerance and parallelizability.

## Monitoring Orchestrations

### Real-time monitoring

```sh
# Watch all sessions in a batch
zag subscribe --tag batch --json

# Follow a specific session
zag listen $sid --rich-text --timestamps

# Get aggregate stats
zag summary --tag batch --stats --json
```

### Programmatic health checks

```sh
# Check all sessions in a batch
for sid in $(zag session list --tag batch --json | jq -r '.[].session_id'); do
  status=$(zag status $sid --json | jq -r '.status')
  echo "$sid: $status"
done
```

### Event-driven alerts

```sh
# Alert on any failure in the batch
zag watch --tag batch --on session_ended --filter "success=false" -- \
  echo "ALERT: Agent {session_id} failed at {ts}"
```

## See Also

    zag man spawn        Launch background sessions
    zag man wait         Block until sessions complete
    zag man collect      Gather multi-session results
    zag man pipe         Chain session results
    zag man input        Send messages to sessions
    zag man broadcast    Send to all sessions
    zag man watch        Event-driven command execution
    zag man subscribe    Multiplexed event stream
    zag man status       Session health check
    zag man cancel       Cancel running sessions
    zag man retry        Re-run failed sessions
    zag man summary      Session summaries and stats
    zag man output       Extract session results
    zag man events       Query structured events
    zag man listen       Real-time log tailing
    zag man log          Append custom events
    zag man env          Session environment variables
    zag man whoami       Session identity introspection
