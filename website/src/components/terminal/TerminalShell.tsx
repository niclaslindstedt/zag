import { useState, useRef, useEffect, useCallback } from "react";
import type { TerminalTab } from "../../data/logStyles";
import { useTerminalAnimation } from "../../hooks/useTerminalAnimation";
import TerminalLine from "./TerminalLine";

export default function TerminalShell({
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
        {lines.map((line, i) => (
          <TerminalLine key={i} line={line} index={i} />
        ))}
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
