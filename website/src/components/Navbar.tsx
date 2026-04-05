import { Link, useLocation } from "react-router-dom";

export default function Navbar() {
  const location = useLocation();
  const isDocsPage = location.pathname.startsWith("/docs");

  // On docs pages, anchor links need to go back to the landing page
  const sectionHref = (hash: string) => (isDocsPage ? `/${hash}` : hash);

  return (
    <nav className="fixed top-0 left-0 right-0 z-50 border-b border-border bg-surface/80 backdrop-blur-md">
      <div className="mx-auto flex max-w-6xl items-center justify-between px-6 py-4">
        <Link to="/" className="flex items-center gap-2 text-xl font-bold text-text-primary">
          <span className="text-accent">⚡</span> zag
        </Link>
        <div className="hidden items-center gap-8 md:flex">
          <a href={sectionHref("#features")} className="text-sm text-text-secondary hover:text-text-primary transition-colors">Features</a>
          <a href={sectionHref("#providers")} className="text-sm text-text-secondary hover:text-text-primary transition-colors">Providers</a>
          <a href={sectionHref("#orchestration")} className="text-sm text-text-secondary hover:text-text-primary transition-colors">Orchestration</a>
          <a href={sectionHref("#sdks")} className="text-sm text-text-secondary hover:text-text-primary transition-colors">SDKs</a>
          <a href={sectionHref("#get-started")} className="text-sm text-text-secondary hover:text-text-primary transition-colors">Get Started</a>
          <Link
            to="/docs/getting-started"
            className={`text-sm transition-colors ${isDocsPage ? "text-accent" : "text-text-secondary hover:text-text-primary"}`}
          >
            Docs
          </Link>
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
