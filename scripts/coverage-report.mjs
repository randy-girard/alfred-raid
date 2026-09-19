import fs from "node:fs";
import path from "node:path";

export const COVERAGE_START = "<!-- coverage:start -->";
export const COVERAGE_END = "<!-- coverage:end -->";

const PACKAGE_ORDER = ["src", "scripts", "src-tauri"];
const PACKAGE_LABELS = {
  src: "src",
  scripts: "scripts",
  "src-tauri": "src-tauri",
};

export function parseLcov(text) {
  const files = [];
  let current = null;
  for (const rawLine of text.split(/\r?\n/)) {
    const line = rawLine.trim();
    if (!line) continue;
    if (line.startsWith("SF:")) {
      current = { path: line.slice(3).trim(), lines: new Map(), found: 0, hit: 0 };
      continue;
    }
    if (!current) continue;
    if (line.startsWith("DA:")) {
      const [num, count] = line.slice(3).split(",");
      const lineNo = Number(num);
      const hits = Number(count);
      if (Number.isFinite(lineNo) && Number.isFinite(hits)) {
        current.lines.set(lineNo, (current.lines.get(lineNo) ?? 0) + hits);
      }
      continue;
    }
    if (line.startsWith("LF:")) {
      current.found = Number(line.slice(3));
      continue;
    }
    if (line.startsWith("LH:")) {
      current.hit = Number(line.slice(3));
      continue;
    }
    if (line === "end_of_record") {
      if (!current.found) current.found = current.lines.size;
      if (!current.hit) {
        current.hit = [...current.lines.values()].filter((n) => n > 0).length;
      }
      files.push(current);
      current = null;
    }
  }
  return files;
}

export function relativize(filePath, root) {
  const absRoot = path.resolve(root);
  const resolved = path.isAbsolute(filePath)
    ? path.resolve(filePath)
    : path.resolve(absRoot, filePath);
  const rel = path.relative(absRoot, resolved);
  if (!rel || rel.startsWith("..") || path.isAbsolute(rel)) return null;
  return rel.split(path.sep).join("/");
}

export function packageOf(relPath) {
  const top = relPath.split("/")[0];
  return top || "(root)";
}

export function shouldKeep(relPath) {
  if (!relPath) return false;
  const skip =
    /(^|\/)(node_modules|target|dist|coverage|gen)(\/|$)/.test(relPath) ||
    /\.test\.(ts|js|mjs)$/.test(relPath) ||
    /\/icons\//.test(relPath);
  return !skip;
}

export function mergeRecords(records, root) {
  const byFile = new Map();
  for (const record of records) {
    const rel = relativize(record.path, root);
    if (!shouldKeep(rel)) continue;
    const existing = byFile.get(rel) ?? {
      path: rel,
      lines: new Map(),
    };
    for (const [line, hits] of record.lines) {
      existing.lines.set(line, (existing.lines.get(line) ?? 0) + hits);
    }
    byFile.set(rel, existing);
  }
  return [...byFile.values()].map((file) => {
    const found = file.lines.size;
    const hit = [...file.lines.values()].filter((n) => n > 0).length;
    return { ...file, found, hit, pct: percent(hit, found) };
  });
}

export function percent(hit, found) {
  if (!found) return 100;
  return (100 * hit) / found;
}

export function fmtPct(value) {
  return `${value.toFixed(1)}%`;
}

export function pctClass(value) {
  if (value >= 90) return "hi";
  if (value >= 70) return "mid";
  return "lo";
}

export function summarize(files) {
  const packages = new Map();
  for (const file of files) {
    const name = packageOf(file.path);
    const pack = packages.get(name) ?? { name, files: [], hit: 0, found: 0 };
    pack.files.push(file);
    pack.hit += file.hit;
    pack.found += file.found;
    packages.set(name, pack);
  }
  for (const pack of packages.values()) {
    pack.files.sort((a, b) => a.path.localeCompare(b.path));
    pack.pct = percent(pack.hit, pack.found);
    pack.label = PACKAGE_LABELS[pack.name] ?? pack.name;
  }
  const ordered = [
    ...PACKAGE_ORDER.filter((name) => packages.has(name)).map((name) => packages.get(name)),
    ...[...packages.values()]
      .filter((pack) => !PACKAGE_ORDER.includes(pack.name))
      .sort((a, b) => a.name.localeCompare(b.name)),
  ];
  const hit = files.reduce((sum, file) => sum + file.hit, 0);
  const found = files.reduce((sum, file) => sum + file.found, 0);
  return {
    hit,
    found,
    pct: percent(hit, found),
    files: [...files].sort((a, b) => a.path.localeCompare(b.path)),
    packages: ordered,
  };
}

export function collectLcovFiles(rawDir) {
  if (!fs.existsSync(rawDir)) return [];
  const found = [];
  for (const entry of fs.readdirSync(rawDir, { recursive: true })) {
    const full = path.join(rawDir, entry.toString());
    if (!fs.statSync(full).isFile()) continue;
    if (entry.toString().endsWith(".lcov") || entry.toString().endsWith("lcov.info")) {
      found.push(full);
    }
  }
  return found.sort();
}

export function loadSummary(lcovFiles, root) {
  const records = lcovFiles.flatMap((file) => parseLcov(fs.readFileSync(file, "utf8")));
  return summarize(mergeRecords(records, root));
}

export function filePageHref(relPath) {
  return `files/${relPath}.html`;
}

export function relativeTo(fromRel, toRel) {
  const fromDir = path.posix.dirname(fromRel);
  let rel = path.posix.relative(fromDir, toRel);
  if (!rel.startsWith(".")) rel = `./${rel}`;
  return rel;
}

export function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

const CSS = `*{box-sizing:border-box}
body{margin:0;font:15px/1.45 ui-sans-serif,system-ui,-apple-system,Segoe UI,sans-serif;color:#1b1f24;background:#f4f1ea}
a{color:#1f4b8f;text-decoration:none}
a:hover{text-decoration:underline}
main{max-width:980px;margin:0 auto;padding:32px 20px 64px}
h1{font-size:1.6rem;margin:0 0 8px}
h2{font-size:1.15rem;margin:28px 0 10px}
.muted{color:#5c6570}
.overall{display:flex;flex-wrap:wrap;gap:16px 28px;align-items:baseline;margin:18px 0 8px}
.pct{font-size:2rem;font-weight:700;letter-spacing:-0.03em}
.hi{color:#1f7a4d}.mid{color:#8a6a12}.lo{color:#a33b32}
.bar{height:8px;background:#e2ddd2;border-radius:99px;overflow:hidden;min-width:120px}
.bar>span{display:block;height:100%;background:#2f9e6b}
.bar.mid>span{background:#d2a21b}
.bar.lo>span{background:#d2645a}
table{width:100%;border-collapse:collapse;background:#fff;border:1px solid #e4dfd4;border-radius:10px;overflow:hidden}
th,td{padding:8px 12px;text-align:left;border-bottom:1px solid #eee9de}
th{font-size:12px;text-transform:uppercase;letter-spacing:.04em;color:#66707a;background:#faf8f3}
td.num,th.num{text-align:right;font-variant-numeric:tabular-nums}
tr:last-child td{border-bottom:0}
.code{margin:0;background:#fff;border:1px solid #e4dfd4;border-radius:10px;overflow:auto}
.code table{border:0;border-radius:0}
.code td{vertical-align:top;font:13px/1.45 ui-monospace,SFMono-Regular,Menlo,Consolas,monospace}
.code .n{width:1%;color:#8a939c;text-align:right;user-select:none;padding-right:8px}
.code .h{width:1%;color:#8a939c;text-align:right;padding-right:12px}
.code .hit td{background:#e8f6ee}
.code .miss td{background:#fdecea}
.crumb{margin-bottom:16px}
`;

export function renderIndex(summary) {
  const packageBlocks = summary.packages
    .map((pack) => {
      const rows = pack.files
        .map((file) => {
          const href = filePageHref(file.path);
          return `<tr>
  <td><a href="${escapeHtml(href)}">${escapeHtml(file.path)}</a></td>
  <td class="num ${pctClass(file.pct)}">${fmtPct(file.pct)}</td>
  <td class="num">${file.hit} / ${file.found}</td>
</tr>`;
        })
        .join("\n");
      return `<section>
  <h2>${escapeHtml(pack.label)} <span class="muted ${pctClass(pack.pct)}">${fmtPct(pack.pct)}</span></h2>
  <p class="muted">${pack.hit} / ${pack.found} lines</p>
  <div class="bar ${pctClass(pack.pct)}"><span style="width:${pack.pct.toFixed(1)}%"></span></div>
  <table>
    <thead><tr><th>File</th><th class="num">Coverage</th><th class="num">Hit / lines</th></tr></thead>
    <tbody>
${rows}
    </tbody>
  </table>
</section>`;
    })
    .join("\n");

  return `<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>Alfred coverage</title>
  <link rel="stylesheet" href="style.css" />
</head>
<body>
<main>
  <h1>Alfred coverage</h1>
  <div class="overall">
    <div class="pct ${pctClass(summary.pct)}">${fmtPct(summary.pct)}</div>
    <div>
      <div><strong>${summary.hit} / ${summary.found}</strong> lines covered</div>
      <div class="muted">${summary.files.length} files in ${summary.packages.length} packages</div>
    </div>
  </div>
  <div class="bar ${pctClass(summary.pct)}"><span style="width:${summary.pct.toFixed(1)}%"></span></div>
  ${packageBlocks}
</main>
</body>
</html>
`;
}

export function renderFilePage(file, source, summaryPct) {
  const hrefIndex = relativeTo(filePageHref(file.path), "index.html");
  const hrefCss = relativeTo(filePageHref(file.path), "style.css");
  const sourceLines = source.replace(/\n$/, "").split("\n");
  const maxLine = Math.max(sourceLines.length, ...file.lines.keys(), 0);
  const rows = [];
  for (let n = 1; n <= maxLine; n += 1) {
    const text = sourceLines[n - 1] ?? "";
    const hits = file.lines.has(n) ? file.lines.get(n) : null;
    const klass = hits == null ? "" : hits > 0 ? "hit" : "miss";
    const hitLabel = hits == null ? "" : String(hits);
    rows.push(
      `<tr class="${klass}"><td class="n">${n}</td><td class="h">${hitLabel}</td><td>${escapeHtml(text) || "&nbsp;"}</td></tr>`,
    );
  }
  return `<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>${escapeHtml(file.path)} coverage</title>
  <link rel="stylesheet" href="${escapeHtml(hrefCss)}" />
</head>
<body>
<main>
  <p class="crumb"><a href="${escapeHtml(hrefIndex)}">All files</a> · ${escapeHtml(packageOf(file.path))}</p>
  <h1>${escapeHtml(file.path)}</h1>
  <div class="overall">
    <div class="pct ${pctClass(file.pct)}">${fmtPct(file.pct)}</div>
    <div class="muted">${file.hit} / ${file.found} lines · project ${fmtPct(summaryPct)}</div>
  </div>
  <div class="code"><table>
${rows.join("\n")}
  </table></div>
</main>
</body>
</html>
`;
}

export function formatReadmeSection(summary) {
  const rows = summary.packages.map(
    (pack) => `| ${pack.label} | ${fmtPct(pack.pct)} | ${pack.hit} / ${pack.found} |`,
  );
  return [
    COVERAGE_START,
    `**Line coverage:** ${fmtPct(summary.pct)} (${summary.hit} / ${summary.found}).`,
    "",
    "| Package | Coverage | Hit / lines |",
    "| --- | ---: | ---: |",
    ...rows,
    "",
    "The HTML report is gitignored. Run `make test` and open `coverage/index.html`.",
    COVERAGE_END,
  ].join("\n");
}

export function patchReadme(markdown, section) {
  const start = markdown.indexOf(COVERAGE_START);
  const end = markdown.indexOf(COVERAGE_END);
  if (start === -1 || end === -1 || end < start) {
    throw new Error("README.md is missing coverage markers");
  }
  const after = markdown.slice(end + COVERAGE_END.length);
  return `${markdown.slice(0, start)}${section}${after.startsWith("\n") ? after : `\n${after}`}`;
}

export function writeCoverageReport({ lcovFiles, root, outDir, readmePath }) {
  const summary = loadSummary(lcovFiles, root);
  fs.mkdirSync(outDir, { recursive: true });
  const filesDir = path.join(outDir, "files");
  fs.rmSync(filesDir, { recursive: true, force: true });
  fs.writeFileSync(path.join(outDir, "style.css"), CSS);
  fs.writeFileSync(path.join(outDir, "index.html"), renderIndex(summary));
  for (const file of summary.files) {
    const abs = path.join(root, file.path);
    const source = fs.existsSync(abs) ? fs.readFileSync(abs, "utf8") : "";
    const dest = path.join(outDir, filePageHref(file.path));
    fs.mkdirSync(path.dirname(dest), { recursive: true });
    fs.writeFileSync(dest, renderFilePage(file, source, summary.pct));
  }
  fs.writeFileSync(path.join(outDir, "summary.json"), `${JSON.stringify(summary, replacer, 2)}\n`);
  if (readmePath) {
    const next = patchReadme(fs.readFileSync(readmePath, "utf8"), formatReadmeSection(summary));
    fs.writeFileSync(readmePath, next);
  }
  return summary;
}

function replacer(_key, value) {
  if (value instanceof Map) return Object.fromEntries(value);
  return value;
}
