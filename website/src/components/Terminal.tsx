import { useState, useRef, useEffect, useCallback } from "react";
import type { TerminalTab } from "../data/terminalDemos";
import { useTerminalAnimation } from "../hooks/useTerminalAnimation";
import type { RenderedLine } from "../hooks/useTerminalAnimation";

function highlightCommand(text: string): React.ReactNode[] {
  const parts: React.ReactNode[] = [];
  let i = 0;
  let key = 0;

  while (i < text.length) {
    // Double-quoted string
    if (text[i] === '"') {
      const end = text.indexOf('"', i + 1);
      if (end !== -1) {
        parts.push(
          <span key={key++} className="text-text-secondary">
            {text.slice(i, end + 1)}
          </span>,
        );
        i = end + 1;
        continue;
      }
    }

    // Flags: --word or -letter
    if (
      text[i] === "-" &&
      (i === 0 || text[i - 1] === " ") &&
      i + 1 < text.length
    ) {
      let end = i + 1;
      if (text[end] === "-") end++; // skip second dash for --
      while (end < text.length && text[end] !== " ") end++;
      parts.push(
        <span key={key++} className="text-accent-light">
          {text.slice(i, end)}
        </span>,
      );
      i = end;
      continue;
    }

    // $( ... ) subshell or variable
    if (text[i] === "$") {
      if (text[i + 1] === "(") {
        // Find matching close paren, but highlight just $( and ) specially
        parts.push(
          <span key={key++} className="text-text-primary">
            $
          </span>,
        );
        i++;
        continue;
      }
      // $VAR
      let end = i + 1;
      while (end < text.length && /\w/.test(text[end])) end++;
      parts.push(
        <span key={key++} className="text-accent-light">
          {text.slice(i, end)}
        </span>,
      );
      i = end;
      continue;
    }

    // Regular text: accumulate until next special char
    let end = i + 1;
    while (
      end < text.length &&
      text[end] !== '"' &&
      !(text[end] === "-" && (end === 0 || text[end - 1] === " ")) &&
      text[end] !== "$"
    ) {
      end++;
    }
    parts.push(
      <span key={key++} className="text-text-primary">
        {text.slice(i, end)}
      </span>,
    );
    i = end;
  }

  return parts;
}

function renderLine(line: RenderedLine, index: number) {
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
  const text = line.text;
  let colorClass = "text-text-dim";
  if (text.startsWith("\u2713")) colorClass = "text-codex";
  else if (text.startsWith("\u21BB")) colorClass = "text-gemini";
  else if (text.startsWith("  +") || text.startsWith("  src/"))
    colorClass = "text-text-secondary";

  return (
    <div key={index} className={colorClass}>
      {text}
    </div>
  );
}

export default function Terminal({
  tabs,
  className = "",
}: {
  tabs: TerminalTab[];
  className?: string;
}) {
  const [activeTab, setActiveTab] = useState(0);
  const [isVisible, setIsVisible] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);
  const bodyRef = useRef<HTMLDivElement>(null);

  const { lines, restart } = useTerminalAnimation(
    tabs[activeTab].sequence,
    isVisible,
  );

  // IntersectionObserver for visibility
  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;

    const observer = new IntersectionObserver(
      ([entry]) => setIsVisible(entry.isIntersecting),
      { threshold: 0.1 },
    );
    observer.observe(el);
    return () => observer.disconnect();
  }, []);

  // Auto-scroll to bottom
  useEffect(() => {
    const el = bodyRef.current;
    if (el) {
      el.scrollTop = el.scrollHeight;
    }
  }, [lines]);

  const switchTab = useCallback(
    (index: number) => {
      if (index === activeTab) {
        restart();
      } else {
        setActiveTab(index);
      }
    },
    [activeTab, restart],
  );

  return (
    <div
      ref={containerRef}
      className={`overflow-hidden rounded-xl border border-border bg-surface-alt shadow-2xl ${className}`}
    >
      {/* Title bar with tabs */}
      <div className="flex items-center border-b border-border px-4 py-3">
        <div className="flex items-center gap-2 mr-4">
          <div className="h-3 w-3 rounded-full bg-[#ff5f57]" />
          <div className="h-3 w-3 rounded-full bg-[#febc2e]" />
          <div className="h-3 w-3 rounded-full bg-[#28c840]" />
        </div>
        <div className="flex gap-1 overflow-x-auto">
          {tabs.map((tab, i) => (
            <button
              key={tab.label}
              onClick={() => switchTab(i)}
              className={`whitespace-nowrap rounded-md px-3 py-1 text-xs font-medium transition-colors ${
                i === activeTab
                  ? "bg-surface text-accent"
                  : "text-text-dim hover:text-text-secondary"
              }`}
            >
              {tab.label}
            </button>
          ))}
        </div>
      </div>

      {/* Terminal body */}
      <div
        ref={bodyRef}
        className="h-[320px] overflow-y-auto p-5 text-left font-mono text-sm leading-relaxed"
      >
        {lines.map((line, i) => renderLine(line, i))}
        {lines.length === 0 && (
          <div className="flex">
            <span className="text-accent mr-2">$</span>
            <span className="animate-blink-cursor" />
          </div>
        )}
      </div>
    </div>
  );
}
