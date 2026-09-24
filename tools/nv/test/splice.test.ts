import { afterEach, describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { ROOT } from "../lib/paths.ts";
import { run } from "../lib/proc.ts";
import { type Scratch, scratch } from "./scratch.ts";

const MAIN = join(ROOT, "tools", "nv", "main.ts");
let tree: Scratch;
afterEach(() => tree?.cleanup());

const nv = (...args: string[]) => run([process.execPath, MAIN, "splice", ...args], { cwd: tree.root, timeoutMs: 60_000 });
const text = (path: string) => readFileSync(join(tree.root, path), "utf8");

/** A two-file tree, and a patch that edits both files. */
function twoFiles(secondAnchor: string): void {
  tree = scratch();
  tree.put("a.txt", "one\ntwo\nthree\n");
  tree.put("b.txt", "alpha\nbeta\n");
  tree.put("p.patch", `--- a.txt\n<<<<<<< OLD\ntwo\n=======\nTWO\n>>>>>>> NEW\n--- b.txt\n<<<<<<< OLD\n${secondAnchor}\n=======\nBETA\n>>>>>>> NEW\n`);
}

describe("nv splice", () => {
  test("a two-file patch applies to both files", async () => {
    twoFiles("beta");
    const r = await nv("--patch", "p.patch");
    expect(r.code).toBe(0);
    expect(r.stdout).toContain("2 block(s) spliced into 2 file(s)");
    expect(text("a.txt")).toBe("one\nTWO\nthree\n");
    expect(text("b.txt")).toBe("alpha\nBETA\n");
  });

  test("a stale anchor in the second file leaves the first untouched", async () => {
    twoFiles("betta");
    const r = await nv("--patch", "p.patch");
    expect(r.code).toBe(1);
    expect(r.stdout).toContain("block 2 of 2 does not appear in b.txt");
    expect(r.stdout).toContain("the anchor matches for its first 3 character(s), up to target line 2");
    expect(r.stdout).toContain("the file has: 'a'");
    expect(text("a.txt")).toBe("one\ntwo\nthree\n");
    expect(text("b.txt")).toBe("alpha\nbeta\n");
  });

  test("a dry run matches every anchor and writes nothing", async () => {
    twoFiles("beta");
    const r = await nv("--patch", "p.patch", "--dry-run");
    expect(r.code).toBe(0);
    expect(r.stdout).toContain("all 2 block(s) match exactly once across 2 file(s)");
    expect(text("a.txt")).toBe("one\ntwo\nthree\n");
  });

  test("an anchor that appears twice names both lines", async () => {
    tree = scratch();
    tree.put("a.txt", "x\ny\nx\n");
    tree.put("p.patch", "<<<<<<< OLD\nx\n=======\nz\n>>>>>>> NEW\n");
    const r = await nv("a.txt", "--patch", "p.patch");
    expect(r.code).toBe(1);
    expect(r.stdout).toContain("the anchor appears 2 times in a.txt (lines 1, 3) -- make it unique");
  });

  test("the two-file form replaces one block, and a CRLF target keeps its line endings", async () => {
    tree = scratch();
    tree.put("a.txt", "one\r\ntwo\r\n");
    tree.put("old", "one\ntwo");
    tree.put("new", "one\n2");
    const r = await nv("a.txt", "old", "new");
    expect(r.code).toBe(0);
    expect(text("a.txt")).toBe("one\r\n2\r\n");
  });

  test("a target on the command line and a header in the patch is a bad argument", async () => {
    twoFiles("beta");
    const r = await nv("a.txt", "--patch", "p.patch");
    expect(r.code).toBe(2);
    expect(r.stdout).toContain("Use one or the other.");
  });
});
