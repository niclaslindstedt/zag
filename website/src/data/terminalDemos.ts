import type { TerminalTab } from "../lib/terminalTypes";

export const terminalDemos: TerminalTab[] = [
  {
    label: "Quick Start",
    sequence: [
      { type: "comment", text: "# Run with any provider" },
      {
        type: "command",
        text: 'zag exec -p claude "Add error handling to src/api.rs"',
      },
      { type: "pause", duration: 600 },
      {
        type: "output",
        delay: 300,
        lines: [
          { text: "\u21BB Connecting to Claude...", style: "processing" },
          { text: "\u21BB Analyzing src/api.rs (247 lines)", style: "processing" },
          { text: "\u21BB Generating patch...", style: "processing" },
          "",
          { text: "\u2713 Applied 3 changes to src/api.rs", style: "success" },
          "  + Added Result<T, ApiError> return types",
          "  + Wrapped DB calls in error handlers",
          "  + Added retry logic for transient failures",
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
      { type: "comment", text: "# Fast task with a small model" },
      {
        type: "command",
        text: 'zag exec -p claude -m small "Quick fix: add null check"',
      },
      { type: "pause", duration: 400 },
      {
        type: "output",
        delay: 200,
        lines: [
          { text: "\u2713 Applied 1 change to src/models/user.rs", style: "success" },
          "Tokens: 312 in / 87 out \u00b7 Cost: $0.001 \u00b7 Duration: 1.1s",
        ],
      },
      { type: "pause", duration: 1200 },
      { type: "comment", text: "# Complex task with a large model" },
      {
        type: "command",
        text: 'zag exec -p gemini -m large "Redesign the auth module"',
      },
      { type: "pause", duration: 800 },
      {
        type: "output",
        delay: 300,
        lines: [
          { text: "\u21BB Connecting to Gemini...", style: "processing" },
          { text: "\u21BB Analyzing auth module (5 files, 1,203 lines)", style: "processing" },
          { text: "\u21BB Generating redesign...", style: "processing" },
          "",
          { text: "\u2713 Modified 5 files, added 2 new files", style: "success" },
          "  src/auth/mod.rs          | 47 +++++---",
          "  src/auth/oauth2.rs       | 89 ++++++++++",
          "  src/auth/tokens.rs       | 34 +++--",
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
        text: 'sid1=$(zag spawn -p claude --tag review "analyze auth module")',
      },
      {
        type: "command",
        text: 'sid2=$(zag spawn -p gemini --tag review "review test coverage")',
      },
      {
        type: "command",
        text: 'sid3=$(zag spawn -p codex --tag review "find security issues")',
      },
      { type: "pause", duration: 400 },
      {
        type: "output",
        delay: 200,
        lines: ["Spawned 3 sessions [tag: review]"],
      },
      { type: "pause", duration: 800 },
      { type: "comment", text: "# Wait for all to finish" },
      { type: "command", text: "zag wait --tag review --timeout 5m" },
      { type: "pause", duration: 1200 },
      {
        type: "output",
        delay: 500,
        lines: [
          { text: "\u21BB Waiting... 1/3 completed (claude)", style: "processing" },
          { text: "\u21BB Waiting... 2/3 completed (gemini)", style: "processing" },
          { text: "\u2713 3/3 sessions completed in 42s", style: "success" },
        ],
      },
      { type: "pause", duration: 600 },
      { type: "comment", text: "# Pipe all results into a synthesis agent" },
      {
        type: "command",
        text: 'zag pipe --tag review -- "create unified report"',
      },
      { type: "pause", duration: 800 },
      {
        type: "output",
        delay: 400,
        lines: [
          { text: "\u21BB Piping 3 session outputs to new agent...", style: "processing" },
          "",
          { text: "\u2713 Report saved to .zag/reports/review-summary.md", style: "success" },
          "  Found: 2 critical issues, 4 suggestions, 91% coverage",
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
