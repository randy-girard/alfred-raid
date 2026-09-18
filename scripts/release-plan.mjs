import { execFileSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import path from "node:path";
import {
  formatChangelog,
  nextVersion,
  parseGitLog,
} from "./changelog.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

function git(args, opts = {}) {
  try {
    return execFileSync("git", args, {
      cwd: root,
      encoding: "utf8",
      stdio: ["ignore", "pipe", "pipe"],
      ...opts,
    }).trim();
  } catch (err) {
    if (opts.allowFail) return "";
    throw err;
  }
}

function currentVersion() {
  return JSON.parse(readFileSync(path.join(root, "package.json"), "utf8")).version;
}

function previousTag() {
  return git(["describe", "--tags", "--abbrev=0", "--match", "v*"], { allowFail: true });
}

function commitsSince(tag) {
  const range = tag ? `${tag}..HEAD` : "HEAD";
  const raw = git(["log", "--no-merges", `--pretty=format:%s%n%b%x1e`, range], {
    allowFail: true,
  });
  return parseGitLog(raw);
}

function setVersion(version) {
  const pkgPath = path.join(root, "package.json");
  const pkg = JSON.parse(readFileSync(pkgPath, "utf8"));
  pkg.version = version;
  writeFileSync(pkgPath, `${JSON.stringify(pkg, null, 2)}\n`);

  const lockPath = path.join(root, "package-lock.json");
  const lock = JSON.parse(readFileSync(lockPath, "utf8"));
  lock.version = version;
  if (lock.packages?.[""]) lock.packages[""].version = version;
  writeFileSync(lockPath, `${JSON.stringify(lock, null, 2)}\n`);

  const tauriPath = path.join(root, "src-tauri/tauri.conf.json");
  const tauri = JSON.parse(readFileSync(tauriPath, "utf8"));
  tauri.version = version;
  writeFileSync(tauriPath, `${JSON.stringify(tauri, null, 2)}\n`);

  const cargoPath = path.join(root, "src-tauri/Cargo.toml");
  const cargo = readFileSync(cargoPath, "utf8");
  let inPackage = false;
  const next = cargo
    .split("\n")
    .map((line) => {
      if (line.startsWith("[")) inPackage = line.trim() === "[package]";
      if (inPackage && /^version\s*=/.test(line)) return `version = "${version}"`;
      return line;
    })
    .join("\n");
  writeFileSync(cargoPath, next);

  const lockToml = path.join(root, "src-tauri/Cargo.lock");
  try {
    const text = readFileSync(lockToml, "utf8");
    writeFileSync(
      lockToml,
      text.replace(
        /name = "alfred-raid"\nversion = "[^"]+"/,
        `name = "alfred-raid"\nversion = "${version}"`,
      ),
    );
  } catch {
    // lockfile may be missing in a fresh checkout
  }
}

function writeOutput(key, value) {
  const file = process.env.GITHUB_OUTPUT;
  if (!file) return;
  writeFileSync(file, `${key}=${value}\n`, { flag: "a" });
}

const bump = process.argv.includes("--bump")
  ? process.argv[process.argv.indexOf("--bump") + 1]
  : "auto";
const explicit = process.argv.includes("--version")
  ? process.argv[process.argv.indexOf("--version") + 1]
  : "";

const previous = previousTag();
const commits = commitsSince(previous);
const version = explicit || nextVersion(currentVersion(), commits, bump || "auto");
const changelog = formatChangelog({
  version,
  previousTag: previous || null,
  commits,
});

setVersion(version);
writeFileSync(path.join(root, "CHANGELOG.release.md"), changelog);

writeOutput("version", version);
writeOutput("tag", `v${version}`);

console.log(changelog);
console.log(`version=${version} tag=v${version} since=${previous || "(none)"}`);
