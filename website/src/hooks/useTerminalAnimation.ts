import { useState, useRef, useEffect, useCallback } from "react";
import type { TerminalLine, RenderedLine } from "../data/logStyles";
import { resolveOutputLine } from "../data/logStyles";

const DEFAULT_TYPING_SPEED = 55;
const OUTPUT_LINE_INTERVAL = 60;
const LOOP_DELAY = 3000;

export function useTerminalAnimation(
  sequence: TerminalLine[],
  isVisible: boolean,
) {
  const [lines, setLines] = useState<RenderedLine[]>([]);
  const [generation, setGeneration] = useState(0);
  const timeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const genRef = useRef(0);
  const stateRef = useRef({
    itemIndex: 0,
    charIndex: 0,
    outputLineIndex: 0,
  });

  const clearTimer = useCallback(() => {
    if (timeoutRef.current !== null) {
      clearTimeout(timeoutRef.current);
      timeoutRef.current = null;
    }
  }, []);

  const schedule = useCallback(
    (fn: () => void, delay: number) => {
      clearTimer();
      timeoutRef.current = setTimeout(fn, delay);
    },
    [clearTimer],
  );

  const restart = useCallback(() => {
    clearTimer();
    genRef.current++;
    stateRef.current = { itemIndex: 0, charIndex: 0, outputLineIndex: 0 };
    setLines([]);
    setGeneration((g) => g + 1);
  }, [clearTimer]);

  useEffect(() => {
    restart();
  }, [sequence, restart]);

  useEffect(() => {
    const gen = genRef.current;

    function tick() {
      if (gen !== genRef.current) return;

      const { itemIndex, charIndex, outputLineIndex } = stateRef.current;

      if (itemIndex >= sequence.length) {
        schedule(() => {
          if (gen !== genRef.current) return;
          restart();
        }, LOOP_DELAY);
        return;
      }

      const item = sequence[itemIndex];

      switch (item.type) {
        case "comment": {
          setLines((prev) => [
            ...prev,
            { text: item.text, type: "comment", isActive: false },
          ]);
          stateRef.current.itemIndex++;
          schedule(tick, 100);
          break;
        }

        case "command": {
          if (charIndex === 0) {
            setLines((prev) => [
              ...prev,
              { text: "", type: "command", isActive: true },
            ]);
          }

          if (charIndex < item.text.length) {
            const partial = item.text.slice(0, charIndex + 1);
            setLines((prev) => {
              const next = [...prev];
              next[next.length - 1] = {
                text: partial,
                type: "command",
                isActive: true,
              };
              return next;
            });
            stateRef.current.charIndex++;
            const speed = item.typingSpeed ?? DEFAULT_TYPING_SPEED;
            const jitter = 0.7 + Math.random() * 0.6;
            schedule(tick, speed * jitter);
          } else {
            setLines((prev) => {
              const next = [...prev];
              next[next.length - 1] = {
                ...next[next.length - 1],
                isActive: false,
              };
              return next;
            });
            stateRef.current.charIndex = 0;
            stateRef.current.itemIndex++;
            schedule(tick, 80);
          }
          break;
        }

        case "output": {
          if (outputLineIndex === 0 && item.delay !== undefined) {
            stateRef.current.outputLineIndex = -1;
            schedule(() => {
              if (gen !== genRef.current) return;
              stateRef.current.outputLineIndex = 0;
              tick();
            }, item.delay);
            break;
          }

          if (outputLineIndex === -1) {
            break;
          }

          const outputLines = item.lines;
          if (outputLineIndex < outputLines.length) {
            const resolved = resolveOutputLine(outputLines[outputLineIndex]);
            setLines((prev) => [
              ...prev,
              {
                text: resolved.text,
                type: "output",
                style: resolved.style,
                isActive: false,
              },
            ]);
            stateRef.current.outputLineIndex++;
            schedule(tick, OUTPUT_LINE_INTERVAL);
          } else {
            stateRef.current.outputLineIndex = 0;
            stateRef.current.itemIndex++;
            schedule(tick, 100);
          }
          break;
        }

        case "pause": {
          stateRef.current.itemIndex++;
          schedule(tick, item.duration);
          break;
        }
      }
    }

    if (isVisible) {
      schedule(tick, 300);
    } else {
      clearTimer();
    }

    return clearTimer;
  }, [isVisible, sequence, generation, schedule, clearTimer, restart]);

  return { lines, restart };
}
