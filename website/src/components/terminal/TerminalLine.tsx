import { LOG_STYLES } from "../../data/logStyles";
import type { RenderedLine, LogStyleName } from "../../data/logStyles";
import { highlightCommand } from "./CommandHighlighter";

function outputClassName(style: LogStyleName | undefined): string {
  if (style && style in LOG_STYLES) {
    return LOG_STYLES[style].className;
  }
  return LOG_STYLES.dim.className;
}

export default function TerminalLine({
  line,
  index,
}: {
  line: RenderedLine;
  index: number;
}) {
  if (line.type === "comment") {
    return (
      <div key={index} className="text-text-dim">
        {line.text}
      </div>
    );
  }

  if (line.type === "command") {
    return (
      <div key={index} className="flex">
        <span className="text-accent mr-2 shrink-0">$</span>
        <span className="flex-1">
          {highlightCommand(line.text)}
          {line.isActive && <span className="animate-blink-cursor" />}
        </span>
      </div>
    );
  }

  // output
  return (
    <div key={index} className={outputClassName(line.style)}>
      {line.text}
    </div>
  );
}
