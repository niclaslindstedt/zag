/**
 * Syntax highlighting for shell commands.
 *
 * Highlights quoted strings, flags (--flag / -f), and $variables/$()
 * subshells in the terminal command line.
 */
export function highlightCommand(text: string): React.ReactNode[] {
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
