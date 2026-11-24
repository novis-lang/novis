// `bun nv api-rename` on a fixture goal file and fixture sources. The names are made up, so this file
// never writes an old name a real goal renames.

import { expect, test } from "bun:test";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { instanceCalls, keepClasses, readRenames, rewrite, run } from "../cmd/api-rename.ts";
import { ROOT } from "../lib/paths.ts";

const GOAL = [
  "# A fixture goal",
  "",
  "```renames",
  "# a comment line",
  "Fixture\\Reply::json Fixture\\Reply::sendJson",
  "Fixture\\Shape\\Picture->resize Fixture\\Shape\\Picture->resized",
  "```",
  "",
].join("\n");

const RENAMES = readRenames(GOAL);

test("the renames block of a goal file is read", () => {
  expect(RENAMES).toEqual([
    { oldClass: "Fixture\\Reply", oldMember: "json", newClass: "Fixture\\Reply", newMember: "sendJson", instance: false },
    { oldClass: "Fixture\\Shape\\Picture", oldMember: "resize", newClass: "Fixture\\Shape\\Picture", newMember: "resized", instance: true },
  ]);
  expect(() => readRenames("# no block\n")).toThrow("no ```renames block");
  expect(() => readRenames("```renames\nFixture\\Reply::json Fixture\\Reply->sendJson\n```\n")).toThrow("one kind");
});

test("the class filter keeps only the pairs of the named classes", () => {
  expect(keepClasses(RENAMES, ["\\Fixture\\Reply"]).map((r) => r.oldMember)).toEqual(["json"]);
  expect(keepClasses(RENAMES, ["Fixture\\Reply", "Fixture\\Shape\\Picture"])).toEqual(RENAMES);
  expect(() => keepClasses(RENAMES, ["Fixture\\Shape"])).toThrow("no pair of the block renames a member of Fixture\\Shape");
});

test("a qualified static call is rewritten", () => {
  const nvs = rewrite("<?nvs\n\\Fixture\\Reply::json($data);\nFixture\\Reply::jsonText($data);\n", ".nvs", RENAMES);
  expect(nvs).toEqual({ text: "<?nvs\n\\Fixture\\Reply::sendJson($data);\nFixture\\Reply::jsonText($data);\n", count: 1 });
  const rust = rewrite('let src = "Fixture\\\\Reply::json($x)";\n', ".rs", RENAMES);
  expect(rust).toEqual({ text: 'let src = "Fixture\\\\Reply::sendJson($x)";\n', count: 1 });
  const page = rewrite("Call `Reply::json` or `Fixture\\Shape\\Picture->resize`.\n", ".md", RENAMES);
  expect(page.text).toBe("Call `Reply::sendJson` or `Fixture\\Shape\\Picture->resized`.\n");
});

test("a short name brought in by use is rewritten", () => {
  const src = "<?nvs\nuse Fixture\\Reply;\nuse Fixture\\Reply as R;\nReply::json($a);\nR::json($b);\nOther::json($c);\n";
  expect(rewrite(src, ".nvs", RENAMES)).toEqual({
    text: "<?nvs\nuse Fixture\\Reply;\nuse Fixture\\Reply as R;\nReply::sendJson($a);\nR::sendJson($b);\nOther::json($c);\n",
    count: 2,
  });
  // Without the `use`, a short name in a program may be another class, so it stays.
  expect(rewrite("<?nvs\nReply::json($a);\n", ".nvs", RENAMES).count).toBe(0);
});

test("an instance call is listed and not rewritten", () => {
  const src = "<?nvs\n$small = $picture->resize(10);\n$other = $this->thumb?->resize(5);\n$myPicture->resizeAll();\n";
  expect(rewrite(src, ".nvs", RENAMES)).toEqual({ text: src, count: 0 });
  expect(instanceCalls(src, RENAMES).map((h) => [h.line, h.text])).toEqual([
    [2, "->resize("],
    [3, "?->resize("],
  ]);
});

test("check fails naming each old name left", async () => {
  const scratch = join(ROOT, ".agent-tmp");
  mkdirSync(scratch, { recursive: true });
  const root = mkdtempSync(join(scratch, "api-rename-"));
  try {
    mkdirSync(join(root, "docs", "goals"), { recursive: true });
    mkdirSync(join(root, "src"));
    writeFileSync(join(root, "docs", "goals", "fixture.md"), GOAL);
    writeFileSync(join(root, "src", "a.nvs"), "<?nvs\nFixture\\Reply::json($a);\n$p->resize(1);\n");
    writeFileSync(join(root, "src", "b.md"), "Use `Picture->resize`.\n");
    const lines: string[] = [];
    const log = console.log;
    console.log = (line: string) => lines.push(line);
    let code: number;
    try {
      code = await run(["--check", "--goal", "fixture", "--root", root]);
    } finally {
      console.log = log;
    }
    expect(code).toBe(1);
    expect(lines).toEqual([
      "src/a.nvs:2: `Fixture\\Reply::json` is an old name (Fixture\\Reply::json → sendJson)",
      "src/b.md:1: `Picture->resize` is an old name (Fixture\\Shape\\Picture->resize → resized)",
      "api-rename: 2 old name(s) of fixture are still written",
    ]);
    // The sweep rewrites both, and then the check passes.
    console.log = () => {};
    try {
      expect(await run(["--goal", "fixture", "--root", root])).toBe(0);
      expect(await run(["--check", "--goal", "fixture", "--root", root])).toBe(0);
    } finally {
      console.log = log;
    }
    expect(readFileSync(join(root, "src", "a.nvs"), "utf8")).toBe("<?nvs\nFixture\\Reply::sendJson($a);\n$p->resize(1);\n");
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
