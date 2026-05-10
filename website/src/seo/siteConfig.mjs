// Single source of truth for SEO copy and configuration (OSS_SPEC §11.3).
//
// Imported by:
//   - src/hooks/useSeo.ts             (runtime <head> updates after hydration)
//   - src/App.tsx, components/*       (per-route SEO calls)
//   - scripts/seo.mjs                 (build-time per-route HTML generation)
//   - scripts/og-image.mjs            (build-time og-default.png rendering)
//
// Tweaking the site's pitch should be a one-file change. Add new fields here
// rather than hard-coding strings in components or build scripts.

export const siteConfig = {
  /** Canonical site origin, no trailing slash. */
  origin: "https://zag.niclaslindstedt.se",

  /** Short brand name used in titles, breadcrumbs, og:site_name. */
  siteName: "zag",

  /** One-line tagline. Reused in OG, hero copy, etc. */
  tagline: "One CLI for all your AI coding agents",

  /** Long-form site description used as the homepage meta description. */
  description:
    "zag is an open-source unified CLI and SDK for AI coding agents. Switch between Claude Code, OpenAI Codex, Google Gemini, GitHub Copilot, and Ollama with one command. Orchestrate multi-agent workflows from Rust, TypeScript, Python, C#, Swift, Java, or Kotlin.",

  /** Homepage <title>. */
  title:
    "zag — One CLI for Claude, Codex, Gemini, Copilot & Ollama AI coding agents",

  /** ISO language code. */
  language: "en",

  /** Author identity. */
  author: {
    name: "Niclas Lindstedt",
    url: "https://niclaslindstedt.se/",
    schemaId: "https://niclaslindstedt.se/#person",
  },

  /** GitHub repository slug + URLs. */
  repo: {
    slug: "niclaslindstedt/zag",
    url: "https://github.com/niclaslindstedt/zag",
    issuesUrl: "https://github.com/niclaslindstedt/zag/issues",
    releasesAtom: "https://github.com/niclaslindstedt/zag/releases.atom",
    licenseUrl: "https://github.com/niclaslindstedt/zag/blob/main/LICENSE",
  },

  /** Distribution metadata. */
  distribution: {
    cratesUrl: "https://crates.io/crates/zag-cli",
    cratesLibUrl: "https://crates.io/crates/zag",
  },

  /** Open Graph image. The PNG is the spec-compliant default; SVG is a
   *  bonus that platforms like GitHub/Discord render natively. */
  ogImage: {
    pngPath: "/og-default.png",
    svgPath: "/og-image.svg",
    width: 1200,
    height: 630,
    type: "image/png",
    alt: "zag — One CLI for all your AI coding agents",
  },

  /** Default robots directive (per OSS_SPEC §11.3). */
  robots:
    "index, follow, max-image-preview:large, max-snippet:-1, max-video-preview:-1",

  /** Long-tail keywords for the meta keywords tag. Adapted to the project's
   *  audience: programmers searching for AI coding tools. */
  keywords: [
    "zag",
    "AI coding agent",
    "AI CLI",
    "Claude Code CLI",
    "OpenAI Codex CLI",
    "Gemini CLI",
    "GitHub Copilot CLI",
    "Ollama CLI",
    "AI agent orchestration",
    "multi-agent",
    "agent SDK",
    "agent framework",
    "Rust agent library",
    "TypeScript agent SDK",
    "Python agent SDK",
    "AI coding tools",
    "LLM CLI",
    "LLM orchestration",
    "AI pair programming",
    "agentic coding",
    "MCP",
    "Model Context Protocol",
    "Cursor alternative",
    "Aider alternative",
    "autonomous coding agent",
  ],

  /** Provider names rendered on the OG image and shown in marketing copy. */
  providers: ["Claude", "Codex", "Gemini", "Copilot", "Ollama"],

  /** Sitemap and feed paths (always rooted at the site origin). */
  paths: {
    sitemap: "/sitemap.xml",
    robots: "/robots.txt",
  },
};

/** Absolute URL helper. */
export function abs(path) {
  if (!path) return siteConfig.origin + "/";
  if (/^https?:/.test(path)) return path;
  return siteConfig.origin + (path.startsWith("/") ? path : `/${path}`);
}

/** Default Open Graph image URL (PNG, spec-compliant). */
export const defaultOgImage = abs(siteConfig.ogImage.pngPath);
