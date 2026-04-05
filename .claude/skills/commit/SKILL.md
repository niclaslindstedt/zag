---
description: "Commit staged changes, push the branch, and create or update a PR with a conventional-commit-formatted title. Use after completing a feature or fix."
---

# Commit, Push & PR

This skill handles the full workflow: verify quality gates → commit → push → create or update a PR.

## Step 1: Quality Gates

Run all checks before committing. All must pass:

```sh
make build   # must compile cleanly
make test    # all tests must pass
make clippy  # zero warnings
make fmt     # code formatted
```

Stop if any check fails. Fix the issue, then re-run.

## Step 2: Create a Feature Branch

**Always work on a feature branch — never commit directly to `main`.**

Check the current branch:

```sh
git branch --show-current
```

If already on `main` (or any protected branch), create and switch to a feature branch before staging anything. Derive the branch name from the commit type and a short summary of the change (kebab-case, no special characters):

```sh
git checkout -b type/short-description
# e.g.: feat/serve-logging, fix/token-output, refactor/spawn-args
```

If already on a feature branch, continue with that branch — do not create another one.

## Step 3: Review Changes

```sh
git status
git diff --staged
git diff
```

Understand what changed so you can write an accurate commit message and PR title.

## Step 4: Stage & Commit

Stage relevant files (prefer specific paths over `git add -A` to avoid accidentally including secrets or build artifacts):

```sh
git add <files...>
```

Write a conventional commit message:

```
type(scope): summary in imperative mood
```

**Changelog-eligible types** (pick the right one — it determines what appears in the changelog):

| Type | Changelog section | Version bump |
|------|-------------------|--------------|
| `feat` | Added | minor |
| `fix` | Fixed | patch |
| `perf` | Performance | patch |
| `docs` | Documentation | none |
| `test` | Tests | none |
| `refactor`, `chore`, `ci`, `build` | *(not included)* | none |

For breaking changes use `feat!:` or `fix!:`, or add a `BREAKING CHANGE:` footer → triggers a major version bump.

Scopes are lowercase, comma-separated if multiple: `feat(serve,connect): ...`

```sh
git commit -m "type(scope): summary"
```

## Step 5: Push

```sh
git push -u origin HEAD
```

## Step 6: Create or Update the PR

**Check if a PR already exists for this branch:**

```sh
gh pr view --json number,title,url 2>/dev/null
```

### If no PR exists — create one:

The PR title **must** follow conventional commit format — it becomes the squashed commit message on `main` and is what drives the changelog. Match it to the overall intent of the branch, not just the latest commit.

```sh
gh pr create \
  --title "type(scope): summary" \
  --body "$(cat <<'EOF'
## Summary

<brief description of the changes and motivation>

## Test plan

- [ ] `make build` passes
- [ ] `make test` passes
- [ ] `make clippy` has zero warnings
- [ ] `make fmt` applied

## Checklist

- [ ] Tests added/updated in separate `*_tests.rs` files
- [ ] Documentation updated (if user-facing behavior changed):
  - [ ] `README.md`
  - [ ] `zag-cli/man/*.md`
  - [ ] Binding READMEs (if builder API changed)
- [ ] Commit messages follow conventional commit style
EOF
)"
```

### If a PR already exists — update it:

Re-evaluate the PR title and description to reflect the **combined** scope of all commits on the branch, then update:

```sh
gh pr edit \
  --title "type(scope): updated summary" \
  --body "$(cat <<'EOF'
## Summary

<updated description covering all changes>

## Test plan

- [ ] `make build` passes
- [ ] `make test` passes
- [ ] `make clippy` has zero warnings
- [ ] `make fmt` applied

## Checklist

- [ ] Tests added/updated in separate `*_tests.rs` files
- [ ] Documentation updated (if user-facing behavior changed):
  - [ ] `README.md`
  - [ ] `zag-cli/man/*.md`
  - [ ] Binding READMEs (if builder API changed)
- [ ] Commit messages follow conventional commit style
EOF
)"
```

## Key Reminders

- **PR title = squashed commit on main = changelog entry.** Choose the type and summary carefully.
- The individual commits within the branch don't appear in the changelog — only the PR title does.
- If the branch touches multiple scopes, use comma-separated scopes: `feat(serve,connect): ...`
- Never skip hooks (`--no-verify`) — fix the underlying issue instead.
