import type { TerminalTab } from "./logStyles";

export const terminalDemos: TerminalTab[] = [
  {
    label: "Quick Start",
    sequence: [
      { type: "comment", text: "# Run with any provider" },
      {
        type: "command",
        text: 'zag exec -p claude "Add error handling to src/api.rs"',
      },
      { type: "pause", duration: 500 },
      {
        type: "output",
        delay: 200,
        lines: [
          { text: "> Session: a1b2c3d4", style: "claude" },
          { text: "\u2713 Claude initialized with model sonnet-4", style: "success" },
        ],
      },
      { type: "pause", duration: 400 },
      {
        type: "output",
        delay: 100,
        lines: [
          { text: "    \u23FA Read(file_path=\"src/api.rs\")", style: "assistant" },
          { text: "    \u2190 247 lines", style: "toolResult" },
          { text: "    \u23FA Analyzing error handling patterns...", style: "assistant" },
          { text: "    \u23FA Edit(file_path=\"src/api.rs\")", style: "assistant" },
          { text: "    \u2190 applied 3 changes", style: "toolResult" },
        ],
      },
      { type: "pause", duration: 300 },
      {
        type: "output",
        lines: [
          "",
          { text: "\u2713 Session completed", style: "success" },
          { text: "  src/api.rs | 14 ++++++---", style: "diffStat" },
          "",
          "Tokens: 1,847 in / 923 out \u00b7 Cost: $0.02 \u00b7 Duration: 8.3s",
        ],
      },
      { type: "pause", duration: 2500 },
    ],
  },
  {
    label: "Multi-Provider",
    sequence: [
      { type: "comment", text: "# Fast fix with a small model" },
      {
        type: "command",
        text: 'zag exec -p claude -m small "Fix null check in user.rs"',
      },
      { type: "pause", duration: 300 },
      {
        type: "output",
        delay: 150,
        lines: [
          { text: "\u2713 Claude initialized with model haiku-4", style: "success" },
          { text: "    \u23FA Read(file_path=\"src/models/user.rs\")", style: "assistant" },
          { text: "    \u23FA Edit(file_path=\"src/models/user.rs\")", style: "assistant" },
          { text: "\u2713 Session completed", style: "success" },
          { text: "  src/models/user.rs | 3 ++-", style: "diffStat" },
          "",
          "Tokens: 312 in / 87 out \u00b7 Cost: $0.001 \u00b7 Duration: 1.1s",
        ],
      },
      { type: "pause", duration: 1200 },
      { type: "comment", text: "# Complex task with a large model" },
      {
        type: "command",
        text: 'zag exec -p gemini -m large "Redesign the auth module"',
      },
      { type: "pause", duration: 600 },
      {
        type: "output",
        delay: 200,
        lines: [
          { text: "\u2713 Gemini initialized with model gemini-3.1-pro", style: "success" },
          { text: "    \u23FA Analyzing auth module (5 files)...", style: "assistant" },
          { text: "    \u23FA Writing new OAuth2 implementation...", style: "assistant" },
          "",
          { text: "\u2713 Session completed", style: "success" },
          { text: "  src/auth/mod.rs    | 47 ++++++---", style: "diffStat" },
          { text: "  src/auth/oauth2.rs | 89 ++++++++++", style: "diffStat" },
          { text: "  src/auth/tokens.rs | 34 +++--", style: "diffStat" },
          "",
          "Tokens: 8,241 in / 4,102 out \u00b7 Cost: $0.09 \u00b7 Duration: 14.7s",
        ],
      },
      { type: "pause", duration: 2500 },
    ],
  },
  {
    label: "Orchestration",
    sequence: [
      { type: "comment", text: "# Spawn parallel review agents" },
      {
        type: "command",
        text: 'zag spawn -p claude --tag review "analyze auth module"',
        typingSpeed: 40,
      },
      {
        type: "command",
        text: 'zag spawn -p gemini --tag review "review test coverage"',
        typingSpeed: 40,
      },
      {
        type: "command",
        text: 'zag spawn -p codex --tag review "find security issues"',
        typingSpeed: 40,
      },
      { type: "pause", duration: 300 },
      {
        type: "output",
        delay: 150,
        lines: [
          { text: "> Spawned 3 sessions [tag: review]", style: "claude" },
        ],
      },
      { type: "pause", duration: 600 },
      { type: "comment", text: "# Wait for all, then synthesize" },
      { type: "command", text: "zag wait --tag review" },
      { type: "pause", duration: 800 },
      {
        type: "output",
        delay: 400,
        lines: [
          { text: "> Waiting... 1/3 completed", style: "claude" },
          { text: "> Waiting... 2/3 completed", style: "claude" },
          { text: "\u2713 3/3 sessions completed in 42s", style: "success" },
        ],
      },
      { type: "pause", duration: 500 },
      {
        type: "command",
        text: 'zag pipe --tag review -- "create unified report"',
      },
      { type: "pause", duration: 600 },
      {
        type: "output",
        delay: 300,
        lines: [
          { text: "> Piping 3 session outputs...", style: "claude" },
          "",
          { text: "\u2713 Report saved to .zag/reports/review-summary.md", style: "success" },
          { text: "  Found: 2 critical issues, 4 suggestions, 91% coverage", style: "diffStat" },
        ],
      },
      { type: "pause", duration: 2500 },
    ],
  },
  {
    label: "JSON Output",
    sequence: [
      {
        type: "command",
        text: 'zag exec --json "list API endpoints in src/"',
      },
      { type: "pause", duration: 600 },
      {
        type: "output",
        delay: 300,
        lines: [
          "{",
          '  "endpoints": [',
          '    { "path": "/api/users",    "method": "GET",    "auth": true  },',
          '    { "path": "/api/users",    "method": "POST",   "auth": true  },',
          '    { "path": "/api/health",   "method": "GET",    "auth": false },',
          '    { "path": "/api/sessions", "method": "GET",    "auth": true  },',
          '    { "path": "/api/sessions", "method": "DELETE", "auth": true  }',
          "  ],",
          '  "total": 5,',
          '  "version": "v2"',
          "}",
        ],
      },
      { type: "pause", duration: 2500 },
    ],
  },
];
