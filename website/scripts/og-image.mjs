#!/usr/bin/env node
// Build-time generator for the 1200×630 default Open Graph image
// (OSS_SPEC §11.3). Output: website/public/og-default.png so Vite copies it
// into dist/. Driven entirely by src/seo/siteConfig.mjs — tweaking the site's
// pitch never requires opening an image editor.

import { writeFileSync, existsSync, mkdirSync } from "fs";
import { dirname, join, resolve } from "path";
import { fileURLToPath } from "url";

import satori from "satori";
import { Resvg } from "@resvg/resvg-js";

import { siteConfig } from "../src/seo/siteConfig.mjs";

const __dirname = dirname(fileURLToPath(import.meta.url));
const WEBSITE_DIR = resolve(__dirname, "..");
const OUT = join(WEBSITE_DIR, "public", "og-default.png");

const W = siteConfig.ogImage.width;
const H = siteConfig.ogImage.height;

// Satori needs at least one font registered. Use the bundled Vercel font
// fetched at build time once; cache to a temp file across runs to keep CI
// builds quick. We fetch from the npm-distributed @vercel/og style fonts via
// a CDN URL that is stable — but to avoid runtime fetching, we ship an SVG
// fallback through the script and ask satori to use a `default` family that
// resvg can render with its built-in font collection.
//
// Practical approach: download Inter (open-source, OFL) from a versioned
// jsDelivr URL when fonts/ is empty, then reuse on subsequent runs.

import { readFileSync } from "fs";
const FONT_DIR = join(WEBSITE_DIR, "scripts", ".fonts");
const FONT_REG = join(FONT_DIR, "inter-regular.ttf");
const FONT_BLD = join(FONT_DIR, "inter-bold.ttf");
const FONT_REG_URL = "https://cdn.jsdelivr.net/npm/@fontsource/inter@5.0.18/files/inter-latin-400-normal.woff";
const FONT_BLD_URL = "https://cdn.jsdelivr.net/npm/@fontsource/inter@5.0.18/files/inter-latin-700-normal.woff";

async function ensureFont(path, url) {
  if (existsSync(path)) return readFileSync(path);
  if (!existsSync(FONT_DIR)) mkdirSync(FONT_DIR, { recursive: true });
  const res = await fetch(url);
  if (!res.ok) throw new Error(`Failed to download font ${url}: ${res.status}`);
  const buf = Buffer.from(await res.arrayBuffer());
  writeFileSync(path, buf);
  return buf;
}

const card = {
  type: "div",
  props: {
    style: {
      width: "100%",
      height: "100%",
      display: "flex",
      flexDirection: "column",
      justifyContent: "space-between",
      padding: "76px 80px",
      background:
        "linear-gradient(135deg, #0a0a0d 0%, #14141b 100%)",
      color: "#f8fafc",
      fontFamily: "Inter",
    },
    children: [
      {
        type: "div",
        props: {
          style: { display: "flex", alignItems: "center", gap: 16 },
          children: [
            { type: "div", props: { style: { fontSize: 88, color: "#ff7a18" }, children: "⚡" } },
            { type: "div", props: { style: { fontSize: 88, fontWeight: 700 }, children: siteConfig.siteName } },
            {
              type: "div",
              props: {
                style: {
                  marginLeft: 24, fontSize: 22, color: "#9ca3af",
                  border: "1px solid #334155", borderRadius: 999,
                  padding: "6px 14px",
                },
                children: "open source · MIT",
              },
            },
          ],
        },
      },
      {
        type: "div",
        props: {
          style: { display: "flex", flexDirection: "column", gap: 8 },
          children: [
            { type: "div", props: { style: { fontSize: 80, fontWeight: 700, lineHeight: 1.05 }, children: "One CLI for all your" } },
            {
              type: "div",
              props: {
                style: {
                  fontSize: 80, fontWeight: 700, lineHeight: 1.05,
                  background: "linear-gradient(90deg, #ff7a18 0%, #ffb066 100%)",
                  backgroundClip: "text",
                  color: "transparent",
                },
                children: "AI coding agents",
              },
            },
          ],
        },
      },
      {
        type: "div",
        props: {
          style: { display: "flex", alignItems: "center", justifyContent: "space-between" },
          children: [
            {
              type: "div",
              props: {
                style: { display: "flex", gap: 12 },
                children: siteConfig.providers.map((p) => ({
                  type: "div",
                  props: {
                    style: {
                      fontSize: 24, fontWeight: 600,
                      padding: "10px 22px", borderRadius: 999,
                      border: "1px solid #334155", background: "#1f2937",
                      color: "#f8fafc",
                    },
                    children: p,
                  },
                })),
              },
            },
            {
              type: "div",
              props: {
                style: { fontSize: 22, color: "#9ca3af" },
                children: siteConfig.origin.replace(/^https?:\/\//, ""),
              },
            },
          ],
        },
      },
      {
        type: "div",
        props: {
          style: {
            fontSize: 28, fontFamily: "Inter",
            padding: "16px 22px", borderRadius: 10,
            background: "#0f172a", border: "1px solid #334155",
            alignSelf: "flex-start", color: "#e2e8f0",
          },
          children: "$ cargo install zag-cli",
        },
      },
    ],
  },
};

async function main() {
  const [reg, bld] = await Promise.all([
    ensureFont(FONT_REG, FONT_REG_URL),
    ensureFont(FONT_BLD, FONT_BLD_URL),
  ]);

  const svg = await satori(card, {
    width: W,
    height: H,
    fonts: [
      { name: "Inter", data: reg, weight: 400, style: "normal" },
      { name: "Inter", data: bld, weight: 700, style: "normal" },
    ],
  });

  const png = new Resvg(svg, { fitTo: { mode: "width", value: W } }).render().asPng();

  if (!existsSync(dirname(OUT))) mkdirSync(dirname(OUT), { recursive: true });
  writeFileSync(OUT, png);
  console.log(`og-image.mjs: wrote ${OUT} (${W}×${H})`);
}

main().catch((err) => {
  console.error("og-image.mjs: failed to render og-default.png");
  console.error(err);
  process.exit(1);
});
