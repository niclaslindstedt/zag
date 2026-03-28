import express from "express";
import { spawn } from "child_process";
import { randomUUID } from "crypto";
import { createInterface } from "readline";

const app = express();
app.use(express.json());

const PORT = 3001;

/**
 * POST /api/session
 * Body: { prompt: string, provider?: string, model?: string }
 *
 * Spawns `zag exec -o stream-json --session <uuid> "<prompt>"` and streams
 * NDJSON events back as Server-Sent Events.
 */
app.post("/api/session", (req, res) => {
  const { prompt, provider, model } = req.body;

  if (!prompt || typeof prompt !== "string") {
    res.status(400).json({ error: "prompt is required" });
    return;
  }

  const sessionId = randomUUID();

  const args: string[] = [];
  if (provider) args.push("-p", provider);
  if (model) args.push("--model", model);
  args.push("exec", "-o", "stream-json", "--session", sessionId, prompt);

  console.log(`[session ${sessionId}] zag ${args.join(" ")}`);

  const child = spawn("zag", args, {
    stdio: ["ignore", "pipe", "pipe"],
    env: { ...process.env },
  });

  // SSE headers
  res.writeHead(200, {
    "Content-Type": "text/event-stream",
    "Cache-Control": "no-cache",
    Connection: "keep-alive",
    "X-Session-Id": sessionId,
  });

  // Send session ID as first event
  res.write(`event: session_id\ndata: ${JSON.stringify({ session_id: sessionId })}\n\n`);

  // Stream stdout lines as SSE
  const rl = createInterface({ input: child.stdout });
  rl.on("line", (line) => {
    if (line.trim()) {
      res.write(`data: ${line}\n\n`);
    }
  });

  // Capture stderr
  const stderrRl = createInterface({ input: child.stderr });
  stderrRl.on("line", (line) => {
    console.error(`[session ${sessionId}] stderr: ${line}`);
  });

  child.on("close", (code) => {
    console.log(`[session ${sessionId}] exited with code ${code}`);
    res.write(`event: done\ndata: ${JSON.stringify({ code })}\n\n`);
    res.end();
  });

  // Client disconnect
  req.on("close", () => {
    child.kill("SIGTERM");
  });
});

/**
 * GET /api/listen/:sessionId
 *
 * Spawns `zag listen --json <sessionId>` and streams events as SSE.
 * Useful for attaching to an already-running session.
 */
app.get("/api/listen/:sessionId", (req, res) => {
  const { sessionId } = req.params;

  const child = spawn("zag", ["listen", "--json", sessionId], {
    stdio: ["ignore", "pipe", "pipe"],
  });

  res.writeHead(200, {
    "Content-Type": "text/event-stream",
    "Cache-Control": "no-cache",
    Connection: "keep-alive",
  });

  const rl = createInterface({ input: child.stdout });
  rl.on("line", (line) => {
    if (line.trim()) {
      res.write(`data: ${line}\n\n`);
    }
  });

  child.on("close", (code) => {
    res.write(`event: done\ndata: ${JSON.stringify({ code })}\n\n`);
    res.end();
  });

  req.on("close", () => {
    child.kill("SIGTERM");
  });
});

/**
 * POST /api/input
 * Body: { sessionId: string, message: string }
 *
 * Spawns `zag input <sessionId> "<message>" -o stream-json` and streams
 * NDJSON events back as Server-Sent Events.
 */
app.post("/api/input", (req, res) => {
  const { sessionId, message } = req.body;

  if (!sessionId || typeof sessionId !== "string") {
    res.status(400).json({ error: "sessionId is required" });
    return;
  }
  if (!message || typeof message !== "string") {
    res.status(400).json({ error: "message is required" });
    return;
  }

  const args = ["input", sessionId, message, "-o", "stream-json"];

  console.log(`[input ${sessionId}] zag ${args.join(" ")}`);

  const child = spawn("zag", args, {
    stdio: ["ignore", "pipe", "pipe"],
    env: { ...process.env },
  });

  // SSE headers
  res.writeHead(200, {
    "Content-Type": "text/event-stream",
    "Cache-Control": "no-cache",
    Connection: "keep-alive",
  });

  // Stream stdout lines as SSE
  const rl = createInterface({ input: child.stdout });
  rl.on("line", (line) => {
    if (line.trim()) {
      res.write(`data: ${line}\n\n`);
    }
  });

  // Capture stderr
  const stderrRl = createInterface({ input: child.stderr });
  stderrRl.on("line", (line) => {
    console.error(`[input ${sessionId}] stderr: ${line}`);
  });

  child.on("close", (code) => {
    console.log(`[input ${sessionId}] exited with code ${code}`);
    res.write(`event: done\ndata: ${JSON.stringify({ code })}\n\n`);
    res.end();
  });

  // Client disconnect
  req.on("close", () => {
    child.kill("SIGTERM");
  });
});

app.listen(PORT, () => {
  console.log(`Server running on http://localhost:${PORT}`);
});
