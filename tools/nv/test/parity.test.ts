import { afterEach, describe, expect, test } from "bun:test";
import { existsSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { compare, layOut, normalize, selectLines, snapshot, sortEntries, type Known } from "../cmd/parity.ts";
import type { RunResult } from "../lib/proc.ts";
import { scratch, type Scratch } from "./scratch.ts";

const TOOL: Known = { side: "python", pattern: "python tools/([a-z_-]+)\\.py", replace: "bun nv $1", why: "renamed" };
const NAME: Known = { side: "python", pattern: "^brief\\.py:", replace: "nv brief:", why: "own name" };

function result(stdout: string, code = 0, stderr = ""): RunResult {
  return { argv: [], code, stdout, stderr, timedOut: false };
}

describe("normalize", () => {
  test("applies a rewrite only to its own side, and records that it matched", () => {
    const used = new Set<Known>();
    expect(normalize("run python tools/peek.py x", "python", [TOOL], used)).toBe("run bun nv peek x");
    expect(used.has(TOOL)).toBe(true);
    const none = new Set<Known>();
    expect(normalize("run python tools/peek.py x", "nv", [TOOL], none)).toBe("run python tools/peek.py x");
    expect(none.size).toBe(0);
  });

  test("anchors match per line, and CRLF reads as LF", () => {
    expect(normalize("a\r\nbrief.py: no\r\n", "python", [NAME], new Set())).toBe("a\nnv brief: no\n");
  });
});

describe("compare", () => {
  test("two runs that differ only where a declaration says match", () => {
    expect(compare(result("see python tools/owners.py\n"), result("see bun nv owners\n"), [TOOL], new Set())).toEqual([]);
  });

  test("names the first differing line and a differing exit status", () => {
    const diff = compare(result("a\nb\nc\n", 0), result("a\nB\nc\n", 1), [], new Set());
    expect(diff[0]).toBe("     exit status: python 0, nv 1");
    expect(diff[1]).toContain("stdout line 2");
    expect(diff[2]).toContain('"b"');
    expect(diff[3]).toContain('"B"');
  });

  test("a missing line reads as no line", () => {
    const diff = compare(result("a\nb"), result("a"), [], new Set());
    expect(diff.at(-1)).toContain("(no line)");
  });

  test("declared unordered entries compare sorted, each with its deeper lines", () => {
    const u = { pattern: "^  gap ", why: "no position" };
    const py = "head\n  gap b\n      why b\n  gap a\ntail\n  gap z\n";
    const nv = "head\n  gap a\n  gap b\n      why b\ntail\n  gap z\n";
    expect(sortEntries(py, u)).toBe(sortEntries(nv, u));
    expect(compare(result(py), result(nv), [], new Set(), u)).toEqual([]);
    expect(compare(result(py), result(nv), [], new Set())).not.toEqual([]);
  });

  test("an unordered run ends at a line that is neither an entry nor deeper", () => {
    const u = { pattern: "^  gap ", why: "no position" };
    expect(sortEntries("  gap b\nmid\n  gap a\n", u)).toBe("  gap b\nmid\n  gap a\n");
  });

  test("a declared selection compares only the lines it names, after the rewrites", () => {
    const s = { pattern: "^== ", why: "prose differs" };
    const py = "== one\nrun python tools/brief.py\n== two\n";
    const nv = "== one\nrun something else\n== two\n";
    expect(selectLines(py, s)).toBe("== one\n== two");
    expect(compare(result(py), result(nv), [], new Set(), undefined, s)).toEqual([]);
    expect(compare(result(py), result("== one\n== three\n"), [], new Set(), undefined, s)[0]).toContain("stdout line 2");
  });

  test("the files two writers left are compared, and a declaration covers a line ending", () => {
    const CR: Known = { side: "nv", pattern: "\\\\r$", replace: "", why: "keeps CRLF" };
    const py = { ...result(""), files: "== a.txt\none\ntwo" };
    const nv = { ...result(""), files: "== a.txt\none\\r\ntwo" };
    expect(compare(py, nv, [], new Set())[0]).toContain("files line 2");
    expect(compare(py, nv, [CR], new Set())).toEqual([]);
  });
});

describe("a writer's tree", () => {
  let tmp: Scratch | undefined;
  afterEach(() => tmp?.cleanup());

  test("is laid out afresh, and a snapshot names only what a run changed", () => {
    tmp = scratch();
    const tree = { "a.txt": "a\n", "sub/b.txt": "b\r\n", "gone.txt": "g\n" };
    tmp.put("stale.txt", "left over");
    layOut(tmp.root, tree);
    expect(existsSync(join(tmp.root, "stale.txt"))).toBe(false);
    expect(snapshot(tmp.root, tree)).toBe("");
    writeFileSync(join(tmp.root, "sub/b.txt"), "B\r\n");
    writeFileSync(join(tmp.root, "new.txt"), "n\n");
    rmSync(join(tmp.root, "gone.txt"));
    expect(snapshot(tmp.root, tree)).toBe("== gone.txt (deleted)\n== new.txt\nn\n\n== sub/b.txt\nB\\r\n");
  });
});
