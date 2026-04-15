# Security Policy

## Reporting a Vulnerability

If you discover a security vulnerability in zag, please report it responsibly.

**Do not open a public GitHub issue for security vulnerabilities.**

Instead, use [GitHub's private vulnerability reporting](https://github.com/niclaslindstedt/zag/security/advisories/new) to submit your report. This ensures the details remain confidential until a fix is available.

### What to include

- A description of the vulnerability and its potential impact
- Steps to reproduce or a proof-of-concept
- The version of zag affected
- Any relevant configuration or environment details

### Response timeline

- **Acknowledgement**: Within 48 hours of your report
- **Assessment**: We will evaluate severity and impact within 7 days
- **Fix**: We aim to release a patch within 90 days of a confirmed vulnerability

### Scope

This policy covers the `zag` CLI binary and the `zag-agent` / `zag-orch` library crates. Vulnerabilities in upstream agent CLIs (Claude, Codex, Gemini, Copilot, Ollama) should be reported to their respective maintainers.

### Disclosure policy

We follow a **90-day coordinated disclosure** model. After a fix is released — or after 90 days from initial report if no fix is forthcoming — reporters are free to publish their findings. We will credit reporters in the release notes unless they request anonymity.

### Safe harbor

We consider security research conducted in good faith to be authorized. We will not pursue legal action against researchers who:

- Make a good-faith effort to avoid privacy violations, data destruction, and service disruption
- Only interact with accounts they own or with explicit permission
- Report vulnerabilities promptly and provide reasonable time for remediation before disclosure

## Supported Versions

| Version | Supported |
|---------|-----------|
| Latest release | Yes |
| Older releases | Best effort |

We recommend always running the latest version of zag.
