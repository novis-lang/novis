import { describe, expect, test } from "bun:test";
import { join } from "node:path";
import { gunzipSync, inflateRawSync } from "node:zlib";
import { ROOT } from "../lib/paths.ts";
import {
  bumpManifest,
  bumpPackageJson,
  CHANGELOG_HEADER,
  classify,
  dockerTags,
  FIRST_NOTES,
  firstReleaseNotes,
  fixesIn,
  greenFullRun,
  nextVersion,
  notesFor,
  parseLog,
  pinProblems,
  planVersion,
  prependChangelog,
  releaseTags,
  renderNotes,
  tarGz,
  zip,
  type ArchiveFile,
  type WorkflowRun,
} from "../cmd/release.ts";

const MANIFEST = [
  "[workspace]",
  'members = ["crates/*"]',
  "",
  "[workspace.package]",
  'version = "0.0.1"',
  'edition = "2024"',
  "",
  "[workspace.dependencies]",
  'nvs-a = { path = "crates/nvs-a", version = "0.0.1" }',
  "# a comment with version = \"9.9.9\" in it",
  'nvs-b = { path = "crates/nvs-b", default-features = false, version = "0.0.1" }',
  'serde = { version = "1.0.200", features = ["derive"] }',
  "",
  "[profile.release]",
  'version = "not a pin"',
  "",
].join("\n");

describe("nv release versions", () => {
  test("below 1.0 the breaking slot is MINOR, and minor and patch give the same number", () => {
    expect(nextVersion("0.0.1", "major")).toBe("0.1.0");
    expect(nextVersion("0.3.4", "minor")).toBe("0.3.5");
    expect(nextVersion("0.3.4", "patch")).toBe("0.3.5");
    expect(nextVersion("1.2.3", "major")).toBe("2.0.0");
    expect(nextVersion("1.2.3", "minor")).toBe("1.3.0");
  });

  test("release tags sort by version, not by name, and other tags are ignored", () => {
    expect(releaseTags("v0.9.0\nv0.10.0\npre-overhaul\nv1.0.0-rc1\n").map((t) => t.name)).toEqual(["v0.10.0", "v0.9.0"]);
  });

  test("with no tag, patch and minor release the current version, and major alone needs the contract", () => {
    expect(planVersion("0.0.1", "patch", "", [], false)).toBe("0.0.1");
    expect(planVersion("0.0.1", "minor", "", [], false)).toBe("0.0.1");
    expect(() => planVersion("0.0.1", "major", "", [], false)).toThrow(/version contract/);
    expect(planVersion("0.0.1", "major", "", [], true)).toBe("0.1.0");
  });

  test("once a tag exists, a release advances past it", () => {
    const known = releaseTags("v0.0.1");
    expect(planVersion("0.0.1", "patch", "", known, false)).toBe("0.0.2");
    expect(() => planVersion("0.0.1", "patch", "0.0.1", known, false)).toThrow(/does not advance/);
    expect(() => planVersion("0.0.1", "patch", "", releaseTags("v0.0.2"), false)).toThrow(/ahead of/);
  });
});

describe("nv release manifest", () => {
  test("a bump reaches every pin the check reads, whatever key comes before its version", () => {
    expect(pinProblems(MANIFEST)).toEqual([]);
    const { text, changed } = bumpManifest(MANIFEST, "0.1.0");
    expect(changed).toBe(3);
    expect(pinProblems(text)).toEqual([]);
    expect(text).toContain('nvs-b = { path = "crates/nvs-b", default-features = false, version = "0.1.0" }');
    expect(text).toContain('serde = { version = "1.0.200"');
    expect(text).toContain('version = "9.9.9"');
    expect(text).toContain('version = "not a pin"');
  });

  test("the check names a pin that disagrees with the workspace", () => {
    expect(pinProblems(MANIFEST.replace('nvs-a", version = "0.0.1"', 'nvs-a", version = "0.0.0"'))).toEqual([
      "nvs-a is pinned at 0.0.0, workspace is 0.0.1",
    ]);
  });

  test("a path under the workspace's `exclude` keeps its own version and is not a pin", () => {
    const excluded = MANIFEST.replace('members = ["crates/*"]', 'members = ["crates/*"]\nexclude = ["extensions"]').replace(
      "[profile.release]",
      'nvs-ext-x = { path = "extensions/x", version = "0.4.0" }\n\n[profile.release]',
    );
    expect(pinProblems(excluded)).toEqual([]);
    expect(bumpManifest(excluded, "0.1.0").text).toContain('nvs-ext-x = { path = "extensions/x", version = "0.4.0" }');
  });

  test("a pin written as a table of its own is an error, not a release with a stale pin", () => {
    const dotted = MANIFEST + '\n[workspace.dependencies.nvs-c]\npath = "crates/nvs-c"\nversion = "0.0.1"\n';
    expect(() => bumpManifest(dotted, "0.1.0")).toThrow(/nvs-c/);
  });

  test("the extension's package.json and both lockfile versions follow the workspace, and nothing else moves", () => {
    const pkg = '{\n  "name": "nvs",\n  "version": "0.0.1",\n  "dependencies": {\n    "x": "^1.0.0"\n  }\n}\n';
    expect(bumpPackageJson(pkg, "0.0.2", "package.json")).toBe(pkg.replace('"0.0.1"', '"0.0.2"'));
    const lock = [
      "{",
      '  "name": "nvs",',
      '  "version": "0.0.1",',
      '  "lockfileVersion": 3,',
      '  "packages": {',
      '    "": {',
      '      "name": "nvs",',
      '      "version": "0.0.1",',
      '      "license": "MIT"',
      "    },",
      '    "node_modules/x": {',
      '      "version": "1.0.0"',
      "    }",
      "  }",
      "}",
      "",
    ].join("\n");
    const bumped = JSON.parse(bumpPackageJson(lock, "0.0.2", "package-lock.json"));
    expect([bumped.version, bumped.packages[""].version, bumped.packages["node_modules/x"].version]).toEqual(["0.0.2", "0.0.2", "1.0.0"]);
    expect(bumpPackageJson(lock, "0.0.1", "package-lock.json")).toBe(lock);
  });

  test("an extension manifest with a version the edit cannot reach is an error", () => {
    expect(() => bumpPackageJson('{\n  "name": "nvs"\n}\n', "0.0.2", "package.json")).toThrow(/package.json/);
    const rootless = '{\n  "version": "0.0.1",\n  "lockfileVersion": 3,\n  "packages": {}\n}\n';
    expect(() => bumpPackageJson(rootless, "0.0.2", "package-lock.json")).toThrow(/package-lock.json/);
  });
});

describe("nv release notes", () => {
  const log = [
    ["a1", "feat(stdlib): a feature", ""],
    ["b2", "fix: a fix", "BREAKING CHANGE: it breaks"],
    ["c3", "docs(agent): a handoff", ""],
    ["d4", "docs(loop): a chain edit", ""],
    ["e5", "test(stdlib): a case", ""],
    ["f6", "added logos", ""],
  ]
    .map((c) => c.join("\x1f") + "\x1e\n")
    .join("");
  const entries = parseLog(log).map(classify);

  test("breaking is lifted to the top, headline types are listed, and the rest is counted", () => {
    const notes = renderNotes("0.1.0", "v0.0.1", entries, "https://example.com/r", "2026-01-02");
    expect(notes.startsWith("## [0.1.0] - 2026-01-02\n\n### Breaking changes\n")).toBe(true);
    expect(notes).toContain("### Features\n\n- **stdlib**: a feature ([`a1`](https://example.com/r/commit/a1))\n");
    expect(notes).not.toContain("### Fixes");
    expect(notes).toContain("### Other\n\n- added logos");
    expect(notes).toContain("3 further commits: docs (2), test (1).");
    expect(notes).toContain("[Full changes](https://example.com/r/compare/v0.0.1...v0.1.0) — 6 commits.");
  });

  test("every commit scoped fmt is listed under Formatter after Performance, and a breaking one is lifted", () => {
    const fmt = parseLog(
      [
        ["p1", "perf: faster", ""],
        ["q2", "fix(fmt): a blank line after a use block", ""],
        ["r3", "refactor(fmt): a match arm is one line", ""],
        ["s4", "feat(fmt)!: braces on their own line", ""],
        ["t5", "refactor(types): a helper", ""],
      ]
        .map((c) => c.join("\x1f") + "\x1e\n")
        .join(""),
    ).map(classify);
    const notes = renderNotes("0.1.0", null, fmt, "", "2026-01-02");
    expect(notes).toContain("### Breaking changes");
    expect(notes).toContain("- **fmt**: braces on their own line (`s4`)\n");
    expect(notes).toContain(
      "### Performance\n\n- faster (`p1`)\n\n### Formatter\n\n- **fmt**: a blank line after a use block (`q2`)\n- **fmt**: a match arm is one line (`r3`)\n",
    );
    expect(notes.split("braces on their own line")).toHaveLength(2);
    expect(notes).not.toContain("### Fixes");
    expect(notes).toContain("1 further commits: refactor (1).");
  });

  test("a section past the cap lists the first entries and counts the rest", () => {
    const feats = parseLog("b\x1ffeat: second\x1f\x1ea\x1ffeat: first\x1f\x1e").map(classify);
    const notes = renderNotes("0.1.0", null, feats, "", "2026-01-02", 1);
    expect(notes).toContain("### Features\n\n- first (`a`)\n- …and 1 more, in the full changes below.\n");
    expect(notes).not.toContain("Full changes](");
  });

  test("--fixes-built holds only the fix commits, a breaking one included", () => {
    expect(fixesIn(entries).map((e) => e.sha)).toEqual(["b2"]);
    const more = parseLog("g7\x1ffix(http)!: a framing fix\x1f\x1eh8\x1fperf: faster\x1f\x1e").map(classify);
    expect(fixesIn(more).map((e) => e.sha)).toEqual(["g7"]);
  });

  test("the first release's notes are the written file under the version heading", async () => {
    expect(firstReleaseNotes("0.0.1", "\nNovis is here.\n\n", "2026-01-02")).toBe("## [0.0.1] - 2026-01-02\n\nNovis is here.\n");
    const written = (await Bun.file(join(ROOT, FIRST_NOTES)).text()).trim();
    const first = await notesFor("0.0.1", null, 10);
    expect(first).toMatch(/^## \[0\.0\.1\] - \d{4}-\d{2}-\d{2}\n\n/);
    expect(first).toContain(written);
  });

  test("a release after a tag renders the commit log, not the written notes", async () => {
    // `HEAD..HEAD` is empty, so this needs no history and runs on a shallow clone.
    const later = await notesFor("0.0.2", "HEAD", 10);
    expect(later).toContain("No commits since the previous release.");
    expect(later).not.toContain((await Bun.file(join(ROOT, FIRST_NOTES)).text()).trim().split("\n")[0]!);
  });

  test("a section is prepended above the newest release, and a first release starts the file", () => {
    expect(prependChangelog("# Changelog\n\n## [0.1.0] - x\n", "## [0.2.0] - y\n")).toBe("# Changelog\n\n## [0.2.0] - y\n\n## [0.1.0] - x\n");
    expect(prependChangelog(null, "## [0.1.0] - x\n")).toBe(`${CHANGELOG_HEADER}\n## [0.1.0] - x\n`);
  });
});

describe("nv release docker tags", () => {
  const known = releaseTags("v0.4.0\nv0.3.0");

  test("floating tags follow MAJOR.MINOR, and MAJOR alone only from 1.0", () => {
    expect(dockerTags("0.4.0", "img", "debian", true, "abc1234", known)).toEqual({
      tags: ["img:0.4.0-debian", "img:sha-abc1234-debian", "img:0.4-debian", "img:latest-debian"],
      behind: null,
    });
    expect(dockerTags("1.2.0", "img", "distroless", true, "abc1234", []).tags).toEqual([
      "img:1.2.0",
      "img:sha-abc1234",
      "img:1.2",
      "img:1",
      "img:latest",
    ]);
  });

  test("a push's dev image claims `dev` alone, floating or not", () => {
    expect(dockerTags("dev", "img", "distroless", true, "abc1234", known)).toEqual({ tags: ["img:dev"], behind: null });
    expect(dockerTags("dev", "img", "debian", false, "abc1234", known).tags).toEqual(["img:dev-debian"]);
  });

  test("an older release gets no moving tags", () => {
    expect(dockerTags("0.3.0", "img", "distroless", true, "abc1234", known)).toEqual({
      tags: ["img:0.3.0", "img:sha-abc1234"],
      behind: "v0.4.0",
    });
  });
});

describe("nv release archives", () => {
  const files: ArchiveFile[] = [
    { name: "README.md", data: new TextEncoder().encode("read me\n"), mode: 0o644, mtime: 1_700_000_000 },
    { name: "nvs", data: new Uint8Array(1000).fill(7), mode: 0o755, mtime: 1_700_000_000 },
  ];

  test("a tar.gz holds the directory and each file under the stem, with a valid checksum and mode", () => {
    const tar = gunzipSync(tarGz("nvs-0.1.0-linux", files, 1_700_000_000));
    expect(tar.length % 10240).toBe(0);
    const text = (at: number, len: number) => new TextDecoder().decode(tar.subarray(at, at + len)).replace(/\0.*$/s, "");
    const headers: { name: string; mode: string }[] = [];
    for (let at = 0; text(at, 100) !== ""; ) {
      const size = parseInt(text(at + 124, 12), 8);
      const stored = parseInt(text(at + 148, 8), 8);
      let sum = 0;
      for (let i = 0; i < 512; i++) sum += i >= 148 && i < 156 ? 32 : tar[at + i]!;
      expect(sum).toBe(stored);
      headers.push({ name: text(at, 100), mode: text(at + 100, 8) });
      at += 512 + Math.ceil(size / 512) * 512;
    }
    expect(headers).toEqual([
      { name: "nvs-0.1.0-linux/", mode: "0000755" },
      { name: "nvs-0.1.0-linux/README.md", mode: "0000644" },
      { name: "nvs-0.1.0-linux/nvs", mode: "0000755" },
    ]);
  });

  test("a zip lists each file once, and its data inflates back with a matching CRC", () => {
    const bytes = Buffer.from(zip("nvs-0.1.0-windows", files));
    const end = bytes.length - 22;
    expect(bytes.readUInt32LE(end)).toBe(0x06054b50);
    expect(bytes.readUInt16LE(end + 10)).toBe(2);
    let at = bytes.readUInt32LE(end + 16);
    const names: string[] = [];
    for (let i = 0; i < 2; i++) {
      const nameLen = bytes.readUInt16LE(at + 28);
      names.push(bytes.subarray(at + 46, at + 46 + nameLen).toString("utf8"));
      const local = bytes.readUInt32LE(at + 42);
      const packed = bytes.readUInt32LE(at + 20);
      const dataAt = local + 30 + bytes.readUInt16LE(local + 26);
      const data = inflateRawSync(bytes.subarray(dataAt, dataAt + packed));
      expect(Buffer.compare(data, Buffer.from(files[i]!.data))).toBe(0);
      expect(Bun.hash.crc32(data) >>> 0).toBe(bytes.readUInt32LE(at + 16));
      expect(bytes.readUInt32LE(at + 38) >>> 16).toBe(0o100000 | files[i]!.mode);
      at += 46 + nameLen;
    }
    expect(names).toEqual(["nvs-0.1.0-windows/README.md", "nvs-0.1.0-windows/nvs"]);
  });
});

describe("greenFullRun", () => {
  const SHA = "a".repeat(40);
  let next = 1;
  const ciRun = (over: Partial<WorkflowRun>): WorkflowRun => {
    const id = next++;
    return {
      id,
      head_sha: SHA,
      head_branch: "main",
      event: "schedule",
      status: "completed",
      conclusion: "success",
      html_url: `https://example.com/runs/${id}`,
      ...over,
    };
  };

  test("a green run on another branch does not count, because it ran that branch's ci.yml", () => {
    for (const event of ["schedule", "workflow_dispatch"]) {
      expect(greenFullRun([ciRun({ event, head_branch: "topic" })], SHA)).toBeNull();
    }
    expect(greenFullRun([ciRun({ head_branch: null })], SHA)).toBeNull();
  });

  test("a green schedule run and a green dispatch run count", () => {
    for (const event of ["schedule", "workflow_dispatch"]) {
      const run = ciRun({ event });
      expect(greenFullRun([run], SHA)).toBe(run);
    }
  });

  test("a green push or pull_request run does not count, because it ran only its diff's lanes", () => {
    expect(greenFullRun([ciRun({ event: "push" }), ciRun({ event: "pull_request" })], SHA)).toBeNull();
  });

  test("a failed, cancelled or unfinished full run does not count", () => {
    const runs = [
      ciRun({ conclusion: "failure" }),
      ciRun({ conclusion: "cancelled" }),
      ciRun({ conclusion: "timed_out" }),
      ciRun({ status: "in_progress", conclusion: null }),
      ciRun({ status: "queued", conclusion: null }),
    ];
    expect(greenFullRun(runs, SHA)).toBeNull();
  });

  test("a green full run of another commit does not count, nor does a prefix of the sha", () => {
    expect(greenFullRun([ciRun({ head_sha: "b".repeat(40) })], SHA)).toBeNull();
    expect(greenFullRun([ciRun({})], SHA.slice(0, 9))).toBeNull();
  });

  test("among several runs it returns a green full one", () => {
    const green = ciRun({ event: "workflow_dispatch" });
    const runs = [ciRun({ event: "push" }), ciRun({ conclusion: "failure" }), ciRun({ head_sha: "c".repeat(40) }), green, ciRun({ status: "in_progress", conclusion: null })];
    expect(greenFullRun(runs, SHA)).toBe(green);
    expect(greenFullRun(runs, SHA.toUpperCase())).toBe(green);
  });

  test("no runs at all is untested", () => {
    expect(greenFullRun([], SHA)).toBeNull();
  });
});
