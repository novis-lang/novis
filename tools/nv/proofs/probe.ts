// `bun nv proofs --probe`: drives every `Core` member past a small memory ceiling and judges how far
// past it each one got, with the same bound `--overshoot` holds an attack to (`overshootBound`).
//
// The members come from `nvs meta --json` on the proof binary, so the probe and the binary always agree.
// Each member gets one program under `.agent-tmp/probe/`, deleted when the run ends. The program lowers
// its own memory limit to `PROBE_CEILING`, then calls the member at each of `PROBE_SIZES`, once per input
// shape its parameters can take (`PROBE_SHAPES`):
//
// - **flat**: one value of that size;
// - **shared**: a list that holds the same `PROBE_ROW` row many times, up to that size;
// - **doubling**: `$g = [$g, $g]` repeated until the graph is that size if copied.
//
// An integer or float parameter takes `0`, `1`, the size and the largest value its type allows, one call
// each. A callback gets a function that returns its first argument. Any other parameter takes its type's
// smallest value: `false`, `0`, an enum's first case, or the value `PROBE_FACTORIES` makes of its class.
// A parameter of a type the probe cannot build (a shape, a numeric list, a class with no factory) leaves
// the member listed as not built, with the type named.
//
// An instance member is called on a receiver its class's `PROBE_FACTORIES` expression makes, fresh at the
// start of each step, with the smallest arguments that make one. A class with no factory is a skip, and
// `PROBE_NO_FACTORY` says why. A factory that throws ends its program, and its row names the error.
//
// Each step is a static method of its own, so its inputs are freed before the next step builds its own.
// Each call is in a `try`: a catchable throw is the member refusing, which passes, and so does a finished
// call. Only the memory reading judges. A step prints `probe: <shape> <size> <value>` before its call, so
// a reading names the input that reached it.
//
// The probe skips the members that touch files, processes, the network, a database, a directory server,
// mail, a shared store, the terminal or the host, or that sleep: `PROBE_SKIPS`, each with its reason.
// `memory-ceiling-holes`'s attacks cover those readers. A class outside `Core\` is a component's, and
// is not probed.
//
// A reading past the bound is red, and so are a program the host killed and a runtime crash. What is not
// a reading is listed and never red: a parameter not built, a program that does not compile, a factory
// that throws. A memory `FATAL` ends a program, so a run a limit stopped within the bound runs again from
// its next step, at the cost of one process per stop. The bound is never raised to make a member pass:
// a member past it is fixed at its site, through `NvsArray`, `ByteWriter`, `StrWriter`, `affords` or
// `nvs_runtime::quoted`, and gets an attack.

import { mkdirSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { abs } from "../lib/paths.ts";
import { progress } from "../lib/progress.ts";
import { metaJson } from "./roster.ts";
import { type MemoryReading, memoryReading, OVERSHOOT_FACTOR, OVERSHOOT_SLACK_BYTES, overshoot, type Ran, spawnProof, hostKilled } from "./run.ts";

/** The memory limit every probe program sets for itself. The bound is then 32 MiB, so the largest input
 * finds any member that grows its input more than four times. */
export const PROBE_CEILING = 16 * 1024 * 1024;
/** The input sizes, smallest first. */
export const PROBE_SIZES = [64 * 1024, 512 * 1024, 2 * 1024 * 1024, 8 * 1024 * 1024];
/** The input shapes, in the order a program tries them at each size. */
export const PROBE_SHAPES = ["flat", "shared", "doubling"] as const;
export type Shape = (typeof PROBE_SHAPES)[number];
/** The size of the one row a shared list repeats, and of a doubling graph's leaf. */
export const PROBE_ROW = 64 * 1024;
/** The leaf of a doubling graph: short, so the graph is small while a copy of it is not. */
const LEAF_BYTES = 64;
/** How many probe programs run at once: each may grow to its ceiling and past it. */
const PROBE_JOBS = 2;
/** How long one probe program may run. Two run at once, so this has room for a member that is slow on
 * 8 MiB of input. */
const PROBE_TIMEOUT_MS = 60_000;
/** Where the programs are written. The run deletes it at its end. */
export const PROBE_DIR = ".agent-tmp/probe";

/**
 * What the probe does not call, and why: a class name skips the class and every class under it, and a
 * `Class::member` name skips one member. Each of these reads or writes something outside the program,
 * or waits, so a probe could change the host or hang. The attacks of `memory-ceiling-holes` cover their
 * readers.
 */
export const PROBE_SKIPS: ReadonlyArray<readonly [string, string]> = [
  ["Core\\IO", "reads and writes files"],
  ["Core\\Process", "starts processes"],
  ["Core\\Script", "starts child processes"],
  ["Core\\Http", "makes network requests"],
  ["Core\\Net", "opens network connections"],
  ["Core\\Socket", "opens WebSocket connections"],
  ["Core\\Sse", "waits on server events"],
  ["Core\\WebPush", "sends push messages"],
  ["Core\\Mail", "sends mail"],
  ["Core\\Db", "talks to a database"],
  ["Core\\Ldap", "talks to a directory server"],
  ["Core\\Queue", "talks to a shared queue"],
  ["Core\\Cache", "talks to a shared store"],
  ["Core\\Topic", "talks to a shared store"],
  ["Core\\RateLimit", "talks to a shared Redis store"],
  ["Core\\Session", "reads and writes the session store"],
  ["Core\\Storage", "reads and writes disk files"],
  ["Core\\Zip", "reads and writes archive files"],
  ["Core\\Log", "writes log files"],
  ["Core\\Response", "sends files"],
  ["Core\\Request\\Part", "saves uploads to disk"],
  ["Core\\Cli", "reads terminal input"],
  ["Core\\Env", "reads the environment"],
  ["Core\\Os", "reads the host"],
  ["Core\\Signal", "waits on OS signals"],
  ["Core\\Time::sleep", "sleeps"],
];

/**
 * How the probe makes a value of each `Core` class it cannot write as a literal: the expression, with the
 * smallest arguments that make one. An instance member's receiver comes from here, made fresh at each
 * step, and so does a parameter of one of these types. `nvs meta --json` lists no constructors, so the
 * table is written by hand. A factory that throws ends its program, and the row names the error.
 */
export const PROBE_FACTORIES: ReadonlyArray<readonly [string, string]> = [
  ["Core\\BigInt", "Core\\BigInt::of(1)"],
  ["Core\\Regex\\Match", "Core\\Regex::match('a', 'a') ?? throw new LogicError('no match')"],
  ["Core\\Time\\Instant", "Core\\Time::fromEpoch(0)"],
  ["Core\\Time\\Zone", "Core\\Time\\Zone::of('UTC')"],
  ["Core\\Time\\DateTime", "Core\\Time::at(2000, 1, 1, Core\\Time\\Zone::of('UTC'))"],
  ["Core\\Time\\Date", "Core\\Time\\Date::at(2000, 1, 1)"],
  ["Core\\Time\\TimeOfDay", "Core\\Time\\TimeOfDay::at(0, 0)"],
  ["Core\\Time\\Duration", "Core\\Time\\Duration::nanoseconds(1)"],
  ["Core\\ObjectMap", "new Core\\ObjectMap<mixed, mixed>()"],
  ["Core\\ObjectSet", "new Core\\ObjectSet<mixed>()"],
  ["Core\\Heap", "new Core\\Heap<mixed>()"],
  ["Core\\Random\\Seeded", "new Core\\Random\\Seeded(1)"],
  ["Core\\Uuid", "Core\\Uuid::v4()"],
  ["Core\\Hash\\Stream", "Core\\Hash::stream(Core\\Hash::Digest::Crc32)"],
  ["Core\\Uri", "Core\\Uri::parse('https://example.com/')"],
  ["Core\\Crypto\\KeyPair", "Core\\Crypto::generateKeyPair(Core\\Crypto\\KeyKind::P256)"],
  ["Core\\Crypto\\PublicKey", "Core\\Crypto::generateKeyPair(Core\\Crypto\\KeyKind::P256)->publicKey()"],
  ["Core\\Xml\\Node", "Core\\Xml::parse('<a/>')"],
  ["Core\\Xml\\Reader", "Core\\Xml::reader('<a/>')"],
  ["Core\\Xml\\Writer", "Core\\Xml::writer()"],
  ["Core\\Compress\\Compressor", "Core\\Compress::compressor(Core\\Compress::Codec::Gzip)"],
  ["Core\\Compress\\Decompressor", "Core\\Compress::decompressor(Core\\Compress::Codec::Gzip)"],
  ["Core\\Reflect\\ClassInfo", "Core\\Reflect::forClass('Probe') ?? throw new LogicError('no class')"],
  ["Core\\Reflect\\EnumInfo", "Core\\Reflect\\EnumInfo::of('Core\\Arr::Order') ?? throw new LogicError('no enum')"],
  ["Core\\Ast\\Node", "Core\\Ast::parse('<?nvs echo 1;')"],
  ["Core\\Html\\Markup", "Core\\Html::escape('a')"],
  ["Core\\Jwe\\Key", "Core\\Jwe\\Key::shared(Core\\Bytes::fill(32, 1) as secret bytes)"],
  ["Core\\Charset", "Core\\Charset::Utf8"],
];

/** The classes with instance members that `PROBE_FACTORIES` has no factory for, and why. */
export const PROBE_NO_FACTORY: ReadonlyArray<readonly [string, string]> = [
  ["Core\\Router\\Match", "is made only from the program's routes"],
  ["Core\\Test", "is made only by `Core\\Test` while a test runs"],
  ["Core\\Task\\Channel", "waits for another task when it is full"],
  ["Core\\Passkey\\Credential", "needs a registered passkey"],
  ["Core\\Request\\Mount", "needs a served request"],
  ["Core\\Reflect\\MethodInfo", "is made only from a `Core\\Reflect\\ClassInfo`"],
  ["Core\\Reflect\\PropertyInfo", "is made only from a `Core\\Reflect\\ClassInfo`"],
  ["Core\\Reflect\\ParameterInfo", "is made only from a `Core\\Reflect\\MethodInfo`"],
  ["Core\\Reflect\\ConstantInfo", "is made only from a `Core\\Reflect\\ClassInfo`"],
  ["Core\\Reflect\\AttributeInfo", "is made only from a `Core\\Reflect\\ClassInfo`"],
];

const FACTORIES = new Map(PROBE_FACTORIES);

/** Why the probe makes no receiver for an instance member of `cls`, or null when `factories` has one. */
export function noFactoryReason(cls: string, factories: ReadonlyMap<string, string> = FACTORIES): string | null {
  if (factories.has(cls)) return null;
  for (const [name, why] of PROBE_NO_FACTORY) if (name === cls || cls.startsWith(`${name}\\`)) return `no factory: the receiver ${why}`;
  return "no factory: `PROBE_FACTORIES` has none for this class";
}

/** Why the probe skips this member, or null when it calls it. */
export function skipReason(cls: string, member: string): string | null {
  for (const [name, why] of PROBE_SKIPS) {
    if (name === `${cls}::${member}` || name === cls || cls.startsWith(`${name}\\`)) return why;
  }
  return null;
}

/** One parameter as `nvs meta --json` publishes it. */
export interface MetaParam {
  name: string;
  type: string;
  default?: string;
  variadic?: boolean;
}

/** One member as `nvs meta --json` publishes it. */
export interface MetaMember {
  kind: string;
  name: string;
  params?: MetaParam[];
  returns?: string;
  /** `decodeAs<T>(string $json): T`: a member that takes type arguments names them here. */
  signature?: string;
}

/** The type argument a member that takes one gets: a list of a one-property shape, which every member
 * that builds a value from data accepts. */
const TYPE_ARGUMENT = "array<{ a: string }>";

/** The part of `nvs meta --json` the probe reads. */
export interface ProbeMeta {
  classes: { name: string; members: MetaMember[] }[];
  enums?: { name: string; doc?: { cases?: { name: string }[] } }[];
}

/** How one parameter's argument is written at one step. `decl` is its variable's type, `setup` the
 * lines that make it, and `expr` what the call passes. */
interface Arg {
  setup: string[];
  expr: string;
}

/** A parameter the probe can build: what kind of value it takes. */
type Plan =
  | { kind: "sized"; decl: string; base: "string" | "bytes" | "mixed" | "list"; elem?: "string" | "bytes" | "mixed"; qual: string; doubles: boolean }
  | { kind: "number"; decl: string; num: "int" | "uint" | "float" }
  | { kind: "fixed"; expr: string }
  | { kind: "callback"; expr: string };

const INT_MAX = "9223372036854775807";
const UINT_MAX = "18446744073709551615";
const FLOAT_MAX = "1.7976931348623157e308";

/** The labels of the four values a number parameter takes, in the order `numberValue` gives them. */
export const NUMBER_VALUES = ["0", "1", "size", "max"] as const;

function numberValue(num: "int" | "uint" | "float", which: (typeof NUMBER_VALUES)[number], size: number): string {
  const max = num === "int" ? INT_MAX : num === "uint" ? UINT_MAX : FLOAT_MAX;
  const v = which === "size" ? String(size) : which === "max" ? max : which;
  return num === "float" && !v.includes(".") && !v.includes("e") ? `${v}.0` : v;
}

/** Splits `t` at its top-level `sep`, outside `<>`, `()` and `{}`. */
function splitTop(t: string, sep: string): string[] {
  const parts: string[] = [];
  let depth = 0;
  let start = 0;
  for (let i = 0; i < t.length; i++) {
    const c = t[i]!;
    if (c === "<" || c === "(" || c === "{") depth++;
    else if (c === ">" || c === ")" || c === "}") depth--;
    else if (depth === 0 && t.startsWith(sep, i)) {
      parts.push(t.slice(start, i).trim());
      start = i + sep.length;
    }
  }
  parts.push(t.slice(start).trim());
  return parts;
}

/** `t` with each generic letter (`T`, `U`, `K`, `V`) written as `mixed`, which binds every one of them. */
const ungeneric = (t: string) => t.replace(/(?<![\\\w])[TUKV](?![\w\\])/g, "mixed");

/** The qualifiers and the base of one union arm: `secret tainted string` is `secret tainted ` and
 * `string`. */
function qualified(arm: string): [string, string] {
  const m = /^((?:(?:secret|tainted)\s+)*)(.*)$/.exec(arm.replace(/^\?/, ""))!;
  return [m[1]!, m[2]!.trim()];
}

/** How the probe builds a value of type `type`, or why it cannot. */
export function planParam(type: string, enums: Map<string, string>, factories: ReadonlyMap<string, string> = FACTORIES): Plan | string {
  if (/^string<|propertyPath<|^method$/.test(type)) return `cannot build a \`${type}\` argument`;
  const decl = ungeneric(type);
  const callable = /^\??callable(\((.*)\)\s*:\s*(.+))?$/s.exec(decl);
  if (callable) return { kind: "callback", expr: callback(callable[2], callable[3]) };
  const arms = splitTop(decl.replace(/^\?/, ""), "|");
  let best: Plan | null = null;
  for (const arm of arms) {
    const [qual, base] = qualified(arm);
    let plan: Plan | null = null;
    if (base === "string" || base === "bytes" || base === "mixed") {
      plan = { kind: "sized", decl, base, qual, doubles: base === "mixed" };
    } else if (base === "int" || base === "uint" || base === "float") {
      plan = arms.length === 1 && !qual ? { kind: "number", decl, num: base } : { kind: "fixed", expr: base === "float" ? "0.0" : "0" };
    } else if (base === "bool") plan = { kind: "fixed", expr: "false" };
    else if (base === "decimal") plan = { kind: "fixed", expr: "(0 as decimal)" };
    else if (enums.has(base)) plan = { kind: "fixed", expr: `${base}::${enums.get(base)}` };
    else if (enums.has(base.replace(/::\w+$/, ""))) plan = { kind: "fixed", expr: base };
    else if (factories.has(base)) plan = { kind: "fixed", expr: factories.get(base)! };
    else if (/^array<(.*)>$/s.test(base)) {
      const inner = /^array<(.*)>$/s.exec(base)![1]!;
      const elem = splitTop(inner, "|")
        .map(qualified)
        .find(([, b]) => b === "string" || b === "bytes" || b === "mixed");
      if (elem) {
        const [eq, eb] = elem;
        plan = { kind: "sized", decl, base: "list", elem: eb as "string" | "bytes" | "mixed", qual: eq, doubles: eb === "mixed" };
      }
    }
    if (plan === null) continue;
    // A sized arm is the one worth probing; a fixed arm is kept only when no arm can take a size.
    if (plan.kind === "sized" || plan.kind === "number") return plan;
    best ??= plan;
  }
  return best ?? `cannot build a \`${type}\` argument`;
}

/** The function a callback parameter gets: it returns its first argument when the return type takes
 * it, and the return type's smallest value otherwise. */
function callback(params: string | undefined, ret: string | undefined): string {
  if (params === undefined || ret === undefined) return "fn() => null";
  const names = params.trim() === "" ? [] : splitTop(params, ",").map((_, i) => `$a${i}`);
  const first = params.trim() === "" ? null : splitTop(params, ",")[0]!;
  const r = ret.trim();
  if (r === "void") return `fn(${names.join(", ")}): void => { }`;
  let body: string;
  if (first !== null && (r === "mixed" || r === first)) body = "$a0";
  else if (r.startsWith("?") || r === "mixed") body = "null";
  else if (r === "bool") body = "false";
  else if (/^(int|uint)(\|.*)?$/.test(r) || r === "int|string") body = "0";
  else if (r === "float") body = "0.0";
  else if (r === "string") body = "''";
  else body = "null";
  return `fn(${names.join(", ")}) => ${body}`;
}

/** One step of a probe program: the shape, the size and the number label its call takes. */
export interface Step {
  /** Where the step stands in its program, from 0. */
  index: number;
  shape: Shape;
  size: number;
  value: string;
}

/** A member the probe can call: the steps it takes, and the static method of each, in step order. */
export interface Probe {
  member: string;
  steps: Step[];
  methods: string[];
}

/** A member the probe does not call, and why. */
export interface Skip {
  member: string;
  reason: string;
}

/** The value of a sized parameter at one shape and size: its setup lines, and the expression passed. */
function sizedArg(p: Extract<Plan, { kind: "sized" }>, i: number, shape: Shape, size: number): Arg {
  const v = `$p${i}`;
  const cast = (expr: string, base: "string" | "bytes" | "mixed", qual: string) =>
    base === "bytes" ? `${expr} as bytes${qual ? ` as ${qual}bytes` : ""}` : qual ? `${expr} as ${qual}string` : expr;
  const flatOf = (base: "string" | "bytes" | "mixed", qual: string, n: number) =>
    base === "bytes" ? `Core\\Bytes::fill(${n}, 97)${qual ? ` as ${qual}bytes` : ""}` : cast(`Core\\Str::repeat('a', ${n})`, base, qual);
  if (p.base !== "list") {
    if (shape === "flat" || p.base !== "mixed") return { setup: [`${p.decl} ${v} = ${flatOf(p.base, p.qual, size)};`], expr: v };
    if (shape === "shared") return { setup: sharedList(`array<mixed>`, `${v}l`, "mixed", "", size), expr: `${v}l` };
    return { setup: doubling(v, size, false), expr: v };
  }
  const elem = p.elem!;
  if (shape === "flat") return { setup: [`${p.decl} ${v} = [${flatOf(elem, p.qual, size)}];`], expr: v };
  if (shape === "shared") return { setup: sharedList(p.decl, v, elem, p.qual, size), expr: v };
  return { setup: doubling(v, size, true), expr: v };

  function sharedList(decl: string, name: string, e: "string" | "bytes" | "mixed", qual: string, n: number): string[] {
    return [
      `${decl} ${name} = [];`,
      `${e === "bytes" ? `${qual}bytes` : e === "mixed" ? "mixed" : `${qual}string`} ${name}r = ${flatOf(e, qual, PROBE_ROW)};`,
      `uint ${name}n = 0;`,
      `while (${name}n < ${Math.max(1, Math.floor(n / PROBE_ROW))}) { ${name}[] = ${name}r; ${name}n = ${name}n + 1; }`,
    ];
  }
}

/** A graph that doubles until a copy of it is `size` bytes: `[$g, $g]`, `levels` deep, over a short leaf.
 * `asList` writes it as an `array<mixed>`, the last doubling's list. */
function doubling(v: string, size: number, asList: boolean): string[] {
  const levels = Math.max(1, Math.ceil(Math.log2(size / LEAF_BYTES)));
  return [
    `mixed ${v}g = '${"x".repeat(LEAF_BYTES)}';`,
    `uint ${v}n = 0;`,
    `while (${v}n < ${levels - 1}) { ${v}g = [${v}g, ${v}g]; ${v}n = ${v}n + 1; }`,
    asList ? `array<mixed> ${v} = [${v}g, ${v}g];` : `mixed ${v} = [${v}g, ${v}g];`,
  ];
}

/** The probe program of one member, or why the probe cannot build it. An instance member's receiver is
 * `factories`' value of its class, made fresh at the start of each step. */
export function probeProgram(cls: string, m: MetaMember, enums: Map<string, string>, factories: ReadonlyMap<string, string> = FACTORIES): Probe | string {
  const member = `${cls}::${m.name}`;
  const instance = m.kind === "instance";
  if (instance && !factories.has(cls)) return noFactoryReason(cls, factories)!;
  const generics = /^\w+<([^>(]*)>\(/.exec(m.signature ?? "");
  const arity = generics ? generics[1]!.split(",").length : 0;
  if (arity > 1) return `needs ${arity} type arguments`;
  const name = instance ? `$r->${m.name}` : member;
  const callee = arity === 1 ? `${name}<${TYPE_ARGUMENT}>` : name;
  const receiver = instance ? [`var $r = ${factories.get(cls)!};`] : [];
  const plans: Plan[] = [];
  for (const p of m.params ?? []) {
    const plan = planParam(p.type, enums, factories);
    if (typeof plan === "string") {
      // An optional parameter the probe cannot build is left out, with every one after it.
      if (p.default !== undefined) break;
      return plan;
    }
    plans.push(plan);
  }
  const sized = plans.filter((p): p is Extract<Plan, { kind: "sized" }> => p.kind === "sized");
  const shapes = PROBE_SHAPES.filter((s) => s === "flat" || (s === "shared" ? sized.some((p) => p.base === "list" || p.base === "mixed") : sized.some((p) => p.doubles)));
  const values = plans.some((p) => p.kind === "number") ? NUMBER_VALUES : (["-"] as const);
  const steps: Step[] = [];
  const methods: string[] = [];
  for (const size of PROBE_SIZES) {
    for (const shape of shapes) {
      for (const value of values) {
        const step = { index: steps.length, shape, size, value };
        const setup: string[] = [...receiver];
        const exprs = plans.map((p, i) => {
          if (p.kind === "sized") {
            const a = sizedArg(p, i, shape, size);
            setup.push(...a.setup);
            return a.expr;
          }
          if (p.kind === "number") return numberValue(p.num, value as (typeof NUMBER_VALUES)[number], size);
          return p.expr;
        });
        methods.push(
          [
            `    public static function step${steps.length}(): void {`,
            ...setup.map((l) => `        ${l}`),
            `        echo 'probe: ${steps.length} ${shape} ${size} ${value}', "\\n";`,
            "        try {",
            `            ${callee}(${exprs.join(", ")});`,
            "        } catch (Throwable $e) {",
            "        }",
            "    }",
          ].join("\n"),
        );
        steps.push(step);
      }
    }
  }
  return { member, steps, methods };
}

/** The program of `probe` from step `from` on. A run that a memory limit stopped within the bound runs
 * again from the step after the one it stopped at, so every step gets its own reading. */
export function probeSource(probe: Probe, from = 0): string {
  const ids = probe.steps.map((_, i) => i).filter((i) => i >= from);
  return [
    "<?nvs",
    `// probe: ${probe.member}`,
    `Core\\Config::set('limits.memory', '${PROBE_CEILING}');`,
    ...ids.map((i) => `Probe::step${i}();`),
    "echo 'probe: done', \"\\n\";",
    "",
    "final class Probe {",
    ids.map((i) => probe.methods[i]!).join("\n\n"),
    "}",
    "",
  ].join("\n");
}

/** The first case of every enum `nvs meta --json` lists with its cases, by the enum's name. */
function enumCases(meta: ProbeMeta): Map<string, string> {
  const out = new Map<string, string>();
  for (const e of meta.enums ?? []) {
    const first = e.doc?.cases?.[0]?.name;
    if (first) out.set(e.name, first);
  }
  return out;
}

/** Every `Core` member's program, static and instance, and every member the probe skips with its reason.
 * An instance member of a class with no factory is a skip. */
export function probePlan(meta: ProbeMeta, factories: ReadonlyMap<string, string> = FACTORIES): { probes: Probe[]; skips: Skip[]; unbuilt: Skip[]; other: number } {
  const enums = enumCases(meta);
  const probes: Probe[] = [];
  const skips: Skip[] = [];
  const unbuilt: Skip[] = [];
  let other = 0;
  for (const c of meta.classes) {
    if (!c.name.startsWith("Core\\")) {
      other++;
      continue;
    }
    for (const m of c.members) {
      if (m.kind !== "static" && m.kind !== "instance") continue;
      const member = `${c.name}::${m.name}`;
      const why = skipReason(c.name, m.name) ?? (m.kind === "instance" ? noFactoryReason(c.name, factories) : null);
      if (why !== null) {
        skips.push({ member, reason: why });
        continue;
      }
      const probe = probeProgram(c.name, m, enums, factories);
      if (typeof probe === "string") unbuilt.push({ member, reason: probe });
      else probes.push(probe);
    }
  }
  return { probes, skips, unbuilt, other };
}

/** One probed member's row: its reading and the step that reached it, or why it has none. */
export interface ProbeRow {
  member: string;
  ms: number;
  reading: MemoryReading | null;
  /** The step whose call was running when the run ended: its shape, size and number value. */
  step: Step | null;
  note?: string;
  red: boolean;
  /** Why it is red, naming the member, the shape and the size. */
  why?: string;
}

const STEP_RE = /^probe: ([0-9]+) (flat|shared|doubling) ([0-9]+) (\S+)\r?$/gm;

/** The step a run was in when it ended: the last `probe:` line it printed. */
export function lastStep(stdout: string): Step | null {
  let step: Step | null = null;
  for (const m of stdout.matchAll(STEP_RE)) step = { index: Number(m[1]), shape: m[2] as Shape, size: Number(m[3]), value: m[4]! };
  return step;
}

/** Whether a run printed the line a probe program ends on. */
const finished = (ran: Ran) => /^probe: done\r?$/m.test(ran.stdout);

/** The worse of two rows of one member: a red row first, then the higher reading. */
function worse(a: ProbeRow | null, b: ProbeRow): ProbeRow {
  if (a === null) return b;
  if (a.red !== b.red) return a.red ? a : b;
  const ratio = (r: ProbeRow) => (r.reading === null ? -1 : r.reading.ceiling === null ? Number.POSITIVE_INFINITY : r.reading.held / Math.max(1, r.reading.ceiling));
  const pick = ratio(b) > ratio(a) ? b : a;
  const note = pick.note ?? (pick === a ? b.note : a.note);
  return { ...pick, ms: a.ms + b.ms, ...(note === undefined ? {} : { note }) };
}

/** Runs one member's program, and runs it again from the step after the one a limit stopped it at, until
 * a run finishes every step, is red, does not compile or runs out of time. The row is the worst run's. */
async function probeMember(nvs: string, probe: Probe, dir: string): Promise<ProbeRow> {
  const name = `${probe.member.replace(/[\\:]+/g, "_")}.nvs`;
  const path = `${PROBE_DIR}/${name}`;
  let row: ProbeRow | null = null;
  let from = 0;
  while (from < probe.steps.length) {
    writeFileSync(join(dir, name), probeSource(probe, from));
    let ran: Ran;
    try {
      ran = await spawnProof([nvs, "run", path], path, PROBE_TIMEOUT_MS, { unlogged: true });
    } catch (e) {
      return worse(row, { member: probe.member, ms: 0, reading: null, step: null, note: `could not run: ${e instanceof Error ? e.message : String(e)}`, red: false });
    }
    const judged = judgeProbe(probe.member, ran);
    row = worse(row, judged);
    if (judged.red || judged.note || finished(ran) || judged.step === null) break;
    from = judged.step.index + 1;
  }
  return row!;
}

const mib = (n: number) => `${(n / 1048576).toFixed(n % 1048576 === 0 ? 0 : 1)} MiB`;

/** One probe run's row. A reading past the bound is red, and so is a run the host killed before it
 * printed one. A program that did not compile is not a reading: it is listed, and is not red. */
export function judgeProbe(member: string, ran: Ran): ProbeRow {
  const reading = memoryReading(ran);
  const step = lastStep(ran.stdout);
  const at = step === null ? "before its first step" : `on a ${step.shape} input of ${mib(step.size)}${step.value === "-" ? "" : ` with number ${step.value}`}`;
  const row: ProbeRow = { member, ms: Math.round(ran.ms), reading, step, red: false };
  const past = reading === null ? null : overshoot(reading);
  if (past !== null) return { ...row, red: true, why: `${member} ${at}: ${past}` };
  if (reading === null && hostKilled(ran, true)) {
    return { ...row, red: true, note: `killed by the host (${ran.signal ?? `exit ${ran.code}`})`, why: `${member} ${at}: the host killed it before its memory limit stopped it` };
  }
  const crash = ran.stderr.split(/\r?\n/).find((l) => /panicked at|internal error|stack overflow/.test(l));
  if (crash !== undefined) return { ...row, red: true, note: "the runtime crashed", why: `${member} ${at}: the runtime crashed: ${crash.trim().slice(0, 200)}` };
  if (ran.timedOut) return { ...row, note: `still running at its time limit, ${at}` };
  if (step === null && /^error\[/m.test(ran.stderr)) {
    const first = ran.stderr.split(/\r?\n/).find((l) => l.startsWith("error[")) ?? "";
    return { ...row, note: `did not compile: ${first.slice(0, 160)}` };
  }
  // Every call is inside a `try`, so an uncaught error came from a step's setup: a receiver's factory.
  const uncaught = /^\{"level":"error","msg":("(?:[^"\\]|\\.)*"),"fields":\{"class":"([^"]+)"/m.exec(ran.stderr);
  if (reading === null && !finished(ran) && uncaught !== null) {
    return { ...row, note: `its setup threw an uncaught ${uncaught[2]} ${at}: ${(JSON.parse(uncaught[1]!) as string).slice(0, 160)}` };
  }
  return row;
}

/** The report lines of a probe run, worst first, then the members it could not build and the skips. */
export function probeReport(rows: ProbeRow[], skips: Skip[], unbuilt: Skip[], other: number, ms: number): { lines: string[]; red: number } {
  const lines: string[] = [];
  const ratio = (r: MemoryReading) => (r.ceiling === null ? Number.POSITIVE_INFINITY : r.held / Math.max(1, r.ceiling));
  const measured = rows.filter((r) => r.reading !== null).sort((a, b) => ratio(b.reading!) - ratio(a.reading!) || a.member.localeCompare(b.member));
  lines.push(`nv proofs probe: ${measured.length} member(s) stopped at a memory limit, worst first (the bound is ${OVERSHOOT_FACTOR}x the ceiling plus ${mib(OVERSHOOT_SLACK_BYTES)}):`);
  for (const r of measured) {
    const { held, ceiling } = r.reading!;
    const figure = ceiling === null ? `server full at ${mib(held)}` : `${ratio(r.reading!).toFixed(2)}x  ${mib(held)} / ${mib(ceiling)}`;
    const at = r.step ? `  ${r.step.shape} ${mib(r.step.size)}${r.step.value === "-" ? "" : ` ${r.step.value}`}` : "";
    lines.push(`  ${r.red ? "FAIL" : "ok  "}  ${figure}  ${r.member}${at}  ${r.ms} ms`);
  }
  for (const r of rows.filter((r) => r.reading === null && r.note).sort((a, b) => a.member.localeCompare(b.member))) {
    lines.push(`  ${r.red ? "FAIL" : "-   "}  ${r.member}: ${r.note}`);
  }
  for (const r of rows.filter((r) => r.red)) lines.push(`  red: ${r.why}`);
  if (unbuilt.length) {
    lines.push(`nv proofs probe: ${unbuilt.length} member(s) not built:`);
    for (const u of unbuilt) lines.push(`  -     ${u.member}: ${u.reason}`);
  }
  if (skips.length) {
    lines.push(`nv proofs probe: ${skips.length} member(s) skipped:`);
    for (const s of skips) lines.push(`  skip  ${s.member}: ${s.reason}`);
  }
  const red = rows.filter((r) => r.red).length;
  const finished = rows.filter((r) => r.reading === null && !r.note).length;
  lines.push(
    `nv proofs probe: ${red} red of ${rows.length} run in ${Math.round(ms / 1000)} s; ${finished} finished every step, ` +
      `${unbuilt.length} not built, ${skips.length} skipped, and ${other} component class(es) not probed.`,
  );
  return { lines, red };
}

/** `bun nv proofs --probe`: writes, runs and judges every probe program on `nvs`, `PROBE_JOBS` at a time.
 * `only` narrows the run to the members whose name starts with it. */
export async function probeScan(nvs: string, out: string[], only: string[] = []): Promise<number> {
  const started = performance.now();
  const plan = probePlan((await metaJson(nvs)) as unknown as ProbeMeta);
  const keep = (member: string) => only.length === 0 || only.some((o) => member === o || member.startsWith(`${o}::`) || member.startsWith(`${o}\\`));
  const probes = plan.probes.filter((p) => keep(p.member));
  const dir = abs(PROBE_DIR);
  rmSync(dir, { recursive: true, force: true });
  mkdirSync(dir, { recursive: true });
  progress(`proofs: probing ${probes.length} member(s), ${PROBE_JOBS} at a time`);
  const rows: ProbeRow[] = [];
  let next = 0;
  const worker = async () => {
    while (next < probes.length) {
      const probe = probes[next++]!;
      rows.push(await probeMember(nvs, probe, dir));
      progress(`proofs: ${rows.length}/${probes.length}`);
    }
  };
  try {
    await Promise.all(Array.from({ length: PROBE_JOBS }, worker));
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
  const report = probeReport(
    rows,
    plan.skips.filter((s) => keep(s.member)),
    plan.unbuilt.filter((s) => keep(s.member)),
    plan.other,
    performance.now() - started,
  );
  out.push(...report.lines);
  return report.red ? 1 : 0;
}
