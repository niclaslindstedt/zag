---
name: maintenance
description: "Use when you want to bring every drift-prone artifact in the repo back into sync. Dispatches to all individual update-* skills in the correct order, aggregates their results, and leaves a single combined PR ready to review."
---

# Maintenance

This is the umbrella skill for zag. It does no rewriting itself — it decides which sync skills are stale, runs each one, and reports a combined summary. Use it when you do not know which specific artifact is out of date, or when several have likely drifted at once (for example, after a large merge).

## When to run

- After a big merge from the default branch when you are not sure which surfaces moved.
- On a cadence (weekly / before a release) as a "drift sweep".
- When CI flags a staleness check but it is unclear which skill to invoke.

Do **not** use this skill for a targeted fix — if you know exactly which artifact is stale, call the corresponding `update-*` skill directly.

## Registry

The registry is the single source of truth for which sync skills exist in this repo. Every `update-*` directory under `.claude/skills/` that is part of the drift sweep must appear here exactly once. Add rows whenever you create a new sync skill.

| Skill | Fixes | Run order |
|---|---|---|
| `update-manpages` | `zag-agent/man/*.md` vs. clap CLI definitions                               | 1 — manpages mirror the CLI parser and must settle first |
| `update-docs`     | `docs/*.md` vs. providers, config, events, and orchestration source-of-truth | 2 |
| `update-readme`   | `README.md` vs. current public surface (commands, flags, providers, bindings)| 3 |
| `update-website`  | `website/src/` vs. commands, providers, version, and config defaults         | 4 |

Run order matters: upstream fixes must land before downstream skills read them. Manpages settle first; `docs/` references those manpages; the README summarizes everything above; the website is rendered from all three.

### Out of scope for this sweep

The per-agent sync skills (`update-claude`, `update-codex`, `update-copilot`, `update-gemini`, `update-ollama`) are **not** part of this umbrella. They track upstream CLI / config changes from external agent tools and are invoked individually when the relevant upstream ships. Do not include them in the registry above.

## Discovery process

For each skill in the registry, decide whether it needs to run:

1. Read the skill's `.last-updated` file:

   ```sh
   BASELINE=$(cat .claude/skills/<skill>/.last-updated)
   ```

   An empty or missing file means "never run" — schedule it.

2. Diff the watched paths for that skill against the baseline:

   ```sh
   git diff --name-only "$BASELINE"..HEAD
   ```

   If any file in the skill's mapping table appears in the diff, schedule the skill.

3. Build the list of skills to run, preserving the run order from the registry.

## Execution

For each scheduled skill, in order:

1. Load `.claude/skills/<skill>/SKILL.md`.
2. Follow its discovery process, mapping table, and update checklist exactly.
3. Verify the skill's own verification section passes.
4. Record the commit hash the skill wrote to its `.last-updated`.

Between skills, do **not** commit — aggregate all edits into a single working tree so the final commit covers the whole sync sweep. Do **not** run the skills in parallel: downstream skills depend on upstream skills having already rewritten the files they read.

## Update checklist

- [ ] Read every skill's `.last-updated` and build the schedule
- [ ] Run each scheduled skill in registry order
- [ ] After all skills finish, run:
    - [ ] `make fmt`
    - [ ] `make clippy`
    - [ ] `make test`
- [ ] Stage every touched file (including each updated `.last-updated`)
- [ ] Commit with a conventional-commit message describing the sweep
- [ ] Update `.claude/skills/maintenance/.last-updated`:

      git rev-parse HEAD > .claude/skills/maintenance/.last-updated

- [ ] Hand off to the `commit` skill to push and open / update the PR

## Verification

1. Every scheduled skill's verification section must pass.
2. `make clippy` and `make test` must pass.
3. The final diff should touch only documentation files, skill `.last-updated` files, and (rarely) small code adjustments that the skills flagged.
4. Every skill that ran must have its `.last-updated` rewritten with the same commit hash.

## Skill self-improvement

After every run, update this file:

1. **Add new sync skills to the registry.** Every new `update-*` skill that is part of the drift sweep must appear here, in alphabetical order, with a clear run-order slot. Per-agent `update-*` skills (claude, codex, copilot, gemini, ollama) stay out of the registry by design — note any new ones in the "Out of scope" section instead.
2. **Adjust run order** if you discovered a hidden dependency (e.g. skill A reads files that skill B rewrites).
3. **Record drift signals.** If a change should have triggered a skill but did not appear in any skill's mapping table, extend that skill's mapping table — not this one.
4. **Commit the skill edits** together with the drift sweep so the orchestration knowledge compounds.
