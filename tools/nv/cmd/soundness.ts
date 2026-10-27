// `bun nv soundness`: the type-soundness matrix, written as conformance cases. A subclass or an implementor
// declares a member again over its parent's; the rule for that member kind says which type relations
// between the two declarations compile, and each way a program reaches the member then says which
// declaration runs and what it reads.
//
//     bun nv soundness --write                 write tests/conformance/soundness/ from the table
//     bun nv soundness --check                 exit 1 when a written file differs from what --write writes
//     bun nv soundness --probe                 run every cell alone through `nvs`; print each that fails
//     bun nv soundness --help                  this usage
//
// **A cell** is one member kind, one relation and one path, and its id is `<member>/<relation>/<path>`.
// Its verdict is the rule's, never what the binary prints today. A relation the rule refuses is a
// compile error at the redeclaration, whatever reaches it later, so it is one cell whose path is
// `declaration`. A relation the rule accepts is one cell per path, and each path's verdict is either the
// output the oracle derives from the rule (which declaration ran, and the run-time type of what it read)
// or the error code the rule gives that path.
//
// **The table** is `MEMBERS` below: one row per member kind, citing the rule that gives its verdicts, with
// the verdict of every relation that is a cell. The sets, explicitly:
//
// - members: `method-param`, `method-return`, `interface-method-param`, `interface-method-return`,
//   `static-method-param`, `static-method-return`, `compare-to-param`, `covariant-method-param`,
//   `property-plain`, `property-promoted`, `property-lateinit`, `property-hooked`, `property-array`,
//   `property-covariant`, `property-readonly`, `static-property`, `class-constant` and
//   `interface-constant`.
// - relations, child's declared type over the parent's (`PAIRS`): `same` (`Animal` over `Animal`),
//   `subclass` (`Dog` over `Animal`), `superclass` (`Animal` over `Dog`), `unrelated` (`Plant` over
//   `Animal`), `int-under-int-string` (`int` over `int|string`), `int-over-float` (`int` over `float`),
//   `float-over-int` (`float` over `int`), `t-under-nullable` (`Animal` over `?Animal`),
//   `nullable-over-t` (`?Animal` over `Animal`), `member-under-union` (`Animal` over `Animal|Plant`) and
//   `wider-union` (`Animal|Plant|Stone` over `Animal|Plant`).
// - paths, per member kind (`PATHS`): a variable of the parent's type, of the child's type, of an
//   interface's type, `static::`, a class name, `self::`, an erased `object` or `mixed` access, a
//   `property<T>` key, a first-class callable and a union of classes, each where the member kind has it.
//
// A member kind writes some relations with other types, and leaves some out:
//
// - `property-array` wraps both sides in `array<T>`, so `same` is `array<Animal>` over `array<Animal>`.
// - `static-property` is initialized by a constant expression, and an object is not one. Its class
//   relations are written nullable (`?Dog` over `?Animal`), and the program assigns the objects. Its `same`,
//   `t-under-nullable` and `nullable-over-t` are written with `int`, so the two classes' values differ.
// - `class-constant` and `interface-constant` hold a folded scalar, so `subclass` and `superclass` are not
//   cells. `unrelated` is `string` over `int`, `member-under-union` is `string` over `int|string`,
//   `t-under-nullable` is `int` over `?int` and `wider-union` is `int|string|bool` over `int|string`.
// - `compare-to-param`'s parent declares `compareTo(self $other)`, a class that is neither a scalar,
//   nullable nor a union, so only `same` (the parent's name), `subclass` (`self`, which is the child),
//   `superclass` (the parent's own superclass), `unrelated` and `nullable-over-t` are cells.
// - `covariant-method-param` and `property-covariant` write `covariant` in the child, so a relation compiles
//   only where the child's type is narrower. Each path but the child's own type passes a value the parent's
//   type takes and the child's does not (`WIDER`), and prints the `LogicError` the test throws. The child's
//   type passes the child's value. The property's parent constructor stores the value the child passes it.
//
// And some paths are not cells:
//
// - A method has no `object` path: a call through `object` is `E0477` whatever the declarations are
//   (`rule:types/erased-member-access`), so it says nothing about a redeclaration. `mixed` is its erased
//   path. A class's own method has no interface path, because no interface declares it.
// - A static method's paths are `static::`, both class names, a variable of the parent's type that holds
//   the child, and a callable taken with `static::m`. A static method binds late to the called
//   class (`rule:statements/static-is-a-member-modifier`), and through an instance that is the
//   object's class, so `$parent->m()` runs the child's body.
// - A static property has no `property<T>` path, because a key names instance properties only
//   (`rule:types/property-key`). An interface declares no property, so no property has an interface
//   path (`rule:classes/interfaces-declare-no-state`).
// - A constant is read through a class name, `self::` or `static::`, never through a variable.
// - `property-readonly` has no relation that compiles, so it has no path.
// - An instance property has two union paths, a variable of `Parent<n>|Other<n>` and one of
//   `Other<n>|Parent<n>`, where `Other<n>` declares the property with the parent's type. A read through
//   a union tests the class of every arm but the last and reads the slot at a fixed offset
//   (`rule:types/a-member-every-class-of-a-union-declares`), so the child is reached once through the
//   parent's tested arm and once through the untested last one. A method has no union path: a call
//   through the union is the call through the parent's class, which `parent-variable` already covers.
//
// **The files.** `--write` writes `tests/conformance/soundness/<member>.nvst`, the cells that run, and
// `<member>.errors.nvst`, the cells that are compile errors, since one error stops a whole program. Each
// relation's classes are `Parent<n>`, `Child<n>`, `Face<n>`, `Root<n>` and `Other<n>`, numbered by the relation's
// place in `RELATIONS`, so a cell left out renames nothing else. Every error the second file expects is
// listed in line order, each with the line its diagnostic points at. Output is deterministic.
//
// **No cell is skipped.** Every cell is in the written files, so a cell that fails is a red `.nvst` run,
// never a skip and never a gap record. `--probe` runs each cell alone through `nvs`: `NVS_BIN`, else the
// `covws` build's (`lib/covws.ts`). `bun nv verify` runs `--check`.
//
// Exits 0 when everything holds, 1 on a finding, and 2 on a bad argument.

import { existsSync, mkdirSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { cpus } from "node:os";
import { join } from "node:path";
import { covwsNvs } from "../lib/covws.ts";
import { ROOT } from "../lib/paths.ts";
import { run as proc } from "../lib/proc.ts";

export const summary = "the type-soundness matrix as conformance cases: nv soundness [--write | --check | --probe]";

const USAGE = `usage: bun nv soundness [--write | --check | --probe | --help]

  --write                write tests/conformance/soundness/ from the table in tools/nv/cmd/soundness.ts
  --check                exit 1 when a written file differs from what --write writes
  --probe                run every cell alone through nvs, and print each one that fails`;

/** Where the cases go, relative to the root. */
const OUT = "tests/conformance/soundness";
/** Where a cell is written to run alone. */
const SCRATCH = ".agent-tmp/soundness";

// ---- the table -------------------------------------------------------------------------------------

const RELATIONS = [
  "same",
  "subclass",
  "superclass",
  "unrelated",
  "int-under-int-string",
  "int-over-float",
  "float-over-int",
  "t-under-nullable",
  "nullable-over-t",
  "member-under-union",
  "wider-union",
] as const;
type Rel = (typeof RELATIONS)[number];

/** Each relation's declared types: the child's, then the parent's. */
const PAIRS: Record<Rel, [string, string]> = {
  same: ["Animal", "Animal"],
  subclass: ["Dog", "Animal"],
  superclass: ["Animal", "Dog"],
  unrelated: ["Plant", "Animal"],
  "int-under-int-string": ["int", "int|string"],
  "int-over-float": ["int", "float"],
  "float-over-int": ["float", "int"],
  "t-under-nullable": ["Animal", "?Animal"],
  "nullable-over-t": ["?Animal", "Animal"],
  "member-under-union": ["Animal", "Animal|Plant"],
  "wider-union": ["Animal|Plant|Stone", "Animal|Plant"],
};

/** A path's verdict: `ok` runs and prints what the oracle derives; a code is the compile error. */
interface PathRow {
  id: string;
  verdict: string;
  rule: string;
}

type Family = "method" | "compare-to" | "property" | "static-property" | "constant";

interface Member {
  id: string;
  /** What the member is, for a case's title. */
  title: string;
  family: Family;
  rule: string;
  /** Every relation that is a cell, and its verdict: `ok` or the code the redeclaration is. */
  verdicts: Partial<Record<Rel, string>>;
  /** The relations this member kind writes with other types than `PAIRS`. */
  pairs?: Partial<Record<Rel, [string, string]>>;
  paths: PathRow[];
  /** The child writes `covariant`, so a path through a wider type passes `WIDER`'s value and reads the throw. */
  covariant?: boolean;
}

const OVERRIDE = "rule:classes/an-override-keeps-its-parents-signature";
const PROPERTY = "rule:classes/a-redeclared-property-keeps-its-parents-type";
const STATIC = "rule:statements/static-is-a-member-modifier";
const CALLABLE = "rule:types/callable-values";
const ERASED = "rule:types/erased-member-access";
const UNION = "rule:types/a-member-every-class-of-a-union-declares";
const COVARIANT = "rule:classes/a-covariant-type-is-checked-where-a-value-enters";

/** A parameter is contravariant: the child's type accepts the parent's, without `int` to `float`. */
const PARAM: Partial<Record<Rel, string>> = {
  same: "ok",
  subclass: "E0404",
  superclass: "ok",
  unrelated: "E0404",
  "int-under-int-string": "E0404",
  "int-over-float": "E0404",
  "float-over-int": "E0404",
  "t-under-nullable": "E0404",
  "nullable-over-t": "ok",
  "member-under-union": "E0404",
  "wider-union": "ok",
};
/** A return is covariant: the child's type is assignable to the parent's, without `int` to `float`. */
const RETURN: Partial<Record<Rel, string>> = {
  same: "ok",
  subclass: "ok",
  superclass: "E0404",
  unrelated: "E0404",
  "int-under-int-string": "ok",
  "int-over-float": "E0404",
  "float-over-int": "E0404",
  "t-under-nullable": "ok",
  "nullable-over-t": "E0404",
  "member-under-union": "ok",
  "wider-union": "E0404",
};
/** An instance property keeps the exact type. */
const EXACT: Partial<Record<Rel, string>> = Object.fromEntries(RELATIONS.map((r) => [r, r === "same" ? "ok" : "E0866"]));
/**
 * A `covariant` type is narrower than the parent's, without `int` to `float`. One the parent's type fits is
 * `E0882`, and one unrelated to it is the member kind's own code.
 */
const covariantVerdicts = (unrelated: string): Partial<Record<Rel, string>> => ({
  same: "E0882",
  subclass: "ok",
  superclass: "E0882",
  unrelated,
  "int-under-int-string": "ok",
  "int-over-float": unrelated,
  "float-over-int": unrelated,
  "t-under-nullable": "ok",
  "nullable-over-t": "E0882",
  "member-under-union": "ok",
  "wider-union": "E0882",
});
/** For each relation a `covariant` row compiles, a value the parent's type takes and the child's does not. */
const WIDER: Partial<Record<Rel, string>> = {
  subclass: "new Animal()",
  "int-under-int-string": '"p"',
  "t-under-nullable": "null",
  "member-under-union": "new Plant()",
};

const INSTANCE_PATHS: PathRow[] = [
  { id: "parent-variable", verdict: "ok", rule: OVERRIDE },
  { id: "child-variable", verdict: "ok", rule: OVERRIDE },
  { id: "mixed", verdict: "ok", rule: ERASED },
  { id: "callable", verdict: "ok", rule: CALLABLE },
];
const INTERFACE_PATHS: PathRow[] = [
  { id: "interface-variable", verdict: "ok", rule: OVERRIDE },
  { id: "class-variable", verdict: "ok", rule: OVERRIDE },
  { id: "mixed", verdict: "ok", rule: ERASED },
  { id: "callable", verdict: "ok", rule: CALLABLE },
];
const STATIC_METHOD_PATHS: PathRow[] = [
  { id: "static", verdict: "ok", rule: STATIC },
  { id: "child-name", verdict: "ok", rule: STATIC },
  { id: "parent-name", verdict: "ok", rule: STATIC },
  { id: "parent-variable", verdict: "ok", rule: STATIC },
  { id: "callable", verdict: "ok", rule: CALLABLE },
];
const PROPERTY_PATHS: PathRow[] = [
  { id: "parent-variable", verdict: "ok", rule: PROPERTY },
  { id: "child-variable", verdict: "ok", rule: PROPERTY },
  { id: "object", verdict: "ok", rule: ERASED },
  { id: "mixed", verdict: "ok", rule: ERASED },
  { id: "property-key", verdict: "ok", rule: "rule:types/property-key-access" },
  { id: "union-tested", verdict: "ok", rule: UNION },
  { id: "union-fallthrough", verdict: "ok", rule: UNION },
];

const STATIC_PAIRS: Partial<Record<Rel, [string, string]>> = {
  same: ["int", "int"],
  subclass: ["?Dog", "?Animal"],
  superclass: ["?Animal", "?Dog"],
  unrelated: ["?Plant", "?Animal"],
  "t-under-nullable": ["int", "?int"],
  "nullable-over-t": ["?int", "int"],
  "member-under-union": ["?Animal", "Animal|Plant|null"],
  "wider-union": ["Animal|Plant|Stone|null", "Animal|Plant|null"],
};
const CONSTANT_PAIRS: Partial<Record<Rel, [string, string]>> = {
  same: ["int", "int"],
  unrelated: ["string", "int"],
  "t-under-nullable": ["int", "?int"],
  "nullable-over-t": ["?int", "int"],
  "member-under-union": ["string", "int|string"],
  "wider-union": ["int|string|bool", "int|string"],
};
/** A constant keeps a type assignable to the ancestor's, without `int` to `float` (`E0833`). */
const CONSTANT: Partial<Record<Rel, string>> = {
  same: "ok",
  unrelated: "E0833",
  "int-under-int-string": "ok",
  "int-over-float": "E0833",
  "float-over-int": "E0833",
  "t-under-nullable": "ok",
  "nullable-over-t": "E0833",
  "member-under-union": "ok",
  "wider-union": "E0833",
};

export const MEMBERS: Member[] = [
  { id: "method-param", title: "a method's parameter", family: "method", rule: OVERRIDE, verdicts: PARAM, paths: INSTANCE_PATHS },
  { id: "method-return", title: "a method's return type", family: "method", rule: OVERRIDE, verdicts: RETURN, paths: INSTANCE_PATHS },
  { id: "interface-method-param", title: "an interface method's parameter", family: "method", rule: OVERRIDE, verdicts: PARAM, paths: INTERFACE_PATHS },
  { id: "interface-method-return", title: "an interface method's return type", family: "method", rule: OVERRIDE, verdicts: RETURN, paths: INTERFACE_PATHS },
  { id: "static-method-param", title: "a static method's parameter", family: "method", rule: OVERRIDE, verdicts: PARAM, paths: STATIC_METHOD_PATHS },
  { id: "static-method-return", title: "a static method's return type", family: "method", rule: OVERRIDE, verdicts: RETURN, paths: STATIC_METHOD_PATHS },
  {
    id: "compare-to-param",
    title: "`compareTo`'s parameter",
    family: "compare-to",
    rule: OVERRIDE,
    verdicts: { same: "ok", subclass: "E0404", superclass: "ok", unrelated: "E0404", "nullable-over-t": "ok" },
    paths: [
      { id: "parent-variable", verdict: "ok", rule: OVERRIDE },
      { id: "child-variable", verdict: "ok", rule: OVERRIDE },
      { id: "interface-variable", verdict: "ok", rule: "rule:classes/comparable-is-same-class-only" },
      { id: "mixed", verdict: "ok", rule: ERASED },
      { id: "callable", verdict: "ok", rule: CALLABLE },
    ],
  },
  {
    id: "covariant-method-param",
    title: "a `covariant` method parameter",
    family: "method",
    rule: COVARIANT,
    verdicts: covariantVerdicts("E0404"),
    paths: INSTANCE_PATHS.map((p) => ({ ...p, rule: COVARIANT })),
    covariant: true,
  },
  { id: "property-plain", title: "a plain instance property", family: "property", rule: PROPERTY, verdicts: EXACT, paths: PROPERTY_PATHS },
  { id: "property-promoted", title: "a promoted constructor parameter", family: "property", rule: PROPERTY, verdicts: EXACT, paths: PROPERTY_PATHS },
  { id: "property-lateinit", title: "a `lateinit` property", family: "property", rule: PROPERTY, verdicts: EXACT, paths: PROPERTY_PATHS },
  { id: "property-hooked", title: "a hooked property", family: "property", rule: PROPERTY, verdicts: EXACT, paths: PROPERTY_PATHS },
  {
    id: "property-array",
    title: "an `array<T>` property",
    family: "property",
    rule: PROPERTY,
    verdicts: EXACT,
    pairs: Object.fromEntries(RELATIONS.map((r) => [r, [`array<${PAIRS[r][0]}>`, `array<${PAIRS[r][1]}>`]])),
    paths: PROPERTY_PATHS,
  },
  {
    id: "property-covariant",
    title: "a `covariant` instance property",
    family: "property",
    rule: COVARIANT,
    verdicts: covariantVerdicts("E0866"),
    paths: PROPERTY_PATHS.map((p) => ({ ...p, rule: COVARIANT })),
    covariant: true,
  },
  {
    id: "property-readonly",
    title: "a `readonly` property",
    family: "property",
    rule: PROPERTY,
    verdicts: Object.fromEntries(RELATIONS.map((r) => [r, "E0866"])),
    paths: [],
  },
  {
    id: "static-property",
    title: "a static property",
    family: "static-property",
    rule: STATIC,
    verdicts: Object.fromEntries(RELATIONS.map((r) => [r, "ok"])),
    pairs: STATIC_PAIRS,
    paths: [
      { id: "parent-name", verdict: "ok", rule: STATIC },
      { id: "child-name", verdict: "ok", rule: STATIC },
      { id: "self-in-parent", verdict: "ok", rule: STATIC },
      { id: "self-in-child", verdict: "ok", rule: STATIC },
      { id: "object", verdict: "ok", rule: ERASED },
      { id: "mixed", verdict: "ok", rule: ERASED },
      { id: "static", verdict: "E0499", rule: STATIC },
      { id: "parent-variable", verdict: "E0869", rule: STATIC },
    ],
  },
  {
    id: "class-constant",
    title: "a class constant",
    family: "constant",
    rule: STATIC,
    verdicts: CONSTANT,
    pairs: CONSTANT_PAIRS,
    paths: [
      { id: "parent-name", verdict: "ok", rule: STATIC },
      { id: "child-name", verdict: "ok", rule: STATIC },
      { id: "self-in-parent", verdict: "ok", rule: STATIC },
      { id: "self-in-child", verdict: "ok", rule: STATIC },
      { id: "static", verdict: "ok", rule: STATIC },
    ],
  },
  {
    id: "interface-constant",
    title: "an interface constant",
    family: "constant",
    rule: STATIC,
    verdicts: CONSTANT,
    pairs: CONSTANT_PAIRS,
    paths: [
      { id: "interface-name", verdict: "ok", rule: STATIC },
      { id: "class-name", verdict: "ok", rule: STATIC },
      { id: "self-in-interface", verdict: "ok", rule: STATIC },
      { id: "self-in-class", verdict: "ok", rule: STATIC },
      { id: "static", verdict: "ok", rule: STATIC },
    ],
  },
];

// ---- values ----------------------------------------------------------------------------------------

/** The value each side stores or returns for a type: the parent's first, then the child's. */
const VALUES: Record<string, [string, string]> = {
  Animal: ["new Animal()", "new Dog()"],
  Dog: ["new Dog()", "new Puppy()"],
  Plant: ["new Plant()", "new Plant()"],
  "Animal|Plant": ["new Plant()", "new Plant()"],
  "Animal|Plant|Stone": ["new Stone()", "new Stone()"],
  int: ["1", "2"],
  float: ["1.5", "2.5"],
  string: ['"p"', '"c"'],
  "int|string": ['"p"', '"c"'],
  "int|string|bool": ["true", "true"],
};

function value(type: string, side: 0 | 1): string {
  const array = /^array<(.*)>$/.exec(type);
  if (array) return `[${value(array[1]!, side)}]`;
  const bare = type.replace(/^\?/, "").replace(/\|null$/, "");
  const v = VALUES[bare];
  if (!v) throw new Error(`soundness: no value for type ${type}`);
  return v[side];
}

/** Whether a value is a constant expression, so a static property's declaration can hold it. */
const constant = (v: string) => !v.includes("new ");

/** What `Show::of` prints for a value expression. */
function describe(e: string): string {
  let m: RegExpExecArray | null;
  if ((m = /^new (\w+)\(\)$/.exec(e))) return m[1]!;
  if (/^-?\d+$/.test(e)) return `int ${e}`;
  if (/^-?\d+\.\d+$/.test(e)) return `float ${e}`;
  if ((m = /^"(.*)"$/.exec(e))) return `string ${m[1]}`;
  if (e === "true" || e === "false") return `bool ${e}`;
  if ((m = /^\[(.*)\]$/.exec(e))) return `[${describe(m[1]!)}]`;
  throw new Error(`soundness: cannot describe ${e}`);
}

/** What a `covariant` test's `LogicError` says the value is, for a value expression. */
function describeArgument(e: string): string {
  const m = /^new (\w+)\(\)$/.exec(e);
  if (m) return `an object of class \`${m[1]}\``;
  if (e === "null") return "`null`";
  return `a value of type \`${describe(e).split(" ")[0]}\``;
}

/** Statements that run `stmt` and print the message of the `LogicError` it throws, after the label. */
function printsLogicError(at: string, stmt: string, e: string): string {
  return `try { ${stmt} } catch (LogicError ${e}) { echo ${at}, ${e}->message, "\\n"; }`;
}

const PRELUDE = [
  "class Show {",
  "    public static function of(mixed $v): tainted string {",
  '        if ($v == null) { return "null"; }',
  '        if ($v is int) { return "int " . ($v as string); }',
  '        if ($v is float) { return "float " . ($v as string); }',
  '        if ($v is string) { return "string " . $v; }',
  '        if ($v is bool) { return $v ? "bool true" : "bool false"; }',
  "        if ($v is array<mixed>) {",
  '            tainted string $s = "[";',
  "            foreach ($v as mixed $e) { $s = $s . Show::of($e); }",
  '            return $s . "]";',
  "        }",
  "        if ($v is object) { return $v::class; }",
  '        return "unknown";',
  "    }",
  "}",
  "",
  "class Animal {}",
  "class Dog extends Animal {}",
  "class Puppy extends Dog {}",
  "class Plant {}",
  "class Stone {}",
];

// ---- the builders ----------------------------------------------------------------------------------

/** Where a diagnostic points: a line of the declaration, or of a path's statements. */
type Site = { in: "decl"; at: number } | { in: "use"; at: number };

interface Use {
  path: string;
  lines: string[];
  /** The line it prints, for a path that runs. */
  expect?: string;
  site?: Site;
}

interface Piece {
  decl: string[];
  /** The redeclaration's line, where a refused relation's diagnostic points. */
  declSite: number;
  setup: string[];
  uses: Use[];
}

interface Ctx {
  member: Member;
  rel: Rel;
  n: number;
  child: string;
  parent: string;
  /** The paths whose code this piece carries. */
  paths: string[];
}

const label = (c: Ctx, path: string) => `${c.rel} ${path}: `;
const q = (s: string) => JSON.stringify(s);

function methodPiece(c: Ctx): Piece {
  const kind = c.member.id.startsWith("interface-") ? "interface" : c.member.id.startsWith("static-") ? "static" : "instance";
  const param = c.member.id.endsWith("-param");
  const n = c.n;
  const parentName = kind === "interface" ? `Face${n}` : `Parent${n}`;
  const childName = `Child${n}`;
  const mod = kind === "static" ? "public static function" : "public function";
  const arg = value(c.parent, 0);
  const sig = (t: string, covariant = false) => (param ? `m(${covariant ? "covariant " : ""}${t} $a): void` : `m(): ${t}`);
  const body = (cls: string, t: string, side: 0 | 1) => (param ? `{ echo "${cls}.m ", Show::of($a), "\\n"; }` : `{ return ${value(t, side)}; }`);
  /** The statements one call makes: `at` is the label expression, `call` the receiver and member. */
  const stmt = (at: string, call: string): string =>
    param ? `echo ${at}; ${call}(${arg});` : `echo ${at}, Show::of(${call}()), "\\n";`;
  const ran = (cls: string, side: 0 | 1, t: string) => (param ? `${cls}.m ${describe(arg)}` : describe(value(t, side)));

  const decl: string[] = [];
  if (kind === "interface") {
    decl.push(`interface ${parentName} {`, `    public function ${sig(c.parent)};`, "}");
  } else {
    decl.push(`class ${parentName} {`, `    ${mod} ${sig(c.parent)} ${body(parentName, c.parent, 0)}`);
    if (kind === "static" && c.paths.includes("static")) decl.push(`    public static function viaStatic(string $at): void { ${stmt("$at", "static::m")} }`);
    if (kind === "static" && c.paths.includes("callable")) decl.push(`    public static function viaCallable(string $at): void { var $f = static::m; ${stmt("$at", "$f")} }`);
    decl.push("}");
  }
  decl.push(`class ${childName} ${kind === "interface" ? "implements" : "extends"} ${parentName} {`);
  const declSite = decl.length + 1;
  decl.push(`    ${mod} ${sig(c.child, c.member.covariant)} ${body(childName, c.child, 1)}`, "}");

  const uses: Use[] = [];
  const child = ran(childName, 1, c.child);
  c.paths.forEach((path, k) => {
    if (c.member.covariant) {
      uses.push(covariantCall(c, path, k, parentName, childName));
      return;
    }
    const v = `$v${n}x${k}`;
    const at = q(label(c, path));
    const typed = (type: string): string[] => [`${type} ${v} = new ${childName}();`, stmt(at, `${v}->m`)];
    let lines: string[];
    let out = child;
    switch (path) {
      case "parent-variable":
      case "interface-variable":
        lines = typed(parentName);
        break;
      case "child-variable":
      case "class-variable":
        lines = typed(childName);
        break;
      case "mixed":
        lines = typed("mixed");
        break;
      case "callable":
        lines = kind === "static" ? [`${childName}::viaCallable(${at});`] : [`${parentName} ${v} = new ${childName}();`, `var $f${n}x${k} = ${v}->m(...);`, stmt(at, `$f${n}x${k}`)];
        break;
      case "static":
        lines = [`${childName}::viaStatic(${at});`];
        break;
      case "child-name":
        lines = [stmt(at, `${childName}::m`)];
        break;
      case "parent-name":
        lines = [stmt(at, `${parentName}::m`)];
        out = ran(parentName, 0, c.parent);
        break;
      default:
        throw new Error(`soundness: no path ${path} for ${c.member.id}`);
    }
    uses.push({ path, lines, expect: `${label(c, path)}${out}` });
  });
  return { decl, declSite, setup: [], uses };
}

/**
 * One path's call of a `covariant` parameter. Through the child's type it passes the child's value, which the
 * body prints. Every other path passes the parent's wider value, and the entry test throws a `LogicError`.
 */
function covariantCall(c: Ctx, path: string, k: number, parentName: string, childName: string): Use {
  const n = c.n;
  const v = `$v${n}x${k}`;
  const at = q(label(c, path));
  if (path === "child-variable") {
    const cv = value(c.child, 1);
    return { path, lines: [`${childName} ${v} = new ${childName}();`, `echo ${at}; ${v}->m(${cv});`], expect: `${label(c, path)}${childName}.m ${describe(cv)}` };
  }
  const wide = WIDER[c.rel];
  if (!wide) throw new Error(`soundness: no wider value for ${c.rel}`);
  const lines = [`${path === "mixed" ? "mixed" : parentName} ${v} = new ${childName}();`];
  if (path === "callable") lines.push(`var $f${n}x${k} = ${v}->m(...);`, printsLogicError(at, `$f${n}x${k}(${wide});`, `$e${n}x${k}`));
  else if (path === "parent-variable" || path === "mixed") lines.push(printsLogicError(at, `${v}->m(${wide});`, `$e${n}x${k}`));
  else throw new Error(`soundness: no path ${path} for ${c.member.id}`);
  const message = `The parameter \`$a\` of \`${childName}::m\` has the type \`${c.child}\`. The argument is ${describeArgument(wide)}.`;
  return { path, lines, expect: `${label(c, path)}${message}` };
}

function compareToPiece(c: Ctx): Piece {
  const n = c.n;
  const written: Partial<Record<Rel, string>> = {
    same: `Parent${n}`,
    subclass: "self",
    superclass: `Root${n}`,
    unrelated: "Plant",
    "nullable-over-t": `?Parent${n}`,
  };
  const type = written[c.rel];
  if (!type) throw new Error(`soundness: compareTo has no relation ${c.rel}`);
  const method = (cls: string, t: string) => `    public function compareTo(${t} $other): int { echo "${cls}.compareTo ", Show::of($other), "\\n"; return 0; }`;
  const decl = [`class Root${n} {}`, `class Parent${n} extends Root${n} implements Comparable {`, method(`Parent${n}`, "self"), "}", `class Child${n} extends Parent${n} {`];
  const declSite = decl.length + 1;
  decl.push(method(`Child${n}`, type), "}");
  const uses: Use[] = c.paths.map((path, k) => {
    const v = `$v${n}x${k}`;
    const at = q(label(c, path));
    const recvType: Record<string, string> = {
      "parent-variable": `Parent${n}`,
      "child-variable": `Child${n}`,
      "interface-variable": "Comparable",
      mixed: "mixed",
      callable: `Parent${n}`,
    };
    const lines = [`${recvType[path]} ${v} = new Child${n}();`];
    if (path === "callable") lines.push(`var $f${n}x${k} = ${v}->compareTo(...);`, `echo ${at}; $f${n}x${k}(new Child${n}());`);
    else lines.push(`echo ${at}; ${v}->compareTo(new Child${n}());`);
    return { path, lines, expect: `${label(c, path)}Child${n}.compareTo Child${n}` };
  });
  return { decl, declSite, setup: [], uses };
}

function propertyPiece(c: Ctx): Piece {
  const n = c.n;
  const form = c.member.id.slice("property-".length);
  const P = c.parent;
  const C = c.child;
  const pv = value(P, 0);
  const cv = value(C, 1);
  const decl = [`class Parent${n} {`];
  let childDecl: string;
  let childCtor: string | null = `    public function constructor() { parent::constructor(); $this->x = ${cv}; }`;
  const parentCtor = `    public function constructor() { $this->x = ${pv}; }`;
  switch (form) {
    case "plain":
      decl.push(`    public ${P} $x;`, parentCtor);
      childDecl = `    public ${C} $x;`;
      break;
    case "array":
      decl.push(`    public ${P} $x = [];`, parentCtor);
      childDecl = `    public ${C} $x = [];`;
      break;
    case "lateinit":
      decl.push(`    public lateinit ${P} $x;`, parentCtor);
      childDecl = `    public lateinit ${C} $x;`;
      break;
    case "hooked":
      decl.push(`    public ${P} $x { set (${P} $v) { $this->x = $v; } }`, parentCtor);
      childDecl = `    public ${C} $x { set (${C} $v) { $this->x = $v; } }`;
      break;
    case "promoted":
      decl.push(`    public function constructor(public ${P} $x) {}`);
      // A relation that compiles passes the child's own value on. A refused one passes the parent's
      // value, so the call adds no second error.
      childDecl = `    public function constructor(public ${C} $x) { parent::constructor(${c.member.verdicts[c.rel] === "ok" ? "$x" : pv}); }`;
      childCtor = null;
      break;
    case "readonly":
      decl.push(`    public readonly ${P} $x;`, parentCtor);
      childDecl = `    public readonly ${C} $x;`;
      break;
    case "covariant":
      // The parent's constructor stores what the child passes, since its own value would not pass the test.
      // A refused relation passes the parent's value, so the call adds no second error.
      decl.push(`    public ${P} $x;`, `    public function constructor(${P} $v) { $this->x = $v; }`);
      childDecl = `    public covariant ${C} $x;`;
      childCtor = `    public function constructor() { parent::constructor(${c.member.verdicts[c.rel] === "ok" ? cv : pv}); }`;
      break;
    default:
      throw new Error(`soundness: no property form ${form}`);
  }
  decl.push("}", `class Child${n} extends Parent${n} {`);
  const declSite = decl.length + 1;
  decl.push(childDecl);
  if (childCtor) decl.push(childCtor);
  decl.push("}");
  // The union paths' other class: the same property, with the parent's type.
  if (c.paths.some((path) => path.startsWith("union-"))) {
    decl.push(`final class Other${n} {`, `    public ${P} $x${form === "array" ? " = []" : ""};`, parentCtor, "}");
  }
  const make = form === "promoted" ? `new Child${n}(${cv})` : `new Child${n}()`;
  const uses: Use[] = c.paths.map((path, k) => {
    const v = `$v${n}x${k}`;
    const at = q(label(c, path));
    const recvType: Record<string, string> = {
      "parent-variable": `Parent${n}`,
      "child-variable": `Child${n}`,
      object: "object",
      mixed: "mixed",
      "property-key": `Parent${n}`,
      "union-tested": `Parent${n}|Other${n}`,
      "union-fallthrough": `Other${n}|Parent${n}`,
    };
    const lines = [`${recvType[path]} ${v} = ${make};`];
    if (form === "covariant") return covariantWrite(c, path, k, lines);
    if (path === "property-key") lines.push(`property<Parent${n}> $k${n}x${k} = "x" as property<Parent${n}>;`, `echo ${at}, Show::of(${v}->$k${n}x${k}), "\\n";`);
    else lines.push(`echo ${at}, Show::of(${v}->x), "\\n";`);
    return { path, lines, expect: `${label(c, path)}${describe(cv)}` };
  });
  return { decl, declSite, setup: [], uses };
}

/**
 * One path's write of a `covariant` property, after `lines` made the receiver. Through the child's type it
 * writes the child's value and reads it back. Every other path writes the parent's wider value, and the
 * write's test throws a `LogicError`.
 */
function covariantWrite(c: Ctx, path: string, k: number, lines: string[]): Use {
  const n = c.n;
  const v = `$v${n}x${k}`;
  const at = q(label(c, path));
  if (path === "child-variable") {
    const cv = value(c.child, 1);
    lines.push(`${v}->x = ${cv};`, `echo ${at}, Show::of(${v}->x), "\\n";`);
    return { path, lines, expect: `${label(c, path)}${describe(cv)}` };
  }
  const wide = WIDER[c.rel];
  if (!wide) throw new Error(`soundness: no wider value for ${c.rel}`);
  const stored = `echo ${at}, Show::of(${v}->x), "\\n";`;
  if (path === "property-key") {
    lines.push(`property<Parent${n}> $k${n}x${k} = "x" as property<Parent${n}>;`, printsLogicError(at, `${v}->$k${n}x${k} = ${wide}; ${stored}`, `$e${n}x${k}`));
  } else {
    lines.push(printsLogicError(at, `${v}->x = ${wide}; ${stored}`, `$e${n}x${k}`));
  }
  const message = `The property \`$x\` of \`Child${n}\` has the type \`${c.child}\`. The value is ${describeArgument(wide)}.`;
  return { path, lines, expect: `${label(c, path)}${message}` };
}

function staticPropertyPiece(c: Ctx): Piece {
  const n = c.n;
  const pv = value(c.parent, 0);
  const cv = value(c.child, 1);
  const decl = [`class Parent${n} {`, `    public static ${c.parent} $x = ${constant(pv) ? pv : "null"};`, `    public static function viaSelf(string $at): void { echo $at, Show::of(self::$x), "\\n"; }`];
  const staticAt = decl.length + 1;
  if (c.paths.includes("static")) decl.push(`    public static function viaStatic(string $at): void { echo $at, Show::of(static::$x), "\\n"; }`);
  decl.push("}", `class Child${n} extends Parent${n} {`);
  const declSite = decl.length + 1;
  decl.push(`    public static ${c.child} $x = ${constant(cv) ? cv : "null"};`, `    public static function viaChildSelf(string $at): void { echo $at, Show::of(self::$x), "\\n"; }`, "}");
  const setup: string[] = [];
  if (!constant(pv)) setup.push(`Parent${n}::$x = ${pv};`);
  if (!constant(cv)) setup.push(`Child${n}::$x = ${cv};`);
  const uses: Use[] = c.paths.map((path, k) => {
    const v = `$v${n}x${k}`;
    const at = q(label(c, path));
    const l = label(c, path);
    switch (path) {
      case "parent-name":
        return { path, lines: [`echo ${at}, Show::of(Parent${n}::$x), "\\n";`], expect: l + describe(pv) };
      case "child-name":
        return { path, lines: [`echo ${at}, Show::of(Child${n}::$x), "\\n";`], expect: l + describe(cv) };
      case "self-in-parent":
        return { path, lines: [`Child${n}::viaSelf(${at});`], expect: l + describe(pv) };
      case "self-in-child":
        return { path, lines: [`Child${n}::viaChildSelf(${at});`], expect: l + describe(cv) };
      case "object":
      case "mixed":
        return {
          path,
          lines: [
            `${path} ${v} = new Child${n}();`,
            `try { mixed $r${n}x${k} = ${v}->x; echo ${at}, Show::of($r${n}x${k}), "\\n"; } catch (Throwable $e${n}x${k}) { echo ${q(`${l}threw`)}, "\\n"; }`,
          ],
          expect: `${l}threw`,
        };
      case "static":
        return { path, lines: [`Child${n}::viaStatic(${at});`], site: { in: "decl", at: staticAt } };
      case "parent-variable":
        return { path, lines: [`Parent${n} ${v} = new Child${n}();`, `echo ${at}, Show::of(${v}->x), "\\n";`], site: { in: "use", at: 2 } };
      default:
        throw new Error(`soundness: no path ${path} for ${c.member.id}`);
    }
  });
  return { decl, declSite, setup, uses };
}

function constantPiece(c: Ctx): Piece {
  const n = c.n;
  const iface = c.member.id === "interface-constant";
  const parentName = iface ? `Face${n}` : `Parent${n}`;
  const pv = value(c.parent, 0);
  const cv = value(c.child, 1);
  const mod = iface ? "public function" : "public static function";
  const decl = [
    `${iface ? "interface" : "class"} ${parentName} {`,
    `    public const ${c.parent} X = ${pv};`,
    `    ${mod} viaSelf(string $at): void { echo $at, Show::of(self::X), "\\n"; }`,
    `    ${mod} viaStatic(string $at): void { echo $at, Show::of(static::X), "\\n"; }`,
    "}",
    `class Child${n} ${iface ? "implements" : "extends"} ${parentName} {`,
  ];
  const declSite = decl.length + 1;
  decl.push(`    public const ${c.child} X = ${cv};`, `    ${mod} viaChildSelf(string $at): void { echo $at, Show::of(self::X), "\\n"; }`, "}");
  const uses: Use[] = c.paths.map((path, k) => {
    const at = q(label(c, path));
    const l = label(c, path);
    const call = (m: string): string[] => (iface ? [`Child${n} $v${n}x${k} = new Child${n}();`, `$v${n}x${k}->${m}(${at});`] : [`Child${n}::${m}(${at});`]);
    switch (path) {
      case "parent-name":
      case "interface-name":
        return { path, lines: [`echo ${at}, Show::of(${parentName}::X), "\\n";`], expect: l + describe(pv) };
      case "child-name":
      case "class-name":
        return { path, lines: [`echo ${at}, Show::of(Child${n}::X), "\\n";`], expect: l + describe(cv) };
      case "self-in-parent":
      case "self-in-interface":
        return { path, lines: call("viaSelf"), expect: l + describe(pv) };
      case "self-in-child":
      case "self-in-class":
        return { path, lines: call("viaChildSelf"), expect: l + describe(cv) };
      case "static":
        return { path, lines: call("viaStatic"), expect: l + describe(cv) };
      default:
        throw new Error(`soundness: no path ${path} for ${c.member.id}`);
    }
  });
  return { decl, declSite, setup: [], uses };
}

const BUILDERS: Record<Family, (c: Ctx) => Piece> = {
  method: methodPiece,
  "compare-to": compareToPiece,
  property: propertyPiece,
  "static-property": staticPropertyPiece,
  constant: constantPiece,
};

// ---- cells -----------------------------------------------------------------------------------------

export interface Cell {
  id: string;
  member: Member;
  rel: Rel;
  /** A path, or `declaration` for a refused relation. */
  path: string;
  /** `ok`, or the compile error's code. */
  verdict: string;
  rule: string;
}

export function cells(): Cell[] {
  const out: Cell[] = [];
  for (const member of MEMBERS) {
    for (const rel of RELATIONS) {
      const verdict = member.verdicts[rel];
      if (verdict === undefined) continue;
      if (verdict !== "ok") {
        out.push({ id: `${member.id}/${rel}/declaration`, member, rel, path: "declaration", verdict, rule: member.rule });
        continue;
      }
      for (const p of member.paths) out.push({ id: `${member.id}/${rel}/${p.id}`, member, rel, path: p.id, verdict: p.verdict, rule: p.rule });
    }
  }
  return out;
}

function pair(member: Member, rel: Rel): [string, string] {
  return member.pairs?.[rel] ?? PAIRS[rel];
}

/** One program: the prelude, then each relation's classes, setup and paths, in table order. */
interface Program {
  text: string;
  /** For a running program, what it prints. */
  expect: string[];
  /** For a refused program, each diagnostic's code and line, in line order. */
  errors: { code: string; line: number; cell: string }[];
}

const HEADER = [
  "<?nvs",
  "// Generated by `bun nv soundness --write` from the table in `tools/nv/cmd/soundness.ts`.",
  "// Do not edit this file by hand: change the table and write it again.",
];

function program(group: Cell[]): Program {
  const lines = [...HEADER, ...PRELUDE];
  const expect: string[] = [];
  const errors: Program["errors"] = [];
  const member = group[0]!.member;
  for (const rel of RELATIONS) {
    const here = group.filter((c) => c.rel === rel);
    if (here.length === 0) continue;
    const [child, parent] = pair(member, rel);
    const paths = here.filter((c) => c.path !== "declaration").map((c) => c.path);
    const piece = BUILDERS[member.family]({ member, rel, n: RELATIONS.indexOf(rel) + 1, child, parent, paths });
    lines.push("");
    const declStart = lines.length;
    lines.push(...piece.decl);
    for (const c of here) if (c.path === "declaration") errors.push({ code: c.verdict, line: declStart + piece.declSite, cell: c.id });
    lines.push(...piece.setup);
    for (const c of here) {
      if (c.path === "declaration") continue;
      const use = piece.uses.find((u) => u.path === c.path)!;
      const useStart = lines.length;
      lines.push(...use.lines);
      if (c.verdict === "ok") expect.push(use.expect!);
      else if (use.site) errors.push({ code: c.verdict, line: (use.site.in === "decl" ? declStart : useStart) + use.site.at, cell: c.id });
      else throw new Error(`soundness: ${c.id} has no line its error points at`);
    }
  }
  errors.sort((a, b) => a.line - b.line);
  return { text: `${lines.join("\n")}\n`, expect, errors };
}

const runs = (c: Cell) => c.verdict === "ok";

function expectfErrors(errors: Program["errors"]): string {
  const out: string[] = [];
  for (const e of errors) out.push(`error[${e.code}]: %s`, `%w--> %s:${e.line}:%d`, "%A");
  out.push(`error: aborting due to ${errors.length} error${errors.length === 1 ? "" : "s"}`, "To read what a code means and how to fix it, run `nvs agent show <code>`.");
  return out.join("\n");
}

function caseText(title: string, p: Program, refused: boolean): string {
  const parts = ["--TEST--", title, "--FILE--", p.text.trimEnd()];
  if (refused) parts.push("--EXPECT--", "", "--EXPECTF-ERROR--", expectfErrors(p.errors));
  else parts.push("--EXPECT--", ...p.expect);
  return `${parts.join("\n")}\n`;
}

/** Every file `--write` writes, by repo-relative path. */
export function files(): Map<string, string> {
  const out = new Map<string, string>();
  const all = cells();
  for (const member of MEMBERS) {
    const mine = all.filter((c) => c.member === member);
    const ok = mine.filter(runs);
    const bad = mine.filter((c) => !runs(c));
    if (ok.length) {
      const title = `Each redeclaration of ${member.title} that compiles runs the child's declaration through every path, and reads a value of its declared type (generated; \`${member.rule}\`)`;
      out.set(`${OUT}/${member.id}.nvst`, caseText(title, program(ok), false));
    }
    if (bad.length) {
      const title = `Each redeclaration of ${member.title} that the rule refuses, and each path it refuses, is a compile error at the line it names (generated; \`${member.rule}\`)`;
      out.set(`${OUT}/${member.id}.errors.nvst`, caseText(title, program(bad), true));
    }
  }
  return out;
}

// ---- running one cell ------------------------------------------------------------------------------

interface Outcome {
  cell: Cell;
  pass: boolean;
  expected: string;
  got: string;
}

function nvsBinary(): string {
  return process.env.NVS_BIN || covwsNvs();
}

async function runCell(cell: Cell, dir: string): Promise<Outcome> {
  const p = program([cell]);
  mkdirSync(dir, { recursive: true });
  writeFileSync(join(dir, "case.nvs"), p.text);
  const r = await proc([nvsBinary(), "run", "case.nvs"], { cwd: dir, timeoutMs: 120_000 });
  const norm = (s: string) => s.replace(/\r\n/g, "\n").split("\n").map((l) => l.trimEnd()).join("\n").trim();
  const stdout = norm(r.stdout);
  const stderr = norm(r.stderr);
  const got = [stdout, stderr].filter(Boolean).join("\n") || `(nothing, exit ${r.code})`;
  if (runs(cell)) {
    const expected = p.expect.join("\n");
    return { cell, pass: r.code === 0 && stdout === expected, expected, got: r.code === 0 ? got : `exit ${r.code}: ${got}` };
  }
  const e = p.errors[0]!;
  const at = new RegExp(`^error\\[${e.code}\\]: .*\\n\\s*--> [^\\n]*case\\.nvs:${e.line}:\\d+`, "m");
  const expected = `error[${e.code}] at line ${e.line}, and no other error`;
  return { cell, pass: r.code !== 0 && at.test(stderr) && /aborting due to 1 error\b/.test(stderr), expected, got };
}

async function runAll(list: Cell[]): Promise<Outcome[]> {
  const root = join(ROOT, SCRATCH);
  rmSync(root, { recursive: true, force: true });
  const out: Outcome[] = new Array(list.length);
  let next = 0;
  const jobs = Math.max(1, cpus().length || 4);
  const worker = async () => {
    while (next < list.length) {
      const i = next++;
      out[i] = await runCell(list[i]!, join(root, String(i)));
    }
  };
  try {
    await Promise.all(Array.from({ length: Math.min(jobs, list.length) }, worker));
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
  return out;
}

const head = (s: string, n = 6) => s.split("\n").slice(0, n).map((l) => `      ${l}`).join("\n");

// ---- the command -----------------------------------------------------------------------------------


function onDisk(): Map<string, string> {
  const out = new Map<string, string>();
  const dir = join(ROOT, OUT);
  if (!existsSync(dir)) return out;
  for (const f of readdirSync(dir).sort()) if (f.endsWith(".nvst")) out.set(`${OUT}/${f}`, readFileSync(join(dir, f), "utf8").replace(/\r\n/g, "\n"));
  return out;
}

export async function run(args: string[]): Promise<number> {
  const flags = new Set<string>();
  for (const a of args) {
    if (a === "--help" || a === "-h") {
      console.log(USAGE);
      return 0;
    }
    if (!["--write", "--check", "--probe"].includes(a)) {
      console.error(`${USAGE}\n\nnv soundness: no option ${a}`);
      return 2;
    }
    flags.add(a);
  }
  const modes = ["--write", "--check", "--probe"].filter((f) => flags.has(f));
  if (modes.length !== 1) {
    console.error(USAGE);
    return 2;
  }

  const all = cells();
  const want = files();

  if (flags.has("--write")) {
    mkdirSync(join(ROOT, OUT), { recursive: true });
    for (const f of onDisk().keys()) if (!want.has(f)) rmSync(join(ROOT, f));
    for (const [f, text] of want) writeFileSync(join(ROOT, f), text);
    console.log(`nv soundness: wrote ${want.size} file(s) under ${OUT}, ${all.length} cell(s)`);
    return 0;
  }

  if (flags.has("--probe")) {
    const got = await runAll(all);
    const failing = got.filter((o) => !o.pass);
    for (const o of failing) console.log(`${o.cell.id}  (${o.cell.rule})\n    expected:\n${head(o.expected)}\n    got:\n${head(o.got)}`);
    console.log(`nv soundness: ${failing.length} of ${all.length} cell(s) fail when run alone`);
    return failing.length ? 1 : 0;
  }

  // --check
  const problems: string[] = [];
  const disk = onDisk();
  for (const [f, text] of want) {
    if (!disk.has(f)) problems.push(`${f} is missing: run \`bun nv soundness --write\``);
    else if (disk.get(f) !== text) problems.push(`${f} differs from what \`bun nv soundness --write\` writes`);
  }
  for (const f of disk.keys()) if (!want.has(f)) problems.push(`${f} is not a file \`bun nv soundness --write\` writes`);
  for (const p of problems) console.error(`nv soundness: ${p}`);
  if (problems.length) return 1;
  console.log(`nv soundness: ${want.size} file(s) current, ${all.length} cell(s) written`);
  return 0;
}
