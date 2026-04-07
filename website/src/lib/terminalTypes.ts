/** Semantic style for terminal output lines. */
export type OutputStyle = "success" | "processing" | "detail" | "default";

/**
 * A single output line: plain string (style resolved at render time)
 * or an object with an explicit style override.
 */
export type OutputLine = string | { text: string; style: OutputStyle };

export type TerminalLine =
  | { type: "command"; text: string; typingSpeed?: number }
  | { type: "output"; lines: OutputLine[]; delay?: number }
  | { type: "comment"; text: string }
  | { type: "pause"; duration: number };

export type TerminalTab = {
  label: string;
  sequence: TerminalLine[];
};

export type RenderedLine = {
  text: string;
  type: "command" | "output" | "comment";
  style?: OutputStyle;
  isActive: boolean;
};
