export type TerminalLine =
  | { type: "command"; text: string; typingSpeed?: number }
  | { type: "output"; lines: string[]; delay?: number }
  | { type: "comment"; text: string }
  | { type: "pause"; duration: number };

export type TerminalTab = {
  label: string;
  sequence: TerminalLine[];
};

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
          "> Session: a1b2c3d4",
          "\u2713 Claude initialized with model sonnet-4",
        ],
      },
      { type: "pause", duration: 400 },
      {
        type: "output",
        delay: 100,
        lines: [
          "    \u23FA Read(file_path=\"src/api.rs\")",
          "    \u2190 247 lines",
          "    \u23FA Analyzing error handling patterns...",
          "    \u23FA Edit(file_path=\"src/api.rs\")",
          "    \u2190 applied 3 changes",
        ],
      },
      { type: "pause", duration: 300 },
      {
        type: "output",
        lines: [
          "",
          "\u2713 Session completed",
          "  src/api.rs | 14 ++++++---",
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
          "\u2713 Claude initialized with model haiku-4",
          "    \u23FA Read(file_path=\"src/models/user.rs\")",
          "    \u23FA Edit(file_path=\"src/models/user.rs\")",
          "\u2713 Session completed",
          "  src/models/user.rs | 3 ++-",
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
          "\u2713 Gemini initialized with model gemini-3.1-pro",
          "    \u23FA Analyzing auth module (5 files)...",
          "    \u23FA Writing new OAuth2 implementation...",
          "",
          "\u2713 Session completed",
          "  src/auth/mod.rs    | 47 ++++++---",
          "  src/auth/oauth2.rs | 89 ++++++++++",
          "  src/auth/tokens.rs | 34 +++--",
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
        lines: ["> Spawned 3 sessions [tag: review]"],
      },
      { type: "pause", duration: 600 },
      { type: "comment", text: "# Wait for all, then synthesize" },
      { type: "command", text: "zag wait --tag review" },
      { type: "pause", duration: 800 },
      {
        type: "output",
        delay: 400,
        lines: [
          "> Waiting... 1/3 completed",
          "> Waiting... 2/3 completed",
          "\u2713 3/3 sessions completed in 42s",
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
          "> Piping 3 session outputs...",
          "",
          "\u2713 Report saved to .zag/reports/review-summary.md",
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
