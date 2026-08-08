#!/usr/bin/env node
// Unified entry point for SOLAI Agent.

import { spawn } from "node:child_process";
import { existsSync, realpathSync } from "fs";
import { createRequire } from "node:module";
import path from "path";
import { fileURLToPath } from "url";

// __dirname equivalent in ESM
const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const require = createRequire(import.meta.url);

const PLATFORM_PACKAGE_BY_PLATFORM_AND_ARCH = {
  linux: {
    x64: "solai-linux-x64",
    arm64: "solai-linux-arm64",
  },
  android: {
    x64: "solai-android-x64",
    arm64: "solai-android-arm64",
  },
  darwin: {
    x64: "solai-darwin-x64",
    arm64: "solai-darwin-arm64",
  },
  win32: {
    x64: "solai-win32-x64",
    arm64: "solai-win32-arm64",
  },
};

const { platform, arch } = process;

let targetTriple = null;
let platformPackage = null;
switch (platform) {
  case "linux":
  case "android":
    switch (arch) {
      case "x64":
        targetTriple = "x86_64-unknown-linux-musl";
        platformPackage = PLATFORM_PACKAGE_BY_PLATFORM_AND_ARCH[platform][arch];
        break;
      case "arm64":
        targetTriple = "aarch64-unknown-linux-musl";
        platformPackage = PLATFORM_PACKAGE_BY_PLATFORM_AND_ARCH[platform][arch];
        break;
      default:
        break;
    }
    break;
  case "darwin":
    switch (arch) {
      case "x64":
        targetTriple = "x86_64-apple-darwin";
        platformPackage = PLATFORM_PACKAGE_BY_PLATFORM_AND_ARCH[platform][arch];
        break;
      case "arm64":
        targetTriple = "aarch64-apple-darwin";
        platformPackage = PLATFORM_PACKAGE_BY_PLATFORM_AND_ARCH[platform][arch];
        break;
      default:
        break;
    }
    break;
  case "win32":
    switch (arch) {
      case "x64":
        targetTriple = "x86_64-pc-windows-msvc";
        platformPackage = PLATFORM_PACKAGE_BY_PLATFORM_AND_ARCH[platform][arch];
        break;
      case "arm64":
        targetTriple = "aarch64-pc-windows-msvc";
        platformPackage = PLATFORM_PACKAGE_BY_PLATFORM_AND_ARCH[platform][arch];
        break;
      default:
        break;
    }
    break;
  default:
    break;
}

if (!targetTriple) {
  throw new Error(`Unsupported platform: ${platform} (${arch})`);
}
if (!platformPackage) {
  throw new Error(`Unsupported platform package for: ${platform} (${arch})`);
}

function findSolaiExecutable() {
  let vendorRoot;
  try {
    const packageJsonPath = require.resolve(`${platformPackage}/package.json`);
    vendorRoot = path.join(path.dirname(packageJsonPath), "vendor");
  } catch {
    vendorRoot = path.join(__dirname, "..", "vendor");
  }

  const codexExecutable = path.join(
    vendorRoot,
    targetTriple,
    "bin",
    process.platform === "win32" ? "codex.exe" : "codex",
  );
  if (existsSync(codexExecutable)) {
    return codexExecutable;
  }

  const packageManager = detectPackageManager();
  const updateCommand =
    packageManager === "bun"
      ? "bun install -g solai@latest"
      : "npm install -g solai@latest";
  throw new Error(
    `Missing optional dependency ${platformPackage}. Reinstall SOLAI Agent: ${updateCommand}`,
  );
}

const binaryPath = findSolaiExecutable();

// Use an asynchronous spawn instead of spawnSync so that Node is able to
// respond to signals (e.g. Ctrl-C / SIGINT) while the native binary is
// executing. This allows us to forward those signals to the child process
// and guarantees that when either the child terminates or the parent
// receives a fatal signal, both processes exit in a predictable manner.

/**
 * Use heuristics to detect the package manager that was used to install SOLAI Agent
 * in order to give the user a hint about how to update it.
 */
function detectPackageManager() {
  const userAgent = process.env.npm_config_user_agent || "";
  if (/\bbun\//.test(userAgent)) {
    return "bun";
  }

  const execPath = process.env.npm_execpath || "";
  if (execPath.includes("bun")) {
    return "bun";
  }

  if (
    __dirname.includes(".bun/install/global") ||
    __dirname.includes(".bun\\install\\global")
  ) {
    return "bun";
  }

  return userAgent ? "npm" : null;
}

const packageManagerEnvVar =
  detectPackageManager() === "bun"
    ? "CODEX_MANAGED_BY_BUN"
    : "CODEX_MANAGED_BY_NPM";
const env = {
  ...process.env,
  [packageManagerEnvVar]: "1",
  CODEX_MANAGED_PACKAGE_ROOT: realpathSync(path.join(__dirname, "..")),
};

const child = spawn(binaryPath, process.argv.slice(2), {
  stdio: "inherit",
  env,
});

child.on("error", (err) => {
  // Typically triggered when the binary is missing or not executable.
  // Re-throwing here will terminate the parent with a non-zero exit code
  // while still printing a helpful stack trace.
  // eslint-disable-next-line no-console
  console.error(err);
  process.exit(1);
});

// Forward common termination signals to the child so that it shuts down
// gracefully. In the handler we temporarily disable the default behavior of
// exiting immediately; once the child has been signaled we simply wait for
// its exit event which will in turn terminate the parent (see below).
const forwardSignal = (signal) => {
  if (child.killed) {
    return;
  }
  try {
    child.kill(signal);
  } catch {
    /* ignore */
  }
};

["SIGINT", "SIGTERM", "SIGHUP"].forEach((sig) => {
  process.on(sig, () => forwardSignal(sig));
});

// When the child exits, mirror its termination reason in the parent so that
// shell scripts and other tooling observe the correct exit status.
// Wrap the lifetime of the child process in a Promise so that we can await
// its termination in a structured way. The Promise resolves with an object
// describing how the child exited: either via exit code or due to a signal.
const childResult = await new Promise((resolve) => {
  child.on("exit", (code, signal) => {
    if (signal) {
      resolve({ type: "signal", signal });
    } else {
      resolve({ type: "code", exitCode: code ?? 1 });
    }
  });
});

if (childResult.type === "signal") {
  // Re-emit the same signal so that the parent terminates with the expected
  // semantics (this also sets the correct exit code of 128 + n).
  process.kill(process.pid, childResult.signal);
} else {
  process.exit(childResult.exitCode);
}
