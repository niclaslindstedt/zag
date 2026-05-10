import { useEffect } from "react";
// @ts-expect-error - .mjs import shared with build scripts (no .d.ts needed)
import { siteConfig, defaultOgImage, abs } from "../seo/siteConfig.mjs";

export interface SeoOptions {
  title: string;
  description: string;
  /** Path-only (e.g. "/docs/getting-started"). Joined with the site origin. */
  path: string;
  /** Optional Open Graph image override (absolute URL). */
  image?: string;
  /** Defaults to "article" for inner pages, "website" for landing. */
  type?: "website" | "article";
  /** Robots meta. Defaults to siteConfig.robots. */
  robots?: string;
  /** Optional JSON-LD object that will be injected/replaced. */
  jsonLd?: Record<string, unknown>;
}

const MANAGED_ATTR = "data-managed-seo";

function setMeta(selector: string, attr: "name" | "property", key: string, value: string) {
  let el = document.head.querySelector<HTMLMetaElement>(selector);
  if (!el) {
    el = document.createElement("meta");
    el.setAttribute(attr, key);
    el.setAttribute(MANAGED_ATTR, "1");
    document.head.appendChild(el);
  }
  el.setAttribute("content", value);
}

function setName(name: string, value: string) {
  setMeta(`meta[name="${name}"]`, "name", name, value);
}

function setProperty(property: string, value: string) {
  setMeta(`meta[property="${property}"]`, "property", property, value);
}

function setCanonical(href: string) {
  let el = document.head.querySelector<HTMLLinkElement>('link[rel="canonical"]');
  if (!el) {
    el = document.createElement("link");
    el.setAttribute("rel", "canonical");
    el.setAttribute(MANAGED_ATTR, "1");
    document.head.appendChild(el);
  }
  el.setAttribute("href", href);
}

function setJsonLd(data: Record<string, unknown> | undefined) {
  const existing = document.head.querySelector<HTMLScriptElement>(
    `script[type="application/ld+json"][${MANAGED_ATTR}="route"]`,
  );
  if (!data) {
    existing?.remove();
    return;
  }
  const node = existing ?? document.createElement("script");
  node.setAttribute("type", "application/ld+json");
  node.setAttribute(MANAGED_ATTR, "route");
  node.textContent = JSON.stringify(data);
  if (!existing) document.head.appendChild(node);
}

export function useSeo(opts: SeoOptions) {
  const {
    title,
    description,
    path,
    image = defaultOgImage,
    type = path === "/" ? "website" : "article",
    robots = siteConfig.robots,
    jsonLd,
  } = opts;

  useEffect(() => {
    const url = abs(path);

    document.title = title;
    setName("description", description);
    setName("robots", robots);
    setCanonical(url);

    setProperty("og:site_name", siteConfig.siteName);
    setProperty("og:type", type);
    setProperty("og:title", title);
    setProperty("og:description", description);
    setProperty("og:url", url);
    setProperty("og:image", image);
    setProperty("og:image:alt", siteConfig.ogImage.alt);

    setName("twitter:card", "summary_large_image");
    setName("twitter:title", title);
    setName("twitter:description", description);
    setName("twitter:image", image);

    setJsonLd(jsonLd);
  }, [title, description, path, image, type, robots, jsonLd]);
}
