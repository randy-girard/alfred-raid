import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { describe, expect, it } from "vitest";
import {
  COVERAGE_END,
  COVERAGE_START,
  collectLcovFiles,
  escapeHtml,
  filePageHref,
  formatReadmeSection,
  mergeRecords,
  packageOf,
  parseLcov,
  patchReadme,
  percent,
  relativeTo,
  relativize,
  renderFilePage,
  renderIndex,
  shouldKeep,
  summarize,
  writeCoverageReport,
} from "./coverage-report.mjs";

const LCOV = `TN:
SF:/repo/src/logic.ts
DA:1,3
DA:2,0
DA:3,1
LF:3
LH:2
end_of_record
TN:
SF:/repo/scripts/changelog.mjs
DA:1,1
LF:1
LH:1
end_of_record
TN:
SF:/repo/src-tauri/src/parser.rs
DA:10,0
DA:11,0
LF:2
LH:0
end_of_record
`;

describe("parseLcov", () => {
  it("reads file records and line hits", () => {
    const files = parseLcov(LCOV);
    expect(files).toHaveLength(3);
    expect(files[0].path).toBe("/repo/src/logic.ts");
    expect(files[0].lines.get(2)).toBe(0);
    expect(files[0].hit).toBe(2);
    expect(files[0].found).toBe(3);
  });

  it("counts hits from DA rows when LF/LH are missing", () => {
    const files = parseLcov("SF:a.ts\nDA:1,2\nDA:2,0\nend_of_record\n");
    expect(files[0].found).toBe(2);
    expect(files[0].hit).toBe(1);
  });
});

describe("path helpers", () => {
  it("relativizes paths inside the repo and drops the rest", () => {
    expect(relativize("/repo/src/logic.ts", "/repo")).toBe("src/logic.ts");
    expect(relativize("src/logic.ts", "/repo")).toBe("src/logic.ts");
    expect(relativize("/elsewhere/foo.ts", "/repo")).toBeNull();
  });

  it("groups files into top-level packages and skips junk", () => {
    expect(packageOf("src-tauri/src/chain.rs")).toBe("src-tauri");
    expect(shouldKeep("src/logic.ts")).toBe(true);
    expect(shouldKeep("src/logic.test.ts")).toBe(false);
    expect(shouldKeep("src-tauri/target/debug/foo.rs")).toBe(false);
  });

  it("builds relative links from a file page back to the index", () => {
    expect(filePageHref("src/logic.ts")).toBe("files/src/logic.ts.html");
    expect(relativeTo("files/src/logic.ts.html", "index.html")).toBe("../../index.html");
    expect(relativeTo("files/src-tauri/src/chain.rs.html", "style.css")).toBe("../../../style.css");
  });
});

describe("summarize", () => {
  it("rolls up line coverage by package in a stable order", () => {
    const files = mergeRecords(parseLcov(LCOV), "/repo");
    const summary = summarize(files);
    expect(summary.packages.map((p) => p.name)).toEqual(["src", "scripts", "src-tauri"]);
    expect(summary.found).toBe(6);
    expect(summary.hit).toBe(3);
    expect(percent(3, 6)).toBe(50);
    expect(summary.packages[0].files[0].path).toBe("src/logic.ts");
  });
});

describe("html report", () => {
  it("lists every file under its package and links to a per-file page", () => {
    const summary = summarize(mergeRecords(parseLcov(LCOV), "/repo"));
    const html = renderIndex(summary);
    expect(html).toContain("Alfred coverage");
    expect(html).toContain("src-tauri");
    expect(html).toContain('href="files/src/logic.ts.html"');
    expect(html).toContain('href="files/src-tauri/src/parser.rs.html"');
    expect(html).toContain("2 / 3");
  });

  it("renders source with hit and miss rows and escapes HTML", () => {
    const file = mergeRecords(parseLcov(LCOV), "/repo").find((f) => f.path === "src/logic.ts")!;
    const page = renderFilePage(file, "const a = 1;\nconst b = '<x>';\nconst c = 3;\n", 50);
    expect(page).toContain('class="hit"');
    expect(page).toContain('class="miss"');
    expect(page).toContain("&lt;x&gt;");
    expect(page).not.toContain("<x>");
    expect(escapeHtml("<script>")).toBe("&lt;script&gt;");
  });
});

describe("readme patch", () => {
  it("replaces the marked coverage block", () => {
    const summary = summarize(mergeRecords(parseLcov(LCOV), "/repo"));
    const section = formatReadmeSection(summary);
    expect(section).toContain("**Line coverage:** 50.0%");
    expect(section).toContain("| src |");
    const next = patchReadme(
      `# Alfred\n\n${COVERAGE_START}\nold\n${COVERAGE_END}\n\n## CI\n`,
      section,
    );
    expect(next).toContain("50.0%");
    expect(next).not.toContain("old");
    expect(next).toContain("## CI");
  });

  it("throws when markers are missing", () => {
    expect(() => patchReadme("# hi\n", "x")).toThrow(/coverage markers/);
  });
});

describe("writeCoverageReport", () => {
  it("writes index, css, per-file pages, and updates the README", () => {
    const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "alfred-cov-"));
    const srcDir = path.join(tmp, "src");
    fs.mkdirSync(srcDir);
    fs.writeFileSync(path.join(srcDir, "logic.ts"), "a\nb\n");
    const rawDir = path.join(tmp, "raw");
    fs.mkdirSync(rawDir);
    fs.writeFileSync(
      path.join(rawDir, "js.lcov"),
      "SF:src/logic.ts\nDA:1,1\nDA:2,0\nLF:2\nLH:1\nend_of_record\n",
    );
    const readme = path.join(tmp, "README.md");
    fs.writeFileSync(readme, `intro\n${COVERAGE_START}\n${COVERAGE_END}\n`);
    const outDir = path.join(tmp, "coverage");
    const summary = writeCoverageReport({
      lcovFiles: collectLcovFiles(rawDir),
      root: tmp,
      outDir,
      readmePath: readme,
    });
    expect(summary.pct).toBe(50);
    expect(fs.existsSync(path.join(outDir, "index.html"))).toBe(true);
    expect(fs.existsSync(path.join(outDir, "style.css"))).toBe(true);
    expect(fs.existsSync(path.join(outDir, "files/src/logic.ts.html"))).toBe(true);
    expect(fs.readFileSync(readme, "utf8")).toContain("50.0%");
    fs.rmSync(tmp, { recursive: true, force: true });
  });
});
