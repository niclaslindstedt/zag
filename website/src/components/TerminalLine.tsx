import { highlightCommand } from "../lib/highlightCommand";
import { resolveOutputStyle, OUTPUT_STYLE_CLASSES } from "../lib/outputStyles";
import type { RenderedLine } from "../lib/terminalTypes";

export default function TerminalLine({ line }: { line: RenderedLine }) {
  if (line.type === "comment") {
    return <div className="text-text-dim">{line.text}</div>;
  }

  if (line.type === "command") {
    return (
      <div className="flex">
        <span className="text-accent mr-2 shrink-0">$</span>
        <span className="flex-1">
          {highlightCommand(line.text)}
          {line.isActive && <span className="animate-blink-cursor" />}
        </span>
      </div>
    );
  }

  // Output: use explicit style if provided, otherwise resolve from content
  const style = line.style ?? resolveOutputStyle(line.text);
  return <div className={OUTPUT_STYLE_CLASSES[style]}>{line.text}</div>;
}
