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
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSubmit();
    }
  };

  return (
    <div className="prompt-input-container">
      <div className="prompt-input-wrapper">
        <textarea
          ref={textareaRef}
          className="prompt-input"
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
          className="prompt-send"
          onClick={handleSubmit}
          disabled={disabled || !value.trim()}
          title="Send (Enter)"
        >
          <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
            <path
              d="M3 13L13 8L3 3V7L9 8L3 9V13Z"
              fill="currentColor"
            />
          </svg>
        </button>
      </div>
      <div className="prompt-hint">
        Press <kbd>Enter</kbd> to send, <kbd>Shift+Enter</kbd> for new line
      </div>
    </div>
  );
}
