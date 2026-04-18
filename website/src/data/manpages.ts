import zagMain from "../../../zag-agent/man/zag.md?raw";
import helpAgent from "../../../zag-agent/man/help-agent.md?raw";
import man from "../../../zag-agent/man/man.md?raw";
import run from "../../../zag-agent/man/run.md?raw";
import exec from "../../../zag-agent/man/exec.md?raw";
import review from "../../../zag-agent/man/review.md?raw";
import config from "../../../zag-agent/man/config.md?raw";
import session from "../../../zag-agent/man/session.md?raw";
import listen from "../../../zag-agent/man/listen.md?raw";
import search from "../../../zag-agent/man/search.md?raw";
import input from "../../../zag-agent/man/input.md?raw";
import output from "../../../zag-agent/man/output.md?raw";
import status from "../../../zag-agent/man/status.md?raw";
import log from "../../../zag-agent/man/log.md?raw";
import events from "../../../zag-agent/man/events.md?raw";
import summary from "../../../zag-agent/man/summary.md?raw";
import orchestration from "../../../zag-agent/man/orchestration.md?raw";
import spawn from "../../../zag-agent/man/spawn.md?raw";
import wait from "../../../zag-agent/man/wait.md?raw";
import collect from "../../../zag-agent/man/collect.md?raw";
import pipe from "../../../zag-agent/man/pipe.md?raw";
import cancel from "../../../zag-agent/man/cancel.md?raw";
import retry from "../../../zag-agent/man/retry.md?raw";
import broadcast from "../../../zag-agent/man/broadcast.md?raw";
import watch from "../../../zag-agent/man/watch.md?raw";
import subscribe from "../../../zag-agent/man/subscribe.md?raw";
import ps from "../../../zag-agent/man/ps.md?raw";
import gc from "../../../zag-agent/man/gc.md?raw";
import env from "../../../zag-agent/man/env.md?raw";
import whoami from "../../../zag-agent/man/whoami.md?raw";
import skills from "../../../zag-agent/man/skills.md?raw";
import mcp from "../../../zag-agent/man/mcp.md?raw";
import capability from "../../../zag-agent/man/capability.md?raw";
import serve from "../../../zag-agent/man/serve.md?raw";
import connect from "../../../zag-agent/man/connect.md?raw";

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
