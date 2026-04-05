import gettingStarted from "../../../docs/getting-started.md?raw";
import configuration from "../../../docs/configuration.md?raw";
import providers from "../../../docs/providers.md?raw";
import orchestration from "../../../docs/orchestration.md?raw";
import sessions from "../../../docs/sessions.md?raw";
import isolation from "../../../docs/isolation.md?raw";
import skillsAndMcp from "../../../docs/skills-and-mcp.md?raw";
import eventsAndLogging from "../../../docs/events-and-logging.md?raw";
import remoteAccess from "../../../docs/remote-access.md?raw";
import languageBindings from "../../../docs/language-bindings.md?raw";
import troubleshooting from "../../../docs/troubleshooting.md?raw";

export interface DocPage {
  slug: string;
  title: string;
  content: string;
}

export const docs: DocPage[] = [
  { slug: "getting-started", title: "Getting Started", content: gettingStarted },
  { slug: "configuration", title: "Configuration", content: configuration },
  { slug: "providers", title: "Providers", content: providers },
  { slug: "orchestration", title: "Orchestration", content: orchestration },
  { slug: "sessions", title: "Sessions", content: sessions },
  { slug: "isolation", title: "Isolation", content: isolation },
  { slug: "skills-and-mcp", title: "Skills & MCP", content: skillsAndMcp },
  { slug: "events-and-logging", title: "Events & Logging", content: eventsAndLogging },
  { slug: "remote-access", title: "Remote Access", content: remoteAccess },
  { slug: "language-bindings", title: "Language Bindings", content: languageBindings },
  { slug: "troubleshooting", title: "Troubleshooting", content: troubleshooting },
];

export function getDocBySlug(slug: string): DocPage | undefined {
  return docs.find((doc) => doc.slug === slug);
}
