---
description: "Use when the website may be stale. Discovers commits since the last website update, identifies what changed, and updates the appropriate React components in website/src/components/ to reflect current features, providers, commands, and bindings."
---

# Updating the Website

The website (`website/src/`) is a React + Vite + Tailwind landing page with 9 components. All content is hardcoded in the component files — there is no CMS or database. It gets stale when new features, providers, commands, or bindings are added without updating the corresponding components.

## Tracking Mechanism

The file `.claude/skills/update-website/.last-updated` contains the git commit hash from the last time the website was comprehensively updated. Use this as the baseline for discovering what changed.

## Discovery Process

1. Read the baseline commit hash:
   ```sh
   BASELINE=$(cat .claude/skills/update-website/.last-updated)
   ```

2. List all commits since the baseline:
   ```sh
   git log --oneline "$BASELINE"..HEAD
   ```

3. Check what files changed:
   ```sh
   git diff --name-only "$BASELINE"..HEAD
   ```

4. Categorize the changes using the component mapping below.

5. For each affected component, read both the component source and the corresponding source-of-truth files to identify discrepancies.

## Component Mapping

| Changed files / commit scope | Website component to update | What to change |
|------------------------------|----------------------------|----------------|
| `zag-agent/src/providers/*/mod.rs` | `Providers.tsx` | Provider table: models, size aliases, feature badges |
| `zag-agent/src/providers/*/models.rs` | `Providers.tsx` | Model names in size mapping columns |
| `zag-cli/src/commands/` | `Orchestration.tsx` | Command reference grid (currently 14 commands) |
| `zag-orch/src/` | `Orchestration.tsx` | Orchestration patterns, code examples |
| New major features | `Features.tsx` | Feature cards (currently 9 cards with icons) |
| `bindings/` (new language) | `Bindings.tsx` | Add new language tab with install + code example |
| Binding API changes | `Bindings.tsx` | Update code examples for affected languages |
| Install method changes | `GettingStarted.tsx` | Installation methods (currently 3) or prerequisites (currently 5 CLIs) |
| New provider added | `GettingStarted.tsx` | Add to agent CLI prerequisites list |
| CLI usage changes | `CodeExamples.tsx` | Update tabbed code examples (4 tabs: Basic, JSON, Sessions, Isolation) |
| Session feature changes | `CodeExamples.tsx` | Sessions tab code examples |
| Version bumps | `Hero.tsx` | Version badge (note: already handled by `scripts/update-versions.sh`) |
| Navigation structure changes | `Navbar.tsx` | Section links |
| New documentation links | `Footer.tsx` | Footer link columns |

## Implementation Files

### Primary — Website components

| Component | File | Content |
|-----------|------|---------|
| Hero | `website/src/components/Hero.tsx` | Version badge, headline, terminal mockup, CTA buttons |
| Features | `website/src/components/Features.tsx` | 9-card feature grid with icons and descriptions |
| Providers | `website/src/components/Providers.tsx` | Provider comparison table with model mappings |
| Orchestration | `website/src/components/Orchestration.tsx` | 3 code patterns + 14-command reference grid |
| CodeExamples | `website/src/components/CodeExamples.tsx` | 4-tab CLI examples |
| Bindings | `website/src/components/Bindings.tsx` | Language SDK showcase (TypeScript, Python, C#) |
| GettingStarted | `website/src/components/GettingStarted.tsx` | 3 install methods + 5 agent CLI prerequisites |
| Navbar | `website/src/components/Navbar.tsx` | Fixed header with section links |
| Footer | `website/src/components/Footer.tsx` | Links to GitHub, Docs, crates.io, License |

### Secondary — Sources of truth (read-only)

| Source | What it tells you |
|--------|-------------------|
| `README.md` | Canonical documentation (update README first via `update-readme` skill) |
| `zag-cli/src/cli.rs` | All CLI flags and commands |
| `zag-agent/src/providers/*/mod.rs` | Provider models, defaults, size aliases |
| `zag-agent/src/builder.rs` | Builder API for programmatic examples |
| `bindings/*/README.md` | Language binding examples |

### Styling

- **Global styles**: `website/src/App.css` — Tailwind theme variables, color scheme
- **Provider colors**: Defined as CSS variables (claude: tan, codex: green, gemini: blue, copilot: pink, ollama: hot pink)
- **Font stack**: Inter (body), JetBrains Mono / Fira Code (code blocks)

## Implementation Patterns

### Adding a new provider to Providers.tsx

Find the `providers` array and add a new entry:

```tsx
{
  name: "NewProvider",
  color: "provider-color",
  default: "default-model",
  small: "small-model",
  medium: "medium-model",
  large: "large-model",
  features: ["feature1", "feature2"]
}
```

Also add a CSS color variable in `App.css` if needed, and update `GettingStarted.tsx` prerequisites.

### Adding a command to Orchestration.tsx

Find the `commands` array and add:

```tsx
{ name: "command-name", description: "What it does" }
```

Commands are displayed in a grid. Keep descriptions concise (under 10 words).

### Adding a feature card to Features.tsx

Find the `features` array and add:

```tsx
{
  icon: <SvgIcon />,
  title: "Feature Name",
  description: "Brief description of the feature"
}
```

The grid is responsive. Currently 9 cards (3x3). Adding cards may require layout adjustment.

### Adding a new language binding to Bindings.tsx

Find the `bindings` array and add:

```tsx
{
  language: "Language",
  install: "install command",
  code: `code example`
}
```

Currently shows TypeScript, Python, C#. Consider adding Swift, Java, Kotlin if they are mature enough.

### Updating code examples in CodeExamples.tsx

Find the `tabs` array. Each tab has a `name` and `code` field. Update the code strings to match current CLI syntax.

### Updating terminal mockup in Hero.tsx

The Hero component has a terminal mockup showing 3 example commands. Update these to showcase the most compelling current features.

## Update Checklist

- [ ] Read baseline from `.last-updated` and run `git log` to identify changes
- [ ] Read all affected component files and source-of-truth files
- [ ] Update `Providers.tsx` if providers/models changed
- [ ] Update `Orchestration.tsx` if commands or patterns changed
- [ ] Update `Features.tsx` if major new features were added
- [ ] Update `CodeExamples.tsx` if CLI syntax or capabilities changed
- [ ] Update `Bindings.tsx` if bindings were added or APIs changed
- [ ] Update `GettingStarted.tsx` if install methods or prerequisites changed
- [ ] Update `Hero.tsx` if headline, tagline, or terminal examples should change
- [ ] Update `Navbar.tsx` if new major sections were added
- [ ] Update `Footer.tsx` if new documentation links are needed
- [ ] Verify the website builds cleanly:
  ```sh
  make website
  ```
- [ ] Visually verify with the dev server:
  ```sh
  make website-dev
  ```
- [ ] Update `.claude/skills/update-website/.last-updated` with current HEAD commit hash:
  ```sh
  git rev-parse HEAD > .claude/skills/update-website/.last-updated
  ```

## Verification

```sh
# Build must succeed
make website

# Optionally start dev server to visually verify
make website-dev
# Then open http://localhost:5173/zag/ and check each section
```

1. All provider data matches current source code
2. Command list is complete and accurate
3. Code examples use correct current syntax
4. No broken layouts from added/removed content
5. Links in Footer and Navbar are valid

## Skill Self-Improvement

After completing an update session, improve this skill file:

1. **Update component descriptions**: If components were refactored or new ones added, update the component table.
2. **Add new patterns**: If you discovered a recurring website update pattern, document it.
3. **Update array names**: If data arrays in components were renamed, update the implementation patterns.
4. **Record layout constraints**: Note any grid/layout issues encountered (e.g., max cards before wrapping).
5. **Commit the skill update** along with the website update so improvements are preserved.
