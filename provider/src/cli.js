#!/usr/bin/env node
import fs from "node:fs";
import http from "node:http";
import os from "node:os";
import path from "node:path";
import process from "node:process";
import { execFileSync, spawn } from "node:child_process";
import { randomBytes } from "node:crypto";
import bs58 from "bs58";
import nacl from "tweetnacl";

const DEFAULT_PORT = 9898;
const DEFAULT_OLLAMA_URL = "http://127.0.0.1:11434";
const SOLAI_HOME = process.env.SOLAI_HOME || path.join(os.homedir(), ".solai");
const CONFIG_FILE = path.join(SOLAI_HOME, "provider.json");
const PID_FILE = path.join(SOLAI_HOME, "provider.pid");
const LOG_FILE = path.join(SOLAI_HOME, "provider.log");

const args = process.argv.slice(2);
const command = args[0] || "help";

main().catch((error) => {
  log(`fatal: ${error.stack || error.message}`);
  console.error(error.message);
  process.exit(1);
});

async function main() {
  ensureHome();

  switch (command) {
    case "enable":
      await enableProvider();
      break;
    case "disable":
      await disableProvider();
      break;
    case "status":
      await printStatus();
      break;
    case "price":
      await setPrice(args.slice(1));
      break;
    case "schedule":
      await setSchedule(args.slice(1));
      break;
    case "daemon":
      await runDaemon();
      break;
    case "help":
    case "--help":
    case "-h":
      printHelp();
      break;
    default:
      throw new Error(`Unknown provider command: ${command}`);
  }
}

function printHelp() {
  console.log(`SOLAI provider commands:
  solai provider enable
  solai provider disable
  solai provider status
  solai provider price <MODEL> <SOLAI_PER_HOUR>
  solai provider schedule --from <HH:MM> --to <HH:MM>
  solai provider daemon

Environment:
  SOLAI_HOME           config directory, default ~/.solai
  SOLAI_PROVIDER_PORT  HTTP port, default 9898
  OLLAMA_HOST          Ollama base URL, default http://127.0.0.1:11434`);
}

async function enableProvider() {
  const config = await loadConfig();
  const detected = await detectSystem();
  const next = {
    ...config,
    enabled: true,
    port: config.port || Number.parseInt(process.env.SOLAI_PROVIDER_PORT || `${DEFAULT_PORT}`, 10),
    ollamaUrl: config.ollamaUrl || process.env.OLLAMA_HOST || DEFAULT_OLLAMA_URL,
    limits: {
      maxConcurrentJobs: 1,
      maxQueueSize: 32,
      maxPromptChars: 120_000,
      ...(config.limits || {}),
    },
    identity: config.identity || createIdentity(),
    hardware: detected.hardware,
    models: detected.models,
    updatedAt: new Date().toISOString(),
  };
  await saveConfig(next);

  if (!isDaemonRunning()) {
    startDetachedDaemon();
  }

  console.log(`Provider enabled: ${next.identity.publicKey}`);
  console.log(`Daemon: ${isDaemonRunning() ? "running" : "starting"} on port ${next.port}`);
  console.log(`Models: ${next.models.length ? next.models.map((model) => model.name).join(", ") : "none detected"}`);
}

async function disableProvider() {
  const config = await loadConfig();
  await saveConfig({ ...config, enabled: false, updatedAt: new Date().toISOString() });

  const pid = readPid();
  if (pid && isPidRunning(pid)) {
    process.kill(pid, "SIGTERM");
    console.log(`Provider disabled. Stopped daemon PID ${pid}.`);
  } else {
    console.log("Provider disabled. No running daemon found.");
  }
  fs.rmSync(PID_FILE, { force: true });
}

async function printStatus() {
  const config = await loadConfig();
  const detected = await detectSystem().catch((error) => ({ error: error.message }));
  const pid = readPid();
  const heartbeat = buildHeartbeat(config, detected);

  console.log(JSON.stringify({
    enabled: Boolean(config.enabled),
    daemon: {
      running: Boolean(pid && isPidRunning(pid)),
      pid: pid || null,
      port: config.port || DEFAULT_PORT,
    },
    provider: {
      publicKey: config.identity?.publicKey || null,
      schedule: config.schedule || null,
      prices: config.prices || {},
      limits: config.limits || {},
    },
    detected,
    heartbeat,
  }, null, 2));
}

async function setPrice(priceArgs) {
  const [model, priceText] = priceArgs;
  const price = Number(priceText);
  if (!model || !Number.isFinite(price) || price < 0) {
    throw new Error("Usage: solai provider price <MODEL> <SOLAI_PER_HOUR>");
  }

  const config = await loadConfig();
  const prices = { ...(config.prices || {}), [model]: price };
  await saveConfig({ ...config, prices, updatedAt: new Date().toISOString() });
  console.log(`Set ${model} price to ${price} SOLAI/hour.`);
}

async function setSchedule(scheduleArgs) {
  const from = valueAfter(scheduleArgs, "--from");
  const to = valueAfter(scheduleArgs, "--to");
  if (!isTime(from) || !isTime(to)) {
    throw new Error("Usage: solai provider schedule --from <HH:MM> --to <HH:MM>");
  }

  const config = await loadConfig();
  await saveConfig({
    ...config,
    schedule: { from, to, timezone: Intl.DateTimeFormat().resolvedOptions().timeZone || "local" },
    updatedAt: new Date().toISOString(),
  });
  console.log(`Provider schedule set from ${from} to ${to}.`);
}

async function runDaemon() {
  const config = await loadConfig();
  if (!config.enabled) {
    throw new Error("Provider is disabled. Run `solai provider enable` first.");
  }

  fs.writeFileSync(PID_FILE, `${process.pid}\n`, { mode: 0o600 });
  process.on("SIGTERM", () => {
    log("received SIGTERM, shutting down");
    fs.rmSync(PID_FILE, { force: true });
    process.exit(0);
  });

  const state = {
    jobs: [],
    running: 0,
    completed: 0,
    failed: 0,
    startedAt: new Date().toISOString(),
    config,
  };

  const server = http.createServer(async (req, res) => {
    try {
      await routeRequest(req, res, state);
    } catch (error) {
      log(`request error: ${error.stack || error.message}`);
      sendJson(res, 500, { error: error.message });
    }
  });

  const port = config.port || Number.parseInt(process.env.SOLAI_PROVIDER_PORT || `${DEFAULT_PORT}`, 10);
  server.listen(port, "127.0.0.1", () => {
    log(`provider daemon listening on 127.0.0.1:${port}`);
    console.log(`SOLAI provider daemon listening on 127.0.0.1:${port}`);
  });
}

async function routeRequest(req, res, state) {
  const url = new URL(req.url, "http://127.0.0.1");
  const config = await loadConfig();
  state.config = config;

  if (!config.enabled) {
    return sendJson(res, 503, { error: "provider disabled" });
  }

  if (req.method === "GET" && url.pathname === "/health") {
    return sendJson(res, 200, { ok: true, enabled: true, startedAt: state.startedAt });
  }

  if (req.method === "GET" && url.pathname === "/metrics") {
    return sendJson(res, 200, await metrics(state));
  }

  if (req.method === "GET" && url.pathname === "/heartbeat") {
    return sendJson(res, 200, buildHeartbeat(config, await detectSystem()));
  }

  if (req.method === "GET" && url.pathname === "/jobs") {
    return sendJson(res, 200, {
      running: state.running,
      queued: state.jobs.length,
      completed: state.completed,
      failed: state.failed,
      jobs: state.jobs.map(({ id, model, status, createdAt }) => ({ id, model, status, createdAt })),
    });
  }

  if (req.method === "POST" && url.pathname === "/jobs") {
    const body = await readBody(req, config.limits?.maxPromptChars || 120_000);
    const payload = JSON.parse(body || "{}");
    return enqueueJob(res, state, {
      model: payload.model,
      prompt: payload.prompt,
      stream: false,
    });
  }

  if (req.method === "POST" && url.pathname === "/v1/chat/completions") {
    const body = await readBody(req, config.limits?.maxPromptChars || 120_000);
    const payload = JSON.parse(body || "{}");
    const prompt = Array.isArray(payload.messages)
      ? payload.messages.map((message) => `${message.role || "user"}: ${message.content || ""}`).join("\n")
      : payload.prompt;
    return enqueueJob(res, state, {
      model: payload.model,
      prompt,
      stream: false,
      openAiShape: true,
    });
  }

  sendJson(res, 404, { error: "not found" });
}

function enqueueJob(res, state, jobInput) {
  const limits = state.config.limits || {};
  const maxQueueSize = limits.maxQueueSize || 32;

  if (!jobInput.model || !jobInput.prompt) {
    return sendJson(res, 400, { error: "model and prompt are required" });
  }

  if (state.jobs.length >= maxQueueSize) {
    return sendJson(res, 429, { error: "provider queue is full" });
  }

  const job = {
    id: cryptoId(),
    model: jobInput.model,
    prompt: jobInput.prompt,
    status: "queued",
    createdAt: new Date().toISOString(),
  };
  state.jobs.push(job);
  log(`queued job ${job.id} model=${job.model}`);
  void pumpQueue(state);

  const wait = waitForJob(job).then((result) => {
    if (jobInput.openAiShape) {
      return sendJson(res, 200, {
        id: job.id,
        object: "chat.completion",
        created: Math.floor(Date.now() / 1000),
        model: job.model,
        choices: [{ index: 0, message: { role: "assistant", content: result.response }, finish_reason: "stop" }],
      });
    }
    sendJson(res, 200, result);
  });
  wait.catch((error) => sendJson(res, 500, { id: job.id, error: error.message }));
}

async function pumpQueue(state) {
  const maxConcurrentJobs = state.config.limits?.maxConcurrentJobs || 1;
  while (state.running < maxConcurrentJobs) {
    const job = state.jobs.find((candidate) => candidate.status === "queued");
    if (!job) {
      return;
    }

    state.running += 1;
    job.status = "running";
    runOllamaJob(state.config, job)
      .then((result) => {
        job.status = "completed";
        job.result = result;
        state.completed += 1;
        log(`completed job ${job.id}`);
      })
      .catch((error) => {
        job.status = "failed";
        job.error = error.message;
        state.failed += 1;
        log(`failed job ${job.id}: ${error.message}`);
      })
      .finally(() => {
        state.running -= 1;
        state.jobs = state.jobs.filter((candidate) => candidate.status === "queued" || candidate.status === "running");
        void pumpQueue(state);
      });
  }
}

async function waitForJob(job) {
  while (job.status === "queued" || job.status === "running") {
    await sleep(100);
  }
  if (job.status === "failed") {
    throw new Error(job.error || "job failed");
  }
  return job.result;
}

async function runOllamaJob(config, job) {
  const response = await fetch(`${config.ollamaUrl || DEFAULT_OLLAMA_URL}/api/generate`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ model: job.model, prompt: job.prompt, stream: false }),
  });

  if (!response.ok) {
    throw new Error(`Ollama returned ${response.status}: ${await response.text()}`);
  }

  const body = await response.json();
  return {
    id: job.id,
    model: job.model,
    response: body.response || "",
    done: Boolean(body.done),
    createdAt: job.createdAt,
    completedAt: new Date().toISOString(),
  };
}

async function metrics(state) {
  const detected = await detectSystem().catch(() => ({ hardware: {}, models: [] }));
  return {
    service: "solai-provider",
    uptimeSeconds: Math.floor(process.uptime()),
    runningJobs: state.running,
    queuedJobs: state.jobs.length,
    completedJobs: state.completed,
    failedJobs: state.failed,
    hardware: detected.hardware,
    models: detected.models,
  };
}

async function detectSystem() {
  const hardware = {
    cpu: os.cpus()?.[0]?.model || "unknown",
    cpuCores: os.cpus()?.length || 0,
    ramBytes: os.totalmem(),
    platform: `${os.type()} ${os.release()} ${os.arch()}`,
    gpu: detectGpu(),
  };
  const models = await detectOllamaModels();
  return { hardware, models, detectedAt: new Date().toISOString() };
}

function detectGpu() {
  try {
    const output = execFileSync("nvidia-smi", [
      "--query-gpu=name,memory.total,temperature.gpu,utilization.gpu",
      "--format=csv,noheader,nounits",
    ], { encoding: "utf8", timeout: 1500 }).trim();
    if (output) {
      return output.split("\n").map((line) => {
        const [name, vramMb, temperatureC, utilizationPct] = line.split(",").map((part) => part.trim());
        return { name, vramMb: Number(vramMb), temperatureC: Number(temperatureC), utilizationPct: Number(utilizationPct) };
      });
    }
  } catch {
    // nvidia-smi is optional.
  }

  try {
    const output = execFileSync("lspci", { encoding: "utf8", timeout: 1500 });
    return output
      .split("\n")
      .filter((line) => /vga|3d|display/i.test(line))
      .map((line) => ({ name: line.trim() }));
  } catch {
    return [];
  }
}

async function detectOllamaModels() {
  try {
    const response = await fetch(`${process.env.OLLAMA_HOST || DEFAULT_OLLAMA_URL}/api/tags`, { signal: AbortSignal.timeout(1500) });
    if (!response.ok) {
      return [];
    }
    const body = await response.json();
    return (body.models || []).map((model) => ({
      name: model.name,
      modifiedAt: model.modified_at,
      size: model.size,
      digest: model.digest,
    }));
  } catch {
    return [];
  }
}

function buildHeartbeat(config, detected) {
  const payload = {
    provider: config.identity?.publicKey || null,
    status: config.enabled ? "ONLINE" : "OFFLINE",
    timestamp: new Date().toISOString(),
    models: detected.models || config.models || [],
    hardware: detected.hardware || config.hardware || {},
    prices: config.prices || {},
    schedule: config.schedule || null,
    capacity: {
      maxConcurrentJobs: config.limits?.maxConcurrentJobs || 1,
    },
  };

  return {
    payload,
    signature: config.identity ? signPayload(config.identity.secretKey, payload) : null,
  };
}

function signPayload(secretKey, payload) {
  const bytes = Buffer.from(JSON.stringify(payload));
  const signature = nacl.sign.detached(bytes, Uint8Array.from(secretKey));
  return Buffer.from(signature).toString("base64");
}

function createIdentity() {
  const keypair = nacl.sign.keyPair.fromSeed(randomBytes(32));
  return {
    publicKey: bs58.encode(keypair.publicKey),
    secretKey: Array.from(keypair.secretKey),
  };
}

async function loadConfig() {
  if (!fs.existsSync(CONFIG_FILE)) {
    return {};
  }
  return JSON.parse(fs.readFileSync(CONFIG_FILE, "utf8"));
}

async function saveConfig(config) {
  fs.writeFileSync(CONFIG_FILE, `${JSON.stringify(config, null, 2)}\n`, { mode: 0o600 });
}

function ensureHome() {
  fs.mkdirSync(SOLAI_HOME, { recursive: true, mode: 0o700 });
}

function startDetachedDaemon() {
  const child = spawn(process.execPath, [new URL(import.meta.url).pathname, "daemon"], {
    detached: true,
    stdio: "ignore",
    env: process.env,
  });
  child.unref();
}

function isDaemonRunning() {
  const pid = readPid();
  return Boolean(pid && isPidRunning(pid));
}

function readPid() {
  try {
    return Number.parseInt(fs.readFileSync(PID_FILE, "utf8"), 10);
  } catch {
    return null;
  }
}

function isPidRunning(pid) {
  try {
    process.kill(pid, 0);
    return true;
  } catch {
    return false;
  }
}

function valueAfter(list, key) {
  const index = list.indexOf(key);
  return index >= 0 ? list[index + 1] : null;
}

function isTime(value) {
  return typeof value === "string" && /^([01]\d|2[0-3]):[0-5]\d$/.test(value);
}

function cryptoId() {
  return `${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 10)}`;
}

function readBody(req, maxChars) {
  return new Promise((resolve, reject) => {
    let body = "";
    req.setEncoding("utf8");
    req.on("data", (chunk) => {
      body += chunk;
      if (body.length > maxChars) {
        reject(new Error(`request body exceeds limit of ${maxChars} chars`));
        req.destroy();
      }
    });
    req.on("end", () => resolve(body));
    req.on("error", reject);
  });
}

function sendJson(res, status, body) {
  res.writeHead(status, { "content-type": "application/json" });
  res.end(`${JSON.stringify(body, null, 2)}\n`);
}

function log(message) {
  ensureHome();
  fs.appendFileSync(LOG_FILE, `${new Date().toISOString()} ${message}\n`);
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
