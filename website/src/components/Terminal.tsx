import { useState, useRef, useEffect, useCallback } from "react";
import type { TerminalTab } from "../lib/terminalTypes";
import { useTerminalAnimation } from "../hooks/useTerminalAnimation";
import TerminalChrome from "./TerminalChrome";
import TerminalLine from "./TerminalLine";

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
    <div ref={containerRef}>
      <TerminalChrome
        tabs={tabs}
        activeTab={activeTab}
        onTabClick={switchTab}
        className={className}
      >
        <div
          ref={bodyRef}
          className="h-[320px] overflow-y-auto p-5 text-left font-mono text-sm leading-relaxed"
        >
          {lines.map((line, i) => (
            <TerminalLine key={i} line={line} />
          ))}
          {lines.length === 0 && (
            <div className="flex">
              <span className="text-accent mr-2">$</span>
              <span className="animate-blink-cursor" />
            </div>
          )}
        </div>
      </TerminalChrome>
    </div>
  );
}
