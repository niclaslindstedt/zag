interface Props {
  oldText: string;
  newText: string;
  filePath?: string;
}

export function DiffView({ oldText, newText, filePath }: Props) {
  const oldLines = oldText.split("\n");
  const newLines = newText.split("\n");

  return (
    <div className="rounded-md border border-zinc-800 overflow-hidden bg-zinc-900">
      {filePath && (
        <div className="px-3 py-1.5 bg-zinc-800/60 border-b border-zinc-800 font-mono text-[11px] text-zinc-500">
          {filePath}
        </div>
      )}
      <div className="p-2 font-mono text-xs leading-relaxed overflow-x-auto">
        {oldLines.map((line, i) => (
          <div key={`old-${i}`} className="px-2 py-0.5 bg-red-950/30 text-red-300/80 rounded-sm">
            <span className="text-red-500/60 mr-2 select-none">-</span>
            {line}
          </div>
        ))}
        {newLines.map((line, i) => (
          <div key={`new-${i}`} className="px-2 py-0.5 bg-green-950/30 text-green-300/80 rounded-sm">
            <span className="text-green-500/60 mr-2 select-none">+</span>
            {line}
          </div>
        ))}
      </div>
    </div>
  );
}
