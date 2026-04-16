---
description: "Umbrella skill that runs all update skills (readme, docs, manpages, website) in parallel to bring the whole repo back into sync."
---

# Maintenance — Umbrella Update Skill

This skill runs all four documentation-sync skills to bring the repository's generated artifacts back into sync with the source code. Use it after a batch of changes, before a release, or whenever you suspect multiple docs surfaces are stale.

## Skills to Run

Run **all four** update skills in parallel using the Agent tool. Each skill is independent and can run concurrently:

| Skill | What it syncs |
|-------|---------------|
| `update-readme` | `README.md` with current commands, flags, providers, bindings |
| `update-docs` | `docs/*.md` with providers, config, events, orchestration |
| `update-manpages` | `zag-cli/man/*.md` with CLI commands and flags |
| `update-website` | `website/src/components/` with source-derived content |

## Execution

1. **Launch all four skills in parallel** using the Agent tool — one agent per skill. Each agent should invoke the corresponding skill via the `Skill` tool (e.g., `Skill("update-readme")`). Run them in the background so they execute concurrently.

2. **Collect results** from each agent as they complete. Track which skills:
   - Made changes (files were updated)
   - Found nothing to update (already in sync)
   - Encountered errors

3. **Report a summary** to the user listing what each skill did:
   ```
   Maintenance complete:
   - update-readme: updated Providers table, Flags table
   - update-docs: already in sync
   - update-manpages: updated config.md, session.md
   - update-website: updated Features component
   ```

4. **Do not commit.** The individual skills update `.last-updated` sentinel files and make content changes, but the commit step is left to the user (or a follow-up `/commit` invocation). This lets the user review all changes before committing.

## When to Run

- After landing multiple features or fixes
- Before a release to ensure all docs are current
- When the user runs `/maintenance`
- When multiple `.last-updated` files are behind HEAD
