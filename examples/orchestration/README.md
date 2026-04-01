# Orchestration Examples

Shell scripts demonstrating multi-agent orchestration patterns using zag's CLI primitives. Each script is self-contained and showcases one or more patterns from the [orchestration guide](../../man/orchestration.md).

## Prerequisites

- [zag](../../) installed and on your PATH
- At least one provider configured (e.g., `ANTHROPIC_API_KEY` for Claude)
- [jq](https://jqlang.github.io/jq/) for JSON parsing (scripts 02, 03, 04)

## Quick Start

```bash
cd examples/orchestration

# Run the simplest example — a three-stage sequential pipeline
./01-sequential-pipeline.sh

# Or specify a target
./01-sequential-pipeline.sh src/main.rs

# Use a different provider
ZAG_PROVIDER=gemini ./01-sequential-pipeline.sh
```

## Scripts

| Script | Pattern(s) | Description | Key Primitives |
|--------|-----------|-------------|----------------|
| [01-sequential-pipeline.sh](01-sequential-pipeline.sh) | Sequential Pipeline | Three-stage code analysis: structure → issues → action plan | `spawn --depends-on --inject-context`, `wait`, `output`, `status`, `summary`, `pipe` |
| [02-parallel-fan-out.sh](02-parallel-fan-out.sh) | Fan-Out / Gather, Race | Multi-perspective code review (security, perf, style) with parallel agents | `spawn --tag`, `wait --tag`, `wait --any`, `collect --json`, `pipe --tag`, `cancel --tag`, `summary` |
| [03-generator-critic.sh](03-generator-critic.sh) | Generator/Critic, Iterative Refinement | Generate code, score it, refine until quality threshold is met | `spawn`, `wait`, `exec --context --json`, `output`, `summary` |
| [04-coordinator-dispatch.sh](04-coordinator-dispatch.sh) | Coordinator / Dispatcher | Classify task complexity, route to appropriate model size | `exec --json`, `exec -m small/large`, `spawn`, `wait`, `output`, `summary` |
| [05-hierarchical-decomposition.sh](05-hierarchical-decomposition.sh) | Hierarchical, Human-in-the-Loop | Plan → human approval → parallel child execution → verification | `spawn --depends-on --inject-context`, `wait --tag`, `output`, `pipe --tag`, `summary` |
| [06-event-driven-composite.sh](06-event-driven-composite.sh) | A2A Communication, Composite | Frontend + backend agents collaborate with messaging and event watching | `spawn --name`, `input --name`, `broadcast --tag`, `watch --on`, `wait --tag`, `pipe --tag`, `summary` |
| [07-decision-arena.sh](07-decision-arena.sh) | Adversarial Debate, Fan-Out, A2A | Advocate vs skeptic debate with rebuttals and judge verdict; optional mixed providers | `spawn --name --tag`, `wait`, `output`, `input --name`, `pipe --tag`, `summary` |

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `ZAG_PROVIDER` | *(system default)* | Provider to use (claude, codex, gemini, copilot, ollama) |
| `ZAG_MODEL` | *(provider default)* | Model name or size alias (small, medium, large) |
| `NO_COLOR` | *(unset)* | Set to any value to disable colored output |

```bash
# Examples
ZAG_PROVIDER=gemini ./02-parallel-fan-out.sh
ZAG_PROVIDER=claude ZAG_MODEL=large ./03-generator-critic.sh
NO_COLOR=1 ./01-sequential-pipeline.sh > output.txt
```

## How It Works

Each script sources `lib.sh` for shared helpers (color output, provider flag resolution, cleanup traps). Every spawned session is tagged with a unique per-run tag (`orch-<name>-<PID>`), so concurrent runs don't interfere with each other. On exit (including Ctrl-C), the cleanup trap cancels all sessions with that tag.

### Pattern Overview

```
Sequential Pipeline (01)        Fan-Out / Gather (02)
┌───┐   ┌───┐   ┌───┐          ┌───┐
│ A ├──►│ B ├──►│ C │          │ A │──┐
└───┘   └───┘   └───┘          └───┘  │  ┌──────────┐
                                ┌───┐  ├─►│ Synthesize│
                                │ B │──┤  └──────────┘
                                └───┘  │
                                ┌───┐  │
                                │ C │──┘
                                └───┘

Generator / Critic (03)         Coordinator (04)
┌─────────┐   ┌────────┐       ┌──────────┐
│ Generate ├──►│ Critic │       │ Classify │
└────┬────┘   └───┬────┘       └────┬─────┘
     │            │ score<8         │
     └────────────┘            ┌────┼────┐
     feedback loop             ▼    ▼    ▼
                              sm   med   lg

Hierarchical (05)               Composite / A2A (06)
┌────────┐                      ┌──────────┐  ┌──────────┐
│ Planner├──[human gate]        │ Frontend │◄►│ Backend  │
└───┬────┘                      └────┬─────┘  └────┬─────┘
    │                                │              │
┌───┼───┐                       ┌────┴──────────────┴────┐
▼   ▼   ▼                       │    Integration Review  │
A   B   C ──► Verify            └────────────────────────┘

Decision Arena (07)
┌──────────┐   ┌──────────┐
│ Advocate │   │ Skeptic  │
│ (FOR)    │   │ (AGAINST)│
└────┬─────┘   └────┬─────┘
     │   cross-poll  │
     │◄─────────────►│
     │   rebuttals   │
     └───────┬───────┘
             ▼
        ┌─────────┐
        │  Judge  │
        │(verdict)│
        └─────────┘
```

## Tips

**Watch progress in real-time** — open a second terminal:
```bash
# Follow a specific session
zag listen <session-id>

# Watch all sessions with a tag
zag subscribe --tag orch-fanout-12345
```

**Debug issues** — check spawn logs:
```bash
# Spawn logs are stored here
ls ~/.zag/logs/spawn/

# Session status
zag status <session-id>
zag status <session-id> --json
```

**Clean up old sessions**:
```bash
zag gc              # Dry run — show what would be cleaned
zag gc --force      # Actually delete
```

**Cancel runaway sessions**:
```bash
zag cancel --tag orch-pipeline-12345
zag ps list --running     # See all running processes
zag ps kill <id>          # Force-kill a specific process
```

## Further Reading

- `zag man orchestration` — full orchestration patterns guide
- `zag man spawn` — background session launching
- `zag man wait` — blocking until sessions complete
- `zag man pipe` — chaining session results
- `zag man collect` — gathering multi-session results
- `zag man watch` — event-driven command execution
- `zag man subscribe` — multiplexed event streams

### Other examples

- [CV Review](../cv-review/) — Programmatic `zag-lib` API usage with parallel agents
- [React Claude Interface](../react-claude-interface/) — Web UI with streaming NDJSON events
