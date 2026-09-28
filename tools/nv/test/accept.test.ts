import { describe, expect, test } from "bun:test";
import {
  type Check,
  type Verdict,
  acceptance,
  allReds,
  firstErrLine,
  heldByGate,
  judgeCommand,
  judgeProgram,
  judgeTests,
  measuresReleaseCli,
  orderedIn,
  owedChecks,
  plainCrateTest,
  programFailLine,
  proofBatches,
  proofGroups,
  proofKeys,
  proofOutcome,
  proofSections,
  releaseBuildArgv,
  stdoutLines,
  subsetRun,
  testExecutables,
  tiers,
  withSetups,
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
  test("stderr's first line, then stdout's last few", () => {
    expect(firstErrLine({ code: 1, out: "a\nb\n", err: "why\nmore" })).toBe("why");
    expect(firstErrLine({ code: 1, out: "a\nlast\n", err: "" })).toBe("a | last");
    expect(firstErrLine({ code: 1, out: "one\ntwo\n\nthree\nfour\n", err: "" })).toBe("two | three | four");
    expect(firstErrLine({ code: 1, out: "", err: "" })).toBe("");
  });
  test("a red `bun nv` check quotes what the script printed, never `bun run`'s exit line", () => {
    const script = 'error: script "nv" exited with code 1';
    const proofs = "== Core\\Attributes\n  FAIL  tests/hostile/a.nvs: still running after 10s\nproofs hostile: 1 ok, 1 failed\n";
    expect(firstErrLine({ code: 1, out: proofs, err: script })).toBe("FAIL  tests/hostile/a.nvs: still running after 10s");
    expect(firstErrLine({ code: 1, out: "nv plan: M9 has no status\nsee --help\n", err: script })).toBe("nv plan: M9 has no status | see --help");
    expect(firstErrLine({ code: 1, out: "", err: `nv verify: the fmt step failed\n${script}` })).toBe("nv verify: the fmt step failed");
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

  test("proofBatches cuts the groups into batches of a bounded size, a check's groups kept together", () => {
    const group = (...gs: string[]) => check({ argv: ["bun", "nv", "proofs", "--verify", ...gs.flatMap((g) => ["--group", g])] });
    const many = Array.from({ length: 30 }, (_, i) => group(`G${i}`));
    const batches = proofBatches(many, 12);
    expect(batches.map((b) => b.length)).toEqual([12, 12, 6]);
    expect(batches.flat()).toEqual(many.map((_, i) => `G${i}`));
    // A check's two groups do not straddle a batch, and a group already placed is not placed again.
    expect(proofBatches([group("A"), group("B"), group("C", "D"), group("A"), group("E")], 3)).toEqual([["A", "B"], ["C", "D", "E"]]);
    // A lone group, and a check that is no proofs-group check, make no batch.
    expect(proofBatches([group("A"), check({ argv: ["bun", "nv", "proofs", "--verify", "--only", "x"] })], 12)).toEqual([]);
    expect(proofBatches([group("A"), group("B"), group("C")], 2)).toEqual([["A", "B"]]);
  });

  test("proofKeys gives a check the batch's shared reads and its own groups', and no other group's", () => {
    const keyed = (...ks: string[]) => new Map(ks.map((k) => [k, ""]));
    const batch = {
      keys: keyed("mod:tools/nv/cmd/proofs.ts", "class:core\\json", "class:*", "card:*"),
      parts: new Map([
        ["Core\\Json", keyed("mod:tools/nv/cmd/proofs.ts", "class:core\\json")],
        ["types:exception", keyed("mod:tools/nv/cmd/proofs.ts", "class:*", "card:*")],
        ["lang:errors", keyed("mod:tools/nv/cmd/proofs.ts")],
      ]),
    };
    expect([...proofKeys(["lang:errors"], batch).keys()]).toEqual(["mod:tools/nv/cmd/proofs.ts"]);
    expect([...proofKeys(["Core\\Json", "types:exception"], batch).keys()].sort()).toEqual(["card:*", "class:*", "class:core\\json", "mod:tools/nv/cmd/proofs.ts"]);
    // A group the run kept no part for holds everything the batch read.
    expect(proofKeys(["lang:errors", "lang:types"], batch)).toEqual(batch.keys);
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
    expect(judgeCommand(check({ want: ["nothing owed", "0 failed"] }), heap, "L")).toBe("L: exit 1 -- FAIL  tests/hostile/core/Heap/01.nvs: exit 3");
    const both = proofOutcome(["Core\\Env", "Core\\Heap"], batch);
    expect(both.code).toBe(1);
    expect(both.out).toBe(`${out.replaceAll("\r", "").trimEnd()}\n`);
  });

  test("a group the batch printed no verdict for fails with the batch's own reason", () => {
    expect(proofOutcome(["Core\\Str"], { code: 1, out: "", err: "nv proofs: no group 'Core\\\\Str'." })).toEqual({
      code: 1,
      out: "",
      err: "nv proofs: no group 'Core\\\\Str'.\nthe batched `nv proofs --verify` printed no verdict for Core\\Str",
    });
    // A batch Bun itself stopped: the verdict quotes Bun's panic, never `bun run`'s exit line.
    const crashed = { code: 3, out: "== Core\\Str\n", err: '====\npanic(main thread): integer does not fit in destination type\noh no: Bun has crashed.\nerror: script "nv" exited with code 3' };
    expect(firstErrLine(proofOutcome(["Core\\Str"], crashed))).toBe("panic(main thread): integer does not fit in destination type");
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

  test("a release test builds early with --no-run and without its runner arguments, and anything else does not build early", () => {
    expect(releaseBuildArgv(check({ kind: "cargo-named", args: ["test", "--release", "-p", "nvs-cli", "--bin", "nvs", "by_the_margin", "--", "--test-threads=1"] }))).toEqual(["cargo", "test", "--release", "-p", "nvs-cli", "--bin", "nvs", "by_the_margin", "--no-run"]);
    expect(releaseBuildArgv(check({ kind: "cargo-named", args: ["build", "--release", "-p", "nvs-cli"] }))).toEqual(["cargo", "build", "--release", "-p", "nvs-cli"]);
    expect(releaseBuildArgv(check({ kind: "cargo-named", args: ["test", "-p", "nvs-cli"] }))).toBeNull();
    expect(releaseBuildArgv(check({ argv: ["bun", "nv", "bench", "--guard"] }))).toBeNull();
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
  const everything = () => true;
  const opts = (sweep: ReturnType<typeof fake>, more: Partial<Parameters<typeof acceptance>[1]> = {}) => ({ label, sweep, reached: everything, collect: false, ...more });

  test("it stops at the first red, after starting the overlap command behind the setup tier", async () => {
    const s = fake(["cmd2"]);
    const r = await acceptance(plan, opts(s));
    expect(r.fail).toBe("cmd2 red");
    expect(s.ran).toEqual(["catch", "setup", "over", "floor-fix", "cmd2"]);
  });

  test("the sweep names each tier's reached checks before the first of them runs", async () => {
    const told: string[][] = [];
    const s = { ...fake([]), batch: (checks: Check[]) => told.push(checks.map((c) => c.id)) };
    await acceptance(plan, opts(s, { reached: (c) => c.id !== "cmd2" }));
    expect(told).toEqual([["catch"], ["setup"], ["floor-fix"], ["cmd3"], ["goal-fix"], ["over"], ["rel"]]);
  });

  test("a collecting sweep runs past each red and names every one", async () => {
    const s = fake(["cmd2", "goal-fix", "rel"]);
    const r = await acceptance(plan, opts(s, { collect: true }));
    expect(r.fail).toBe("cmd2 red\n       also red: goal-fix red\n       also red: rel red");
    expect(r.ran).toBe(plan.length);
  });

  test("a check the change does not reach is never started, and its verdict is the store's", async () => {
    const none = fake([]);
    const r = await acceptance(plan, opts(none, { reached: () => false }));
    expect(r).toEqual({ fail: "", ran: 0, answered: plan.length, verdicts: new Map(plan.map((c) => [c.id, true])) });
    expect(none.ran).toEqual([]);
    const one = fake([]);
    const traced: string[] = [];
    await acceptance(plan, opts(one, { reached: (c) => c.id === "cmd3", trace: (c) => traced.push(c.id) }));
    expect(one.ran).toEqual(["cmd3"]);
    expect(traced).toEqual(["cmd3"]);
    const judged = await acceptance(plan, opts(fake([]), { reached: () => false, judged: (c) => ({ fail: c.id === "cmd2" ? "cmd2: a named test did not run" : "", short: "" }), collect: true }));
    expect(judged.fail).toBe("cmd2: a named test did not run");
    expect(judged.verdicts.get("cmd2")).toBe(false);
  });

  test("an overlap command the change does not reach is not started beside the tiers", async () => {
    const s = fake([]);
    await acceptance(plan, opts(s, { reached: (c) => c.id !== "over" }));
    expect(s.ran).not.toContain("over");
  });

  /** A sweep whose overlap command and release builds each end only when the test says so. */
  const held = (red: string[] = []) => {
    const log: string[] = [];
    const end = { over: () => {}, build: () => {} };
    const sweep = {
      check: async (c: Check): Promise<Verdict> => {
        log.push(`run ${c.id}`);
        if (c.id === "over") await new Promise<void>((r) => (end.over = () => (log.push("over ends"), r())));
        return { fail: red.includes(c.id) ? `${c.id} red` : "", short: "" };
      },
      prebuild: (checks: Check[]) => {
        log.push(`build ${checks.map((c) => c.id).join(",")}`);
        return new Promise<void>((r) => (end.build = () => (log.push("build ends"), r())));
      },
    };
    return { log, end, sweep };
  };
  const until = async (f: () => boolean) => {
    while (!f()) await Bun.sleep(1);
  };

  test("the release builds start beside the overlap command, and the release checks run once both end", async () => {
    const h = held();
    const swept = acceptance(plan, { label, sweep: h.sweep, reached: everything, collect: false });
    await until(() => h.log.includes("build rel"));
    h.end.over();
    await Bun.sleep(5);
    expect(h.log).not.toContain("run rel");
    h.end.build();
    expect((await swept).fail).toBe("");
    expect(h.log).toEqual(["run catch", "run setup", "run over", "run floor-fix", "run cmd2", "run cmd3", "run goal-fix", "build rel", "over ends", "build ends", "run rel"]);
  });

  test("a sweep that stops at a red overlap command returns only once the release builds end", async () => {
    const h = held(["over"]);
    let returned = false;
    const swept = acceptance(plan, { label, sweep: h.sweep, reached: everything, collect: false }).then((r) => ((returned = true), r));
    await until(() => h.log.includes("build rel"));
    h.end.over();
    await Bun.sleep(5);
    expect(returned).toBe(false);
    h.end.build();
    expect((await swept).fail).toBe("over red");
    expect(h.log).not.toContain("run rel");
  });

  test("no release build starts when the change reaches no release check", async () => {
    const h = held();
    const swept = acceptance(plan, { label, sweep: h.sweep, reached: (c) => c.id !== "rel", collect: false });
    await until(() => h.log.includes("run over"));
    h.end.over();
    await swept;
    expect(h.log.some((l) => l.startsWith("build"))).toBe(false);
  });

  /** A `beside` run, as the Linux legs are one, that ends only when the test says so. */
  const legs = (log: string[]) => {
    const end = { legs: () => {} };
    const beside = () => {
      log.push("legs start");
      return new Promise<void>((r) => (end.legs = () => (log.push("legs end"), r())));
    };
    return { end, beside };
  };

  test("the legs start with the release builds, and the release checks run only once the legs end too", async () => {
    const h = held();
    const l = legs(h.log);
    const swept = acceptance(plan, { label, sweep: h.sweep, reached: everything, collect: false, beside: l.beside });
    await until(() => h.log.includes("legs start"));
    expect(h.log.slice(-2)).toEqual(["build rel", "legs start"]);
    h.end.over();
    h.end.build();
    await Bun.sleep(5);
    expect(h.log).not.toContain("run rel");
    l.end.legs();
    expect((await swept).fail).toBe("");
    expect(h.log.slice(-2)).toEqual(["legs end", "run rel"]);
  });

  test("the legs start even when the change reaches no release check, and the sweep returns only once they end", async () => {
    const h = held();
    const l = legs(h.log);
    let returned = false;
    const swept = acceptance(plan, { label, sweep: h.sweep, reached: (c) => c.id !== "rel", collect: false, beside: l.beside }).then((r) => ((returned = true), r));
    await until(() => h.log.includes("legs start"));
    h.end.over();
    await Bun.sleep(5);
    expect(returned).toBe(false);
    l.end.legs();
    expect((await swept).fail).toBe("");
  });

  test("a sweep that stops at a red check before the overlap tier never starts the legs", async () => {
    const h = held(["cmd2"]);
    const l = legs(h.log);
    const r = await acceptance(plan, { label, sweep: h.sweep, reached: (c) => c.id !== "over", collect: false, beside: l.beside });
    expect(r.fail).toBe("cmd2 red");
    expect(h.log).not.toContain("legs start");
  });

  test("a short suite is reported, and its verdict is not green", async () => {
    const odd = [check({ id: "short", kind: "nvs-suite" }), check({ id: "fine" })];
    const r = await acceptance(odd, opts(fake([], ["short"])));
    expect(r.fail).toBe("short short");
    expect(r.verdicts.get("short")).toBe(false);
    expect(r.verdicts.get("fine")).toBe(true);
  });

  test("a failed build stops a collecting sweep at once", async () => {
    const s = { ran: [] as string[], check: async (c: Check): Promise<Verdict> => ({ fail: c.id === "cmd2" ? "the native build failed -- error: x" : c.id === "catch" ? "catch red" : "", short: "" }) };
    const r = await acceptance(plan, { label, sweep: s, reached: everything, collect: true });
    expect(r.fail).toBe("catch red\n       also red: the native build failed -- error: x");
  });

  test("owed is the carried checks the change reaches, less stage 0, the goal's own and memoize = false", () => {
    const live = check({ id: "live", stage: 1, memoize: false });
    const reached = new Set(["floor-fix", "setup", "cmd3", "live", "catch"]);
    const owed = owedChecks([...plan, live], label, (c: Check) => reached.has(c.id));
    expect(owed.map((c) => c.id)).toEqual(["floor-fix", "setup"]);
    expect(owedChecks(plan, label, everything).map((c) => c.id)).toEqual(["over", "floor-fix", "setup"]);
  });

  describe("a setup command with the floor gate shut", () => {
    // The shape of the queue fixture's pair: a migration never memoized, and the fixture that reads its tables.
    const migrate = check({ id: "migrate", argv: ["{nvs}", "queue", "migrate"], setup: true, memoize: false });
    const fixture = check({ id: "queue-fix", kind: "exact", file: "q.nvs", want: [] });
    const bench = check({ id: "bench", argv: ["bun", "nv", "bench", "--guard"] });
    const live = check({ id: "live", argv: ["e"], memoize: false });
    const whole = [bench, fixture, live, migrate];

    test("is never held, while every other heavy check is", () => {
      expect(heldByGate(migrate)).toBe(false);
      expect([bench, live].map(heldByGate)).toEqual([true, true]);
      expect(whole.filter((c) => !heldByGate(c)).map((c) => c.id)).toEqual(["queue-fix", "migrate"]);
    });

    test("rides along with a filter that selects a fixture, and with no other one", () => {
      expect(withSetups([fixture], whole).map((c) => c.id)).toEqual(["queue-fix", "migrate"]);
      expect(withSetups([fixture, migrate], whole).map((c) => c.id)).toEqual(["queue-fix", "migrate"]);
      expect(withSetups([live], whole).map((c) => c.id)).toEqual(["live"]);
    });

    test("runs before the fixture that reads what it writes", async () => {
      const s = fake([]);
      const shown = withSetups(whole.filter((c) => c.id === "queue-fix" && !heldByGate(c)), whole);
      await acceptance(shown, opts(s));
      expect(s.ran).toEqual(["migrate", "queue-fix"]);
    });
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
  expect(exes.get("nvs-abi-probe")).toEqual([{ target: "perf_guards", kind: "test", exe: "t2.exe", dir: "d:/mwl/benches/abi-probe" }]);
});

test("subsetRun names what a filtered `cargo test` runs, and nothing when a flag can change it", () => {
  expect(subsetRun(["test", "-p", "nvs-cli"])).toEqual({ crate: "nvs-cli", kind: null, target: null });
  expect(subsetRun(["test", "-p", "nvs-cli", "--bin", "nvs", "units_held_stay_bounded"])).toEqual({ crate: "nvs-cli", kind: "bin", target: "nvs" });
  expect(subsetRun(["test", "-p", "nvs-stdlib", "--lib", "math::tests", "--", "--exact"])).toEqual({ crate: "nvs-stdlib", kind: "lib", target: null });
  expect(subsetRun(["test", "-p", "nvs-cli", "--test", "live_config", "reload"])).toEqual({ crate: "nvs-cli", kind: "test", target: "live_config" });
  expect(subsetRun(["test", "-p", "nvs-db", "--features", "mssql"])).toBeNull();
  expect(subsetRun(["test", "-p", "nvs-db", "x", "--", "--ignored"])).toBeNull();
  expect(subsetRun(["test", "--release", "-p", "nvs-db"])).toBeNull();
  expect(subsetRun(["build", "-p", "nvs-db"])).toBeNull();
});
