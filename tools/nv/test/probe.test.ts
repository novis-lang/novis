import { expect, test } from "bun:test";
import { judgeProbe, PROBE_SHAPES, PROBE_SIZES, probePlan, probeSource, type ProbeMeta } from "../proofs/probe.ts";
import type { Ran } from "../proofs/run.ts";

// `bun nv proofs --probe`'s program writer and judge, over a fixture of `nvs meta --json`, so no binary
// runs and every verdict is exact.

const META: ProbeMeta = {
  classes: [
    {
      name: "Core\\Json",
      members: [{ kind: "static", name: "encode", params: [{ name: "value", type: "mixed" }], returns: "string", signature: "encode(mixed $value): string" }],
    },
    {
      name: "Core\\Arr",
      members: [
        {
          kind: "static",
          name: "map",
          params: [
            { name: "a", type: "array<T>" },
            { name: "fn", type: "callable(T, string): U" },
          ],
          returns: "array<U>",
          signature: "map(array<T> $a, callable(T, string): U $fn): array<U>",
        },
        { kind: "instance", name: "count", params: [] },
      ],
    },
    {
      name: "Core\\BigInt",
      members: [{ kind: "instance", name: "add", params: [{ name: "other", type: "Core\\BigInt" }], returns: "Core\\BigInt", signature: "add(Core\\BigInt $other): Core\\BigInt" }],
    },
    {
      name: "Core\\ObjectMap",
      members: [
        {
          kind: "instance",
          name: "set",
          params: [
            { name: "key", type: "K" },
            { name: "value", type: "V" },
          ],
          returns: "void",
          signature: "set(K $key, V $value): void",
        },
      ],
    },
    { name: "Core\\IO", members: [{ kind: "static", name: "read", params: [{ name: "path", type: "string" }], returns: "string" }] },
    { name: "Core\\Time", members: [{ kind: "static", name: "sleep", params: [{ name: "d", type: "Core\\Time\\Duration" }], returns: "void" }] },
  ],
};

const ran = (stdout: string, stderr: string, code = 1): Ran => ({ code, stdout, stderr, timedOut: false, ms: 12 });

test("a member gets a flat, a shared and a doubling input at four sizes", () => {
  const { probes } = probePlan(META);
  const encode = probes.find((p) => p.member === "Core\\Json::encode")!;
  for (const size of PROBE_SIZES) {
    for (const shape of PROBE_SHAPES) expect(encode.steps.some((s) => s.size === size && s.shape === shape)).toBe(true);
  }
  expect(encode.steps.length).toBe(PROBE_SIZES.length * PROBE_SHAPES.length);
  const source = probeSource(encode);
  // The program lowers its own ceiling to 16 MiB before its first call.
  expect(source).toContain("Core\\Config::set('limits.memory', '16777216');");
  expect(source).toContain("Core\\Str::repeat('a', 8388608)");
  expect(source).toMatch(/\$p0g = \[\$p0g, \$p0g\]/);
  expect(source).toContain("probe: 11 doubling 8388608 -");
  // A run resumed after step 5 holds only the steps from there on.
  expect(probeSource(encode, 6)).not.toContain("Probe::step5();");
  expect(probeSource(encode, 6)).toContain("Probe::step6();");
});

test("a callback parameter gets a function that returns its argument", () => {
  const { probes } = probePlan(META);
  const map = probes.find((p) => p.member === "Core\\Arr::map")!;
  expect(probeSource(map)).toContain("Core\\Arr::map($p0, fn($a0, $a1) => $a0);");
});

test("an instance member gets a receiver from its factory", () => {
  const { probes, skips } = probePlan(META);
  const add = probes.find((p) => p.member === "Core\\BigInt::add")!;
  const source = probeSource(add);
  // Each step makes a fresh receiver, and a parameter of a class with a factory gets one too.
  expect(source).toContain("var $r = Core\\BigInt::of(1);");
  expect(source).toContain("$r->add(Core\\BigInt::of(1));");
  const set = probes.find((p) => p.member === "Core\\ObjectMap::set")!;
  expect(probeSource(set)).toContain("var $r = new Core\\ObjectMap<mixed, mixed>();");
  expect(probeSource(set)).toMatch(/\$r->set\(\$p0, \$p1\);/);
  // A class with no factory is a skip, with its reason.
  expect(probes.some((p) => p.member === "Core\\Arr::count")).toBe(false);
  expect(skips).toContainEqual({ member: "Core\\Arr::count", reason: "no factory: `PROBE_FACTORIES` has none for this class" });
  // A factory that throws is named, and is not red.
  const threw = judgeProbe("Core\\BigInt::add", ran("", '{"level":"error","msg":"no match","fields":{"class":"LogicError"},"nodes":[]}\n'));
  expect(threw.red).toBe(false);
  expect(threw.note).toBe("its setup threw an uncaught LogicError before its first step: no match");
});

test("a member of an excluded class is listed as skipped with its reason", () => {
  const { probes, skips } = probePlan(META);
  expect(skips).toContainEqual({ member: "Core\\IO::read", reason: "reads and writes files" });
  expect(skips).toContainEqual({ member: "Core\\Time::sleep", reason: "sleeps" });
  expect(probes.some((p) => p.member.startsWith("Core\\IO"))).toBe(false);
});

test("a reading past the bound is red and names the member, the shape and the size", () => {
  const fatal = (held: number) => `FATAL: the program exceeded its memory limit — ${held} bytes held against a ceiling of 16777216, of which 1048576 is reserved`;
  const steps = "probe: 0 flat 65536 -\nprobe: 1 shared 65536 -\nprobe: 11 doubling 8388608 -\n";
  // 16 MiB times 1.5 plus 8 MiB is the bound: 32 MiB.
  const past = judgeProbe("Core\\Json::encode", ran(steps, fatal(40 * 1048576)));
  expect(past.red).toBe(true);
  expect(past.why).toContain("Core\\Json::encode");
  expect(past.why).toContain("doubling input of 8 MiB");
  expect(past.step).toEqual({ index: 11, shape: "doubling", size: 8388608, value: "-" });
  const within = judgeProbe("Core\\Json::encode", ran(steps, fatal(17 * 1048576)));
  expect(within.red).toBe(false);
  expect(within.reading).toEqual({ held: 17 * 1048576, ceiling: 16777216 });
  const killed = judgeProbe("Core\\Json::encode", { ...ran(steps, "", -1), signal: "SIGKILL" });
  expect(killed.red).toBe(true);
  expect(judgeProbe("Core\\Json::encode", ran(`${steps}probe: done\n`, "", 0)).red).toBe(false);
});
