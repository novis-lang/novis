import { describe, expect, test } from "bun:test";
import { mkdtempSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { ROOT } from "../lib/paths.ts";
import {
  type Check,
  type Verdict,
  GreenMemo,
  acceptance,
  allReds,
  firstErrLine,
  judgeCommand,
  judgeProgram,
  judgeTests,
  measuresReleaseCli,
  orderedIn,
  owedChecks,
  plainCrateTest,
  programFailLine,
  proofGroups,
  proofOutcome,
  proofSections,
  stdoutLines,
  testExecutables,
  tiers,
} from "../driver/accept.ts";

const check = (over: Partial<Check>): Check => ({ id: "c", kind: "command", stage: 1, ...over });
const ok = (out = "", err = "") => ({ code: 0, out, err });

describe("firstErrLine", () => {
  test("a failed test outranks a panic, and a panic outranks an error line", () => {
    const out = "running 2 tests\ntest a::b ... FAILED\n";
    const err = "warning: unused\nthread 'a::b' panicked at src/x.rs:1\nerror: test failed";
    expect(firstErrLine({ code: 101, out, err })).toBe("test a::b ... FAILED");
    expect(firstErrLine({ code: 101, out: "", err })).toBe("thread 'a::b' panicked at src/x.rs:1");
    expect(firstErrLine({ code: 1, out: "", err: "warning: x\nerror[E0308]: mismatched" })).toBe("error[E0308]: mismatched");
  });
  test("stderr's first line, then stdout's last", () => {
    expect(firstErrLine({ code: 1, out: "a\nb\n", err: "why\nmore" })).toBe("why");
    expect(firstErrLine({ code: 1, out: "a\nlast\n", err: "" })).toBe("last");
    expect(firstErrLine({ code: 1, out: "", err: "" })).toBe("");
  });
});

test("stdoutLines drops CR and the trailing newline", () => {
  expect(stdoutLines("a\r\nb\r\n")).toEqual(["a", "b"]);
  expect(stdoutLines("")).toEqual([""]);
});

test("orderedIn needs each piece after the end of the one before", () => {
  expect(orderedIn("one two three", ["one", "three"])).toBe(true);
  expect(orderedIn("one two three", ["three", "one"])).toBe(false);
  expect(orderedIn("abab", ["ab", "ab"])).toBe(true);
  expect(orderedIn("ab", ["ab", "ab"])).toBe(false);
  expect(orderedIn("", [])).toBe(true);
});

test("plainCrateTest names a bare crate test or one with one --test", () => {
  expect(plainCrateTest(["test", "-p", "nvs-cli"])).toEqual(["nvs-cli", null]);
  expect(plainCrateTest(["test", "-p", "nvs-cli", "--test", "meta"])).toEqual(["nvs-cli", "meta"]);
  expect(plainCrateTest(["test", "-p", "nvs-cli", "--release"])).toBeNull();
  expect(plainCrateTest(["test", "--workspace"])).toBeNull();
});

test("measuresReleaseCli is a bench command", () => {
  expect(measuresReleaseCli(check({ argv: ["bun", "nv", "bench", "--guard"] }))).toBe(true);
  expect(measuresReleaseCli(check({ argv: ["bun", "nv", "proofs", "--bench"] }))).toBe(false);
});

describe("judgeProgram", () => {
  const L = "native examples/a.nvs [1 floor]";
  test("exact compares every line", () => {
    const c = check({ kind: "exact", file: "examples/a.nvs", want: ["1", "2"] });
    expect(judgeProgram(c, ok("1\r\n2\r\n"), L)).toBe("");
    expect(judgeProgram(c, ok("1\n3\n"), L)).toBe(`${L}: stdout was [1 / 3], wanted [1 / 2]`);
    expect(judgeProgram(check({ kind: "exact", want: [""] }), ok(""), L)).toBe("");
  });
  test("the exit comes first, and exit = nonzero inverts it", () => {
    const c = check({ kind: "exact", want: [] });
    expect(judgeProgram(c, { code: 2, out: "", err: "E0621: bad" }, L)).toBe(`${L}: exit 2 -- E0621: bad`);
    expect(judgeProgram(check({ kind: "contains", exit: "nonzero", stderrContains: ["E0621"] }), ok(), L)).toBe(`${L}: exited 0, wanted non-zero`);
    expect(judgeProgram(check({ kind: "contains", exit: "nonzero", stderrContains: ["E0621"] }), { code: 1, out: "", err: "E0621" }, L)).toBe("");
  });
  test("ordered reads the named stream", () => {
    const c = check({ kind: "ordered", stream: "stderr", want: ["a", "b"] });
    expect(judgeProgram(c, ok("a b", "b a"), L)).toBe(`${L}: stderr lacks, in order: a -> b`);
    expect(judgeProgram(c, ok("", "a b"), L)).toBe("");
  });
  test("contains and min-bytes", () => {
    expect(judgeProgram(check({ kind: "contains", stdoutContains: ["x"] }), ok("y"), L)).toBe(`${L}: stdout lacks "x"`);
    expect(judgeProgram(check({ kind: "min-bytes", minBytes: 4 }), ok("abc"), L)).toBe(`${L}: only 3 bytes of output, wanted 4`);
    expect(judgeProgram(check({ kind: "min-bytes", minBytes: 3 }), ok("abc"), L)).toBe("");
  });
});

test("judgeCommand reads want across both streams in order", () => {
  const c = check({ want: ["audit: a", "audit: b"] });
  expect(judgeCommand(c, ok("audit: a\n", "audit: b\n"), "L")).toBe("");
  expect(judgeCommand(c, ok("audit: b\n", "audit: a\n"), "L")).toBe("L: output lacks, in order: audit: a -> audit: b");
  expect(judgeCommand(c, { code: 1, out: "", err: "error: script \"nv\" exited with code 1" }, "L")).toBe('L: exit 1 -- error: script "nv" exited with code 1');
  expect(judgeCommand(check({ exit: "nonzero" }), ok(), "L")).toBe("L: exit 0, and this check asserts a non-zero exit");
});

describe("the batched proofs run", () => {
  test("proofGroups reads only a plain `--verify --group` check", () => {
    expect(proofGroups(check({ argv: ["bun", "nv", "proofs", "--verify", "--group", "Core\\Str", "--group=lang:types"] }))).toEqual(["Core\\Str", "lang:types"]);
    expect(proofGroups(check({ argv: ["bun", "nv", "proofs", "--verify", "--only", "directive:app"] }))).toBeNull();
    expect(proofGroups(check({ argv: ["bun", "nv", "proofs", "--verify", "--group", "Core\\Str", "--no-cache"] }))).toBeNull();
    expect(proofGroups(check({ argv: ["bun", "nv", "proofs", "--run", "--group", "Core\\Str"] }))).toBeNull();
    expect(proofGroups(check({ argv: ["bun", "nv", "proofs", "--verify", "--group", "Core\\Str"], cwd: "tools" }))).toBeNull();
    expect(proofGroups(check({ argv: ["bun", "nv", "proofs", "--verify"] }))).toBeNull();
  });

  const out = [
    "== Core\\Env",
    "nv proofs gate: nothing owed in Core\\Env (2 features).",
    "proofs examples: 9 ok, 0 skipped, 0 known-gap, 0 failed",
    "-- Core\\Env: passed",
    "== Core\\Heap",
    "nv proofs gate: nothing owed in Core\\Heap (1 feature).",
    "  FAIL  tests/hostile/core/Heap/01.nvs: exit 3",
    "proofs hostile: 0 ok, 0 skipped, 0 known-gap, 1 failed",
    "-- Core\\Heap: failed",
    "",
  ].join("\r\n");

  test("proofSections cuts each group's lines, headers kept", () => {
    const s = proofSections(out);
    expect([...s.keys()]).toEqual(["Core\\Env", "Core\\Heap"]);
    expect(s.get("Core\\Env")).toEqual({ passed: true, lines: out.split("\r\n").slice(0, 4) });
    expect(s.get("Core\\Heap")!.passed).toBe(false);
  });

  test("proofOutcome prints one group bare, and several with their sections", () => {
    const batch = { code: 1, out, err: "nv proofs: building the proof binary" };
    expect(proofOutcome(["Core\\Env"], batch)).toEqual({ code: 0, out: "nv proofs gate: nothing owed in Core\\Env (2 features).\nproofs examples: 9 ok, 0 skipped, 0 known-gap, 0 failed\n", err: "" });
    const heap = proofOutcome(["Core\\Heap"], batch);
    expect(heap.code).toBe(1);
    expect(judgeCommand(check({ want: ["nothing owed", "0 failed"] }), heap, "L")).toBe("L: exit 1 -- proofs hostile: 0 ok, 0 skipped, 0 known-gap, 1 failed");
    const both = proofOutcome(["Core\\Env", "Core\\Heap"], batch);
    expect(both.code).toBe(1);
    expect(both.out).toBe(`${out.replaceAll("\r", "").trimEnd()}\n`);
  });

  test("a group the batch printed no verdict for fails with the batch's own reason", () => {
    expect(proofOutcome(["Core\\Str"], { code: 1, out: "", err: "nv proofs: no group 'Core\\\\Str'." })).toEqual({ code: 1, out: "", err: "nv proofs: no group 'Core\\\\Str'." });
    expect(proofOutcome(["Core\\Str"], { code: 0, out, err: "" }).code).toBe(1);
    expect(proofOutcome(["Core\\Str"], { code: 0, out, err: "" }).err).toBe("the batched `nv proofs --verify` printed no verdict for Core\\Str");
  });
});

describe("judgeTests", () => {
  const disk = (rel: string) => rel !== "tests/conformance/missing.nvst";
  test("a suite needs its summary, no failure, every named case run", () => {
    const c = check({ kind: "nvs-suite", cases: ["tests/conformance/a.nvst"] });
    expect(judgeTests(c, ok("PASS tests\\conformance\\a.nvst\n3 passed, 0 failed"), "L", disk)).toEqual({ fail: "", short: "" });
    expect(judgeTests(c, ok("nothing"), "L", disk).fail).toBe("L: no 'N passed, M failed' summary line in the output");
    expect(judgeTests(c, ok("3 passed, 1 failed"), "L", disk).fail).toBe("L: 1 case(s) failed");
    expect(judgeTests(c, ok("SKIP tests\\conformance\\a.nvst\n3 passed, 0 failed"), "L", disk).fail).toBe("L: case tests/conformance/a.nvst was skipped, so nothing ran it");
    const absent = check({ kind: "nvs-suite", cases: ["tests/conformance/missing.nvst"] });
    expect(judgeTests(absent, ok("3 passed, 0 failed"), "L", disk).fail).toBe("L: case tests/conformance/missing.nvst is not written yet");
  });
  test("minPassing is the stopping condition, reported as short", () => {
    const c = check({ kind: "nvs-suite", minPassing: 5 });
    expect(judgeTests(c, ok("3 passed, 0 failed"), "L", disk)).toEqual({ fail: "", short: "L: only 3 passing case(s), wanted at least 5" });
  });
  test("cargo-named needs each test's name in the output", () => {
    const c = check({ kind: "cargo-named", tests: ["meta::cards", "meta::names"] });
    expect(judgeTests(c, ok("test meta::cards ... ok\ntest meta::names ... ok"), "L", disk).fail).toBe("");
    expect(judgeTests(c, ok("test meta::cards ... ok"), "L", disk).fail).toBe('L: test "meta::names" did not run');
  });
});

describe("the whole sweep", () => {
  const label = (n: number) => (n === 1 ? "1 floor" : `${n} stage`);
  const plan: Check[] = [
    check({ id: "rel", stage: 2, argv: ["bun", "nv", "bench", "--guard"] }),
    check({ id: "goal-fix", kind: "exact", stage: 3, file: "g.nvs", want: [] }),
    check({ id: "cmd3", stage: 3, argv: ["a"] }),
    check({ id: "cmd2", stage: 2, argv: ["b"] }),
    check({ id: "over", stage: 1, argv: ["c"], overlap: true }),
    check({ id: "floor-fix", kind: "exact", stage: 1, file: "f.nvs", want: [] }),
    check({ id: "setup", stage: 1, argv: ["d"], setup: true }),
    check({ id: "catch", stage: 0, kind: "cargo-named", args: ["test"] }),
  ];

  test("tiers run catch-up, setup, floor fixtures, the rest by stage, goal fixtures, overlap, release", () => {
    const got = tiers(plan, label).map((t) => [t.name, t.checks.map((c) => c.id)]);
    expect(got).toEqual([
      ["catch-up", ["catch"]],
      ["setup", ["setup"]],
      ["floor fixtures", ["floor-fix"]],
      ["cargo and command checks", ["cmd2", "cmd3"]],
      ["goal fixtures", ["goal-fix"]],
      ["overlap", ["over"]],
      ["release", ["rel"]],
    ]);
  });

  test("a fixture tier's reds are one line, and a collecting sweep's are also-red lines", () => {
    const fails = ["a", "b", "b"].map((f, i) => ({ c: check({ kind: "exact", stage: i + 1, file: `${f}.nvs` }), fail: `native ${f}.nvs: exit 1\nmore` }));
    expect(programFailLine(fails.slice(0, 1), label)).toBe("native a.nvs: exit 1\nmore");
    expect(programFailLine(fails, label)).toBe("native a.nvs: exit 1\nmore\n       (and 2 later fixture(s) red, in stage order: b.nvs [2 stage], b.nvs [3 stage])");
    expect(allReds(["one\ntail", " two\ntail"])).toBe("one\ntail\n       also red: two");
  });

  /** A sweep that fails the ids in `red` and records the order it ran them in. */
  const fake = (red: string[], short: string[] = []) => {
    const ran: string[] = [];
    return {
      ran,
      check: async (c: Check): Promise<Verdict> => {
        ran.push(c.id);
        return { fail: red.includes(c.id) ? `${c.id} red` : "", short: short.includes(c.id) ? `${c.id} short` : "" };
      },
    };
  };
  const opts = (sweep: ReturnType<typeof fake>, memo = new GreenMemo(), more: Partial<Parameters<typeof acceptance>[1]> = {}) => ({ label, sweep, key: (c: Check) => `k-${c.id}`, memo, full: false, collect: false, ...more });

  test("it stops at the first red, after starting the overlap command behind the setup tier", async () => {
    const s = fake(["cmd2"]);
    const r = await acceptance(plan, opts(s));
    expect(r.fail).toBe("cmd2 red");
    expect(s.ran).toEqual(["catch", "setup", "over", "floor-fix", "cmd2"]);
  });

  test("the sweep names each tier's checks the memo does not answer before the first of them runs", async () => {
    const told: string[][] = [];
    const s = { ...fake([]), batch: (checks: Check[]) => told.push(checks.map((c) => c.id)) };
    await acceptance(plan, opts(s, new GreenMemo({ cmd2: "k-cmd2" })));
    expect(told).toEqual([["catch"], ["setup"], ["floor-fix"], ["cmd3"], ["goal-fix"], ["over"], ["rel"]]);
  });

  test("a collecting sweep runs past each red and names every one", async () => {
    const s = fake(["cmd2", "goal-fix", "rel"]);
    const r = await acceptance(plan, opts(s, new GreenMemo(), { collect: true }));
    expect(r.fail).toBe("cmd2 red\n       also red: goal-fix red\n       also red: rel red");
    expect(r.ran).toBe(plan.length);
  });

  test("the memo answers a check green over the same key, and --full asks it nothing", async () => {
    const memo = new GreenMemo();
    expect((await acceptance(plan, opts(fake([]), memo))).fail).toBe("");
    const again = fake([]);
    expect(await acceptance(plan, opts(again, memo))).toEqual({ fail: "", ran: 0, answered: plan.length });
    const moved = fake([]);
    await acceptance(plan, opts(moved, memo, { key: (c: Check) => (c.id === "cmd3" ? "new" : `k-${c.id}`) }));
    expect(moved.ran).toEqual(["cmd3"]);
    const full = fake([]);
    await acceptance(plan, opts(full, memo, { full: true }));
    expect(full.ran.length).toBe(plan.length);
  });

  test("a short suite, a memoize = false check and a keyless one are never remembered", async () => {
    const memo = new GreenMemo();
    const odd = [check({ id: "short", kind: "nvs-suite" }), check({ id: "live", memoize: false }), check({ id: "keyless" })];
    const r = await acceptance(odd, opts(fake([], ["short"]), memo, { key: (c: Check) => (c.id === "keyless" ? null : "k") }));
    expect(r.fail).toBe("short short");
    const again = fake([]);
    await acceptance(odd, opts(again, memo, { key: (c: Check) => (c.id === "keyless" ? null : "k") }));
    expect(again.ran).toEqual(["short", "live", "keyless"]);
  });

  test("owed is the carried checks the memo does not answer, less stage 0, the goal's own and memoize = false", () => {
    const live = check({ id: "live", stage: 1, memoize: false });
    const memo = new GreenMemo({ over: "k-over", setup: "old", cmd3: "k-cmd3" });
    const owed = owedChecks([...plan, live], label, memo, (c: Check) => `k-${c.id}`);
    expect(owed.map((c) => c.id)).toEqual(["floor-fix", "setup"]);
    expect(owedChecks(plan, label, memo, () => null).map((c) => c.id)).toEqual(["over", "floor-fix", "setup"]);
  });

  test("the memo keeps only the checks still in the plan", () => {
    const dir = mkdtempSync(join(ROOT, ".agent-tmp", "memo-"));
    const memo = new GreenMemo({ gone: "k" });
    memo.remember(check({ id: "kept" }), "k2");
    memo.save(join(dir, "green.json"), new Set(["kept"]));
    expect(JSON.parse(readFileSync(join(dir, "green.json"), "utf8"))).toEqual({ green: { kept: "k2" } });
    expect(GreenMemo.load(join(dir, "green.json")).answers(check({ id: "kept" }), "k2")).toBe(true);
    expect(GreenMemo.load(join(dir, "absent.json")).answers(check({ id: "kept" }), "k2")).toBe(false);
  });
});

test("testExecutables groups test artefacts by package", () => {
  const lines = [
    { reason: "compiler-artifact", package_id: "path+file:///d:/mwl/crates/nvs-types#0.0.1", manifest_path: "d:/mwl/crates/nvs-types/Cargo.toml", target: { name: "nvs_types", kind: ["lib"] }, profile: { test: true }, executable: "t1.exe" },
    { reason: "compiler-artifact", package_id: "path+file:///d:/mwl/benches/abi-probe#nvs-abi-probe@0.0.1", manifest_path: "d:/mwl/benches/abi-probe/Cargo.toml", target: { name: "perf_guards", kind: ["test"] }, profile: { test: true }, executable: "t2.exe" },
    { reason: "compiler-artifact", package_id: "path+file:///d:/mwl/crates/nvs-types#0.0.1", target: { name: "nvs_types", kind: ["lib"] }, profile: { test: false }, executable: null },
    { reason: "build-finished", success: true },
  ].map((l) => JSON.stringify(l));
  const exes = testExecutables([...lines, "not json"].join("\n"));
  expect([...exes.keys()].sort()).toEqual(["nvs-abi-probe", "nvs-types"]);
  expect(exes.get("nvs-abi-probe")).toEqual([{ target: "perf_guards", exe: "t2.exe", dir: "d:/mwl/benches/abi-probe" }]);
});
