# CV Review Pipeline

A two-pass AI review pipeline that scores CVs against job descriptions using the `zag` library crate.

## How it works

1. **Recruiter Screen** — An AI agent scores each CV against the job description, producing structured JSON with category scores (experience, skills, education, communication, culture fit), strengths, weaknesses, and a recommendation.
2. **Hiring Committee** — A second agent reviews the recruiter's scores, makes calibrated adjustments with transparent justifications, and produces the final hire/no-hire recommendation.

Both passes use JSON schema validation to ensure structured, parseable output. When reviewing multiple CVs, they run in parallel using `tokio::spawn`.

## Prerequisites

- Rust 1.85+
- A configured provider (e.g., `ANTHROPIC_API_KEY` for Claude)

## Usage

```bash
# Review a single CV against a job description
cargo run -p cv-review -- --cv cvs/01_alex_chen.txt --job jobs/senior_backend.txt

# Batch-review all CVs in a directory
cargo run -p cv-review -- --cv-dir cvs/ --job jobs/senior_backend.txt

# Use custom scoring rules
cargo run -p cv-review -- --cv-dir cvs/ --job jobs/senior_backend.txt --rules scoring_rules.toml
```

To use a different provider or model, edit the `AgentBuilder` calls in `src/main.rs` or set defaults via `zag config`:

```bash
zag config provider gemini
zag config model large
```

## What to expect

Each CV produces two rounds of output:

1. **Recruiter Screen** — structured JSON with scores (1-10) for experience, skills, education, communication, and culture fit, plus strengths, weaknesses, and a recommendation
2. **Hiring Committee** — calibrated adjustments with transparent justifications and a final hire/no-hire recommendation

When batch-reviewing, all CVs are processed in parallel. A summary table is printed at the end with all candidates ranked by overall score.

## Sample data

- `cvs/` — 10 sample candidate CVs with varying experience levels
- `jobs/` — 3 job descriptions (senior backend, fullstack lead, ML platform)
- `scoring_rules.toml` — Configurable thresholds and category weights

## Key zag features demonstrated

- `AgentBuilder` programmatic API
- JSON schema validation (`json_schema()`)
- Custom `ProgressHandler` for terminal output
- Parallel agent invocations with `tokio::spawn`

## See also

- [zag-lib README](../../zag-lib/README.md) — Rust library API docs
- [Root README](../../README.md) — Full CLI documentation
- [Other examples](../README.md)

## License

[MIT](../../LICENSE)
