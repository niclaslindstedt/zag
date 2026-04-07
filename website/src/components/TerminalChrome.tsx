export default function TerminalChrome({
  tabs,
  activeTab,
  onTabClick,
  children,
  className = "",
}: {
  tabs: { label: string }[];
  activeTab: number;
  onTabClick: (index: number) => void;
  children: React.ReactNode;
  className?: string;
}) {
  return (
    <div
      className={`overflow-hidden rounded-xl border border-border bg-surface-alt shadow-2xl ${className}`}
    >
      {/* Title bar with traffic lights and tabs */}
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
              onClick={() => onTabClick(i)}
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

      {/* Body */}
      {children}
    </div>
  );
}
