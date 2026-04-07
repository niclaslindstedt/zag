/**
 * Log line styles for the simulated terminal.
 *
 * Symbols are aligned with the text format in zag-orch/src/listen.rs
 * (format_event_text). If the Rust symbols change, update this file.
 *
 *   \u{2713} → tool result success (green)
 *   \u{2717} → tool result failure (red)
 *   \u{25cf} → session started/ended
 *   \u{276f} → user message
 *   \u{23fa} → assistant message
 *   \u{26a1} → tool call
 *   \u{2026} → reasoning / thinking
 */

// ---------------------------------------------------------------------------
// Style registry
// ---------------------------------------------------------------------------

export interface LogStyle {
  /** Tailwind CSS class(es) applied to the rendered line */
  className: string;
}

/** Named styles that output lines can reference. */
export const LOG_STYLES = {
  /** ✓  success / completion (green) */
  success: { className: "text-codex" },
  /** ✗✘ failure / error (red) */
  failure: { className: "text-[#f87171]" },
  /** >  provider status, Claude-colored */
  claude: { className: "text-claude" },
  /** ⏱  assistant / tool-call activity (Gemini blue) */
  assistant: { className: "text-gemini" },
  /** ←  tool result arrow (green) */
  toolResult: { className: "text-codex" },
  /** diff-stat lines  (src/… | +3 --) */
  diffStat: { className: "text-text-secondary" },
  /** default dim output */
  dim: { className: "text-text-dim" },
} as const;

export type LogStyleName = keyof typeof LOG_STYLES;

// ---------------------------------------------------------------------------
// Terminal line types (shared between demo data and animation hook)
// ---------------------------------------------------------------------------

/** A single output line: plain string (defaults to "dim") or annotated. */
export type OutputLine = string | { text: string; style: LogStyleName };

export type TerminalLine =
  | { type: "command"; text: string; typingSpeed?: number }
  | { type: "output"; lines: OutputLine[]; delay?: number }
  | { type: "comment"; text: string }
  | { type: "pause"; duration: number };

export type TerminalTab = {
  label: string;
  sequence: TerminalLine[];
};

/** Produced by useTerminalAnimation, consumed by TerminalLine renderer. */
export type RenderedLine = {
  text: string;
  type: "command" | "output" | "comment";
  style?: LogStyleName;
  isActive: boolean;
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Resolve an OutputLine to its text and style name. */
export function resolveOutputLine(line: OutputLine): {
  text: string;
  style: LogStyleName;
} {
  if (typeof line === "string") {
    return { text: line, style: "dim" };
  }
  return line;
}
