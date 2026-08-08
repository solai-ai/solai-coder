import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";

import { afterEach, beforeEach } from "@jest/globals";

const originalSolaiAgentHome = process.env.CODEX_HOME;
let currentSolaiAgentHome: string | undefined;

beforeEach(async () => {
  currentSolaiAgentHome = await fs.mkdtemp(path.join(os.tmpdir(), "codex-sdk-test-"));
  process.env.CODEX_HOME = currentSolaiAgentHome;
});

afterEach(async () => {
  const codexHomeToDelete = currentSolaiAgentHome;
  currentSolaiAgentHome = undefined;

  if (originalSolaiAgentHome === undefined) {
    delete process.env.CODEX_HOME;
  } else {
    process.env.CODEX_HOME = originalSolaiAgentHome;
  }

  if (codexHomeToDelete) {
    await fs.rm(codexHomeToDelete, { recursive: true, force: true });
  }
});
