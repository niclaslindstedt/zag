export default function Navbar() {
  return (
    <nav className="fixed top-0 left-0 right-0 z-50 border-b border-border bg-surface/80 backdrop-blur-md">
      <div className="mx-auto flex max-w-6xl items-center justify-between px-6 py-4">
        <a href="#" className="flex items-center gap-2 text-xl font-bold text-text-primary">
          <span className="text-accent">⚡</span> zag
        </a>
        <div className="hidden items-center gap-8 md:flex">
          <a href="#features" className="text-sm text-text-secondary hover:text-text-primary transition-colors">Features</a>
          <a href="#providers" className="text-sm text-text-secondary hover:text-text-primary transition-colors">Providers</a>
          <a href="#orchestration" className="text-sm text-text-secondary hover:text-text-primary transition-colors">Orchestration</a>
          <a href="#sdks" className="text-sm text-text-secondary hover:text-text-primary transition-colors">SDKs</a>
          <a href="#get-started" className="text-sm text-text-secondary hover:text-text-primary transition-colors">Get Started</a>
        </div>
        <a
          href="https://github.com/niclaslindstedt/zag"
          target="_blank"
          rel="noopener noreferrer"
          className="rounded-lg border border-border px-4 py-2 text-sm text-text-secondary hover:border-accent hover:text-text-primary transition-all"
        >
          GitHub
        </a>
      </div>
    </nav>
  );
}
