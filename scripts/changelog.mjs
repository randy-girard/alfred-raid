const TYPE_HEADINGS = [
  ["breaking", "Breaking changes"],
  ["feat", "Features"],
  ["fix", "Fixes"],
  ["perf", "Performance"],
  ["refactor", "Refactors"],
  ["docs", "Docs"],
  ["test", "Tests"],
  ["build", "Build"],
  ["ci", "CI"],
  ["chore", "Chores"],
  ["revert", "Reverts"],
  ["other", "Other"],
];

const TYPE_RE =
  /^(feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert)(\([^)]+\))?(!)?:\s*(.+)$/i;
const SKIP_RE = /^(merge\b|chore\(release\):)/i;

export function parseCommit(subject, body = "") {
  const line = subject.trim();
  if (!line || SKIP_RE.test(line)) return null;
  const breakingBody = /BREAKING CHANGE:/i.test(body);
  const match = line.match(TYPE_RE);
  if (!match) {
    return {
      type: "other",
      scope: null,
      breaking: breakingBody,
      subject: line,
    };
  }
  return {
    type: match[1].toLowerCase(),
    scope: match[2] ? match[2].slice(1, -1) : null,
    breaking: Boolean(match[3]) || breakingBody,
    subject: match[4].trim(),
  };
}

export function parseGitLog(raw) {
  if (!raw.trim()) return [];
  return raw
    .split("\x1e")
    .map((chunk) => chunk.replace(/^\n+|\n+$/g, ""))
    .filter(Boolean)
    .map((chunk) => {
      const nl = chunk.indexOf("\n");
      const subject = nl === -1 ? chunk : chunk.slice(0, nl);
      const body = nl === -1 ? "" : chunk.slice(nl + 1);
      return parseCommit(subject, body);
    })
    .filter(Boolean);
}

export function nextVersion(current, commits, bump = "auto") {
  const parts = current.split(".").map((n) => Number(n));
  if (parts.length !== 3 || parts.some((n) => !Number.isFinite(n))) {
    throw new Error(`Invalid version: ${current}`);
  }
  let kind = bump;
  if (kind === "auto") {
    if (commits.some((c) => c.breaking)) kind = "major";
    else if (commits.some((c) => c.type === "feat")) kind = "minor";
    else kind = "patch";
  }
  if (kind === "major") return `${parts[0] + 1}.0.0`;
  if (kind === "minor") return `${parts[0]}.${parts[1] + 1}.0`;
  if (kind === "patch") return `${parts[0]}.${parts[1]}.${parts[2] + 1}`;
  throw new Error(`Invalid bump: ${bump}`);
}

function entry(commit) {
  const scope = commit.scope ? `**${commit.scope}:** ` : "";
  return `- ${scope}${commit.subject}`;
}

export function formatChangelog({ version, previousTag, commits }) {
  const groups = new Map(TYPE_HEADINGS.map(([key]) => [key, []]));
  for (const commit of commits) {
    if (commit.breaking) {
      groups.get("breaking").push(commit);
      continue;
    }
    const bucket = groups.has(commit.type) ? commit.type : "other";
    groups.get(bucket).push(commit);
  }

  const lines = [`# Alfred ${version}`, ""];
  lines.push(previousTag ? `Changes since ${previousTag}.` : "Initial release.");
  lines.push("");

  let sections = false;
  for (const [key, heading] of TYPE_HEADINGS) {
    const items = groups.get(key);
    if (!items.length) continue;
    sections = true;
    lines.push(`## ${heading}`, "");
    for (const commit of items) lines.push(entry(commit));
    lines.push("");
  }
  if (!sections) {
    lines.push("No conventional commits since the last release.", "");
  }
  return lines.join("\n").trim() + "\n";
}
