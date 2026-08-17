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
const REGISTRY_FILE = path.join(SOLAI_HOME, "provider-registry.json");
const PROVIDER_CHOICES_FILE = path.join(SOLAI_HOME, "provider-choices.json");
const PROVIDER_LEASES_FILE = path.join(SOLAI_HOME, "provider-leases.json");
const MARKETPLACE_LEASES_FILE = path.join(SOLAI_HOME, "marketplace-leases.json");
const MARKETPLACE_IDENTITY_FILE = path.join(SOLAI_HOME, "marketplace-client.json");

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
    case "register":
      await registerLocalProvider(args.slice(1));
      break;
    case "probe":
      await probeProvider(args.slice(1));
      break;
    case "list":
    case "discover":
      await listProviders(args.slice(1));
      break;
    case "refresh":
      await refreshProviders(args.slice(1));
      break;
    case "quote":
      await quoteProvider(args.slice(1));
      break;
    case "rent":
      await rentProvider(args.slice(1));
      break;
    case "leases":
      await listMarketplaceLeases(args.slice(1));
      break;
    case "lease":
      await showMarketplaceLease(args.slice(1));
      break;
    case "release":
      await releaseMarketplaceLease(args.slice(1));
      break;
    case "run":
      await runMarketplaceJob(args.slice(1));
      break;
    case "remove":
      await removeProvider(args.slice(1));
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
  solai provider register [--endpoint <URL>] [--name <NAME>]
  solai provider probe <ENDPOINT> [--name <NAME>]
  solai provider list [--model <MODEL>] [--max-price <SOLAI_PER_HOUR>] [--all] [--json]
  solai provider refresh [--json]
  solai provider quote (--choice <N> | --model <MODEL>) [--hours <HOURS>] [--max-price <SOLAI_PER_HOUR>] [--json]
  solai provider rent (--choice <N> | --model <MODEL>) --hours <HOURS> [--provider <PUBLIC_KEY>] [--max-price <SOLAI_PER_HOUR>] [--json]
  solai provider leases [--json]
  solai provider lease <LEASE_ID> [--json]
  solai provider release <LEASE_ID>
  solai provider run (--choice <N> | --model <MODEL>) (--prompt <TEXT> | --prompt-file <PATH>) [--lease <LEASE_ID>] [--provider <PUBLIC_KEY>] [--max-price <SOLAI_PER_HOUR>]
  solai provider remove <PROVIDER_PUBLIC_KEY>
  solai provider daemon

Environment:
  SOLAI_HOME           config directory, default ~/.solai
  SOLAI_PROVIDER_HOST  HTTP bind host, default 127.0.0.1
  SOLAI_PROVIDER_PORT  HTTP port, default 9898
  OLLAMA_HOST          Ollama base URL, default http://127.0.0.1:11434`);
}

async function enableProvider() {
  const config = await loadConfig();
  const detected = await detectSystem();
  const next = {
    ...config,
    enabled: true,
    host: config.host || process.env.SOLAI_PROVIDER_HOST || "127.0.0.1",
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

async function registerLocalProvider(registerArgs) {
  const config = await loadConfig();
  if (!config.identity?.publicKey) {
    throw new Error("Provider identity is missing. Run `solai provider enable` first.");
  }

  const endpoint = valueAfter(registerArgs, "--endpoint") || `http://${config.host || "127.0.0.1"}:${config.port || DEFAULT_PORT}`;
  const name = valueAfter(registerArgs, "--name") || os.hostname();
  const detected = await detectSystem().catch(() => ({ hardware: config.hardware || {}, models: config.models || [] }));
  const heartbeat = buildHeartbeat(config, detected);
  const record = providerRecordFromHeartbeat(heartbeat, { endpoint, name, source: "local" });
  await upsertProviderRecord(record);

  console.log(`Registered provider ${record.id}`);
  console.log(`Endpoint: ${record.endpoint}`);
  console.log(`Models: ${record.models.length ? record.models.map((model) => model.name).join(", ") : "none detected"}`);
}

async function probeProvider(probeArgs) {
  const endpoint = probeArgs.find((arg) => !arg.startsWith("--"));
  if (!endpoint) {
    throw new Error("Usage: solai provider probe <ENDPOINT> [--name <NAME>]");
  }

  const heartbeatUrl = new URL("/heartbeat", normalizeEndpoint(endpoint));
  const response = await fetch(heartbeatUrl, { signal: AbortSignal.timeout(3000) });
  if (!response.ok) {
    throw new Error(`Provider heartbeat returned ${response.status}: ${await response.text()}`);
  }

  const heartbeat = await response.json();
  verifyHeartbeat(heartbeat);
  const record = providerRecordFromHeartbeat(heartbeat, {
    endpoint: normalizeEndpoint(endpoint),
    name: valueAfter(probeArgs, "--name") || null,
    source: "probe",
  });
  await upsertProviderRecord(record);

  console.log(`Discovered provider ${record.id}`);
  console.log(`Endpoint: ${record.endpoint}`);
  console.log(`Status: ${record.status}`);
}

async function listProviders(listArgs) {
  const registry = await loadRegistry();
  const modelFilter = valueAfter(listArgs, "--model");
  const maxPriceText = valueAfter(listArgs, "--max-price");
  const maxPrice = maxPriceText == null ? null : Number(maxPriceText);
  const includeUnavailable = listArgs.includes("--all");
  const json = listArgs.includes("--json");

  if (maxPriceText != null && (!Number.isFinite(maxPrice) || maxPrice < 0)) {
    throw new Error("--max-price must be a non-negative number");
  }

  const choices = marketplaceChoices(registry.providers, { modelFilter, maxPrice, includeUnavailable });
  await saveProviderChoices(choices);

  if (json) {
    console.log(JSON.stringify({ choices }, null, 2));
    return;
  }

  if (choices.length === 0) {
    console.log("No available providers found.");
    console.log("Tip: run `solai marketplace probe <ENDPOINT>` or use `--all` to include unavailable providers.");
    return;
  }

  console.log("Available SOLAI compute");
  console.log("Use: solai marketplace rent --choice <N> --hours <HOURS>");
  console.log("");
  for (const choice of choices) {
    console.log(`${choice.choice}. ${choice.name}`);
    console.log(`   model: ${choice.model}`);
    console.log(`   price: ${formatPrice(choice.pricePerHour)} SOLAI/hour   available: ${choice.availableNow ? "yes" : "no"}   status: ${choice.status}`);
    console.log(`   reputation: score ${choice.reputation.score}, rentals ${choice.reputation.rentals}, failures ${choice.reputation.failures}`);
    console.log(`   endpoint: ${choice.endpoint}`);
  }
}

async function refreshProviders(refreshArgs) {
  const json = refreshArgs.includes("--json");
  const registry = await loadRegistry();
  const refreshed = [];

  for (const provider of registry.providers) {
    try {
      const heartbeatUrl = new URL("/heartbeat", provider.endpoint);
      const response = await fetch(heartbeatUrl, { signal: AbortSignal.timeout(3000) });
      if (!response.ok) {
        throw new Error(`HTTP ${response.status}`);
      }
      const heartbeat = await response.json();
      verifyHeartbeat(heartbeat);
      refreshed.push({
        ...providerRecordFromHeartbeat(heartbeat, {
          endpoint: provider.endpoint,
          name: provider.name,
          source: provider.source,
        }),
        reputation: provider.reputation,
        firstSeenAt: provider.firstSeenAt,
        lastError: null,
      });
    } catch (error) {
      refreshed.push({
        ...provider,
        status: "OFFLINE",
        lastError: error.message,
        updatedAt: new Date().toISOString(),
      });
    }
  }

  const next = { ...registry, providers: refreshed, updatedAt: new Date().toISOString() };
  await saveRegistry(next);

  if (json) {
    console.log(JSON.stringify(next, null, 2));
    return;
  }

  const online = refreshed.filter((provider) => provider.status === "ONLINE").length;
  console.log(`Refreshed ${refreshed.length} provider(s). Online: ${online}.`);
}

async function quoteProvider(quoteArgs) {
  const choice = await choiceFromArgs(quoteArgs);
  const model = choice?.model || valueAfter(quoteArgs, "--model");
  const hours = parseHours(valueAfter(quoteArgs, "--hours") || "1");
  const providerIdFromChoice = choice?.providerId || null;
  const maxPriceText = valueAfter(quoteArgs, "--max-price");
  const maxPrice = maxPriceText == null ? null : Number(maxPriceText);
  const json = quoteArgs.includes("--json");

  if (!model || !Number.isFinite(hours)) {
    throw new Error("Usage: solai provider quote (--choice <N> | --model <MODEL>) [--hours <HOURS>] [--max-price <SOLAI_PER_HOUR>] [--json]");
  }
  if (maxPriceText != null && (!Number.isFinite(maxPrice) || maxPrice < 0)) {
    throw new Error("--max-price must be a non-negative number");
  }

  const provider = await selectProvider({ model, maxPrice, providerId: providerIdFromChoice, availableOnly: true });
  const quote = {
    providerId: provider.id,
    endpoint: provider.endpoint,
    model,
    solaiPerHour: minProviderPrice(provider, model),
    hours,
    totalSolai: minProviderPrice(provider, model) * hours,
    availableNow: isAvailableNow(provider.schedule),
    reputation: provider.reputation,
  };

  if (json) {
    console.log(JSON.stringify(quote, null, 2));
    return;
  }

  console.log(`Provider: ${quote.providerId}`);
  console.log(`Endpoint: ${quote.endpoint}`);
  console.log(`Model: ${quote.model}`);
  console.log(`Price: ${formatPrice(quote.solaiPerHour)} SOLAI/hour`);
  console.log(`Hours: ${quote.hours}`);
  console.log(`Total: ${formatPrice(quote.totalSolai)} SOLAI`);
  console.log(`Available now: ${quote.availableNow ? "yes" : "no"}`);
}

async function rentProvider(rentArgs) {
  const choice = await choiceFromArgs(rentArgs);
  const model = choice?.model || valueAfter(rentArgs, "--model");
  const hours = parseHours(valueAfter(rentArgs, "--hours"));
  const providerId = choice?.providerId || valueAfter(rentArgs, "--provider");
  const maxPriceText = valueAfter(rentArgs, "--max-price");
  const maxPrice = maxPriceText == null ? null : Number(maxPriceText);
  const json = rentArgs.includes("--json");

  if (!model || !Number.isFinite(hours)) {
    throw new Error("Usage: solai provider rent (--choice <N> | --model <MODEL>) --hours <HOURS> [--provider <PUBLIC_KEY>] [--max-price <SOLAI_PER_HOUR>] [--json]");
  }
  if (maxPriceText != null && (!Number.isFinite(maxPrice) || maxPrice < 0)) {
    throw new Error("--max-price must be a non-negative number");
  }

  const renter = await loadMarketplaceIdentity();
  const provider = await selectProvider({ model, maxPrice, providerId, availableOnly: true });
  const response = await fetch(new URL("/leases", provider.endpoint), {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ renter: renter.publicKey, model, hours }),
    signal: AbortSignal.timeout(10_000),
  });
  if (!response.ok) {
    throw new Error(`Provider lease request returned ${response.status}: ${await response.text()}`);
  }

  const lease = await response.json();
  const localLease = {
    ...lease,
    providerId: provider.id,
    endpoint: provider.endpoint,
    renter: renter.publicKey,
    status: "ACTIVE",
    updatedAt: new Date().toISOString(),
  };
  await upsertMarketplaceLease(localLease);

  if (json) {
    console.log(JSON.stringify(publicLocalLease(localLease), null, 2));
    return;
  }

  console.log(`Lease: ${localLease.id}`);
  console.log(`Provider: ${localLease.providerId}`);
  console.log(`Model: ${localLease.model}`);
  console.log(`Hours: ${localLease.hours}`);
  console.log(`Total: ${formatPrice(localLease.totalSolai)} SOLAI`);
  console.log(`Expires: ${localLease.expiresAt}`);
}

async function listMarketplaceLeases(leaseArgs) {
  const json = leaseArgs.includes("--json");
  const leases = (await loadMarketplaceLeases()).leases.map((lease) => ({
    ...publicLocalLease(lease),
    activeNow: isLeaseActive(lease),
  }));

  if (json) {
    console.log(JSON.stringify({ leases }, null, 2));
    return;
  }

  if (leases.length === 0) {
    console.log("No marketplace leases found.");
    return;
  }

  console.log(`${"LEASE".padEnd(16)}  ${"STATUS".padEnd(8)}  MODEL  EXPIRES`);
  for (const lease of leases) {
    console.log(`${lease.id.padEnd(16)}  ${(lease.activeNow ? "ACTIVE" : lease.status || "EXPIRED").padEnd(8)}  ${lease.model}  ${lease.expiresAt}`);
  }
}

async function showMarketplaceLease(leaseArgs) {
  const leaseId = leaseArgs.find((arg) => !arg.startsWith("--"));
  const json = leaseArgs.includes("--json");
  if (!leaseId) {
    throw new Error("Usage: solai provider lease <LEASE_ID> [--json]");
  }

  const lease = await findMarketplaceLease(leaseId);
  if (!lease) {
    throw new Error(`Lease not found: ${leaseId}`);
  }

  const output = { ...publicLocalLease(lease), activeNow: isLeaseActive(lease) };
  if (json) {
    console.log(JSON.stringify(output, null, 2));
    return;
  }

  console.log(`Lease: ${output.id}`);
  console.log(`Provider: ${output.providerId}`);
  console.log(`Endpoint: ${output.endpoint}`);
  console.log(`Model: ${output.model}`);
  console.log(`Status: ${output.activeNow ? "ACTIVE" : output.status || "EXPIRED"}`);
  console.log(`Expires: ${output.expiresAt}`);
}

async function releaseMarketplaceLease(leaseArgs) {
  const leaseId = leaseArgs[0];
  if (!leaseId) {
    throw new Error("Usage: solai provider release <LEASE_ID>");
  }

  const lease = await findMarketplaceLease(leaseId);
  if (!lease) {
    throw new Error(`Lease not found: ${leaseId}`);
  }

  const response = await fetch(new URL(`/leases/${lease.id}`, lease.endpoint), {
    method: "DELETE",
    headers: leaseHeaders(lease),
    signal: AbortSignal.timeout(10_000),
  });
  if (!response.ok) {
    throw new Error(`Provider lease release returned ${response.status}: ${await response.text()}`);
  }

  await markMarketplaceLeaseReleased(lease.id);
  console.log(`Released lease ${lease.id}.`);
}

async function runMarketplaceJob(runArgs) {
  const choice = await choiceFromArgs(runArgs);
  const model = choice?.model || valueAfter(runArgs, "--model");
  const leaseId = valueAfter(runArgs, "--lease");
  const providerId = choice?.providerId || valueAfter(runArgs, "--provider");
  const maxPriceText = valueAfter(runArgs, "--max-price");
  const maxPrice = maxPriceText == null ? null : Number(maxPriceText);
  const promptText = valueAfter(runArgs, "--prompt");
  const promptFile = valueAfter(runArgs, "--prompt-file");

  if (!model || (!promptText && !promptFile)) {
    throw new Error("Usage: solai provider run (--choice <N> | --model <MODEL>) (--prompt <TEXT> | --prompt-file <PATH>) [--lease <LEASE_ID>] [--provider <PUBLIC_KEY>] [--max-price <SOLAI_PER_HOUR>]");
  }
  if (maxPriceText != null && (!Number.isFinite(maxPrice) || maxPrice < 0)) {
    throw new Error("--max-price must be a non-negative number");
  }

  const prompt = promptFile ? fs.readFileSync(promptFile, "utf8") : promptText;
  const lease = leaseId
    ? await findMarketplaceLease(leaseId)
    : await findActiveMarketplaceLease({ model, providerId });
  if (leaseId && !lease) {
    throw new Error(`Lease not found: ${leaseId}`);
  }
  if (lease && !isLeaseActive(lease)) {
    throw new Error(`Lease is not active: ${lease.id}`);
  }
  const provider = lease
    ? { id: lease.providerId, endpoint: lease.endpoint }
    : await selectProvider({ model, maxPrice, providerId, availableOnly: true });
  const startedAt = Date.now();

  try {
    const response = await fetch(new URL("/jobs", provider.endpoint), {
      method: "POST",
      headers: {
        "content-type": "application/json",
        ...(lease ? leaseHeaders(lease) : {}),
      },
      body: JSON.stringify({ model, prompt }),
      signal: AbortSignal.timeout(300_000),
    });
    if (!response.ok) {
      throw new Error(`Provider returned ${response.status}: ${await response.text()}`);
    }

    const result = await response.json();
    await recordProviderOutcome(provider.id, true);
    console.log(JSON.stringify({
      providerId: provider.id,
      endpoint: provider.endpoint,
      leaseId: lease?.id || null,
      elapsedMs: Date.now() - startedAt,
      result,
    }, null, 2));
  } catch (error) {
    await recordProviderOutcome(provider.id, false);
    throw error;
  }
}

async function removeProvider(removeArgs) {
  const id = removeArgs[0];
  if (!id) {
    throw new Error("Usage: solai provider remove <PROVIDER_PUBLIC_KEY>");
  }

  const registry = await loadRegistry();
  const nextProviders = registry.providers.filter((provider) => provider.id !== id);
  if (nextProviders.length === registry.providers.length) {
    throw new Error(`Provider not found: ${id}`);
  }

  await saveRegistry({ ...registry, providers: nextProviders, updatedAt: new Date().toISOString() });
  console.log(`Removed provider ${id}.`);
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
    leases: await loadProviderLeases(),
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
  const host = config.host || process.env.SOLAI_PROVIDER_HOST || "127.0.0.1";
  server.listen(port, host, () => {
    log(`provider daemon listening on ${host}:${port}`);
    console.log(`SOLAI provider daemon listening on ${host}:${port}`);
  });
}

async function routeRequest(req, res, state) {
  const url = new URL(req.url, "http://127.0.0.1");
  const config = await loadConfig();
  state.config = config;
  state.leases = await loadProviderLeases();

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

  if (req.method === "GET" && url.pathname === "/marketplace/provider") {
    const heartbeat = buildHeartbeat(config, await detectSystem());
    return sendJson(res, 200, providerRecordFromHeartbeat(heartbeat, {
      endpoint: `http://127.0.0.1:${config.port || DEFAULT_PORT}`,
      name: os.hostname(),
      source: "daemon",
    }));
  }

  if (req.method === "GET" && url.pathname === "/marketplace/providers") {
    return sendJson(res, 200, await loadRegistry());
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

  if (req.method === "GET" && url.pathname === "/leases") {
    const leases = await loadProviderLeases();
    return sendJson(res, 200, publicLeases(leases));
  }

  if (req.method === "POST" && url.pathname === "/leases") {
    const body = await readBody(req, 32_000);
    const payload = JSON.parse(body || "{}");
    return createProviderLease(res, state, payload);
  }

  const leaseMatch = url.pathname.match(/^\/leases\/([^/]+)$/);
  if (leaseMatch && req.method === "GET") {
    const leases = await loadProviderLeases();
    const lease = leases.leases.find((candidate) => candidate.id === leaseMatch[1]);
    return lease ? sendJson(res, 200, publicLease(lease)) : sendJson(res, 404, { error: "lease not found" });
  }

  if (leaseMatch && req.method === "DELETE") {
    return releaseProviderLease(res, req, leaseMatch[1]);
  }

  if (req.method === "POST" && url.pathname === "/jobs") {
    const body = await readBody(req, config.limits?.maxPromptChars || 120_000);
    const payload = JSON.parse(body || "{}");
    return enqueueJob(res, state, {
      model: payload.model,
      prompt: payload.prompt,
      stream: false,
      leaseAuth: readLeaseAuth(req),
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
      leaseAuth: readLeaseAuth(req),
      openAiShape: true,
    });
  }

  sendJson(res, 404, { error: "not found" });
}

function enqueueJob(res, state, jobInput) {
  const limits = state.config.limits || {};
  const maxQueueSize = limits.maxQueueSize || 32;
  const activeLease = activeProviderLease(state.leases);

  if (!jobInput.model || !jobInput.prompt) {
    return sendJson(res, 400, { error: "model and prompt are required" });
  }

  if (activeLease && !leaseAllowsJob(activeLease, jobInput.leaseAuth)) {
    return sendJson(res, 423, {
      error: "provider is locked by an active lease",
      leaseId: activeLease.id,
      expiresAt: activeLease.expiresAt,
    });
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

function providerRecordFromHeartbeat(heartbeat, options) {
  const payload = heartbeat.payload || {};
  const id = payload.provider;
  if (!id) {
    throw new Error("Provider heartbeat is missing provider identity");
  }
  const models = mergeHeartbeatModels(payload.models || [], payload.prices || {});

  return {
    id,
    name: options.name || id,
    endpoint: normalizeEndpoint(options.endpoint),
    source: options.source,
    status: payload.status || "UNKNOWN",
    models,
    hardware: payload.hardware || {},
    prices: payload.prices || {},
    schedule: payload.schedule || null,
    capacity: payload.capacity || {},
    heartbeat,
    reputation: {
      score: 0,
      rentals: 0,
      failures: 0,
    },
    updatedAt: new Date().toISOString(),
  };
}

function mergeHeartbeatModels(models, prices) {
  const byName = new Map();
  for (const model of models) {
    if (model?.name) {
      byName.set(model.name, model);
    }
  }
  for (const model of Object.keys(prices)) {
    if (!byName.has(model)) {
      byName.set(model, { name: model, source: "price" });
    }
  }
  return Array.from(byName.values());
}

function verifyHeartbeat(heartbeat) {
  const payload = heartbeat.payload;
  const signature = heartbeat.signature;
  const publicKey = payload?.provider;
  if (!payload || !signature || !publicKey) {
    throw new Error("Provider heartbeat is missing payload, signature or provider key");
  }

  const ok = nacl.sign.detached.verify(
    Buffer.from(JSON.stringify(payload)),
    Buffer.from(signature, "base64"),
    bs58.decode(publicKey),
  );
  if (!ok) {
    throw new Error(`Invalid heartbeat signature for provider ${publicKey}`);
  }
}

async function upsertProviderRecord(record) {
  const registry = await loadRegistry();
  const existing = registry.providers.find((provider) => provider.id === record.id);
  const providers = registry.providers.filter((provider) => provider.id !== record.id);
  providers.push({
    ...record,
    reputation: existing?.reputation || record.reputation,
    firstSeenAt: existing?.firstSeenAt || record.updatedAt,
  });
  await saveRegistry({ ...registry, providers, updatedAt: new Date().toISOString() });
}

async function selectProvider({ model, maxPrice, providerId, availableOnly }) {
  const registry = await loadRegistry();
  const providers = registry.providers
    .map((provider) => ({ ...provider, availableNow: isAvailableNow(provider.schedule) }))
    .filter((provider) => !providerId || provider.id === providerId)
    .filter((provider) => provider.status === "ONLINE")
    .filter((provider) => provider.models.some((candidate) => candidate.name?.toLowerCase().includes(model.toLowerCase())))
    .filter((provider) => Number.isFinite(minProviderPrice(provider, model)))
    .filter((provider) => maxPrice == null || minProviderPrice(provider, model) <= maxPrice)
    .filter((provider) => !availableOnly || provider.availableNow)
    .sort(compareProviders);

  if (providers.length === 0) {
    throw new Error("No matching online provider found. Run `solai provider refresh` or probe a provider endpoint.");
  }

  return providers[0];
}

async function recordProviderOutcome(providerId, success) {
  const registry = await loadRegistry();
  const providers = registry.providers.map((provider) => {
    if (provider.id !== providerId) {
      return provider;
    }

    const reputation = {
      score: provider.reputation?.score || 0,
      rentals: provider.reputation?.rentals || 0,
      failures: provider.reputation?.failures || 0,
    };
    reputation.rentals += 1;
    if (success) {
      reputation.score += 1;
    } else {
      reputation.failures += 1;
      reputation.score -= 1;
    }
    return { ...provider, reputation, updatedAt: new Date().toISOString() };
  });
  await saveRegistry({ ...registry, providers, updatedAt: new Date().toISOString() });
}

function marketplaceChoices(providers, { modelFilter, maxPrice, includeUnavailable }) {
  const choices = [];
  for (const provider of providers) {
    const availableNow = isAvailableNow(provider.schedule);
    const reputation = normalizeReputation(provider.reputation);
    const pricedModels = Object.entries(provider.prices || {})
      .filter(([model, price]) => {
        return Number.isFinite(Number(price))
          && (!modelFilter || model.toLowerCase().includes(modelFilter.toLowerCase()))
          && (maxPrice == null || Number(price) <= maxPrice);
      });

    for (const [model, price] of pricedModels) {
      const status = provider.status || "UNKNOWN";
      if (!includeUnavailable && (status !== "ONLINE" || !availableNow)) {
        continue;
      }
      choices.push({
        choice: choices.length + 1,
        providerId: provider.id,
        name: provider.name || provider.id,
        endpoint: provider.endpoint,
        model,
        pricePerHour: Number(price),
        status,
        availableNow,
        reputation,
        hardware: provider.hardware || {},
        updatedAt: provider.updatedAt,
      });
    }
  }

  return choices.sort(compareChoices).map((choice, index) => ({ ...choice, choice: index + 1 }));
}

function compareChoices(a, b) {
  if (a.availableNow !== b.availableNow) {
    return a.availableNow ? -1 : 1;
  }
  if (a.status !== b.status) {
    return a.status === "ONLINE" ? -1 : 1;
  }
  if (a.pricePerHour !== b.pricePerHour) {
    return a.pricePerHour - b.pricePerHour;
  }
  return b.reputation.score - a.reputation.score;
}

function normalizeReputation(reputation) {
  return {
    score: reputation?.score || 0,
    rentals: reputation?.rentals || 0,
    failures: reputation?.failures || 0,
  };
}

async function saveProviderChoices(choices) {
  fs.writeFileSync(PROVIDER_CHOICES_FILE, `${JSON.stringify({
    version: 1,
    generatedAt: new Date().toISOString(),
    choices,
  }, null, 2)}\n`, { mode: 0o600 });
}

async function loadProviderChoices() {
  if (!fs.existsSync(PROVIDER_CHOICES_FILE)) {
    throw new Error("No provider choices found. Run `solai marketplace list` first.");
  }
  const choices = JSON.parse(fs.readFileSync(PROVIDER_CHOICES_FILE, "utf8"));
  return Array.isArray(choices.choices) ? choices.choices : [];
}

async function choiceFromArgs(args) {
  const choiceText = valueAfter(args, "--choice");
  if (!choiceText) {
    return null;
  }
  const choiceNumber = Number(choiceText);
  if (!Number.isInteger(choiceNumber) || choiceNumber < 1) {
    throw new Error("--choice must be a positive number from `solai marketplace list`");
  }
  const choices = await loadProviderChoices();
  const choice = choices.find((candidate) => candidate.choice === choiceNumber);
  if (!choice) {
    throw new Error(`Choice ${choiceNumber} was not found. Run ` + "`solai marketplace list` again.");
  }
  return choice;
}

async function createProviderLease(res, state, payload) {
  const model = payload.model;
  const renter = payload.renter;
  const hours = parseHours(payload.hours);
  if (!model || !renter || !Number.isFinite(hours)) {
    return sendJson(res, 400, { error: "model, renter and positive integer hours are required" });
  }

  const activeLease = activeProviderLease(state.leases);
  if (activeLease) {
    return sendJson(res, 423, {
      error: "provider already has an active exclusive lease",
      leaseId: activeLease.id,
      expiresAt: activeLease.expiresAt,
    });
  }

  const pricePerHour = Number(state.config.prices?.[model]);
  if (!Number.isFinite(pricePerHour)) {
    return sendJson(res, 400, { error: `provider has no SOLAI/hour price for ${model}` });
  }

  const startsAt = new Date();
  const expiresAt = new Date(startsAt.getTime() + hours * 60 * 60 * 1000);
  const lease = {
    id: cryptoId(),
    provider: state.config.identity?.publicKey || null,
    renter,
    model,
    hours,
    pricePerHour,
    totalSolai: pricePerHour * hours,
    startsAt: startsAt.toISOString(),
    expiresAt: expiresAt.toISOString(),
    status: "ACTIVE",
    authToken: randomBytes(32).toString("base64url"),
    createdAt: startsAt.toISOString(),
    updatedAt: startsAt.toISOString(),
  };

  const leases = await loadProviderLeases();
  leases.leases = expireLeases(leases.leases).concat(lease);
  leases.updatedAt = new Date().toISOString();
  await saveProviderLeases(leases);
  state.leases = leases;
  log(`created lease ${lease.id} renter=${renter} model=${model} hours=${hours}`);

  return sendJson(res, 201, publicLease(lease, { includeAuthToken: true }));
}

async function releaseProviderLease(res, req, leaseId) {
  const auth = readLeaseAuth(req);
  const leases = await loadProviderLeases();
  const lease = leases.leases.find((candidate) => candidate.id === leaseId);
  if (!lease) {
    return sendJson(res, 404, { error: "lease not found" });
  }
  if (!leaseAllowsJob(lease, auth)) {
    return sendJson(res, 403, { error: "lease token is invalid" });
  }

  const updated = { ...lease, status: "RELEASED", releasedAt: new Date().toISOString(), updatedAt: new Date().toISOString() };
  leases.leases = leases.leases.map((candidate) => candidate.id === lease.id ? updated : candidate);
  leases.updatedAt = new Date().toISOString();
  await saveProviderLeases(leases);
  log(`released lease ${lease.id}`);
  return sendJson(res, 200, publicLease(updated));
}

async function loadMarketplaceIdentity() {
  if (fs.existsSync(MARKETPLACE_IDENTITY_FILE)) {
    return JSON.parse(fs.readFileSync(MARKETPLACE_IDENTITY_FILE, "utf8"));
  }
  const identity = createIdentity();
  fs.writeFileSync(MARKETPLACE_IDENTITY_FILE, `${JSON.stringify(identity, null, 2)}\n`, { mode: 0o600 });
  return identity;
}

async function loadMarketplaceLeases() {
  if (!fs.existsSync(MARKETPLACE_LEASES_FILE)) {
    return { version: 1, leases: [], updatedAt: null };
  }
  const leases = JSON.parse(fs.readFileSync(MARKETPLACE_LEASES_FILE, "utf8"));
  return {
    version: leases.version || 1,
    leases: Array.isArray(leases.leases) ? leases.leases : [],
    updatedAt: leases.updatedAt || null,
  };
}

async function saveMarketplaceLeases(leases) {
  fs.writeFileSync(MARKETPLACE_LEASES_FILE, `${JSON.stringify(leases, null, 2)}\n`, { mode: 0o600 });
}

async function upsertMarketplaceLease(lease) {
  const leases = await loadMarketplaceLeases();
  leases.leases = leases.leases.filter((candidate) => candidate.id !== lease.id).concat(lease);
  leases.updatedAt = new Date().toISOString();
  await saveMarketplaceLeases(leases);
}

async function findMarketplaceLease(leaseId) {
  const leases = await loadMarketplaceLeases();
  return leases.leases.find((lease) => lease.id === leaseId) || null;
}

async function findActiveMarketplaceLease({ model, providerId }) {
  const leases = await loadMarketplaceLeases();
  return leases.leases.find((lease) => {
    return isLeaseActive(lease)
      && (!model || lease.model === model)
      && (!providerId || lease.providerId === providerId);
  }) || null;
}

async function markMarketplaceLeaseReleased(leaseId) {
  const leases = await loadMarketplaceLeases();
  leases.leases = leases.leases.map((lease) => {
    if (lease.id !== leaseId) {
      return lease;
    }
    return { ...lease, status: "RELEASED", releasedAt: new Date().toISOString(), updatedAt: new Date().toISOString() };
  });
  leases.updatedAt = new Date().toISOString();
  await saveMarketplaceLeases(leases);
}

async function loadProviderLeases() {
  if (!fs.existsSync(PROVIDER_LEASES_FILE)) {
    return { version: 1, leases: [], updatedAt: null };
  }
  const leases = JSON.parse(fs.readFileSync(PROVIDER_LEASES_FILE, "utf8"));
  return {
    version: leases.version || 1,
    leases: expireLeases(Array.isArray(leases.leases) ? leases.leases : []),
    updatedAt: leases.updatedAt || null,
  };
}

async function saveProviderLeases(leases) {
  fs.writeFileSync(PROVIDER_LEASES_FILE, `${JSON.stringify(leases, null, 2)}\n`, { mode: 0o600 });
}

function expireLeases(leases) {
  const now = Date.now();
  return leases.map((lease) => {
    if (lease.status === "ACTIVE" && new Date(lease.expiresAt).getTime() <= now) {
      return { ...lease, status: "EXPIRED", updatedAt: new Date().toISOString() };
    }
    return lease;
  });
}

function activeProviderLease(leases) {
  return expireLeases(leases.leases || []).find(isLeaseActive) || null;
}

function isLeaseActive(lease) {
  return lease?.status === "ACTIVE"
    && new Date(lease.startsAt).getTime() <= Date.now()
    && new Date(lease.expiresAt).getTime() > Date.now();
}

function leaseAllowsJob(lease, auth) {
  return isLeaseActive(lease)
    && auth?.leaseId === lease.id
    && auth?.token === lease.authToken;
}

function readLeaseAuth(req) {
  return {
    leaseId: req.headers["x-solai-lease-id"] || null,
    token: req.headers["x-solai-lease-token"] || null,
  };
}

function leaseHeaders(lease) {
  return {
    "x-solai-lease-id": lease.id,
    "x-solai-lease-token": lease.authToken,
  };
}

function publicLeases(leases) {
  return {
    version: leases.version || 1,
    leases: (leases.leases || []).map((lease) => publicLease(lease)),
    updatedAt: leases.updatedAt || null,
  };
}

function publicLease(lease, options = {}) {
  const output = {
    id: lease.id,
    provider: lease.provider,
    renter: lease.renter,
    model: lease.model,
    hours: lease.hours,
    pricePerHour: lease.pricePerHour,
    totalSolai: lease.totalSolai,
    startsAt: lease.startsAt,
    expiresAt: lease.expiresAt,
    status: isLeaseActive(lease) ? "ACTIVE" : lease.status,
    createdAt: lease.createdAt,
    updatedAt: lease.updatedAt,
  };
  if (options.includeAuthToken) {
    output.authToken = lease.authToken;
  }
  return output;
}

function publicLocalLease(lease) {
  const { authToken, ...publicLease } = lease;
  return publicLease;
}

function parseHours(value) {
  const hours = Number(value);
  if (!Number.isInteger(hours) || hours < 1 || hours > 720) {
    return Number.NaN;
  }
  return hours;
}

async function loadRegistry() {
  if (!fs.existsSync(REGISTRY_FILE)) {
    return { version: 1, providers: [], updatedAt: null };
  }
  const registry = JSON.parse(fs.readFileSync(REGISTRY_FILE, "utf8"));
  return {
    version: registry.version || 1,
    providers: Array.isArray(registry.providers) ? registry.providers : [],
    updatedAt: registry.updatedAt || null,
  };
}

async function saveRegistry(registry) {
  fs.writeFileSync(REGISTRY_FILE, `${JSON.stringify(registry, null, 2)}\n`, { mode: 0o600 });
}

function normalizeEndpoint(endpoint) {
  const withProtocol = /^https?:\/\//i.test(endpoint) ? endpoint : `http://${endpoint}`;
  const url = new URL(withProtocol);
  url.pathname = url.pathname.replace(/\/+$/, "");
  url.search = "";
  url.hash = "";
  return url.toString().replace(/\/$/, "");
}

function minProviderPrice(provider, modelFilter) {
  const prices = provider.prices || {};
  const entries = Object.entries(prices).filter(([model]) => {
    return !modelFilter || model.toLowerCase().includes(modelFilter.toLowerCase());
  });
  if (entries.length === 0) {
    return Number.POSITIVE_INFINITY;
  }
  return Math.min(...entries.map(([, price]) => Number(price)).filter(Number.isFinite));
}

function formatPrice(price) {
  return Number.isFinite(price) ? `${price}` : "-";
}

function compareProviders(a, b) {
  const aPrice = minProviderPrice(a, null);
  const bPrice = minProviderPrice(b, null);
  if (Number.isFinite(aPrice) && Number.isFinite(bPrice) && aPrice !== bPrice) {
    return aPrice - bPrice;
  }
  if (Number.isFinite(aPrice) !== Number.isFinite(bPrice)) {
    return Number.isFinite(aPrice) ? -1 : 1;
  }
  return new Date(b.updatedAt).getTime() - new Date(a.updatedAt).getTime();
}

function isAvailableNow(schedule) {
  if (!schedule?.from || !schedule?.to || !isTime(schedule.from) || !isTime(schedule.to)) {
    return true;
  }

  const now = new Date();
  const minutes = now.getHours() * 60 + now.getMinutes();
  const [fromHour, fromMinute] = schedule.from.split(":").map(Number);
  const [toHour, toMinute] = schedule.to.split(":").map(Number);
  const from = fromHour * 60 + fromMinute;
  const to = toHour * 60 + toMinute;

  if (from === to) {
    return true;
  }
  if (from < to) {
    return minutes >= from && minutes < to;
  }
  return minutes >= from || minutes < to;
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
