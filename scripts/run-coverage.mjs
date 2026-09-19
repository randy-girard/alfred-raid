import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { collectLcovFiles, writeCoverageReport } from "./coverage-report.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const rawDir = path.join(root, "coverage", "raw");
const outDir = path.join(root, "coverage");
const updateReadme = process.argv.includes("--update-readme");

function llvmEnv() {
  const env = { ...process.env };
  if (env.LLVM_COV && env.LLVM_PROFDATA) return env;
  for (const dir of ["/opt/homebrew/opt/llvm/bin", "/usr/local/opt/llvm/bin"]) {
    const cov = path.join(dir, "llvm-cov");
    const prof = path.join(dir, "llvm-profdata");
    if (fs.existsSync(cov) && fs.existsSync(prof)) {
      env.LLVM_COV = cov;
      env.LLVM_PROFDATA = prof;
      break;
    }
  }
  return env;
}

function run(command, args, extra = {}) {
  const result = spawnSync(command, args, {
    cwd: root,
    stdio: "inherit",
    shell: process.platform === "win32",
    ...extra,
  });
  if ((result.status ?? 1) !== 0) {
    process.exit(result.status ?? 1);
  }
}

function hasCargoLlvmCov() {
  const result = spawnSync("cargo", ["llvm-cov", "--version"], {
    cwd: root,
    encoding: "utf8",
    env: llvmEnv(),
    shell: process.platform === "win32",
  });
  return result.status === 0;
}

fs.rmSync(rawDir, { recursive: true, force: true });
fs.mkdirSync(rawDir, { recursive: true });

run("npx", ["vitest", "run", "--coverage"]);

if (hasCargoLlvmCov()) {
  run("cargo", [
    "llvm-cov",
    "--manifest-path",
    "src-tauri/Cargo.toml",
    "--lcov",
    "--output-path",
    path.join(rawDir, "rust.lcov"),
    "--ignore-filename-regex",
    "(^|/)(target|gen)(/|$)",
  ], { env: llvmEnv() });
} else {
  console.warn(
    "cargo-llvm-cov not found; Rust tests will run without coverage. Install with:\n  rustup component add llvm-tools-preview\n  cargo install cargo-llvm-cov --locked",
  );
  run("cargo", ["test", "--manifest-path", "src-tauri/Cargo.toml"]);
}

const lcovFiles = collectLcovFiles(rawDir);
if (!lcovFiles.length) {
  console.error("No LCOV files were produced under coverage/raw");
  process.exit(1);
}

const summary = writeCoverageReport({
  lcovFiles,
  root,
  outDir,
  readmePath: updateReadme ? path.join(root, "README.md") : null,
});

console.log(
  `Coverage ${summary.pct.toFixed(1)}% (${summary.hit} / ${summary.found}) → ${path.join(outDir, "index.html")}`,
);
