export default function Footer() {
  return (
    <footer className="border-t border-border py-12">
      <div className="mx-auto max-w-6xl px-6">
        <div className="flex flex-col items-center justify-between gap-6 md:flex-row">
          <div>
            <span className="text-lg font-bold text-text-primary">
              <span className="text-accent">⚡</span> zag
            </span>
            <p className="mt-1 text-sm text-text-dim">One CLI for all your AI coding agents</p>
          </div>

          <div className="flex gap-8 text-sm text-text-secondary">
            <a
              href="https://github.com/niclaslindstedt/zag"
              target="_blank"
              rel="noopener noreferrer"
              className="hover:text-text-primary transition-colors"
            >
              GitHub
            </a>
            <a
              href="https://github.com/niclaslindstedt/zag/tree/main/docs"
              target="_blank"
              rel="noopener noreferrer"
              className="hover:text-text-primary transition-colors"
            >
              Documentation
            </a>
            <a
              href="https://crates.io/crates/zag-cli"
              target="_blank"
              rel="noopener noreferrer"
              className="hover:text-text-primary transition-colors"
            >
              crates.io
            </a>
            <a
              href="https://github.com/niclaslindstedt/zag/blob/main/LICENSE"
              target="_blank"
              rel="noopener noreferrer"
              className="hover:text-text-primary transition-colors"
            >
              MIT License
            </a>
          </div>
        </div>
      </div>
    </footer>
  );
}
