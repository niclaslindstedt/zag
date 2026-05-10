import { useParams, Link, Navigate } from "react-router-dom";
import { useState, useEffect } from "react";
import { docs, getDocBySlug } from "../data/docs";
import MarkdownRenderer from "./MarkdownRenderer";
import { useSeo } from "../hooks/useSeo";

function firstParagraph(md: string, fallback: string): string {
  const lines = md.split("\n");
  for (const raw of lines) {
    const line = raw.trim();
    if (!line) continue;
    if (line.startsWith("#")) continue;
    if (line.startsWith("```") || line.startsWith("---")) continue;
    if (line.startsWith("|") || line.startsWith("- ") || line.startsWith("* ")) continue;
    const stripped = line
      .replace(/!?\[([^\]]*)\]\([^)]*\)/g, "$1")
      .replace(/`([^`]+)`/g, "$1")
      .replace(/\*\*([^*]+)\*\*/g, "$1")
      .replace(/\*([^*]+)\*/g, "$1")
      .trim();
    if (stripped.length < 40) continue;
    return stripped.length > 200 ? stripped.slice(0, 197) + "…" : stripped;
  }
  return fallback;
}

export default function Documentation() {
  const { slug } = useParams<{ slug: string }>();
  const [sidebarOpen, setSidebarOpen] = useState(false);

  const currentSlug = slug || "getting-started";
  const currentDoc = getDocBySlug(currentSlug);

  const seoTitle = currentDoc
    ? `${currentDoc.title} — zag documentation`
    : "zag documentation";
  const seoDescription = currentDoc
    ? firstParagraph(
        currentDoc.content,
        `${currentDoc.title} documentation for zag — the unified CLI for Claude, Codex, Gemini, Copilot, and Ollama.`,
      )
    : "Documentation for zag, the unified CLI and SDK for AI coding agents.";

  useSeo({
    title: seoTitle,
    description: seoDescription,
    path: `/docs/${currentSlug}`,
    type: "article",
    jsonLd: currentDoc
      ? {
          "@context": "https://schema.org",
          "@type": "TechArticle",
          headline: `${currentDoc.title} — zag documentation`,
          name: currentDoc.title,
          description: seoDescription,
          url: `https://zag.niclaslindstedt.se/docs/${currentSlug}`,
          inLanguage: "en",
          isPartOf: { "@id": "https://zag.niclaslindstedt.se/#website" },
          about: { "@id": "https://zag.niclaslindstedt.se/#software" },
          author: { "@type": "Person", name: "Niclas Lindstedt" },
        }
      : undefined,
  });

  useEffect(() => {
    window.scrollTo(0, 0);
  }, [currentSlug]);

  if (!currentDoc) {
    return <Navigate to="/docs/getting-started" replace />;
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
          {currentDoc.title}
        </button>
      </div>

      {/* Backdrop for mobile sidebar */}
      {sidebarOpen && (
        <div
          className="fixed inset-0 z-40 bg-black/50 lg:hidden"
          onClick={() => setSidebarOpen(false)}
        />
      )}

      <div className="mx-auto flex max-w-7xl">
        {/* Sidebar */}
        <aside
          className={`
            fixed top-[118px] bottom-0 z-40 w-full shrink-0 overflow-y-auto border-r border-border bg-surface px-4 py-6
            transition-transform duration-200 ease-in-out
            sm:w-72
            lg:sticky lg:top-[73px] lg:w-64 lg:translate-x-0 lg:block
            ${sidebarOpen ? "translate-x-0" : "-translate-x-full"}
          `}
        >
          <nav className="space-y-1">
            {docs.map((doc) => (
              <Link
                key={doc.slug}
                to={`/docs/${doc.slug}`}
                onClick={() => setSidebarOpen(false)}
                className={`
                  block rounded-md px-3 py-2 text-sm transition-colors
                  ${doc.slug === currentSlug
                    ? "bg-accent/10 text-accent font-medium"
                    : "text-text-secondary hover:bg-surface-hover hover:text-text-primary"
                  }
                `}
              >
                {doc.title}
              </Link>
            ))}
          </nav>
        </aside>

        {/* Main content */}
        <main className="min-w-0 flex-1 px-6 py-8 lg:px-12 lg:py-10">
          <MarkdownRenderer content={currentDoc.content} />
        </main>
      </div>
    </div>
  );
}
