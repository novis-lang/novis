// `bun nv suite` over a judge that reads fixed facts through `suiteVerdict`, the sweep's own suite
// conditions, so no case runs: the arguments build the check a goal record holds, and the exit code and
// the printed line follow the verdict.

import { describe, expect, test } from "bun:test";
import type { Check } from "../accept/accept.ts";
import { type SuiteFacts, suiteVerdict } from "../accept/runner.ts";
import { suite, suiteCheck } from "../cmd/suite.ts";

const TREE = "tests/demo";
const CASES = [`${TREE}/one.nvst`, `${TREE}/two.nvst`];

/** Runs `bun nv suite` with `facts` as the store, and returns its exit code, its lines and the check it judged. */
async function suiteOver(args: string[], facts: Partial<SuiteFacts>): Promise<{ code: number; lines: string[]; check: Check | null }> {
  const lines: string[] = [];
  let check: Check | null = null;
  const f: SuiteFacts = { verdict: () => "green", skipped: () => false, onDisk: (p) => CASES.includes(p), out: "", ...facts };
  const code = await suite(args, {
    judge: async (c) => {
      check = c;
      return suiteVerdict(c, CASES, "suite", f);
    },
    say: (l) => lines.push(l),
  });
  return { code, lines, check };
}

describe("bun nv suite", () => {
  test("nv suite passes when every named case ran and none failed", async () => {
    const got = await suiteOver([TREE, "--case", CASES[0]!, "--min-passing", "2"], {});
    expect(got.check).toMatchObject({ kind: "nvs-suite", args: ["test", TREE], cases: [CASES[0]], minPassing: 2 });
    expect(got.code).toBe(0);
    expect(got.lines).toEqual([`(pass) ${CASES[0]}`, `GREEN nvs test ${TREE} [nv suite]`]);
  });

  test("nv suite fails on a named case that is not on disk", async () => {
    const got = await suiteOver([TREE, "--case", `${TREE}/missing.nvst`], {});
    expect(got.code).toBe(1);
    expect(got.lines).toEqual([`RED suite: case ${TREE}/missing.nvst is not written yet`]);
  });

  test("nv suite fails on a named case that was skipped", async () => {
    const got = await suiteOver([TREE, "--case", CASES[1]!], { skipped: (p) => p === CASES[1] });
    expect(got.code).toBe(1);
    expect(got.lines).toEqual([`RED suite: case ${CASES[1]} was skipped, so nothing ran it`]);
  });

  test("nv suite fails on a named case its tree does not run, such as an .lspt case under nvs test", async () => {
    const got = await suiteOver([TREE, "--case", `${TREE}/three.lspt`], { onDisk: () => true });
    expect(got.code).toBe(1);
    expect(got.lines).toEqual([`RED suite: case ${TREE}/three.lspt is not a case this suite runs, so nothing ran it`]);
  });

  test("nv suite lsp-test builds the check nvs lsp-test runs", () => {
    expect(suiteCheck(["lsp-test", TREE, "--case", `${TREE}/three.lspt`])).toMatchObject({
      check: { kind: "nvs-suite", name: `nvs lsp-test ${TREE}`, args: ["lsp-test", TREE], cases: [`${TREE}/three.lspt`] },
    });
  });

  test("nv suite fails when fewer cases pass than --min-passing", async () => {
    const got = await suiteOver([TREE, "--min-passing", "3"], {});
    expect(got.code).toBe(1);
    expect(got.lines).toEqual(["SHORT suite: only 2 passing case(s), wanted at least 3"]);
  });

  test("nv suite takes only trees directly under tests/ and whole numbers", () => {
    expect(suiteCheck([])).toEqual({ error: "name at least one tree" });
    expect(suiteCheck(["tests/demo/one.nvst"])).toEqual({ error: "tests/demo/one.nvst is not a tree under tests/" });
    expect(suiteCheck([TREE, "--min-passing", "x"])).toEqual({ error: "--min-passing takes a whole number, not x" });
  });
});
