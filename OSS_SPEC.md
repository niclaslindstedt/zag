---
title: Open Source Project Bootstrap Specification
description: A prescriptive, language-agnostic specification for bootstrapping a new open source project with the licensing, documentation, automation, governance, and release plumbing that users and contributors expect from a well-run OSS codebase.
version: 2.3.0
---

# Open Source Project Bootstrap Specification

This document is a prescriptive specification for bootstrapping a new open
source project from an empty repository. Following it gives a project the
foundational infrastructure — licensing, documentation, automation,
governance, release plumbing — that users and contributors expect from a
well-run OSS codebase.

The spec is deliberately opinionated. Where it says "must", the item is
non-negotiable for any project that claims to follow this bootstrap. Where
it says "should", the item is the recommended default but may be omitted
for small projects. Where it says "may", the item is optional.

The spec is language- and domain-agnostic. It applies equally to a CLI
tool, a library, a web service, a browser extension, or a data pipeline.
Replace `<project>` throughout with the actual project name.

---

## 1. Repository layout

A new repository must contain the following files at its root before the
first public commit:

```
<repo>/
├── LICENSE                  # SPDX-identified license text (see §2)
├── README.md                # Project overview (see §3)
├── CONTRIBUTING.md          # How to contribute (see §4)
├── CODE_OF_CONDUCT.md       # Community standards (see §5)
├── SECURITY.md              # Vulnerability reporting (see §6)
├── CHANGELOG.md             # Release notes (see §8)
├── AGENTS.md                # Guidance for AI coding agents (see §7)
├── .gitignore               # Language-appropriate ignores
├── .editorconfig            # Cross-editor formatting baseline
├── .github/
│   ├── workflows/           # CI/CD pipelines (see §10)
│   ├── ISSUE_TEMPLATE/      # Bug report, feature request (see §15)
│   ├── PULL_REQUEST_TEMPLATE.md
│   ├── dependabot.yml       # Dependency updates (see §14)
│   └── CODEOWNERS           # Review routing
├── docs/                    # Topic-specific documentation (see §11.1)
├── man/                     # CLI manpages, <cli>-command (see §12.3)
├── examples/                # Runnable example projects (see §13)
├── website/                 # Showcase and hosted docs site (see §11.2)
├── prompts/                 # Versioned LLM prompts (see §13.5)
├── scripts/                 # Automation scripts (release, lint helpers)
└── Makefile                 # Standard developer entry points (see §9)
```

`AGENTS.md` is the canonical file for AI coding agent guidance. Tool-
specific files (`CLAUDE.md`, `.github/copilot-instructions.md`,
`.cursorrules`, `.windsurfrules`, `GEMINI.md`, etc.) must exist as
**symlinks** to `AGENTS.md` rather than as separate copies — see §7.

`man/` is required for any project whose primary deliverable is a CLI
binary; see §12 for the full set of CLI-specific requirements. The
`docs/`, `examples/`, and `website/` directories are required for any
project that has a public audience. Only a pure internal library with
no users may omit them.

## 2. License

Every project must include a `LICENSE` file at the repository root. The
file must contain the full, unmodified license text, the copyright year,
and the copyright holder.

Recommended defaults:

- **MIT** for maximum permissiveness and minimal friction.
- **Apache-2.0** when explicit patent grants matter.
- **MPL-2.0** for file-level copyleft without infecting dependent code.
- **AGPL-3.0** only when the project is a hosted service whose source must
  remain open to users who interact with it over a network.

Avoid GPL-family licenses for libraries intended to be embedded by others.
Every source file's header comment (where the language conventionally has
one) should reference the license by SPDX identifier:
`SPDX-License-Identifier: MIT`.

## 3. README.md

The `README.md` is the project's front page. It must answer, in this
order:

1. **What the project is** — a one-sentence description directly under
   the title. No marketing fluff.
2. **Why it exists** — a short "Why?" section with 3–5 bullet points
   stating the concrete value the project provides.
3. **Prerequisites** — runtime and development dependencies with version
   bounds.
4. **Install** — at least one install command that works end-to-end. If
   the project publishes to multiple registries, list each one.
5. **Quick start** — a minimal working example that a reader can copy,
   paste, and run successfully.
6. **Usage / commands / API** — the reference surface of the project.
7. **Configuration** — if applicable, with file paths and key names.
8. **Examples** — a pointer to `examples/` with brief descriptions.
9. **Troubleshooting** — common failure modes and their fixes.
10. **Documentation** — links to `docs/` pages and any hosted docs.
11. **Contributing** — a pointer to `CONTRIBUTING.md`.
12. **License** — a pointer to `LICENSE`.

The top of the README should carry a row of status badges:

- CI status (build / test / lint).
- Release / latest version for each publishing target.
- License.
- Optional: code coverage, security scanning, downloads.

Badges must be clickable and point at the corresponding CI run, release
page, or registry listing.

## 4. CONTRIBUTING.md

`CONTRIBUTING.md` is the contract between the project and external
contributors. It must cover:

- **Prerequisites** — exact tooling versions required to build and test.
- **Getting the source** — `git clone` command and initial setup.
- **Build / test / lint** — the canonical commands (see §9 Makefile).
- **Development workflow** — fork, branch, commit, PR.
- **Commit message conventions** — conventional commits (see §8).
- **Branch naming** — e.g. `feat/<slug>`, `fix/<slug>`.
- **Testing expectations** — where tests live, how to add them, coverage
  expectations if any.
- **Documentation expectations** — which docs must be updated alongside
  code changes (README, man pages, `docs/` topics, agent guidance files).
- **Pull request process** — review requirements, merge strategy, and who
  can merge.
- **Code of conduct reference** — a link to `CODE_OF_CONDUCT.md`.
- **Security reporting reference** — a link to `SECURITY.md`.

## 5. CODE_OF_CONDUCT.md

Projects must adopt a code of conduct. The recommended baseline is the
[Contributor Covenant](https://www.contributor-covenant.org/) v2.1 or
later.

`CODE_OF_CONDUCT.md` **must link out** to the canonical external text of
the chosen code (e.g. the Contributor Covenant v2.1 URL) rather than
embedding the full document verbatim. This is a deliberate constraint:
AI coding agents — which bootstrap and maintain many OSS_SPEC.md
projects — are commonly blocked by content filters from reproducing
sections of a code of conduct verbatim (harassment examples, protected
characteristics, etc.), so a link-first policy is the only form that can
be reliably generated and updated end-to-end by an agent.

The file must:

- Name the code being adopted and link to its canonical URL.
- Describe briefly where it applies (project spaces, issues, PRs, chat).
- Point reporters at the contact path defined in `SECURITY.md` for
  reporting violations — `SECURITY.md` is the single source of truth for
  contact addresses; do not duplicate an email here.

The file must **not** be required to contain the full Contributor
Covenant text, a named individual enforcement responder, or a contact
address of its own. Conformance checks (including AI quality review)
must not flag a link-only `CODE_OF_CONDUCT.md` as a violation.

## 6. SECURITY.md

`SECURITY.md` must describe:

- **Supported versions** — which release lines receive security fixes.
- **Reporting channel** — a private reporting path (GitHub Security
  Advisories, dedicated email, or HackerOne). Public issues must not be
  the intake channel for vulnerabilities.
- **Response expectations** — acknowledgment and triage timelines.
- **Disclosure policy** — coordinated disclosure window.
- **Scope** — what is considered in-scope vs. out-of-scope for the
  project's threat model.

## 7. AI agent guidance — AGENTS.md as the single source of truth

Modern OSS projects are regularly edited by AI coding agents. A
machine-readable guidance file at the repository root captures the
project's conventions so agents produce changes that match the rest of
the codebase on the first attempt.

**`AGENTS.md` is the canonical and only source of truth** for agent
guidance. It must live at the repository root and cover:

- **Build and test commands** — the canonical Makefile or script targets.
- **Commit and PR conventions** — conventional commits, PR title format,
  squash-merge policy.
- **Architecture summary** — a paragraph or two on module layout and
  dependency direction.
- **Where new code goes** — a routing table mapping common change types
  to the directories they belong in.
- **Test file conventions** — where tests live and how they are named
  (see §20 for the naming rule and rationale).
- **Documentation sync points** — a table of "if you change X, update Y".
- **Parity / checklist rules** — any cross-cutting rules (e.g. updating
  multiple bindings, keeping a CLI and library in sync).
- **Website staleness policy** — a pointer to §11.2 stating that the
  website must be regenerated whenever source-derived content changes.
- **Maintenance skills** — a pointer to §21 describing the agent
  skills the project ships for keeping drift-prone artifacts in sync.

### 7.1 Tool-specific files as symlinks

Every AI tool expects its guidance file at a different path. To avoid
duplication and drift, projects must create every tool-specific guidance
file as a **symbolic link** to `AGENTS.md`, not as a copy:

```bash
ln -s AGENTS.md CLAUDE.md
ln -s ../AGENTS.md .github/copilot-instructions.md
ln -s AGENTS.md .cursorrules
ln -s AGENTS.md .windsurfrules
ln -s AGENTS.md GEMINI.md
ln -s AGENTS.md .aider.conf.md
```

Required symlinks:

| Link path                              | Tool                  |
|----------------------------------------|-----------------------|
| `CLAUDE.md`                            | Claude Code           |
| `.github/copilot-instructions.md`      | GitHub Copilot        |
| `.cursorrules`                         | Cursor                |
| `.windsurfrules`                       | Windsurf              |
| `GEMINI.md`                            | Gemini CLI            |

Editing any tool-specific file (rather than `AGENTS.md`) is forbidden and
should be prevented by a pre-commit hook that refuses commits which
dereference the symlinks into regular files. A CI job should additionally
verify that each listed path is a symlink and resolves to `AGENTS.md`.

Projects on platforms without symlink support (Windows without developer
mode, some CI runners) should enable symlinks explicitly rather than
abandoning the single-source-of-truth rule:

```bash
git config --global core.symlinks true
```

## 8. Commits, versioning, and changelog

### 8.1 Conventional commits

Projects must use [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <summary>

[optional body]

[optional footer(s)]
```

Allowed types:

| Type       | Purpose                                  | Changelog section | SemVer bump |
|------------|------------------------------------------|-------------------|-------------|
| `feat`     | New user-facing feature                  | Added             | minor       |
| `fix`      | User-facing bug fix                      | Fixed             | patch       |
| `perf`     | Performance improvement                  | Performance       | patch       |
| `docs`     | Documentation only                       | Documentation     | none        |
| `test`     | Test-only changes                        | Tests             | none        |
| `refactor` | Code change that is neither fix nor feat | —                 | none        |
| `chore`    | Tooling, dependencies, housekeeping      | —                 | none        |
| `ci`       | CI/CD configuration                      | —                 | none        |
| `build`    | Build system, packaging                  | —                 | none        |
| `style`    | Whitespace, formatting                   | —                 | none        |

Breaking changes use `<type>!:` or a `BREAKING CHANGE:` footer and force a
major version bump.

### 8.2 Pull request merging

Projects must pick and document one merge strategy. The recommended
default is **squash-merge**, in which case:

- The PR title must follow conventional-commit format, because it becomes
  the single commit on the default branch.
- Individual commits inside a PR branch have no effect on the changelog.
- When additional commits are pushed to an open PR, the PR title and
  description must be updated to reflect the combined scope.

### 8.3 Semantic versioning

Projects must follow [SemVer 2.0.0](https://semver.org). Version numbers
are bumped automatically from the conventional-commit stream at release
time (see §10.3). Pre-1.0 projects may break compatibility in minor
releases but must still flag breaking changes with `!` or
`BREAKING CHANGE:` so that the changelog reflects them.

### 8.4 CHANGELOG.md

Projects must maintain a `CHANGELOG.md` in the [Keep a Changelog](https://keepachangelog.com/)
format. The file must be **generated automatically** from the
conventional-commit history at release time. Manual edits to
`CHANGELOG.md` are forbidden and should be enforced by a pre-commit check
or a CI lint.

## 9. Build system — Makefile

Every project must expose a small, uniform set of developer entry points.
A top-level `Makefile` (or equivalent task runner for the ecosystem) is
the recommended mechanism. The following targets are required:

| Target         | Purpose                                           |
|----------------|---------------------------------------------------|
| `make build`   | Developer build                                   |
| `make test`    | Run the full test suite                           |
| `make lint`    | Run the linter(s) with zero-warning policy        |
| `make fmt`     | Format the codebase in place                      |
| `make release` | Release/optimized build                           |
| `make clean`   | Remove build artifacts                            |

Recommended optional targets:

| Target             | Purpose                                       |
|--------------------|-----------------------------------------------|
| `make fmt-check`   | Verify formatting without modifying files    |
| `make coverage`    | Run tests with coverage reporting            |
| `make docs`        | Build local documentation                    |
| `make website`     | Build the marketing website                  |
| `make website-dev` | Run a local website dev server               |
| `make install`     | Install the built artifact locally           |
| `make bench`       | Run benchmarks                               |

CI pipelines must invoke these exact targets rather than reimplementing
their commands, so that local and CI environments stay in sync.

## 10. Continuous integration and release

### 10.1 CI pipeline

Every push to a branch and every pull request must run:

1. Checkout with full history (required for changelog generation).
2. Toolchain setup (pinned minimum version — see §10.3 for the
   per-language floor versions, and §10.5 for pinning the **exact**
   local-developer version that CI resolves against).
3. Dependency cache restore.
4. `make build`
5. `make test`
6. `make lint`
7. `make fmt-check`
8. Test result and coverage upload (optional but recommended).

The CI pipeline must fail on the first error and must treat warnings from
the linter as errors. CI must also run on multiple operating systems
(`ubuntu-latest`, `macos-latest`, `windows-latest`) for projects that
claim cross-platform support.

### 10.2 Status checks

The default branch must be protected. Required status checks must
include:

- All CI matrix jobs.
- At least one human review (or `CODEOWNERS`-based review routing).
- A passing `fmt-check` and `lint` job.
- Up-to-date branch before merge.

Force pushes and direct pushes to the default branch must be disallowed.

### 10.3 Release pipeline

Releases must be fully automated, reproducible, and triggered by an
explicit human intent — never by an incidental push. The canonical flow
uses **two chained workflows**, a `version-bump` workflow that a
maintainer dispatches manually, and a `release` workflow that runs
automatically when the tag that `version-bump` pushes lands on the
repository.

The release pipeline is triggered by the **`v*` tag push** that the
`version-bump` workflow performs — not by a `workflow_run` event on
`version-bump`. `workflow_run` from a sibling workflow does not
reliably fire when the upstream workflow pushed a tag with a PAT /
GitHub App token, and the event that does fire runs against the
default-branch commit rather than the tagged commit, which is the
wrong ref for every downstream build and publish step. Triggering on
`push: tags: ['v*']` is what works end-to-end, and it keeps the audit
trail simple: there is exactly one release per tag, and every tag can
only be created by a successful run of the `version-bump` workflow
(which is itself `workflow_dispatch`-only — see below).

Humans still must not push `v*` tags by hand. That restriction is
enforced by branch/tag protection rules on the repository, not by the
workflow trigger.

#### Workflow 1 — `version-bump`

Trigger: `workflow_dispatch` only, with a single input:

```yaml
on:
  workflow_dispatch:
    inputs:
      bump:
        description: 'Version bump type (auto | patch | minor | major)'
        required: false
        default: 'auto'
        type: choice
        options: ['auto', 'patch', 'minor', 'major']
```

Behaviour:

1. Check out the default branch (`main`) with full history.
2. Determine the next version:
   - `auto` (default) — read the conventional-commit range since the
     previous `v*` tag and pick `major`, `minor`, or `patch` according
     to the bump table in §8.1.
   - `patch` / `minor` / `major` — force the corresponding bump.
3. Verify the working tree is clean, the branch is `main`, and the
   computed tag does not already exist.
4. Create an **unannotated** lightweight tag `vX.Y.Z` on the current
   `HEAD` of `main` and push it to origin.
5. Exit successfully.

The version-bump workflow does **not** touch `CHANGELOG.md`, does
**not** rewrite package manifests, and does **not** push any commits.
All of that work happens in the release workflow, below. Keeping
version-bump small and read-only makes it safe to re-run if the
release workflow fails mid-flight.

The scripted logic (computing the next version, tagging, pushing)
should live in `scripts/release.sh` so a maintainer can also run it
locally, with the same semantics, as a break-glass procedure.

#### Workflow 2 — `release`

Trigger: the `push` event on any `v*` tag. The tag is pushed by the
`version-bump` workflow using `RELEASE_TOKEN` (see below), which is
what causes the downstream trigger to fire — the default
`GITHUB_TOKEN` deliberately suppresses recursive workflow triggers, so
`version-bump` must authenticate the tag push with a PAT or GitHub App
token.

```yaml
on:
  push:
    tags:
      - 'v*'
```

**Why not `workflow_run`?** A `workflow_run:
workflows: ['version-bump']` trigger does not work reliably: it fires
against the default-branch commit rather than the tagged commit, and
in practice it does not fire at all when the upstream workflow pushes
its tag with a PAT / GitHub App token. The `push: tags: ['v*']` form
runs cleanly end-to-end — the triggering event carries the tag ref,
so every downstream checkout/build/publish step sees the right
sources without any extra `git describe` gymnastics.

Hand-pushed tags must still not be accepted. That is enforced
out-of-band by tag protection rules on the repository (restricting
`v*` creation to the release bot identity that `version-bump` uses),
not by the workflow trigger.

The release workflow performs the following steps in order:

1. **Resolve the new tag.** The workflow reads the tag from
   `GITHUB_REF_NAME` (the tag that caused the `push` event). No `git
   describe` fallback is needed — the triggering event already names
   the tag.
2. **Check out the default branch** with full history
   (`fetch-depth: 0`) — not the tagged commit. The workflow must
   commit back to `main`.
3. **Generate `CHANGELOG.md`** from the conventional-commit range
   between the previous `v*` tag and the new one. A
   `scripts/generate-changelog.sh` helper is the recommended home for
   the logic.
4. **Update version numbers in every package manifest** the project
   ships. A `scripts/update-versions.sh` helper (or equivalent) must
   rewrite **all** of the following that apply:
   - `Cargo.toml` (and every workspace member).
   - `package.json` (and every workspace package, plus
     `package-lock.json`).
   - `pyproject.toml` / `setup.cfg` / `__version__.py`.
   - `*.csproj` / `Directory.Build.props`.
   - `build.gradle` / `build.gradle.kts` / `gradle.properties`.
   - `pom.xml`.
   - `Package.swift`.
   - Helm `Chart.yaml`, Docker image labels, and any embedded version
     constant in source.
5. **Commit the changes** back to the default branch with a
   conventional-commit message of the form
   `chore(release): update changelog and versions for vX.Y.Z`, then
   `git push origin main`. If both the generated changelog and every
   manifest are already in the correct state, the step must be a
   no-op and must not create an empty commit.
6. **Move the tag to the new commit and force-push it.** The tag
   pushed by `version-bump` (pointing at the last regular `main`
   commit) must now point at the release commit containing the
   generated changelog and bumped versions:

   ```bash
   git tag -f "${TAG}" HEAD
   git push origin "${TAG}" --force
   ```

   This is the **only** place in the entire project where
   force-pushing a git ref is permitted, and it is permitted only for
   the exact tag that `version-bump` just created. Branches must
   never be force-pushed.
7. **Build release artifacts in a matrix** covering every target
   platform the project ships — operating systems, architectures,
   language toolchains, container variants. Matrix jobs run in
   parallel, each checking out the **rewritten** tag so they see the
   bumped versions and generated changelog.
8. **Run the project's test suite** on the rewritten tag in at least
   one matrix job before any artifact is published.
9. **Publish to the relevant registries**: crates.io, npm, PyPI,
   NuGet, Maven Central, Docker Hub, GitHub Releases, and any
   language-specific index the project targets. Publish jobs depend
   on successful matrix builds. **Every publish step must
   authenticate via OIDC-based trusted publishing** — never a
   long-lived API token or password. See "Trusted publishing" below.
10. **Extract release notes** for the new version from `CHANGELOG.md`
    (the section between the latest and previous `## [vX.Y.Z]`
    headings) and attach them to the GitHub Release along with the
    built artifacts.
11. **Publish signed artifacts with provenance attestations** (SLSA
    level 3 recommended; GitHub's built-in artifact attestations are
    a reasonable baseline).

Design constraints:

- **Single entry point.** `workflow_dispatch` on `version-bump` is
  the only supported way to start a release. Hand-pushing `v*` tags
  must be prevented by tag protection rules on the repository — the
  release workflow will happily run against any `v*` tag push, so
  the integrity of the pipeline depends on the protection rule
  scoping tag creation to the release bot identity.
- **Idempotent version-bump step.** If the computed version already
  matches every manifest, step 5 is a no-op.
- **RELEASE_TOKEN secret.** Both workflows need a token with write
  access to `main` and to tags. `version-bump` needs it so its tag
  push actually triggers the `release` workflow (the default
  `GITHUB_TOKEN` deliberately suppresses downstream workflow
  triggers, so a tag pushed with it would not fire the release pipeline);
  `release` needs it for the commit-to-`main` and force-push-tag
  steps. A dedicated `RELEASE_TOKEN` PAT or GitHub App token is the
  canonical choice.
- **Branch protection.** `main` must be protected, and the release
  bot (or `github-actions[bot]`) must have a narrowly scoped
  exception to push the `chore(release): ...` commit. Disable branch
  protection globally at your peril.
- **Trusted publishing is mandatory.** Every package published by the
  release pipeline **must** authenticate to its registry via OIDC-based
  trusted publishing. Long-lived API tokens, passwords, and personal
  access tokens **must not** be used to publish releases. This applies
  to every registry the project targets that supports trusted
  publishing — including PyPI, npm, RubyGems, crates.io, NuGet, Maven
  Central (via Sonatype Central Portal), GitHub Container Registry,
  Docker Hub, and GitHub Releases. The publish job must exchange the
  GitHub-issued OIDC token for a short-lived registry credential at
  publish time.

  If a target registry does not yet support trusted publishing, the
  project must (a) document the exception in `SECURITY.md`, (b) scope
  the fallback credential to a single registry and a single package,
  (c) store it as a GitHub environment secret gated on the `release`
  environment with required reviewers, and (d) track removal of the
  exception as an open issue. Publishing with long-lived credentials
  is an escape hatch, not a steady state.

- **Pinned toolchain minimum versions.** Every CI and release job
  that sets up a language toolchain **must** declare an explicit
  minimum version, not a floating specifier such as `stable`,
  `latest`, or `lts/*`. Trusted publishing gives the registry a
  cryptographic guarantee about who is publishing; pinning the
  toolchain gives *reviewers* a guarantee about **what** is being
  built. The two controls are complementary: an OIDC-authenticated
  publish that silently built with a toolchain the author has never
  tested is still a supply-chain risk.

  The floor versions below apply to every language this spec
  supports. Projects are free to pin higher, but `oss-spec validate`
  will fail the build if a workflow declares anything lower or uses
  a floating specifier:

  | Language | Minimum | `setup-*` specifier |
  |---|---|---|
  | Rust   | 1.88 | `dtolnay/rust-toolchain@1.88.0` |
  | Python | 3.12 | `actions/setup-python` with `python-version: "3.12"` |
  | Node   | 24   | `actions/setup-node` with `node-version: "24"` |
  | Go     | 1.22 | `actions/setup-go` with `go-version: "1.22"` |

  The same minimums must be reflected in `README.md` "Prerequisites"
  (§3) and `CONTRIBUTING.md` "Prerequisites" (§4) so that
  contributors discover them before a CI failure does.

  **Local/CI parity.** The toolchain version pinned in CI **should**
  match the version developers use locally (e.g. via
  `rust-toolchain.toml`, `.python-version`, `.node-version`, or
  `.go-version`). When local and CI environments diverge, code that
  passes on a developer's machine may break in CI — or vice versa —
  leading to wasted cycles and eroded trust in the pipeline.
  Projects should treat their CI configuration as the canonical
  environment definition and keep local tooling in sync.

- **Least-privilege workflow permissions.** Every job that publishes
  a release artifact **must** declare an explicit job-level
  `permissions:` block. Implicit, workflow-level, or default
  `GITHUB_TOKEN` scopes are not acceptable for publish jobs — the
  scopes must be written down where the job runs so that a reviewer
  can audit them in one place. The minimum for a trusted-publishing
  job is:

  ```yaml
  permissions:
    contents: read     # checkout only; bump to `write` only if
                       # the job itself pushes commits or tags
    id-token: write    # required to mint the OIDC token that
                       # trusted publishing exchanges for a
                       # short-lived registry credential
  ```

  Additional scopes (`packages: write` for GHCR, `attestations:
  write` for SLSA provenance, `pull-requests: write` for release
  PRs, etc.) must be added explicitly and only on the jobs that
  need them. The top of the workflow file must set
  `permissions: {}` (or the most restrictive scope required by
  non-publish jobs) so that any job without its own block gets
  nothing by default. A CI check must fail the build if a publish
  job is missing `id-token: write`, if it relies on the default
  token scopes, or if `contents: write` is granted to a job that
  does not push to the repository.

The tag created by `version-bump` (`vX.Y.Z` pointing at the last
regular commit on `main`) and the final tag state (`vX.Y.Z` pointing
at the generated release commit) are intentionally different.
Consumers and downstream CI always see the final, rewritten tag.

### 10.4 Website deployment — every commit to `main`

The website must be deployed on **every push to the default branch**,
not only on release. A dedicated `pages` workflow triggered by
`push: branches: ['main']` (with a `workflow_dispatch` escape hatch)
installs website dependencies, runs the source-data extraction step
defined in §11.2, builds the website, uploads it as a Pages artifact,
and deploys it via `actions/deploy-pages`.

Because the extractor reads from the latest `v*` tag when one exists
(see §11.2), the deployed site shows the most recent released version
rather than unreleased in-progress work, even though the workflow
itself runs on every `main` commit. Doc and example changes on `main`
still reach the site immediately, because they are read from the
working tree rather than from the tag.

Concurrency must be configured so that only one deploy runs at a
time (`concurrency: { group: pages, cancel-in-progress: false }`) and
in-flight deploys are never cancelled.

The `pages` workflow is independent of the release pipeline: a
release does not wait for Pages, and a Pages deploy does not wait for
a release. Each delivers its own artifact to its own audience.

### 10.5 Local/CI environment parity

Every project must pin its language toolchain in a **repository-root
pin file** that both the local developer's toolchain manager and the
CI workflow read. CI's toolchain step must resolve to that same file
(or to a literal that matches it exactly). A lint, test, or build
that succeeds locally must not fail on CI solely because the two
environments booted different toolchain versions.

Why this matters:

- Linters and compilers gain, remove, and reword diagnostics between
  minor versions; an unpinned local toolchain produces noise that
  only shows up on CI (the canonical failure mode: `cargo clippy`
  passes on the contributor's Rust 1.90 install, then fails on CI's
  pinned 1.88.0 because a new lint fired).
- A single pin file prevents the version string from being duplicated
  in CI YAML, where it silently drifts.
- Contributors running `rustup show` / `pyenv install` / `nvm use` /
  `go build` in a fresh clone pick up the correct version without
  reading the CI config.

Per-language pin file (`must`):

| Language | Pin file | Example contents | CI reads it via |
|---|---|---|---|
| Rust | `rust-toolchain.toml` | `[toolchain]`<br>`channel = "1.88.0"`<br>`components = ["clippy", "rustfmt"]`<br>`profile = "minimal"` | `dtolnay/rust-toolchain@<channel>` matching the pin, or `rustup show` (auto-reads the file) |
| Python | `.python-version` | `3.12` | `actions/setup-python@v5` with `python-version-file: .python-version` |
| Node | `.nvmrc` (+ `"engines": { "node": ">=24" }` in `package.json`) | `24` | `actions/setup-node@v4` with `node-version-file: .nvmrc` |
| Go | `go.mod` with a `toolchain` directive | `go 1.22`<br>`toolchain go1.22.6` | `actions/setup-go@v5` with `go-version-file: go.mod` |
| Generic / polyglot | `.tool-versions` (asdf / mise) or a devcontainer | `rust 1.88.0`<br>`python 3.12.5` | Matching `asdf install` / devcontainer setup step |

Floating specifiers (`stable`, `latest`, `lts`, `lts/*`, `*`) are
**not permitted** in the pin file, same as in CI (§10.3).

Enforcement: `oss-spec validate` detects the project's languages from
their root manifest (`Cargo.toml`, `pyproject.toml`, `package.json`,
`go.mod`) and requires the corresponding pin file for each one. It
also cross-checks the pin-file version against the version referenced
by `ci.yml` and reports a violation if they disagree.

## 11. Documentation and website

Projects expose their surface in three complementary layers:

- **`README.md`** — the entry point. A reader who lands on the GitHub
  page must be able to evaluate and start using the project from the
  README alone (see §3).
- **`docs/`** — the reference manual. Topic-specific markdown.
- **`website/`** — the public showcase and hosted docs. A real web page
  with a landing hero, live-looking examples, and the `docs/` content
  rendered into a navigable site.

### 11.1 `docs/` directory

`docs/` is the authoritative reference. Each topic lives in its own
markdown file and is linked from the README's "Documentation" section.
Recommended starter topics:

- `docs/getting-started.md` — step-by-step tutorial.
- `docs/configuration.md` — complete configuration reference.
- `docs/architecture.md` — module layout and design decisions.
- `docs/troubleshooting.md` — common problems and fixes.

Topic files must avoid duplicating the README's quick start and must
instead go deeper. Each file should be self-contained enough to stand
alone when linked from an issue or a search result.

Documentation is kept in sync with code via the "documentation sync
points" table in `AGENTS.md` (see §7) so that contributors know exactly
which pages to touch for each kind of change.

### 11.2 `website/` — showcase and hosted docs

**Every project must ship a website.** The website is the project's
public face: a new visitor who does not yet know what the project is
should be able to understand it, see it in action, and find a reason to
try it — all from the landing page. It doubles as the hosted home for
everything under `docs/`.

#### Required content

1. **Hero** — project name, one-sentence description, primary call to
   action (install command, "Get started" button, or both).
2. **Feature showcase** — a few concrete capabilities expressed as
   short visuals, code samples, or animated demos. This section must be
   compelling in under ten seconds of reading.
3. **Live example(s)** — at least one realistic usage example rendered
   on the page, ideally with syntax highlighting and a "copy" button.
4. **Providers / integrations / supported platforms** — whatever the
   project's compatibility matrix is, rendered as a table or grid.
5. **Hosted docs** — the full contents of `docs/` rendered into a
   navigable sidebar + content pane, with search if feasible.
6. **Install / download** — registry badges and direct binary links.
7. **Footer** — license, repository link, version, last-updated
   timestamp, and a link to the CHANGELOG.

#### Content must be generated from source, not hand-maintained

The website is not a second source of truth. Any fact that already
exists in the codebase must be **extracted from source at build time**
rather than hard-coded into the website. Hard-coded duplicates decay and
must be prevented by tooling.

The project must include a source-extraction script (Node, Python, or
any language with a fast start time — Node is the recommended default
because the surrounding tooling is already JavaScript) that runs as the
first step of the website build. The script reads from the source tree
and emits a single generated data file that the website imports.

A minimal specification for the extraction script:

- **Location:** `website/scripts/extract-source-data.{mjs,ts,py}`.
- **Output:** `website/src/generated/sourceData.{ts,json}`. The output
  path must be gitignored and must be regenerated on every build.
- **Inputs it must resolve from source**, where applicable:
  - Current version from the authoritative manifest (`Cargo.toml`,
    `package.json`, `pyproject.toml`, etc.).
  - Command list and flags (parse from CLI definition files, e.g.
    `clap` structs, `argparse` definitions, or equivalent).
  - Supported platforms / providers / integrations (parse from a
    capability table, enum, or registry in source).
  - Default configuration values (parse from the config module).
  - Example snippets (read from `examples/` files or from fenced code
    blocks in `README.md` and `docs/`).
  - Changelog for the latest release (read `CHANGELOG.md`).
- **Git-aware extraction:** when a `v*` tag exists, the script should
  read source files at that tag (`git show <tag>:<path>`) so that the
  deployed website reflects the latest *released* version rather than
  in-progress work on `main`. Falling back to the working tree is
  acceptable when there is no tag yet.
- **Fail loudly:** the script must exit non-zero if it cannot find an
  expected marker in source, rather than silently emitting stale data.
  A missing command, a renamed enum, or a deleted capability must break
  the website build and force the developer to update the extractor.

The `website/package.json` (or equivalent) must expose the extraction
as a named script and must chain it into the build command so that it
is impossible to build the website without regenerating the data:

```json
{
  "scripts": {
    "extract": "node scripts/extract-source-data.mjs",
    "dev": "npm run extract && vite",
    "build": "npm run extract && vite build",
    "preview": "vite preview"
  }
}
```

The top-level `Makefile` exposes `make website` and `make website-dev`
as thin wrappers that delegate to these scripts (see §9).

#### Recommended stack

The spec is framework-agnostic, but the recommended baseline is:

- **Vite** as the build tool — instant dev server, fast production
  builds, trivial GitHub Pages output.
- **React** (or Preact, Svelte, SolidJS — any modern component
  framework) for the landing page and navigation.
- **Tailwind CSS** for styling — avoids a bespoke CSS system.
- **`react-markdown` + `remark-gfm`** (or equivalent) for rendering the
  contents of `docs/` inside the hosted-docs section without duplicating
  the markdown into the website source.
- **TypeScript** throughout, so the extractor's output types are
  checked against the component code that consumes them.

#### Deployment

The website is deployed from CI on every push to the default branch by
a dedicated workflow (see §10.4). GitHub Pages via
`actions/deploy-pages` is the recommended default. The workflow must:

1. Install website dependencies.
2. Run the source-extraction script.
3. Build the website.
4. Upload the built output as a Pages artifact.
5. Deploy.

A staleness CI check should run on every pull request: build the
website in dry-run mode, and fail if the extractor reports that any
source-derived field no longer matches what the website components
expect. This prevents PRs from silently breaking the showcase.

## 12. Additional requirements for CLI projects

Projects that ship a command-line interface have extra obligations
around discoverability and agent-friendliness. Modern CLI users include
AI coding agents that need to learn a CLI on the fly, troubleshoot
failures, and call it productively inside shell pipelines. The surfaces
below turn a CLI into a self-describing, agent-compatible tool. They
are required for any project whose primary deliverable is a CLI binary.

### 12.1 `--help-agent` — self-describing prompt injection

Every CLI must expose a top-level `--help-agent` flag that prints a
compact, prompt-injectable description of what the CLI is, what it
does, and how to discover more. The output is designed to be spliced
into an agent's prompt via command substitution so that the agent does
not need to be told from scratch what the tool is:

```bash
claude "Help me rewrite all workday commits to 16-21 UTC $(git-rewriter --help-agent)"
```

After this substitution the agent knows, without further research,
what `git-rewriter` is and which subcommands it should consider.

Requirements:

- **Output:** plain text on stdout, no ANSI escapes, suitable for
  embedding inside a larger prompt.
- **Length:** short enough not to dominate the surrounding prompt —
  typically 50–200 lines.
- **Contents:**
  1. A one-sentence description of the tool.
  2. The list of top-level commands, each with a one-line description.
  3. The most important flags and environment variables.
  4. A pointer to `<cli> commands` and `<cli> commands <name>` as the
     recommended discovery mechanism for agents (see §12.4).
  5. A pointer to `<cli> docs` and `<cli> man` for deeper reference
     reading (see §12.3).
  6. The current version and binary name.

- **Freshness:** the `--help-agent` output must be generated from the
  same source of truth that drives `--help`, the `commands` command,
  the manpages, and the README. It must not be a hand-maintained
  string. A CI snapshot test should assert that the help-agent text
  regenerates deterministically from source.

### 12.2 `--debug-agent` — self-describing troubleshooting context

Every CLI must expose a top-level `--debug-agent` flag that prints a
compact troubleshooting context block for the same prompt-injection
workflow:

```bash
claude "Help me debug why this project doesn't work. $(mytool --debug-agent)"
```

The output must give an agent everything it needs to investigate a
failure of this CLI without having to probe the filesystem from
scratch. Required contents:

1. **Log file locations and formats** — where the CLI writes logs,
   how they are rotated, how to read them.
2. **Config file paths and precedence** — every location the CLI
   reads config from, in the order it resolves them.
3. **Environment variables** — every variable the CLI reads, with a
   one-line description of its effect.
4. **Common failure modes with diagnostic commands** — for each known
   class of failure, the exact command to run to diagnose it (e.g.
   "to verify that X is installed, run Y; to see recent activity,
   run Z").
5. **How to increase verbosity** — the `--debug`, `--verbose`, or
   equivalent flags, and any relevant environment variables.
6. **How to capture a reproducer** — session ID, trace file, core
   dump, or bug-report bundle.
7. **Version and build metadata** — semver, commit SHA, build time,
   toolchain version.

As with `--help-agent`, the contents must be generated from the same
source as the rest of the documentation and must be kept up to date as
commands, config keys, environment variables, and log paths evolve.
Stale troubleshooting guidance is worse than none — a CI snapshot test
and a lint that cross-references config keys and environment variables
against their definitions in source must guard against drift.

### 12.3 `docs` and `man` commands

Every CLI must expose two documentation commands that read embedded
content so users and agents can access the docs offline and without
leaving the terminal:

- **`<cli> docs [topic]`** — reads topic markdown files from the
  `docs/` directory (see §11.1), compiled into the binary at build
  time. With no argument, lists available topics. With a topic
  argument, prints the corresponding file on stdout.
- **`<cli> man [command]`** — reads manual pages from `man/<command>.md`,
  compiled into the binary at build time. With no argument, lists
  every command that has a manpage. With a command argument, prints
  that command's manpage on stdout.

#### Manpage format

Manpages are **reference material**, not tutorials. Each
`man/<command>.md` must exhaustively document a single command:

- The command's one-line summary.
- Its full usage signature.
- Every subcommand, with a short description and its own usage
  signature.
- Every flag and argument, with type, default value, and description.
- Exit codes and their meanings.
- Environment variables the command reads.
- Multiple example invocations covering the common cases.
- Cross-references to related commands.

Tutorial content — "getting started", "how to …", "what's new in
vN.M" — belongs in `docs/`, not in manpages.

#### Embedding

Manpages and docs must be compiled into the binary via `include_str!`
(Rust), `//go:embed` (Go), embedded resources (.NET, Java, Kotlin),
bundler assets (JavaScript), or the equivalent mechanism in the
project's language. The CLI must work offline and must not fetch
documentation at runtime. The `docs/` and `man/` directories in the
source tree are the single source of truth; the embedded copies are
produced at build time.

#### Freshness check

A CI job must verify, on every pull request, that:

- Every command and subcommand declared in the CLI has a corresponding
  `man/<command>.md` file. Missing manpages fail the build.
- Every flag documented in a manpage exists in the CLI's flag
  definitions, and vice versa. Orphan flags on either side fail the
  build.
- Every topic referenced by `docs/` in the README or website exists as
  a real file.

### 12.4 `commands` — machine-readable command index

Every CLI must expose a `commands` subcommand that lists all commands
the tool supports, in a grep-friendly format. This is the primary
discovery surface for agents that need to find a capability without
reading prose documentation, and it is what `--help-agent` should
point at as the recommended next step.

Required behaviors:

- **`<cli> commands`** — lists every command, one per line, in the
  form `<name>  <usage signature>`:

  ```
  run        <cli> run [prompt] [--flag ...]
  exec       <cli> exec <prompt> [--flag ...]
  review     <cli> review [--uncommitted | --base <ref> | --commit <sha>]
  ...
  ```

  The output is stable, machine-parseable, and grep-friendly:

  ```bash
  <cli> commands | grep worktree
  ```

- **`<cli> commands <name>`** — prints the full usage specification
  for a single command: its usage signature, every flag with its type,
  default, and one-line description, and its exit codes. This is
  effectively a condensed form of the manpage, intended for agents
  that want the contract without the prose.

- **`<cli> commands --examples`** — prints every command together
  with one or more realistic example invocations. This is the
  "show me how to use this tool" firehose, and combines with grep
  to answer "how do I do anything related to X?":

  ```bash
  <cli> commands --examples | grep -A5 worktree
  ```

- **`<cli> commands <name> --examples`** — prints realistic example
  invocations for a **single** named command only, without the noise
  of every other command. This is the "show me how to use command X"
  targeted form, for agents that have already narrowed down which
  command they want and just need to see it in action.

Requirements:

- Output is plain text on stdout with no ANSI escapes, in a line
  format that does not change across patch releases.
- Command definitions, flag specifications, and example invocations
  must come from the **same single source of truth** that drives
  `--help`, manpages, `--help-agent`, and the README. They must not be
  duplicated across multiple hand-maintained tables.
- `--help-agent` (see §12.1) must explicitly tell the agent that the
  `commands` subcommand is the recommended way to discover further
  capabilities, and must show a one-line usage example of it.

### 12.5 Discoverability contract

Taken together, the surfaces above form a contract: an agent that
knows nothing about a CLI can learn everything it needs by running a
handful of deterministic commands.

| Question                                   | Command                             |
|--------------------------------------------|-------------------------------------|
| What is this tool?                         | `<cli> --help-agent`                |
| What commands does it have?                | `<cli> commands`                    |
| How do I call command X?                   | `<cli> commands <name>`             |
| Show me examples of every command.         | `<cli> commands --examples`         |
| Show me realistic examples of command X.   | `<cli> commands <name> --examples`  |
| Give me the full reference for X.          | `<cli> man <name>`                  |
| Explain concept Y in depth.                | `<cli> docs <topic>`                |
| Why is it broken / how do I debug it?      | `<cli> --debug-agent`               |

Every CLI project must implement every row of this table before
tagging its first stable release. Because all eight surfaces are
generated from a single source of truth, keeping them in sync is a
property of the build system, not a manual obligation on contributors.

## 13. Examples (`examples/`)

Projects should ship runnable examples under `examples/`. Each example
must:

- Live in its own subdirectory with a `README.md`.
- Build and run using the same toolchain as the main project.
- Be exercised by CI so that examples cannot silently rot.

Examples should demonstrate real-world usage patterns rather than
toy snippets that only duplicate the README's quick start.

### 13.5 LLM prompts (`prompts/`)

Any project that sends prompts to a large language model — directly
(via an SDK or HTTP call) or indirectly (via a wrapper like `zag`) —
must store those prompts as versioned files on disk under `prompts/`,
not as inline string literals in source code.

**Layout.** One subdirectory per logical prompt; one Markdown file per
version inside it:

```
prompts/
├── interpret-prompt/
│   ├── 1_0_0.md
│   └── 1_1_0.md
├── fix-conformance/
│   ├── 1_0_0.md
│   └── 1_1_0.md
└── …
```

**File name.** `<major>_<minor>_<patch>.md`, matching [semver]
(https://semver.org/). Bump **patch** for wording fixes that do not
change the contract (typos, clarifications). Bump **minor** for
non-breaking additions (new placeholders, expanded scope, new
guidance bullets). Bump **major** for breaking rewrites (removed
placeholders, changed JSON schema, fundamentally new task). Loaders
must always pick the highest version of a prompt unless explicitly
pinned.

**Never edit an existing versioned file.** Once a `<major>_<minor>_
<patch>.md` file is committed, its contents are immutable — every
change, no matter how small, lands as a new file at a new version.
This keeps every prompt a point-in-time artifact that can be diffed,
bisected, and blamed. The only time you may edit an existing file is
to correct a bug *before* it has ever been shipped or referenced from
a tagged release.

**Required YAML front matter.** Every prompt file must begin with a
YAML front-matter block declaring the prompt's `name`, `description`,
and `version`. The `version` value must match the filename stem
(e.g. `1_0_0.md` → `version: 1.0.0`). Loaders must strip the front
matter before passing the prompt to the model — it is metadata, not
instruction content.

```markdown
---
name: <prompt-name>
description: "<one-sentence description of what this prompt does>"
version: <major>.<minor>.<patch>
---

# <prompt-name>

## System

…system instructions for the model…

## User

…user message body. May contain {{ jinja }} placeholders that the
loader renders with runtime values…
```

The `## System` section is sent verbatim as the system prompt. The
`## User` section is rendered with whatever templating engine the
project already uses (this repo uses minijinja) and sent as the user
message. The YAML front matter, the `# Title` heading, and any other
prose outside the two required sections are ignored by the loader and
exist purely for humans reading the file.

**Why.** Inline prompts are invisible to reviewers, impossible to diff
across versions without reading source, and indistinguishable from
ordinary string literals to anyone trying to audit what a model is
being asked to do. A versioned `prompts/` tree makes prompt changes
first-class artifacts: they show up in PR diffs, they can be linted
and snapshot-tested, and the history of what the model was told is
preserved next to the code that calls it.

A project that performs no LLM calls may omit `prompts/` entirely.
Any project that *does* call an LLM must satisfy this rule before its
first public tag.

## 14. Dependency hygiene

- Enable automated dependency updates via `.github/dependabot.yml` or
  equivalent (Renovate). Configure it for the package ecosystem, GitHub
  Actions versions, and Docker base images.
- Enable secret scanning and push protection on the repository.
- Enable dependency review on pull requests.
- Pin CI actions by commit SHA, not by floating tag, to prevent
  supply-chain substitution.
- Run a software composition analysis tool (`cargo audit`, `npm audit`,
  `pip-audit`, `osv-scanner`, or similar) as a CI job, and fail the
  build on high-severity advisories.

## 15. Issue and pull request templates

`.github/ISSUE_TEMPLATE/` must contain at least:

- `bug_report.md` — reproduction steps, expected vs. actual behavior,
  environment details, version.
- `feature_request.md` — problem, proposed solution, alternatives.
- `config.yml` — disable blank issues and link to `SECURITY.md` for
  vulnerability reports.

`.github/PULL_REQUEST_TEMPLATE.md` must prompt the author for:

- A short summary.
- A linked issue (if any).
- A test plan.
- Checklist items: tests added, docs updated, changelog-relevant type in
  the PR title.

## 16. Formatting, linting, and pre-commit hooks

Projects must enforce formatting and linting in CI. They should also
enforce them locally via a pre-commit framework
([`pre-commit`](https://pre-commit.com/), `lefthook`, `husky`, or
equivalent) with hooks for:

- Formatter (`make fmt-check`).
- Linter (`make lint`).
- Commit message validation (conventional commits).
- Trailing whitespace and end-of-file fixes.
- Forbidden edits (e.g., `CHANGELOG.md` outside release commits).

Pre-commit hooks must be installable with a single documented command.

## 17. Governance

Every project must document its governance model, even if it is as
simple as "the author merges everything". Options include:

- **BDFL** — one person has the final say; good for young projects.
- **Maintainer team** — a named group with merge rights; scaling option.
- **Steering committee** — for larger projects with multiple stakeholder
  organizations.

The governance document must specify:

- Who has commit / merge rights.
- How decisions are made.
- How new maintainers are added.
- How the project handles disagreements.
- How the project can be forked or transferred if it is abandoned.

For small projects, governance can live as a section at the bottom of
`CONTRIBUTING.md`. Larger projects should promote it to `GOVERNANCE.md`.

## 18. Communication channels

Projects should declare, in the README and in `CONTRIBUTING.md`, where
discussion happens:

- GitHub Issues for bugs and feature requests.
- GitHub Discussions (or a dedicated forum) for questions and ideas.
- A chat channel (Discord, Matrix, Slack) if real-time discussion is
  expected.

The absence of a discussion venue pushes all conversation into issue
threads, which degrades triage quality.

## 19. Logging and diagnostic output

A project must use structured logging rather than raw print statements
(e.g. `println!` in Rust, `print()` in Python, `console.log` in Node,
`fmt.Println` in Go). All diagnostic output must flow through a central
output module so formatting and routing can be changed in one place.

### 19.1 Log levels

| Level   | Purpose                                           | Destination        |
|---------|---------------------------------------------------|--------------------|
| `error` | Unrecoverable failures                            | stderr + log file  |
| `warn`  | Recoverable issues the user should know about     | stderr + log file  |
| `info`  | Normal operational messages (status, progress)    | stderr + log file  |
| `debug` | Verbose diagnostics for troubleshooting           | log file (always); stderr only with `--debug` |

### 19.2 Always-on file logging

Every run must append to a persistent log file at a platform-appropriate
location (e.g. `~/.local/state/<project>/debug.log` on Linux,
`~/Library/Application Support/<project>/debug.log` on macOS). The file
log captures all levels including `debug`. No user action is required to
enable file logging — it is always on.

Log files should include timestamps and log levels. Rotation is optional
for v1 but must be documented (e.g. "truncate the file manually or set
up logrotate").

### 19.3 The `--debug` flag

CLI projects must accept a `--debug` global flag. When set, `debug`-level
messages are also printed to stderr. When unset, only `info` and above
appear on the terminal. The `--debug-agent` output (§12.2) must document
the log file path and the `--debug` flag.

### 19.4 Central output module

All user-facing output must route through a central output module (e.g.
`src/output.rs` in Rust, `lib/output.ts` in Node) that provides semantic
functions:

- **status** — success messages (e.g. green checkmark prefix).
- **warn** — warning messages (e.g. yellow prefix).
- **info** — informational messages.
- **header** — bold section headers.
- **error** — error messages (e.g. red prefix).

Each function writes to the terminal with appropriate styling **and** to
the log file via the logging framework. Raw print statements
(`println!`, `print()`, `console.log`, `fmt.Println`) must not appear
outside the output module except for machine-readable output required by
a contract (e.g. §12 agent discoverability surfaces, which require plain
text on stdout with no ANSI escapes).

## 20. Source and test organization

This section covers how source code is organized — both the separation
of tests from production source and the size of source files
themselves. The two rules reinforce each other: keeping tests in
dedicated files makes it easier to keep source files small, and the
size cap in §20.5 makes it harder for inline tests to accumulate
unnoticed.

Tests must live in **dedicated test files**, separate from the source
files they exercise. Inline test blocks embedded in production source
files (e.g. Rust `#[cfg(test)] mod tests { … }`, Python `if __name__ ==
"__main__"` test harnesses, or ad-hoc assertions at module scope) are
forbidden.

Using `#[cfg(test)]` to **import** a separate test file (e.g.
`#[cfg(test)] mod check_test;`) or to gate test-only `use` statements
is allowed — the rule targets inline test *bodies*, not the conditional-
compilation attribute itself.

### 20.1 Why separate test files?

Keeping tests out of source files provides three concrete benefits:

1. **Different rules for source and tests.** Linters, formatters, and
   review tools can apply stricter policies to production code (e.g.
   no `unwrap()`, mandatory doc comments) while relaxing them in test
   code — without file-level `#[allow(...)]` annotations or language-
   specific lint toggles.
2. **Agent hooks and automation.** CI, pre-commit hooks, and AI coding
   agents can detect when a change modifies tests vs. production code
   by simple path or filename matching. This enables workflows like
   "require a test change for every source change" or "re-run only
   affected test files."
3. **Clean reading.** Agents and humans reading source code see only
   production logic, without hundreds of lines of test scaffolding
   interleaved. Agents that need to understand the test suite can
   target the test directory directly.

### 20.2 Test file naming convention

Every test file's **stem** (the filename without its extension) must end
with one of the following suffixes:

| Suffix   | Example                          |
|----------|----------------------------------|
| `_test`  | `check_test.rs`, `utils_test.py` |
| `_tests` | `check_tests.rs`, `utils_tests.py` |
| `Test`   | `CheckTest.java`, `UtilsTest.kt` |
| `Tests`  | `CheckTests.cs`, `UtilsTests.swift` |

Expressed as a regex on the stem: `_?[Tt]ests?$`.

This convention is already idiomatic in most ecosystems (Go's `_test.go`,
JUnit's `*Test.java`, pytest's `test_*.py` / `*_test.py`) and enables
glob-based tooling (`*_test.*`, `*Test.*`) to enumerate all test files
without parsing build configs.

### 20.3 Where test files live

| Language / ecosystem | Test location | Notes |
|---|---|---|
| Rust | `tests/` directory at crate root | No `#[cfg(test)]` blocks in `src/`. Functions that need testing from outside the crate must be `pub`. |
| Python | `tests/` directory at project root | Follow pytest discovery: files named `test_*.py` or `*_test.py`. |
| Go | `*_test.go` alongside source files | Go enforces separate test files by convention; they already match the naming rule. |
| Node / TypeScript | `tests/` or `__tests__/` directory | Frameworks like Jest and Vitest discover `*.test.ts` / `*.spec.ts` by default; prefer `*_test.ts` or `*Test.ts` to stay within the naming convention. |
| JVM (Java, Kotlin) | `src/test/` per Maven/Gradle convention | Files named `*Test.java`, `*Tests.java`, `*Test.kt`, etc. |
| C# / .NET | Separate test project (e.g. `*.Tests.csproj`) | Files named `*Test.cs` or `*Tests.cs`. |

Projects using a language not listed above must document their test
location and naming convention in `AGENTS.md` and ensure the naming
rule in §20.2 is satisfied.

### 20.4 AGENTS.md must describe testing patterns

The `AGENTS.md` file (§7) must include a **Test conventions** section
that tells agents and contributors:

- Where test files live (directory / path pattern).
- The naming convention in use (which suffix from §20.2).
- How to run tests (`make test` at minimum, plus any subset commands).
- Any test-specific dependencies or setup (e.g. `tempfile` crate,
  Docker containers, fixture files).

### 20.5 Source file size limits

No non-test source file may exceed **1000 physical lines** (raw
newline-delimited lines, as reported by `wc -l`). Test files — those
whose stem matches the §20.2 regex `_?[Tt]ests?$` — are exempt; their
size is governed by whatever the test subject requires.

The limit is a **size smell**, not a precise complexity metric.
Physical lines are deliberately chosen over SLOC or cyclomatic
complexity so the rule is trivial to measure, predictable for
contributors, and immune to language-specific comment conventions. A
file over 1000 lines is almost always doing too much: aggregating
unrelated responsibilities, hiding inline tests, or waiting to be
split by concern.

**Why this rule.** Three motivations converge here:

1. **Readability.** Files that fit in a single screenful of a human
   reviewer's attention — or a single AI agent's working context —
   get reviewed carefully. Files that exceed it get skimmed.
2. **Decomposition pressure.** A hard line cap pushes authors to
   extract submodules, helpers, and sibling files before a large
   concern calcifies into an unsplittable monolith.
3. **Teeth for §20.** The easiest way to blow the 1000-line limit is
   to keep tests inline. §20.5 and §20 reinforce each other:
   extracting inline test blocks to their own file is usually
   sufficient to bring a large source file back under the cap.

#### 20.5.1 Exception mechanism

A file may declare itself exempt by carrying an **allow-large-file
marker** in any comment within its **first 20 lines**:

```
oss-spec:allow-large-file: <reason>
```

The marker's comment syntax follows the host language (`//` for
C-family, `#` for Python/Ruby/shell, `--` for SQL/Haskell, etc.) —
only the literal `oss-spec:allow-large-file:` token and the reason
are checked. The reason **must be non-empty**: a marker with no
motivation does not exempt the file. Validators must reject
`oss-spec:allow-large-file:` followed only by whitespace.

Exceptions are expected to be **rare and per-file**, not a project-
wide dial. Legitimate reasons include:

- **Generated code** — a file produced by a build step (protobuf,
  OpenAPI bindings, parser tables) that is not meant to be edited by
  hand.
- **Cohesive state machines** — a single enum or match tree whose
  arms cannot be meaningfully split without obscuring the design.
- **Third-party snapshots** — vendored code checked in verbatim.
- **Inherent density** — a configuration schema, rule catalogue, or
  lookup table that only grows linearly with real-world coverage.

Reviewers should treat an added or edited marker the same as any
other code change: ask whether the reason is honest, whether the
file has since become splittable, and whether the alternative (a
mechanical split) is genuinely worse than leaving the file oversized.

#### 20.5.2 Auto-fix scope

When `oss-spec fix` (or an equivalent automated refactor) encounters
a §20.5 violation, it must only attempt an **easy** refactor:
extracting inline test blocks (a §20 violation that commonly
co-occurs with §20.5) into a separate file under `tests/`. In
practice, doing so resolves both findings at once on files whose
bulk came from tests.

Automated refactors of **genuinely large source files** — splitting
modules, extracting helpers, decomposing responsibilities — are out
of scope. They require design judgment the tooling cannot
responsibly make. When the auto-fixer sees a §20.5 violation on a
file without a companion §20 violation, it must leave the file
alone and surface the finding for a human to either split manually
or annotate with an `oss-spec:allow-large-file:` marker.

## 21. Agent skills — maintenance playbooks for drift-prone artifacts

### 21.1 Motivation

Every non-trivial project has curated or generated artifacts whose truth
lives somewhere else: a README that describes a CLI, docs that explain
config keys, man pages that mirror flags, a website that restates
features, SDK bindings that wrap an API, examples that exercise the
current surface. When the source of truth changes and the mirror
doesn't, the project rots — and readers get contradictory answers
depending on which file they read first.

CI can *detect* drift (§12.3 manpage ↔ flag parity, §11.2 website
staleness) but cannot usually *fix* it. An **agent skill** closes that
gap: it is a versioned, machine-readable playbook that gives an AI
coding agent the exact procedure for bringing one drift-prone artifact
back into sync with its sources of truth. Skills are stored alongside
the code, improved over time, and re-run on demand.

### 21.2 Canonical location

Agent skills live at:

```
.agent/skills/<skill-name>/SKILL.md
```

`.agent/` is the generic, tool-neutral home for any file an AI coding
agent needs but a human typically does not. Tool-specific directories
(e.g. `.claude/skills/` for Claude Code) must be **symbolic links** to
`.agent/skills/` so that any tool which discovers skills from a fixed
path sees the same canonical set. This is the same single-source-of-
truth rule as §7.1.

Required directory symlinks:

| Link path            | Tool          | Target            |
|----------------------|---------------|-------------------|
| `.claude/skills`     | Claude Code   | `../.agent/skills`|

Additional tool-specific paths may be added as support lands, but every
such path must be a symlink — editing skills through a tool-specific
path (turning the symlink into a real directory) is forbidden and
should be caught by the same kind of symlink-verification job used in
§7.1.

### 21.3 Required SKILL.md structure

Every `SKILL.md` must contain:

1. **YAML front matter** with at least `name` and `description`:

   ```markdown
   ---
   name: update-readme
   description: "Use when README.md may be stale. Discovers commits since the last README update, identifies what changed, and merges updates into README.md."
   ---
   ```

   The `description` must be a one-sentence imperative that tells an
   agent *when* to invoke the skill. This field is what a parent agent
   reads when deciding whether the skill applies to the current task.

2. **An H1 heading** naming the skill's purpose.

3. **A "Tracking mechanism" section** pointing at a sibling `.last-updated`
   file that holds the git commit hash of the last successful run.

4. **A "Discovery process" section** containing the exact shell commands
   the agent should run to compute what has changed since the baseline
   (typically `git log` and `git diff --name-only`).

5. **A mapping table** that maps changed source paths or commit scopes
   to the output files that need updating. This is the skill's core
   asset — it is where domain knowledge accumulates.

6. **An "Update checklist"** the agent walks through while fixing drift.

7. **A "Verification" section** describing how the agent confirms the
   update is correct (typically by re-reading the updated files and
   comparing them against the sources of truth, and by running the
   relevant checks such as `make test` or `oss-spec validate .`).

8. **A "Skill self-improvement" section** that instructs the agent to
   update the mapping table, patterns, and checklist with any new
   knowledge discovered during the run, and to commit those skill
   edits alongside the documentation edits. Without this, the skill
   rots the same way the docs it fixes would.

### 21.4 Tracking file

Each skill directory must contain a `.last-updated` file:

```
.agent/skills/<skill-name>/.last-updated
```

It holds a single line: the git commit hash of the last successful run
of the skill. The skill updates it at the end of every run. An empty
file means "never run"; the skill must then use the repository's
initial commit as the baseline.

Using a committed tracking file (as opposed to, say, a git tag or CI
artifact) keeps the baseline visible in diffs and lets agents reason
about staleness without network or API access.

### 21.5 Required maintenance skills

Every project must ship at least one maintenance skill for each
drift-prone artifact it publishes. The following are required whenever
the corresponding artifact exists:

| Artifact      | Required skill      | Exists when                        |
|---------------|---------------------|------------------------------------|
| `README.md`   | `update-readme`     | Always (§3)                        |
| `docs/`       | `update-docs`       | Always (§11.1)                     |
| `man/`        | `update-manpages`   | CLI projects (§12.3)               |
| `website/`    | `update-website`    | A website is published (§11.2)     |
| *(umbrella)*  | `maintenance`       | Always — routes to all `update-*`  |

Projects with additional drift-prone surfaces should add further skills
such as `update-bindings` (SDK bindings mirroring a core API),
`update-examples` (examples exercising the current CLI), or project-
specific skills like `update-spec` (for spec repositories). Skill names
must be kebab-case and should start with a verb.

Any project that claims conformance to this spec should additionally
ship a **`sync-oss-spec`** skill whose job is to run the project's
conformance validator (for a repository bootstrapped by `oss-spec`,
that is `oss-spec validate .`), walk the resulting violations, and fix
each one until the repo is back in sync with `OSS_SPEC.md`. Unlike
`update-spec` — which reacts to a change in the spec by propagating
the new mandate into code — `sync-oss-spec` reacts to a change in the
repo by bringing it back under the spec's existing mandates. Running
`sync-oss-spec` as the final step of a drift sweep catches residual
violations that the per-artifact skills (`update-readme`, `update-docs`,
etc.) did not touch.

The skills in §21.5 are the floor, not the ceiling. A healthy project
adds a skill for every recurring "I forgot to update X when I changed
Y" bug report.

### 21.6 The `maintenance` umbrella skill

In addition to the per-artifact skills above, every project must ship a
**`maintenance`** skill whose sole job is to dispatch to the individual
`update-*` skills in the correct order and aggregate their output.
`.agent/skills/maintenance/SKILL.md` is the entry point for any agent
that wants to bring the whole repository back into sync without first
diagnosing *which* artifact is stale.

The `maintenance` skill must contain a **Registry** section: a single
table listing every `update-*` skill that exists in the repository,
together with a deterministic **run order**. The registry is the only
source of truth for which sync skills exist — adding a new `update-*`
skill without adding its row to the registry is a drift bug in its own
right.

Run order matters: upstream fixes must land before downstream skills
read them. Typical order is `update-spec` → `update-manpages` →
`update-docs` → `update-readme` → `update-website`. Projects that do
not publish a given artifact simply omit its row.

The `maintenance` skill does no rewriting itself. It only schedules
other skills, runs them in order, aggregates the combined diff, and
(after a successful sweep) rewrites its own `.last-updated` file.

### 21.7 What skills are not

- Skills are **not** CI jobs. They complement CI: CI detects drift;
  skills fix it. A skill run may be initiated by a human, by an agent
  noticing a failing CI check, or by another skill.
- Skills are **not** git hooks. Hooks run synchronously and must be
  fast; skills are long-running procedures that expect an agent in the
  loop.
- Skills are **not** one-shot prompts. They are iterated on over time
  and committed to version control; the mapping table and checklist are
  the skill's long-lived memory.
- Skills are **not** a substitute for good module boundaries. If a
  skill's mapping table keeps growing without bound, that is a signal
  that the underlying code needs refactoring, not that the skill needs
  more entries.

### 21.8 AGENTS.md integration

The `AGENTS.md` file (§7) must include a **Maintenance skills** section
that lists every skill the project ships and describes when each one
should run. This is the discovery surface for agents that do not yet
autoload skills from `.agent/skills/`.

## 22. Bootstrap checklist

Use this checklist when creating a new repository. Every box should be
checked before the first public tag.

```
[ ] LICENSE                                             (§2)
[ ] README.md with badges and quick start               (§3)
[ ] CONTRIBUTING.md                                     (§4)
[ ] CODE_OF_CONDUCT.md                                  (§5)
[ ] SECURITY.md                                         (§6)
[ ] AGENTS.md as single source of truth                 (§7)
[ ] CLAUDE.md / copilot-instructions.md / .cursorrules
    as symlinks to AGENTS.md                            (§7.1)
[ ] Symlink-verification CI job                         (§7.1)
[ ] CHANGELOG.md (empty, auto-generated)                (§8.4)
[ ] Conventional commits enforced                       (§8.1)
[ ] Default branch protected with status checks         (§10.2)
[ ] Makefile with build/test/lint/fmt/website targets   (§9)
[ ] CI workflow: build, test, lint, fmt-check           (§10.1)
[ ] version-bump workflow (workflow_dispatch, pushes
    `v*` tag via RELEASE_TOKEN)                         (§10.3)
[ ] release workflow triggered by `push: tags: ['v*']`,
    generating changelog, updating versions,
    force-pushing the rewritten tag, matrix-building
    and publishing                                      (§10.3)
[ ] RELEASE_TOKEN secret with main-branch bypass        (§10.3)
[ ] Trusted publishing (OIDC) configured for every
    target registry; no long-lived publish tokens       (§10.3)
[ ] Publish jobs declare explicit least-privilege
    permissions (contents: read, id-token: write)       (§10.3)
[ ] docs/ with at least getting-started.md              (§11.1)
[ ] website/ with source-extraction script and build    (§11.2)
[ ] pages workflow deploys website on every main push   (§10.4, §11.2)
[ ] Website staleness CI check                          (§11.2)
[ ] examples/ (if applicable) exercised by CI           (§13)
[ ] prompts/<name>/<major>_<minor>_<patch>.md for every
    LLM prompt the project sends (if applicable)        (§13.5)
[ ] Every prompt has YAML front matter with name,
    description, and version fields matching the stem  (§13.5)
[ ] Dependabot / Renovate configured                    (§14)
[ ] Secret scanning enabled                             (§14)
[ ] CI actions pinned by SHA                            (§14)
[ ] .github/ISSUE_TEMPLATE/ populated                   (§15)
[ ] .github/PULL_REQUEST_TEMPLATE.md                    (§15)
[ ] Pre-commit hooks installable                        (§16)
[ ] Governance documented                               (§17)
[ ] Communication channels linked in README             (§18)
[ ] Tests in separate files (*_test, *_tests, *Test,
    *Tests), no inline test blocks in source              (§20)
[ ] AGENTS.md documents test conventions                  (§20.4)
[ ] Central output module, no raw print statements       (§19.4)
[ ] Always-on debug log file                             (§19.2)
[ ] --debug flag for verbose terminal output             (§19.3)
[ ] .agent/skills/update-readme/ with SKILL.md +
    .last-updated                                       (§21.5)
[ ] .agent/skills/update-docs/ with SKILL.md +
    .last-updated                                       (§21.5)
[ ] .agent/skills/maintenance/ umbrella skill routing
    to every update-* skill                             (§21.6)
[ ] .claude/skills symlinked to ../.agent/skills         (§21.2)
[ ] AGENTS.md documents maintenance skills                (§21.8)

CLI projects additionally:

[ ] --help-agent flag with source-generated output      (§12.1)
[ ] --debug-agent flag with source-generated output     (§12.2)
[ ] docs command reading embedded docs/                 (§12.3)
[ ] man command reading embedded man/<command>.md       (§12.3)
[ ] man/<command>.md for every command (reference-style) (§12.3)
[ ] commands subcommand: list, <name>, --examples,
    <name> --examples                                   (§12.4)
[ ] CI check: manpage ↔ flag parity                     (§12.3)
[ ] CI snapshot test: --help-agent / --debug-agent      (§12.1, §12.2)
[ ] .agent/skills/update-manpages/ with SKILL.md +
    .last-updated                                       (§21.5)
```

A repository that satisfies this checklist has the foundational
infrastructure of a healthy open source project and is ready to accept
its first contribution.
