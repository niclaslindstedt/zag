import { useParams, Link, Navigate } from "react-router-dom";
import { useState, useEffect, useMemo } from "react";
import { manPageGroups, getManPageBySlug } from "../data/manpages";
import MarkdownRenderer from "./MarkdownRenderer";

// Convert `zag man <command>` references to internal links
function preprocessContent(content: string): string {
  return content
    // Indented "zag man <command> ..." lines (See Also sections)
    .replace(/^ {4}zag man ([\w-]+)\s+.*$/gm, "- [zag man $1](/manual/$1)")
    // Standalone indented "zag man <command>" lines
    .replace(/^ {4}zag man ([\w-]+)\s*$/gm, "- [zag man $1](/manual/$1)")
    // Inline backtick references: `zag man <command>`
    .replace(/`zag man ([\w-]+)`/g, "[`zag man $1`](/manual/$1)");
}

export default function Manual() {
  const { slug } = useParams<{ slug: string }>();
  const [sidebarOpen, setSidebarOpen] = useState(false);

  const currentSlug = slug || "zag";
  const currentPage = getManPageBySlug(currentSlug);

  const processedContent = useMemo(
    () => (currentPage ? preprocessContent(currentPage.content) : ""),
    [currentPage],
  );

  useEffect(() => {
    window.scrollTo(0, 0);
  }, [currentSlug]);

  if (!currentPage) {
    return <Navigate to="/manual/zag" replace />;
  }

  return (
    <div className="min-h-screen pt-[73px]">
      {/* Mobile sidebar toggle */}
      <div className="sticky top-[73px] z-40 border-b border-border bg-surface/95 backdrop-blur-sm px-4 py-3 lg:hidden">
        <button
          onClick={() => setSidebarOpen(!sidebarOpen)}
          className="flex items-center gap-2 text-sm text-text-secondary hover:text-text-primary transition-colors"
        >
          <svg className="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
            <path strokeLinecap="round" strokeLinejoin="round" d={sidebarOpen ? "M6 18L18 6M6 6l12 12" : "M4 6h16M4 12h16M4 18h16"} />
          </svg>
          {currentPage.title}
        </button>
      </div>

      {/* Backdrop for mobile sidebar */}
      {sidebarOpen && (
        <div
          className="fixed inset-0 z-20 bg-black/50 lg:hidden"
          onClick={() => setSidebarOpen(false)}
        />
      )}

      <div className="mx-auto flex max-w-7xl">
        {/* Sidebar */}
        <aside
          className={`
            fixed top-[73px] bottom-0 z-30 w-64 shrink-0 overflow-y-auto border-r border-border bg-surface px-4 py-6
            transition-transform duration-200 ease-in-out
            lg:sticky lg:translate-x-0 lg:block
            ${sidebarOpen ? "translate-x-0" : "-translate-x-full"}
          `}
        >
          <nav className="space-y-4">
            {manPageGroups.map((group) => (
              <div key={group.label}>
                <div className="px-3 pb-1 text-xs font-semibold uppercase tracking-wider text-text-secondary">
                  {group.label}
                </div>
                <div className="space-y-0.5">
                  {group.pages.map((page) => (
                    <Link
                      key={page.slug}
                      to={`/manual/${page.slug}`}
                      onClick={() => setSidebarOpen(false)}
                      className={`
                        block rounded-md px-3 py-1.5 text-sm transition-colors
                        ${page.slug === currentSlug
                          ? "bg-accent/10 text-accent font-medium"
                          : "text-text-secondary hover:bg-surface-hover hover:text-text-primary"
                        }
                      `}
                    >
                      {page.title}
                    </Link>
                  ))}
                </div>
              </div>
            ))}
          </nav>
        </aside>

        {/* Main content */}
        <main className="min-w-0 flex-1 px-6 py-8 lg:px-12 lg:py-10">
          <MarkdownRenderer content={processedContent} basePath="/manual" />
        </main>
      </div>
    </div>
  );
}
