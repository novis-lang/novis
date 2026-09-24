import { describe, expect, test } from "bun:test";
import { type Check, firstErrLine, judgeCommand, judgeProgram, judgeTests, measuresReleaseCli, orderedIn, plainCrateTest, stdoutLines, testExecutables } from "../driver/accept.ts";

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
  expect(measuresReleaseCli(check({ argv: ["python", "tools/bench.py", "--guard"] }))).toBe(true);
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
