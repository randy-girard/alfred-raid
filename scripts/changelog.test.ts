import { describe, expect, it } from "vitest";
import {
  formatChangelog,
  nextVersion,
  parseCommit,
  parseGitLog,
} from "./changelog.mjs";

describe("parseCommit", () => {
  it("reads type, scope, and breaking bang", () => {
    expect(parseCommit("feat(chain): add rampage panel")).toEqual({
      type: "feat",
      scope: "chain",
      breaking: false,
      subject: "add rampage panel",
    });
    expect(parseCommit("feat!: drop say-channel parsing")).toEqual({
      type: "feat",
      scope: null,
      breaking: true,
      subject: "drop say-channel parsing",
    });
  });

  it("treats BREAKING CHANGE in the body as breaking", () => {
    expect(
      parseCommit("fix: rename snapshot event", "BREAKING CHANGE: listeners must use raid-updated"),
    ).toMatchObject({ type: "fix", breaking: true });
  });

  it("skips merges and release chores", () => {
    expect(parseCommit("Merge pull request #12 from goodguys/main")).toBeNull();
    expect(parseCommit("chore(release): 0.2.0")).toBeNull();
  });

  it("buckets unknown subjects as other", () => {
    expect(parseCommit("tweak the tray icon")).toEqual({
      type: "other",
      scope: null,
      breaking: false,
      subject: "tweak the tray icon",
    });
  });
});

describe("parseGitLog", () => {
  it("splits records on the git separator", () => {
    const commits = parseGitLog(
      "feat: add skip for letters\n\n\x1efix(ui): save settings on change\n\n\x1e",
    );
    expect(commits.map((c) => c.type)).toEqual(["feat", "fix"]);
  });
});

describe("nextVersion", () => {
  it("bumps major for breaking, minor for feat, otherwise patch", () => {
    expect(nextVersion("1.2.3", [{ type: "feat", breaking: true }])).toBe("2.0.0");
    expect(nextVersion("1.2.3", [{ type: "feat", breaking: false }])).toBe("1.3.0");
    expect(nextVersion("1.2.3", [{ type: "fix", breaking: false }])).toBe("1.2.4");
  });

  it("honors an explicit bump", () => {
    expect(nextVersion("1.2.3", [{ type: "feat", breaking: false }], "patch")).toBe("1.2.4");
    expect(nextVersion("1.2.3", [], "minor")).toBe("1.3.0");
  });
});

describe("formatChangelog", () => {
  it("groups commits by conventional type since the last tag", () => {
    const md = formatChangelog({
      version: "0.2.0",
      previousTag: "v0.1.0",
      commits: [
        { type: "feat", scope: "chain", breaking: false, subject: "add a rampage panel" },
        { type: "feat", scope: null, breaking: false, subject: "save settings on change" },
        { type: "fix", scope: "ui", breaking: false, subject: "stop take from starting the clock" },
        {
          type: "feat",
          scope: null,
          breaking: true,
          subject: "emit raid-updated instead of chain-updated",
        },
      ],
    });
    expect(md).toContain("# Alfred 0.2.0");
    expect(md).toContain("Changes since v0.1.0.");
    expect(md.indexOf("## Breaking changes")).toBeLessThan(md.indexOf("## Features"));
    expect(md.indexOf("## Features")).toBeLessThan(md.indexOf("## Fixes"));
    expect(md).toContain("- **chain:** add a rampage panel");
    expect(md).toContain("- **ui:** stop take from starting the clock");
    expect(md).toContain("- emit raid-updated instead of chain-updated");
    const features = md.slice(md.indexOf("## Features"), md.indexOf("## Fixes"));
    expect(features).not.toContain("raid-updated");
  });
});
