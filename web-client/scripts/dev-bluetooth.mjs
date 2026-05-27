import { existsSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { spawn, spawnSync } from "node:child_process";

const root = process.cwd();
const url = process.env.YUNDRONE_WEB_URL ?? "http://localhost:5173/";
const profileDir = process.env.YUNDRONE_CHROME_PROFILE ?? join(tmpdir(), "yundrone-webbt-use");
const openOnly = process.argv.includes("--open-only");

let viteProcess;

if (!openOnly) {
  viteProcess = await ensureDevServer();
}

openBrowser();

if (viteProcess) {
  console.log("Vite is running for Web Bluetooth. Press Ctrl+C to stop.");
  installShutdown(viteProcess);
  await new Promise((resolve) => viteProcess.on("exit", resolve));
}

async function ensureDevServer() {
  if (await isReachable(url)) {
    console.log(`Using existing dev server at ${url}`);
    return undefined;
  }

  const vite = viteBin();
  const child = spawn(vite, ["--host", "0.0.0.0"], {
    cwd: root,
    env: process.env,
    stdio: "inherit",
  });
  await waitForServer(url);
  return child;
}

function openBrowser() {
  const browser = resolveBrowser();
  const args = [
    `--user-data-dir=${profileDir}`,
    "--no-first-run",
    "--disable-default-apps",
    "--enable-experimental-web-platform-features",
    "--new-window",
    url,
  ];
  const child = spawn(browser.path, args, {
    detached: true,
    stdio: "ignore",
  });
  child.unref();
  console.log(`Opened ${url} with Web Bluetooth flags via ${browser.name}: ${browser.path}`);
}

function resolveBrowser() {
  const candidates = browserCandidates();

  for (const candidate of candidates) {
    if (candidate.path.includes("\\") && !existsSync(candidate.path)) {
      continue;
    }
    const result = spawnSync(candidate.path, ["--version"], { encoding: "utf8" });
    if (result.status === 0) {
      return candidate;
    }
  }

  fail("Chrome or Edge was not found. Install one or set CHROME_BIN=/path/to/browser.");
}

function browserCandidates() {
  if (process.platform === "win32") {
    return windowsBrowserCandidates();
  }
  if (process.platform === "darwin") {
    return macBrowserCandidates();
  }
  return linuxBrowserCandidates();
}

function windowsBrowserCandidates() {
  const local = process.env.LOCALAPPDATA;
  const programFiles = process.env.ProgramFiles;
  const programFilesX86 = process.env["ProgramFiles(x86)"];
  return [
    namedBrowser("configured browser", process.env.CHROME_BIN),
    namedBrowser("Google Chrome", joinMaybe(programFiles, "Google/Chrome/Application/chrome.exe")),
    namedBrowser("Google Chrome", joinMaybe(programFilesX86, "Google/Chrome/Application/chrome.exe")),
    namedBrowser("Google Chrome", joinMaybe(local, "Google/Chrome/Application/chrome.exe")),
    namedBrowser("Microsoft Edge", joinMaybe(programFiles, "Microsoft/Edge/Application/msedge.exe")),
    namedBrowser("Microsoft Edge", joinMaybe(programFilesX86, "Microsoft/Edge/Application/msedge.exe")),
    namedBrowser("Microsoft Edge", joinMaybe(local, "Microsoft/Edge/Application/msedge.exe")),
    namedBrowser("Google Chrome", "chrome"),
    namedBrowser("Microsoft Edge", "msedge"),
  ].filter((item) => item.path);
}

function macBrowserCandidates() {
  return [
    namedBrowser("configured browser", process.env.CHROME_BIN),
    namedBrowser("Google Chrome", "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"),
    namedBrowser("Microsoft Edge", "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge"),
    namedBrowser("Google Chrome", "google-chrome"),
  ].filter((item) => item.path);
}

function linuxBrowserCandidates() {
  return [
    namedBrowser("configured browser", process.env.CHROME_BIN),
    namedBrowser("Google Chrome", "/opt/google/chrome/chrome"),
    namedBrowser("Google Chrome", "/usr/bin/google-chrome"),
    namedBrowser("Google Chrome", "/usr/bin/google-chrome-stable"),
    namedBrowser("Google Chrome", "google-chrome"),
    namedBrowser("Google Chrome", "google-chrome-stable"),
    namedBrowser("Chromium", "chromium"),
    namedBrowser("Chromium", "chromium-browser"),
    namedBrowser("Microsoft Edge", "microsoft-edge"),
  ].filter((item) => item.path);
}

function namedBrowser(name, path) {
  return { name, path };
}

function joinMaybe(base, tail) {
  return base ? join(base, tail) : undefined;
}

function viteBin() {
  const name = process.platform === "win32" ? "vite.cmd" : "vite";
  const local = join(root, "node_modules", ".bin", name);
  if (existsSync(local)) {
    return local;
  }
  fail("Vite is not installed. Run npm ci in web-client first.");
}

async function waitForServer(targetUrl, timeoutMs = 15000) {
  const started = Date.now();
  while (Date.now() - started < timeoutMs) {
    if (await isReachable(targetUrl)) {
      return;
    }
    await sleep(250);
  }
  fail(`Dev server did not become reachable at ${targetUrl}`);
}

async function isReachable(targetUrl) {
  try {
    const response = await fetch(targetUrl, { method: "HEAD" });
    return response.ok;
  } catch {
    return false;
  }
}

function installShutdown(child) {
  for (const signal of ["SIGINT", "SIGTERM"]) {
    process.on(signal, () => {
      child.kill("SIGTERM");
      process.exit(signal === "SIGINT" ? 130 : 143);
    });
  }
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function fail(message) {
  console.error(message);
  process.exit(1);
}
