import { existsSync, readFileSync, readdirSync, statSync } from "node:fs";
import { extname, join, relative } from "node:path";

const root = process.cwd();
const dist = join(root, "dist");
const allowedExts = new Set([".html", ".css", ".js", ".map", ".txt", ".json", ".ico", ".png", ".svg", ".webp", ".woff", ".woff2"]);
const forbiddenPatterns = [
  { pattern: /(?:src|href)="\/assets\//, message: "index.html uses root-relative assets" },
  { pattern: /(?:src|href)="https?:\/\//, message: "index.html references remote assets" },
  { pattern: /\/src\/main\.tsx/, message: "index.html references the dev entrypoint" },
];
const requiredPatterns = [
  { pattern: /<meta name="referrer" content="no-referrer"\s*\/?>/, message: "index.html is missing referrer policy" },
];

if (!existsSync(dist)) {
  fail("dist/ does not exist. Run npm run build first.");
}

const files = collectFiles(dist);
if (!files.includes("index.html")) {
  fail("dist/index.html is missing");
}

for (const file of files) {
  const ext = extname(file);
  if (!allowedExts.has(ext)) {
    fail(`dist/${file} is not an allowed static asset type`);
  }
}

const html = readFileSync(join(dist, "index.html"), "utf8");
for (const check of forbiddenPatterns) {
  if (check.pattern.test(html)) {
    fail(check.message);
  }
}
for (const check of requiredPatterns) {
  if (!check.pattern.test(html)) {
    fail(check.message);
  }
}

console.log(`OK: dist contains ${files.length} static file(s) and relative assets`);

function collectFiles(dir) {
  const entries = [];
  for (const name of readdirSync(dir)) {
    const path = join(dir, name);
    const stat = statSync(path);
    if (stat.isDirectory()) {
      entries.push(...collectFiles(path));
    } else {
      entries.push(relative(dist, path));
    }
  }
  return entries.sort();
}

function fail(message) {
  console.error(`Static build check failed: ${message}`);
  process.exit(1);
}
