import { useState, useRef, useEffect } from "react";

interface Props {
  onSubmit: (prompt: string) => void;
  disabled: boolean;
  placeholder?: string;
}

export function PromptInput({ onSubmit, disabled, placeholder }: Props) {
  const [value, setValue] = useState("");
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    if (!disabled) textareaRef.current?.focus();
  }, [disabled]);

  const handleSubmit = () => {
    const trimmed = value.trim();
    if (!trimmed || disabled) return;
    onSubmit(trimmed);
    setValue("");
    if (textareaRef.current) {
      textareaRef.current.style.height = "auto";
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSubmit();
    }
  };

  return (
    <div className="px-6 py-4 border-t border-zinc-800/80 bg-zinc-900/60 backdrop-blur-sm">
      <div className="flex items-end gap-3 bg-zinc-800/40 border border-zinc-700/50 rounded-xl px-4 py-3 transition-all focus-within:border-amber-700/40 focus-within:ring-1 focus-within:ring-amber-700/20">
        <textarea
          ref={textareaRef}
          className="flex-1 bg-transparent border-none outline-none text-zinc-100 text-sm font-sans resize-none min-h-[21px] max-h-[200px] placeholder:text-zinc-600 disabled:opacity-40"
          value={value}
          onChange={(e) => setValue(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder={placeholder || "Send a message..."}
          disabled={disabled}
          rows={1}
          onInput={(e) => {
            const target = e.target as HTMLTextAreaElement;
            target.style.height = "auto";
            target.style.height = Math.min(target.scrollHeight, 200) + "px";
          }}
        />
        <button
          className="bg-amber-600 hover:bg-amber-500 disabled:opacity-25 disabled:cursor-not-allowed text-zinc-950 rounded-lg w-8 h-8 flex items-center justify-center flex-shrink-0 transition-colors"
          onClick={handleSubmit}
          disabled={disabled || !value.trim()}
          title="Send (Enter)"
        >
          <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
            <path d="M3 13L13 8L3 3V7L9 8L3 9V13Z" fill="currentColor" />
          </svg>
        </button>
      </div>
      <div className="text-zinc-600 text-xs text-center mt-2">
        Press <kbd className="font-mono text-[10px] px-1.5 py-0.5 bg-zinc-800 border border-zinc-700 rounded">Enter</kbd> to send, <kbd className="font-mono text-[10px] px-1.5 py-0.5 bg-zinc-800 border border-zinc-700 rounded">Shift+Enter</kbd> for new line
      </div>
    </div>
  );
}
