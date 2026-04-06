import Markdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { Link } from "react-router-dom";
import type { Components } from "react-markdown";
import type { ReactNode } from "react";
import CodeBlock from "./CodeBlock";

interface MarkdownRendererProps {
  content: string;
  basePath?: string;
}

function slugify(text: string): string {
  return text
    .toLowerCase()
    .replace(/[^\w\s-]/g, "")
    .replace(/\s+/g, "-")
    .replace(/-+/g, "-")
    .replace(/^-|-$/g, "");
}

function extractText(children: ReactNode): string {
  if (typeof children === "string") return children;
  if (Array.isArray(children)) return children.map(extractText).join("");
  if (children && typeof children === "object" && "props" in children) {
    return extractText(children.props.children);
  }
  return String(children ?? "");
}

function heading(Tag: "h1" | "h2" | "h3" | "h4" | "h5" | "h6") {
  return function HeadingComponent({ children }: { children?: ReactNode }) {
    const id = slugify(extractText(children));
    return <Tag id={id}>{children}</Tag>;
  };
}

function makeComponents(basePath: string): Components {
  return {
    pre: CodeBlock,
    h1: heading("h1"),
    h2: heading("h2"),
    h3: heading("h3"),
    h4: heading("h4"),
    h5: heading("h5"),
    h6: heading("h6"),
    a({ href, children }) {
      if (href && href.endsWith(".md")) {
        const slug = href.replace(/\.md$/, "");
        return (
          <Link to={`${basePath}/${slug}`} className="text-accent hover:text-accent-light transition-colors underline">
            {children}
          </Link>
        );
      }

      const isExternal = href && (href.startsWith("http://") || href.startsWith("https://"));
      return (
        <a
          href={href}
          className="text-accent hover:text-accent-light transition-colors underline"
          {...(isExternal ? { target: "_blank", rel: "noopener noreferrer" } : {})}
        >
          {children}
        </a>
      );
    },
  };
}

export default function MarkdownRenderer({ content, basePath = "/docs" }: MarkdownRendererProps) {
  const components = makeComponents(basePath);
  return (
    <div className="markdown-content">
      <Markdown remarkPlugins={[remarkGfm]} components={components}>
        {content}
      </Markdown>
    </div>
  );
}
