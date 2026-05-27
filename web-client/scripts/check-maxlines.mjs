import { readdirSync, readFileSync, statSync } from "node:fs";
import { extname, join, relative } from "node:path";

const root = process.cwd();
const config = JSON.parse(readFileSync(join(root, "check-maxline.json"), "utf8"));
const maxLines = config.max_lines ?? 250;
const includeExts = new Set((config.include_exts ?? []).map((ext) => `.${ext}`));
const excludeDirs = new Set(config.exclude_dirs ?? []);
const excludeFiles = new Set(config.exclude_files ?? []);
const excludeGlobs = config.exclude_globs ?? [];
const violations = [];
let counted = 0;

walk(root);

if (violations.length > 0) {
  for (const item of violations) {
    console.error(`${item.file}: ${item.lines} lines exceeds ${maxLines}`);
  }
  process.exit(1);
}

console.log(`OK: checked ${counted} file(s), all <= ${maxLines} lines`);

function walk(dir) {
  for (const name of readdirSync(dir)) {
    const path = join(dir, name);
    const rel = relative(root, path);
    const stat = statSync(path);
    if (stat.isDirectory()) {
      if (!excludeDirs.has(name)) {
        walk(path);
      }
      continue;
    }
    if (!includeExts.has(extname(name)) || excludeFiles.has(rel) || excludedByGlob(rel)) {
      continue;
    }
    counted += 1;
    const lines = countLines(readFileSync(path, "utf8"));
    if (lines > maxLines) {
      violations.push({ file: rel, lines });
    }
  }
}

function countLines(text) {
  const lines = text.split(/\r\n|\r|\n/);
  if (lines.at(-1) === "") {
    lines.pop();
  }
  return lines.length;
}

function excludedByGlob(rel) {
  return excludeGlobs.some((glob) => glob === rel || glob.endsWith("/**/*") && rel.startsWith(glob.slice(0, -4)));
}
