import Markdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { Link } from "react-router-dom";
import type { Components } from "react-markdown";
import CodeBlock from "./CodeBlock";

interface MarkdownRendererProps {
  content: string;
}

const components: Components = {
  pre: CodeBlock,
  a({ href, children }) {
    if (href && href.endsWith(".md")) {
      const slug = href.replace(/\.md$/, "");
      return (
        <Link to={`/docs/${slug}`} className="text-accent hover:text-accent-light transition-colors underline">
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

export default function MarkdownRenderer({ content }: MarkdownRendererProps) {
  return (
    <div className="markdown-content">
      <Markdown remarkPlugins={[remarkGfm]} components={components}>
        {content}
      </Markdown>
    </div>
  );
}
