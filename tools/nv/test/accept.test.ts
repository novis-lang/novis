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

  plainCrateTest,
  programFailLine,
  PROOF_SCOPES_PER_LIMIT,
  proofBatch,
  proofBatchArgv,
  proofComplete,
  proofGroups,
  proofKeys,
  proofLimitMs,
  proofOnly,
  proofOutcome,
  proofResult,
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

  test("proofOnly reads only a plain `--verify --only` check", () => {
    expect(proofOnly(check({ argv: ["bun", "nv", "proofs", "--verify", "--only", "Core\\Str::at", "Core\\Str::after"] }))).toEqual(["Core\\Str::at", "Core\\Str::after"]);
    expect(proofOnly(check({ argv: ["bun", "nv", "proofs", "--verify", "--only", "x", "--gate"] }))).toBeNull();
    expect(proofOnly(check({ argv: ["bun", "nv", "proofs", "--only", "x", "--gate"] }))).toBeNull();
    expect(proofOnly(check({ argv: ["bun", "nv", "proofs", "--verify", "--only", "x"], cwd: "tools" }))).toBeNull();
    expect(proofOnly(check({ argv: ["bun", "nv", "proofs", "--verify", "--only"] }))).toBeNull();
  });

  const group = (id: string, ...gs: string[]) => check({ id, argv: ["bun", "nv", "proofs", "--verify", ...gs.flatMap((g) => ["--group", g])] });
  const only = (id: string, ...ids: string[]) => check({ id, argv: ["bun", "nv", "proofs", "--verify", "--only", ...ids] });

  test("proofBatch puts every group and every feature list of the checks in one process", () => {
    const many = Array.from({ length: 30 }, (_, i) => group(`g${i}`, `G${i}`));
    const b = proofBatch([...many, only("o1", "A::x", "A::y"), check({ id: "cargo", kind: "cargo-named" }), only("o2", "B::z"), group("gAB", "A", "B")])!;
    expect(b.groups).toEqual([...many.map((_, i) => `G${i}`), "A", "B"]);
    expect(b.named).toEqual([
      { label: "only-1", ids: ["A::x", "A::y"] },
      { label: "only-2", ids: ["B::z"] },
    ]);
    expect(b.parts.get("g3")).toEqual(["G3"]);
    expect(b.parts.get("gAB")).toEqual(["A", "B"]);
    expect(b.parts.get("o2")).toEqual(["only-2"]);
    expect(b.parts.has("cargo")).toBe(false);
    const argv = proofBatchArgv(b);
    expect(argv.slice(0, 6)).toEqual(["bun", "nv", "proofs", "--verify", "--group", "G0"]);
    expect(argv.slice(-11)).toEqual(["--group", "A", "--group", "B", "--only-as", "only-1", "A::x", "A::y", "--only-as", "only-2", "B::z"]);
  });

  test("proofBatch shares a feature list two checks name, skips a label a group has, and makes no batch of one scope", () => {
    const b = proofBatch([group("g", "only-1"), only("a", "X::f"), only("b", "X::f"), only("c", "Y::g")])!;
    expect(b.named).toEqual([
      { label: "only-2", ids: ["X::f"] },
      { label: "only-3", ids: ["Y::g"] },
    ]);
    expect(b.parts.get("a")).toEqual(["only-2"]);
    expect(b.parts.get("b")).toEqual(["only-2"]);
    expect(proofBatch([group("g", "A")])).toBeNull();
    expect(proofBatch([only("a", "X::f"), only("b", "X::f")])).toBeNull();
    expect(proofBatch([group("g", "A"), group("h", "A")])).toBeNull();
    expect(proofBatch([group("g", "A"), only("a", "X::f")])).not.toBeNull();
  });

  test("proofLimitMs gives a batch one check's limit for each twelve scopes it starts", () => {
    expect(PROOF_SCOPES_PER_LIMIT).toBe(12);
    expect(proofLimitMs(0, 1000)).toBe(1000);
    expect(proofLimitMs(2, 1000)).toBe(1000);
    expect(proofLimitMs(12, 1000)).toBe(1000);
    expect(proofLimitMs(13, 1000)).toBe(2000);
    expect(proofLimitMs(148, 1000)).toBe(13000);
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

  // A folded run: a group, then an `--only-as` scope, whose gate line names its features as `--only` does.
  const folded = [
    "== Core\\Env",
    "nv proofs gate: nothing owed in Core\\Env (2 features).",
    "proofs examples: 9 ok, 0 skipped, 0 known-gap, 0 failed",
    "-- Core\\Env: passed",
    "== only-1",
    "nv proofs gate: nothing owed in 2 named feature(s) (2 features).",
    "  FAIL  docs/examples/core/Str/at/01.nvs: stdout differs",
    "proofs examples: 3 ok, 0 skipped, 0 known-gap, 1 failed",
    "-- only-1: failed",
    "proofs: 1 program(s) ended differently on the recording run, so each runs every time until they agree:",
    "",
  ].join("\n");

  test("an `--only-as` scope is judged as its `--only` check run alone", () => {
    const alone = { code: 1, out: "nv proofs gate: nothing owed in 2 named feature(s) (2 features).\n  FAIL  docs/examples/core/Str/at/01.nvs: stdout differs\nproofs examples: 3 ok, 0 skipped, 0 known-gap, 1 failed\n", err: "" };
    expect(proofComplete(["only-1"], folded)).toBe(true);
    expect(proofComplete(["Core\\Env", "only-1"], folded)).toBe(true);
    expect(proofComplete(["only-2"], folded)).toBe(false);
    const o = proofOutcome(["only-1"], { code: 1, out: folded, err: "" });
    expect(o).toEqual(alone);
    const c = check({ want: ["nothing owed", "0 failed"] });
    expect(judgeCommand(c, o, "L")).toBe(judgeCommand(c, alone, "L"));
    expect(judgeCommand(c, o, "L")).toBe("L: exit 1 -- FAIL  docs/examples/core/Str/at/01.nvs: stdout differs");
    expect(allReds(["first: red", judgeCommand(c, o, "L")])).toContain("also red: L: exit 1 -- FAIL  docs/examples/core/Str/at/01.nvs: stdout differs");
  });

  describe("proofResult", () => {
    const keyed = (...ks: string[]) => new Map(ks.map((k) => [k, ""]));
    const batchRun = (o: { code: number; out: string; err: string }) => ({ o, keys: keyed("all"), parts: new Map([["only-1", keyed("mine")], ["Core\\Env", keyed("env")]]) });
    const aloneRun = { o: { code: 0, out: "alone\n", err: "" }, keys: keyed("alone"), parts: new Map() };

    test("a check the batch closed a section for is cut from it, and never runs alone", async () => {
      let alone = 0;
      const r = await proofResult(["only-1"], async () => batchRun({ code: 1, out: folded, err: "" }), async () => {
        alone++;
        return aloneRun;
      });
      expect(alone).toBe(0);
      expect(r.o.code).toBe(1);
      expect([...r.keys.keys()]).toEqual(["mine"]);
    });

    test("a batch that died runs each check it left without a verdict alone, and says why", async () => {
      const whys: string[] = [];
      const dead = async () => batchRun({ code: -1, out: "", err: "timed out after 3600s" });
      const r = await proofResult(["only-1"], dead, async (why) => {
        whys.push(why);
        return aloneRun;
      });
      expect(r).toEqual({ o: aloneRun.o, keys: keyed("alone") });
      expect(whys).toEqual([" (run alone: the batched run did not finish and printed no verdict for it)"]);
      // Bun crashed after one section: the closed one is cut from it, the other runs alone.
      const half = async () => batchRun({ code: 3, out: folded.split("== only-1")[0]!, err: "panic(main thread): oh no" });
      const env = await proofResult(["Core\\Env"], half, async () => aloneRun);
      expect(env.o.code).toBe(0);
      expect([...env.keys.keys()]).toEqual(["env"]);
      const mine = await proofResult(["only-1"], half, async (why) => {
        whys.push(why);
        return aloneRun;
      });
      expect(mine.o).toEqual(aloneRun.o);
      expect(whys[1]).toBe(" (run alone: the batched run ended with exit 3 and printed no verdict for it)");
    });

    test("a check outside a batch runs alone with nothing added to its line", async () => {
      const whys: string[] = [];
      await proofResult([], null, async (why) => {
        whys.push(why);
        return aloneRun;
      });
      expect(whys).toEqual([""]);
    });
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
  const label = (n: number) => `${n} stage`;
  const plan: Check[] = [
    check({ id: "rel", stage: 2, argv: ["bun", "nv", "bench", "--guard"] }),
    check({ id: "goal-fix", kind: "exact", stage: 3, file: "g.nvs", want: [] }),
    check({ id: "cmd3", stage: 3, argv: ["a"] }),
    check({ id: "cmd2", stage: 2, argv: ["b"] }),
    check({ id: "over", stage: 1, argv: ["c"], overlap: true }),
    check({ id: "catch-fix", kind: "exact", stage: 0, file: "f.nvs", want: [] }),
    check({ id: "setup", stage: 1, argv: ["d"], setup: true }),
    check({ id: "catch", stage: 0, kind: "cargo-named", args: ["test"] }),
  ];

  test("tiers run catch-up, setup, catch-up fixtures, the rest by stage, goal fixtures, overlap, release", () => {
    const got = tiers(plan).map((t) => [t.name, t.checks.map((c) => c.id)]);
    expect(got).toEqual([
      ["catch-up", ["catch"]],
      ["setup", ["setup"]],
      ["catch-up fixtures", ["catch-fix"]],
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
    expect(s.ran).toEqual(["catch", "setup", "over", "catch-fix", "cmd2"]);
  });

  test("the sweep names each tier's reached checks before the first of them runs", async () => {
    const told: string[][] = [];
    const s = { ...fake([]), batch: (checks: Check[]) => told.push(checks.map((c) => c.id)) };
    await acceptance(plan, opts(s, { reached: (c) => c.id !== "cmd2" }));
    expect(told).toEqual([["catch"], ["setup"], ["catch-fix"], ["cmd3"], ["goal-fix"], ["over"], ["rel"]]);
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
    const told: { stop: AbortSignal | null } = { stop: null };
    const sweep = {
      check: async (c: Check): Promise<Verdict> => {
        log.push(`run ${c.id}`);
        if (c.id === "over") await new Promise<void>((r) => (end.over = () => (log.push("over ends"), r())));
        return { fail: red.includes(c.id) ? `${c.id} red` : "", short: "" };
      },
      prebuild: (checks: Check[], stop: AbortSignal) => {
        told.stop = stop;
        log.push(`build ${checks.map((c) => c.id).join(",")}`);
        return new Promise<void>((r) => (end.build = () => (log.push("build ends"), r())));
      },
    };
    return { log, end, sweep, told };
  };
  const until = async (f: () => boolean) => {
    while (!f()) await Bun.sleep(1);
  };

  test("the release builds start with the goal fixtures tier, and the release checks run once they and the overlap command end", async () => {
    const h = held();
    const swept = acceptance(plan, { label, sweep: h.sweep, reached: everything, collect: false });
    await until(() => h.log.includes("run goal-fix"));
    h.end.over();
    await Bun.sleep(5);
    expect(h.log).not.toContain("run rel");
    h.end.build();
    expect((await swept).fail).toBe("");
    expect(h.log).toEqual(["run catch", "run setup", "run over", "run catch-fix", "run cmd2", "run cmd3", "build rel", "run goal-fix", "over ends", "build ends", "run rel"]);
  });

  test("the release builds start after the cargo and command tier and before the goal's fixtures, and a release check waits for them", async () => {
    const h = held();
    const swept = acceptance(plan, { label, sweep: h.sweep, reached: (c) => c.id !== "over", collect: false });
    await until(() => h.log.includes("run goal-fix"));
    expect(h.log).toEqual(["run catch", "run setup", "run catch-fix", "run cmd2", "run cmd3", "build rel", "run goal-fix"]);
    await Bun.sleep(5);
    expect(h.log).not.toContain("run rel");
    expect(h.told.stop?.aborted).toBe(false);
    h.end.build();
    expect((await swept).fail).toBe("");
    expect(h.log.slice(-2)).toEqual(["build ends", "run rel"]);
  });

  test("a sweep that stops at a red overlap command returns only once the release builds end", async () => {
    const h = held(["over"]);
    let returned = false;
    const swept = acceptance(plan, { label, sweep: h.sweep, reached: everything, collect: false }).then((r) => ((returned = true), r));
    await until(() => h.log.includes("run goal-fix"));
    h.end.over();
    await Bun.sleep(5);
    expect(returned).toBe(false);
    expect(h.told.stop?.aborted).toBe(true);
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

  test("the legs start with the overlap tier, and the release checks run only once the legs end too", async () => {
    const h = held();
    const l = legs(h.log);
    const swept = acceptance(plan, { label, sweep: h.sweep, reached: everything, collect: false, beside: l.beside });
    await until(() => h.log.includes("legs start"));
    expect(h.log.slice(-2)).toEqual(["run goal-fix", "legs start"]);
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

  test("a sweep that stops at a red check in the cargo and command tier starts neither the legs nor a release build", async () => {
    const h = held(["cmd2"]);
    const l = legs(h.log);
    const r = await acceptance(plan, { label, sweep: h.sweep, reached: (c) => c.id !== "over", collect: false, beside: l.beside });
    expect(r.fail).toBe("cmd2 red");
    expect(h.log).not.toContain("legs start");
    expect(h.log.some((l) => l.startsWith("build"))).toBe(false);
  });

  test("a sweep that stops at a red goal fixture tells the release builds to stop, and returns once the one in progress ends", async () => {
    const h = held(["goal-fix"]);
    const l = legs(h.log);
    let returned = false;
    const swept = acceptance(plan, { label, sweep: h.sweep, reached: (c) => c.id !== "over", collect: false, beside: l.beside }).then((r) => ((returned = true), r));
    await until(() => h.told.stop?.aborted === true);
    await Bun.sleep(5);
    expect(returned).toBe(false);
    h.end.build();
    expect((await swept).fail).toContain("goal-fix red");
    expect(h.log).not.toContain("legs start");
    expect(h.log).not.toContain("run rel");
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
