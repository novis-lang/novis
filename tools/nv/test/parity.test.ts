import { describe, expect, test } from "bun:test";
import { compare, normalize, type Known } from "../cmd/parity.ts";
import type { RunResult } from "../lib/proc.ts";

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
});
