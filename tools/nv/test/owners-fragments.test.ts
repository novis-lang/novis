// A not-built paragraph and the gap records it cites: which paragraphs `bun nv owners` reads in a rule
// fragment and in a Rust doc comment, and which of them it reports as the `rule` prose kind.
import { afterEach, describe, expect, test } from "bun:test";
import { notBuilt } from "../cmd/owners.ts";
import { write } from "../lib/store.ts";
import { gap } from "../schema/gap.ts";
import { scratch, type Scratch } from "./scratch.ts";

let tmp: Scratch;
afterEach(() => tmp?.cleanup());

/** A scratch tree with gap record `nvs-x/kept` and `files`. */
function seed(files: Record<string, string>): void {
  tmp = scratch();
  write(gap, "nvs-x/kept", { module: "crates/nvs-x/src/lib.rs", title: "Kept", text: "Kept.", goal: "kept" }, tmp.root);
  for (const [path, text] of Object.entries(files)) tmp.put(path, text);
}

function findings(): string[] {
  return notBuilt(tmp.root).map((p) => `${p.file}:${p.line}`);
}

describe("a rule fragment's not-built paragraph", () => {
  test("a Not on disk paragraph that cites no gap record is a finding", () => {
    seed({ "docs/rules/x/a.md": "The rule.\n\n**Not on disk.** There is no `Core\\Shop`.\n" });
    expect(findings()).toEqual(["docs/rules/x/a.md:3"]);
  });

  test("a Not shipped paragraph is read the same way", () => {
    seed({ "docs/rules/x/a.md": "The rule.\n\n**Not shipped.** Writing ods.\n" });
    expect(findings()).toEqual(["docs/rules/x/a.md:3"]);
  });

  test("a paragraph that cites an existing gap record is not a finding", () => {
    seed({ "docs/rules/x/a.md": "The rule.\n\n**Not on disk.** There is no `Core\\Shop`\n(gap:nvs-x/kept).\n" });
    expect(findings()).toEqual([]);
  });

  test("a paragraph that cites a gap record that does not exist is a finding", () => {
    seed({ "docs/rules/x/a.md": "The rule.\n\n**Not on disk.** There is no `Core\\Shop` (gap:nvs-x/gone).\n" });
    expect(notBuilt(tmp.root).map((p) => p.lead)).toEqual([
      "cites no record gap:nvs-x/gone: **Not on disk.** There is no `Core\\Shop` (gap:nvs-x/gone).",
    ]);
  });

  test("unmarked running prose such as not written yet is not a finding", () => {
    seed({ "docs/rules/x/a.md": "The rule. The page is not written yet, and nothing is on disk.\n" });
    expect(findings()).toEqual([]);
  });

  test("a chapter file under docs/rules is not read", () => {
    seed({ "docs/rules/x.md": "# X\n\n**Not on disk.** There is no `Core\\Shop`.\n" });
    expect(findings()).toEqual([]);
  });
});

describe("a doc comment's not-built list", () => {
  const list = [
    "//! The module.",
    "//!",
    "//! **Not done yet.** These are the rest:",
    "//!",
    "//! - A table without a `width` fills its block.",
    "//! - A cell's `vertical-align` is always `top`.",
  ];

  test("a Not done yet list in a module doc that cites no gap record is a finding", () => {
    seed({ "extensions/pdf/src/lib.rs": [...list, "", "fn main() {}", ""].join("\n") });
    expect(findings()).toEqual(["extensions/pdf/src/lib.rs:3"]);
  });

  test("a Cost paragraph after a Not done yet list is not part of the list", () => {
    const cost = ["//!", "//! Cost: each cell is laid out twice (gap:nvs-x/kept).", "", "fn main() {}", ""];
    seed({ "crates/nvs-x/src/lib.rs": [...list, ...cost].join("\n") });
    expect(findings()).toEqual(["crates/nvs-x/src/lib.rs:3"]);
  });
});
