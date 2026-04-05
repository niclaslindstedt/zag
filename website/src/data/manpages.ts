import zagMain from "../../../zag-cli/man/zag.md?raw";
import helpAgent from "../../../zag-cli/man/help-agent.md?raw";
import man from "../../../zag-cli/man/man.md?raw";
import run from "../../../zag-cli/man/run.md?raw";
import exec from "../../../zag-cli/man/exec.md?raw";
import review from "../../../zag-cli/man/review.md?raw";
import config from "../../../zag-cli/man/config.md?raw";
import session from "../../../zag-cli/man/session.md?raw";
import listen from "../../../zag-cli/man/listen.md?raw";
import search from "../../../zag-cli/man/search.md?raw";
import input from "../../../zag-cli/man/input.md?raw";
import output from "../../../zag-cli/man/output.md?raw";
import status from "../../../zag-cli/man/status.md?raw";
import log from "../../../zag-cli/man/log.md?raw";
import events from "../../../zag-cli/man/events.md?raw";
import summary from "../../../zag-cli/man/summary.md?raw";
import orchestration from "../../../zag-cli/man/orchestration.md?raw";
import spawn from "../../../zag-cli/man/spawn.md?raw";
import wait from "../../../zag-cli/man/wait.md?raw";
import collect from "../../../zag-cli/man/collect.md?raw";
import pipe from "../../../zag-cli/man/pipe.md?raw";
import cancel from "../../../zag-cli/man/cancel.md?raw";
import retry from "../../../zag-cli/man/retry.md?raw";
import broadcast from "../../../zag-cli/man/broadcast.md?raw";
import watch from "../../../zag-cli/man/watch.md?raw";
import subscribe from "../../../zag-cli/man/subscribe.md?raw";
import ps from "../../../zag-cli/man/ps.md?raw";
import gc from "../../../zag-cli/man/gc.md?raw";
import env from "../../../zag-cli/man/env.md?raw";
import whoami from "../../../zag-cli/man/whoami.md?raw";
import skills from "../../../zag-cli/man/skills.md?raw";
import mcp from "../../../zag-cli/man/mcp.md?raw";
import capability from "../../../zag-cli/man/capability.md?raw";
import serve from "../../../zag-cli/man/serve.md?raw";
import connect from "../../../zag-cli/man/connect.md?raw";

export interface ManPage {
  slug: string;
  title: string;
  content: string;
}

export interface ManPageGroup {
  label: string;
  pages: ManPage[];
}

export const manPageGroups: ManPageGroup[] = [
  {
    label: "Overview",
    pages: [
      { slug: "zag", title: "zag", content: zagMain },
      { slug: "help-agent", title: "zag --help-agent", content: helpAgent },
      { slug: "man", title: "zag man", content: man },
    ],
  },
  {
    label: "Core Commands",
    pages: [
      { slug: "run", title: "zag run", content: run },
      { slug: "exec", title: "zag exec", content: exec },
      { slug: "review", title: "zag review", content: review },
      { slug: "config", title: "zag config", content: config },
    ],
  },
  {
    label: "Sessions",
    pages: [
      { slug: "session", title: "zag session", content: session },
      { slug: "listen", title: "zag listen", content: listen },
      { slug: "search", title: "zag search", content: search },
      { slug: "input", title: "zag input", content: input },
      { slug: "output", title: "zag output", content: output },
      { slug: "status", title: "zag status", content: status },
      { slug: "log", title: "zag log", content: log },
      { slug: "events", title: "zag events", content: events },
      { slug: "summary", title: "zag summary", content: summary },
    ],
  },
  {
    label: "Orchestration",
    pages: [
      { slug: "orchestration", title: "zag orchestration", content: orchestration },
      { slug: "spawn", title: "zag spawn", content: spawn },
      { slug: "wait", title: "zag wait", content: wait },
      { slug: "collect", title: "zag collect", content: collect },
      { slug: "pipe", title: "zag pipe", content: pipe },
      { slug: "cancel", title: "zag cancel", content: cancel },
      { slug: "retry", title: "zag retry", content: retry },
      { slug: "broadcast", title: "zag broadcast", content: broadcast },
      { slug: "watch", title: "zag watch", content: watch },
      { slug: "subscribe", title: "zag subscribe", content: subscribe },
    ],
  },
  {
    label: "Process Management",
    pages: [
      { slug: "ps", title: "zag ps", content: ps },
      { slug: "gc", title: "zag gc", content: gc },
      { slug: "env", title: "zag env", content: env },
      { slug: "whoami", title: "zag whoami", content: whoami },
    ],
  },
  {
    label: "Extensions",
    pages: [
      { slug: "skills", title: "zag skills", content: skills },
      { slug: "mcp", title: "zag mcp", content: mcp },
      { slug: "capability", title: "zag capability", content: capability },
    ],
  },
  {
    label: "Remote Access",
    pages: [
      { slug: "serve", title: "zag serve", content: serve },
      { slug: "connect", title: "zag connect", content: connect },
    ],
  },
];

export const manPages: ManPage[] = manPageGroups.flatMap((group) => group.pages);

export function getManPageBySlug(slug: string): ManPage | undefined {
  return manPages.find((page) => page.slug === slug);
}
