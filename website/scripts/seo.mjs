#!/usr/bin/env node
// Post-build SEO step: per-route prerendered HTML + sitemap.xml + verification.
//
// Implements OSS_SPEC §11.3:
//   - Required <head> metadata per route (title, description, canonical, robots,
//     full Open Graph + Twitter Card set, route-specific JSON-LD).
//   - Sitemap with real <lastmod> derived from `git log` of each source file
//     (NOT a build-time `now()`).
//   - Pre-rendered metadata for the SPA: every public route gets its own
//     dist/<route>/index.html with route-specific <head> spliced in. Body
//     remains the framework's hydration root.
//   - dist/404.html as the homepage copy (SPA fallback).
//   - CI verification: fails with non-zero exit if any required output
//     (sitemap.xml, robots.txt, homepage JSON-LD, per-route <title> +
//     canonical) is missing.
//
// All copy and configuration is read from src/seo/siteConfig.mjs (SSOT).

import {
  readFileSync, writeFileSync, readdirSync, mkdirSync, existsSync, copyFileSync,
  statSync,
} from "fs";
import { dirname, join, resolve, relative } from "path";
import { fileURLToPath } from "url";
import { execSync } from "child_process";

import { siteConfig, defaultOgImage } from "../src/seo/siteConfig.mjs";

const __dirname = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(__dirname, "../..");
const WEBSITE_DIR = resolve(__dirname, "..");
const DIST = join(WEBSITE_DIR, "dist");
const DOCS_DIR = join(REPO_ROOT, "docs");
const MAN_DIR = join(REPO_ROOT, "zag-agent/man");

const SITE = siteConfig.origin;

if (!existsSync(DIST)) {
  console.error(`seo.mjs: ${DIST} not found — run vite build first.`);
  process.exit(1);
}

const template = readFileSync(join(DIST, "index.html"), "utf-8");

// ---------------------------------------------------------------------------
// Slug → display title overrides for manpages (must mirror data/manpages.ts).
// ---------------------------------------------------------------------------
const MAN_TITLE_OVERRIDES = {
  zag: "zag",
  "help-agent": "zag --help-agent",
};

const DOC_TITLE_OVERRIDES = {
  "getting-started": "Getting Started",
  "skills-and-mcp": "Skills & MCP",
  "events-and-logging": "Events & Logging",
  "remote-access": "Remote Access",
  "language-bindings": "Language Bindings",
};

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

function titleCase(slug) {
  return slug.split("-").map((s) => s.charAt(0).toUpperCase() + s.slice(1)).join(" ");
}

function readMarkdown(path) {
  try { return readFileSync(path, "utf-8"); } catch { return ""; }
}

function firstParagraph(md, fallback) {
  for (const raw of md.split("\n")) {
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

function manSummary(md, slug, title, fallback) {
  const desc = md.match(/##\s+DESCRIPTION\s*\n+([\s\S]*?)(?:\n##|$)/i);
  const nameLine = md.match(/##\s+NAME\s*\n+([\s\S]*?)(?:\n##|$)/i);
  const candidate = (desc?.[1] || nameLine?.[1] || "").trim();
  if (!candidate) return fallback;
  const cleaned = candidate
    .split("\n").map((l) => l.trim()).filter(Boolean).join(" ")
    .replace(/`([^`]+)`/g, "$1")
    .replace(/\*\*([^*]+)\*\*/g, "$1")
    .replace(/\*([^*]+)\*/g, "$1")
    .replace(/\s+/g, " ").trim();
  if (cleaned.length < 40) return fallback;
  return cleaned.length > 200 ? cleaned.slice(0, 197) + "…" : cleaned;
}

function escapeHtml(s) {
  return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;").replace(/'/g, "&#39;");
}
function escapeXml(s) {
  return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;").replace(/'/g, "&apos;");
}

// Real <lastmod> from `git log` (OSS_SPEC §11.3 — not now()).
const lastModCache = new Map();
function gitLastMod(...filesRelToRepo) {
  const key = filesRelToRepo.join("|");
  if (lastModCache.has(key)) return lastModCache.get(key);
  let latest = null;
  for (const file of filesRelToRepo) {
    try {
      const iso = execSync(
        `git log -1 --format=%cI -- "${file}"`,
        { cwd: REPO_ROOT, encoding: "utf-8", stdio: ["pipe", "pipe", "pipe"] },
      ).trim();
      if (!iso) continue;
      if (!latest || iso > latest) latest = iso;
    } catch {
      // git not available or file untracked — ignore, we'll fall back below
    }
  }
  if (!latest) {
    // Fallback: file mtime of the first existing file. We deliberately do NOT
    // use `now()` here per OSS_SPEC §11.3.
    for (const file of filesRelToRepo) {
      const abs = join(REPO_ROOT, file);
      if (existsSync(abs)) {
        latest = statSync(abs).mtime.toISOString();
        break;
      }
    }
  }
  if (!latest) latest = "1970-01-01T00:00:00Z";
  const day = latest.slice(0, 10);
  lastModCache.set(key, day);
  return day;
}

// ---------------------------------------------------------------------------
// Discover routes
// ---------------------------------------------------------------------------

function listMarkdown(dir) {
  if (!existsSync(dir)) return [];
  return readdirSync(dir).filter((f) => f.endsWith(".md")).map((f) => f.replace(/\.md$/, ""));
}

const docSlugs = listMarkdown(DOCS_DIR);
const manSlugs = listMarkdown(MAN_DIR);

// Manpages explicitly included in the website navigation (mirror data/manpages.ts).
const MANPAGES_INCLUDED = new Set([
  "zag", "help-agent", "man",
  "run", "exec", "review", "config",
  "session", "listen", "search", "input", "output", "status", "log", "events", "summary",
  "orchestration", "spawn", "wait", "collect", "pipe", "cancel", "retry",
  "broadcast", "watch", "subscribe",
  "ps", "gc", "env", "whoami",
  "skills", "mcp", "capability",
  "serve", "connect",
]);

const docRoutes = docSlugs.map((slug) => {
  const file = `docs/${slug}.md`;
  const md = readMarkdown(join(DOCS_DIR, `${slug}.md`));
  const title = DOC_TITLE_OVERRIDES[slug] || titleCase(slug);
  return {
    kind: "doc",
    slug,
    path: `/docs/${slug}/`,
    canonical: `${SITE}/docs/${slug}`,
    title: `${title} — zag documentation`,
    description: firstParagraph(
      md,
      `${title} documentation for zag — the unified CLI for ${siteConfig.providers.join(", ")} AI coding agents.`,
    ),
    priority: "0.7",
    lastmod: gitLastMod(file),
    schemaType: "TechArticle",
  };
});

const docIndex = (() => {
  const slug = "getting-started";
  const md = readMarkdown(join(DOCS_DIR, `${slug}.md`));
  return {
    kind: "doc",
    slug: "",
    path: "/docs/",
    canonical: `${SITE}/docs/`,
    title: "Documentation — zag",
    description: firstParagraph(
      md,
      "Documentation for zag, the unified CLI and SDK for AI coding agents.",
    ),
    priority: "0.8",
    lastmod: gitLastMod(`docs/${slug}.md`),
    schemaType: "CollectionPage",
  };
})();

const manRoutes = manSlugs
  .filter((slug) => MANPAGES_INCLUDED.has(slug))
  .map((slug) => {
    const file = `zag-agent/man/${slug}.md`;
    const md = readMarkdown(join(MAN_DIR, `${slug}.md`));
    const title = MAN_TITLE_OVERRIDES[slug] || `zag ${slug}`;
    return {
      kind: "man",
      slug,
      path: `/manual/${slug}/`,
      canonical: `${SITE}/manual/${slug}`,
      title: `${title} — zag manual`,
      description: manSummary(
        md, slug, title,
        `Reference manual for ${title} in zag, the unified CLI for ${siteConfig.providers.join(", ")} AI coding agents.`,
      ),
      priority: "0.6",
      lastmod: gitLastMod(file),
      schemaType: "TechArticle",
    };
  });

const manIndex = {
  kind: "man",
  slug: "",
  path: "/manual/",
  canonical: `${SITE}/manual/`,
  title: "Manual — zag",
  description:
    "Reference manual pages for the zag CLI, covering every command and subcommand for orchestrating " +
    `${siteConfig.providers.join(", ")} agents.`,
  priority: "0.7",
  lastmod: gitLastMod("zag-agent/man/zag.md", "zag-cli/src/cli.rs"),
  schemaType: "CollectionPage",
};

const home = {
  kind: "home",
  slug: "",
  path: "/",
  canonical: `${SITE}/`,
  title: siteConfig.title,
  description: siteConfig.description,
  priority: "1.0",
  lastmod: gitLastMod("zag-cli/Cargo.toml", "README.md"),
  schemaType: "WebSite",
};

const allRoutes = [home, docIndex, ...docRoutes, manIndex, ...manRoutes];

// ---------------------------------------------------------------------------
// Per-route HTML rewriting
// ---------------------------------------------------------------------------

function rewriteHtml(route) {
  let html = template;

  // <title>
  html = html.replace(
    /<title>[\s\S]*?<\/title>/i,
    `<title>${escapeHtml(route.title)}</title>`,
  );

  const setMetaTag = (attr, key, value) => {
    const re = new RegExp(`<meta\\s+${attr}="${key}"[^>]*>`, "i");
    const tag = `<meta ${attr}="${key}" content="${escapeHtml(value)}" />`;
    if (re.test(html)) {
      html = html.replace(re, tag);
    } else {
      html = html.replace(/<\/head>/i, `    ${tag}\n  </head>`);
    }
  };

  setMetaTag("name", "description", route.description);
  setMetaTag("name", "robots", siteConfig.robots);
  setMetaTag("property", "og:site_name", siteConfig.siteName);
  setMetaTag("property", "og:title", route.title);
  setMetaTag("property", "og:description", route.description);
  setMetaTag("property", "og:url", route.canonical);
  setMetaTag("property", "og:type", route.kind === "home" ? "website" : "article");
  setMetaTag("property", "og:image", defaultOgImage);
  setMetaTag("property", "og:image:width", String(siteConfig.ogImage.width));
  setMetaTag("property", "og:image:height", String(siteConfig.ogImage.height));
  setMetaTag("property", "og:image:alt", siteConfig.ogImage.alt);
  setMetaTag("name", "twitter:card", "summary_large_image");
  setMetaTag("name", "twitter:title", route.title);
  setMetaTag("name", "twitter:description", route.description);
  setMetaTag("name", "twitter:image", defaultOgImage);

  // canonical
  html = html.replace(
    /<link\s+rel="canonical"[^>]*>/i,
    `<link rel="canonical" href="${escapeHtml(route.canonical)}" />`,
  );

  // Per-route JSON-LD (article pages get TechArticle / CollectionPage; homepage
  // already has the rich @graph injected by Vite, so skip there).
  if (route.kind !== "home") {
    const articleLd = {
      "@context": "https://schema.org",
      "@type": route.schemaType,
      headline: route.title,
      name: route.title,
      description: route.description,
      url: route.canonical,
      inLanguage: siteConfig.language,
      isPartOf: { "@id": `${SITE}/#website` },
      about: { "@id": `${SITE}/#software` },
      author: { "@type": "Person", name: siteConfig.author.name, url: siteConfig.author.url },
      datePublished: route.lastmod,
      dateModified: route.lastmod,
    };
    html = html.replace(
      /<\/head>/i,
      `    <script type="application/ld+json" data-route="1">\n${JSON.stringify(articleLd, null, 2)}\n    </script>\n  </head>`,
    );
  }

  return html;
}

function writeRouteHtml(route, html) {
  if (route.path === "/") {
    writeFileSync(join(DIST, "index.html"), html, "utf-8");
    writeFileSync(join(DIST, "404.html"), html, "utf-8");
    return;
  }
  const dir = join(DIST, route.path.replace(/^\/|\/$/g, ""));
  mkdirSync(dir, { recursive: true });
  writeFileSync(join(dir, "index.html"), html, "utf-8");
}

let written = 0;
for (const route of allRoutes) {
  const html = rewriteHtml(route);
  writeRouteHtml(route, html);
  written++;
}

// ---------------------------------------------------------------------------
// sitemap.xml
// ---------------------------------------------------------------------------

const sitemap =
  `<?xml version="1.0" encoding="UTF-8"?>\n` +
  `<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n` +
  allRoutes.map((r) =>
    `  <url>\n` +
    `    <loc>${escapeXml(r.canonical)}</loc>\n` +
    `    <lastmod>${r.lastmod}</lastmod>\n` +
    `    <changefreq>${r.kind === "home" ? "weekly" : "monthly"}</changefreq>\n` +
    `    <priority>${r.priority}</priority>\n` +
    `  </url>`,
  ).join("\n") +
  `\n</urlset>\n`;

writeFileSync(join(DIST, "sitemap.xml"), sitemap, "utf-8");

// Make sure SVG fallback is exposed (Vite's public/ handling already copies it,
// but be defensive in case og-image.svg is removed later).
const svgSrc = join(WEBSITE_DIR, "public", "og-image.svg");
const svgDst = join(DIST, "og-image.svg");
if (existsSync(svgSrc) && !existsSync(svgDst)) copyFileSync(svgSrc, svgDst);

console.log(`seo.mjs: wrote ${written} HTML files`);
console.log(`  Home: 1`);
console.log(`  Docs: ${docRoutes.length + 1}`);
console.log(`  Manual: ${manRoutes.length + 1}`);
console.log(`  Sitemap: ${relative(WEBSITE_DIR, join(DIST, "sitemap.xml"))} (${allRoutes.length} URLs)`);

// ---------------------------------------------------------------------------
// Verification (OSS_SPEC §11.3 — CI must fail on missing SEO outputs).
// ---------------------------------------------------------------------------

const errors = [];

function mustExist(path, label) {
  if (!existsSync(path)) errors.push(`Missing required output: ${label} (${relative(WEBSITE_DIR, path)})`);
}

mustExist(join(DIST, "sitemap.xml"), "sitemap.xml");
mustExist(join(DIST, "robots.txt"), "robots.txt");
mustExist(join(DIST, "og-default.png"), "og-default.png");

const homepage = readFileSync(join(DIST, "index.html"), "utf-8");
if (!/<script type="application\/ld\+json">/.test(homepage)) {
  errors.push("Homepage is missing required <script type=\"application/ld+json\"> block");
}

for (const route of allRoutes) {
  const file = route.path === "/"
    ? join(DIST, "index.html")
    : join(DIST, route.path.replace(/^\/|\/$/g, ""), "index.html");
  if (!existsSync(file)) {
    errors.push(`Missing pre-rendered file for ${route.canonical}: ${relative(WEBSITE_DIR, file)}`);
    continue;
  }
  const html = readFileSync(file, "utf-8");
  if (!/<title>.+<\/title>/.test(html)) {
    errors.push(`${route.canonical}: missing <title>`);
  }
  if (!/<link\s+rel="canonical"[^>]*href="[^"]+"/.test(html)) {
    errors.push(`${route.canonical}: missing canonical <link>`);
  }
  if (!/<meta\s+name="description"[^>]*content="[^"]+"/.test(html)) {
    errors.push(`${route.canonical}: missing <meta name="description">`);
  }
}

if (errors.length) {
  console.error("\nseo.mjs: SEO verification failed:");
  for (const e of errors) console.error(`  - ${e}`);
  process.exit(1);
}

console.log("seo.mjs: verification passed.");
