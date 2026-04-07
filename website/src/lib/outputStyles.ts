import type { OutputStyle } from "./terminalTypes";

export type OutputStyleRule = {
  /** Prefix string to match against the start of a line. */
  match: string;
  /** The semantic style to apply when matched. */
  style: OutputStyle;
};

/** Tailwind class for each output style. */
export const OUTPUT_STYLE_CLASSES: Record<OutputStyle, string> = {
  success: "text-codex",
  processing: "text-gemini",
  detail: "text-text-secondary",
  default: "text-text-dim",
};

/**
 * Default rules evaluated top-to-bottom; first match wins.
 * These mirror the symbols used in the CLI output.
 */
export const DEFAULT_OUTPUT_RULES: OutputStyleRule[] = [
  { match: "\u2713", style: "success" }, // ✓ checkmark
  { match: "\u21BB", style: "processing" }, // ↻ spinner
  { match: "  +", style: "detail" }, // diff additions
  { match: "  src/", style: "detail" }, // file paths
];

/**
 * Resolve the output style for a line of text by running through the rules.
 * Returns "default" if no rule matches.
 */
export function resolveOutputStyle(
  text: string,
  rules: OutputStyleRule[] = DEFAULT_OUTPUT_RULES,
): OutputStyle {
  for (const rule of rules) {
    if (text.startsWith(rule.match)) return rule.style;
  }
  return "default";
}
