// `bun nv names [--check] [--nvs <binary>]`: checks every name of `Core` and of every first-party package
// against `rule:core-api/verb-lexicon` and `rule:core-api/members-are-full-words`, and prints each name
// that breaks them with the part of the rule it breaks. `--check` exits 1 when one does; without it the
// command prints the same list and exits 0. It exits 2 when the binary cannot print `nvs meta --json`.
//
// Its sources are `nvs meta --json` from the binary, which has `Core` and the extensions'
// `Novis\…` classes, and the `public` declarations under `packages/rest/src/`. Its tables are
// `tools/data/api-names.json`. The binary is `--nvs`, else `NVS_BIN`, else `target/debug/nvs`.
//
// What it judges, one finding per name and part, in this order:
//
//   * **part 1**: a member returning `void` starts with an effect verb (`effectVerbs`);
//   * **part 3**: an instance member returning its own class is in a class listed under `classes`. In a
//     `value` class it is named as a copy: a `copyWords` word, a past participle (a first word ending in
//     `ed` and not in `notParticiples`, or an `irregularParticiples` word), a whole math name, or a
//     `reads` row. A participle ending in `en` counts only from the list, since `flatten` and `open`
//     end so too. In a
//     `changing` class it starts with an effect verb or is a `reads` row;
//   * **part 4**: a member returning `bool` starts with a `predicates` word or an effect verb;
//   * **part 2**: a member that takes an argument besides its subject (any argument of an instance
//     member, the second and later of a static one) starts with a verb of any list, or is a `reads` row.
//     This is the one shape the types cannot judge: a noun with an effect that returns another type;
//   * **full words**: no word of a member name, an option field name or a package class name is a
//     `truncations` word, and a math name is a whole member name, unless it is also an English word
//     (`mathWordsAlsoEnglish`).
//
// Constructors and properties are judged for full words alone, and a `private` member and an
// `internal` class not at all, since a program never calls them. A `reads` row is `Class::member`
// for an instance member and a static alike.
//
// What it spends: one `nvs meta --json`, about a second on a debug binary, and a read of the package
// sources. `bun nv ci --check` runs it after the build.

import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { newestNvs } from "../lib/nvs-binary.ts";
import { ROOT } from "../lib/paths.ts";
import { run as proc } from "../lib/proc.ts";

export const summary = "check every Core and first-party package name against the naming rules: nv names [--check] [--nvs <binary>]";

/** `tools/data/api-names.json`. */
export interface Tables {
  effectVerbs: string[];
  verbs: string[];
  predicates: string[];
  copyWords: string[];
  irregularParticiples: string[];
  notParticiples: string[];
  mathNames: string[];
  mathWordsAlsoEnglish: string[];
  acronyms: string[];
  truncations: Record<string, string>;
  classes: { value: string[]; changing: string[] };
  reads: string[];
}

/** One member as the check reads it. `params` counts the arguments, an options bag excluded. */
export interface Member {
  name: string;
  kind: "static" | "instance" | "constructor" | "property";
  returns: string;
  params: number;
  options?: string[] | undefined;
}

/** One class: its fully qualified name, and whether its own class name is judged for full words. */
export interface Class {
  name: string;
  members: Member[];
  judgeClassName?: boolean;
}

/** One name that breaks a rule. `part` is `part 1` to `part 4` of `rule:core-api/verb-lexicon`, or `full words`. */
export interface Finding {
  where: string;
  part: string;
  message: string;
}

export const TABLES = join(ROOT, "tools", "data", "api-names.json");

/** Reads the tables. A word that is both a truncation and an allowed short word (an acronym or a math
 * name) is an error in the tables themselves, so it throws. */
export function loadTables(path: string = TABLES): Tables {
  const t = JSON.parse(readFileSync(path, "utf8")) as Tables;
  const allowed = new Set([...t.acronyms, ...t.mathNames]);
  const both = Object.keys(t.truncations).filter((w) => allowed.has(w));
  if (both.length > 0) throw new Error(`${path}: ${both.join(", ")} is both a truncation and an allowed short word`);
  return t;
}

/** The words of a camelCase or PascalCase name, lower-cased: `fromCwd` is `from`, `cwd`. */
export function words(name: string): string[] {
  return name
    .replace(/([a-z0-9])([A-Z])/g, "$1 $2")
    .replace(/([A-Z]+)([A-Z][a-z])/g, "$1 $2")
    .toLowerCase()
    .split(/[\s_]+/)
    .filter((w) => w.length > 0);
}

const short = (name: string) => name.slice(name.lastIndexOf("\\") + 1);

/** Every name in `classes` that breaks a naming rule, in class and member order. */
export function judge(classes: Class[], t: Tables): Finding[] {
  const effect = new Set(t.effectVerbs);
  const anyVerb = new Set([...t.effectVerbs, ...t.verbs, ...t.predicates, ...t.copyWords, ...t.irregularParticiples]);
  const predicates = new Set(t.predicates);
  const copy = new Set(t.copyWords);
  const irregular = new Set(t.irregularParticiples);
  const math = new Set(t.mathNames);
  const mathEnglish = new Set(t.mathWordsAlsoEnglish);
  const reads = new Set(t.reads);
  const valueClasses = new Set(t.classes.value);
  const changing = new Set(t.classes.changing);
  const notParticiples = new Set(t.notParticiples);
  const participle = (w: string) => irregular.has(w) || (w.length > 4 && w.endsWith("ed") && !notParticiples.has(w));
  const out: Finding[] = [];

  const fullWords = (where: string, name: string, whole: boolean) => {
    for (const w of words(name)) {
      const full = Object.hasOwn(t.truncations, w) ? t.truncations[w] : undefined;
      if (full !== undefined) {
        out.push({ where, part: "full words", message: `\`${w}\` is a truncation: write ${full}` });
      } else if (whole && math.has(w) && w !== name.toLowerCase() && !mathEnglish.has(w)) {
        out.push({ where, part: "full words", message: `the math name \`${w}\` is allowed only as a whole member name` });
      }
    }
  };

  for (const c of classes) {
    if (c.judgeClassName) fullWords(c.name, short(c.name), false);
    for (const m of c.members) {
      const where = `${c.name}::${m.name}`;
      fullWords(where, m.name, true);
      for (const o of m.options ?? []) fullWords(`${where} option \`${o}\``, o, false);
      if (m.kind !== "static" && m.kind !== "instance") continue;
      const first = words(m.name)[0] ?? "";
      const isRead = reads.has(where);
      if (m.returns === "void" && !effect.has(first)) {
        out.push({ where, part: "part 1", message: "returns `void`, so it has an effect and starts with an effect verb" });
      }
      if (m.kind === "instance" && (m.returns === c.name || m.returns === short(c.name))) {
        if (valueClasses.has(c.name)) {
          if (!(copy.has(first) || participle(first) || math.has(m.name) || isRead)) {
            out.push({ where, part: "part 3", message: `returns a changed copy of a value class, so it is named as a copy (\`with…\`, \`plus\`, a past participle)` });
          }
        } else if (changing.has(c.name)) {
          if (!(effect.has(first) || isRead)) {
            out.push({ where, part: "part 3", message: "changes its object in place, so it starts with an effect verb" });
          }
        } else {
          out.push({ where, part: "part 3", message: `returns its own class: list \`${c.name}\` under \`value\` or \`changing\` in tools/data/api-names.json` });
        }
      }
      if (m.returns === "bool" && !predicates.has(first) && !effect.has(first)) {
        out.push({ where, part: "part 4", message: "returns `bool`, so it starts with a predicate (`is…`, `has…`, `can…`) or an effect verb" });
      }
      const extra = m.kind === "instance" ? m.params : m.params - 1;
      if (extra > 0 && !anyVerb.has(first) && !participle(first) && !math.has(m.name) && !isRead) {
        out.push({ where, part: "part 2", message: "takes an argument and starts with no verb: rename it, or list it under `reads` if it has no effect" });
      }
    }
  }
  return out;
}

/** The `nvs meta --json` classes, in the check's form. An `internal` class and a `private` member are left out. */
export function fromMeta(meta: unknown): Class[] {
  type Raw = {
    name: string;
    internal?: boolean;
    members?: { name: string; kind: string; returns?: string; params?: unknown[]; options?: { name: string }[]; visibility?: string }[];
  };
  const classes = ((meta as { classes?: Raw[] }).classes ?? []).filter((c) => !c.internal);
  return classes.map((c) => ({
    name: c.name,
    judgeClassName: !c.name.startsWith("Core\\"),
    members: (c.members ?? [])
      .filter((m) => m.visibility !== "private" && m.visibility !== "protected")
      .map((m) => ({
        name: m.name,
        kind: m.kind as Member["kind"],
        returns: m.returns ?? "void",
        params: m.params?.length ?? 0,
        options: m.options?.map((o) => o.name),
      })),
  }));
}

/** The number of top-level comma-separated parts of a parameter list. */
function countParams(list: string): number {
  let depth = 0;
  let count = list.trim() === "" ? 0 : 1;
  for (const ch of list) {
    if (ch === "(" || ch === "<" || ch === "{" || ch === "[") depth++;
    else if (ch === ")" || ch === ">" || ch === "}" || ch === "]") depth--;
    else if (ch === "," && depth === 0) count++;
  }
  return list.trim().endsWith(",") ? count - 1 : count;
}

/** The `public` classes and members of a package's `.nvs` sources, read from its declarations. */
export function fromSource(text: string): Class[] {
  const ns = /^namespace\s+([\w\\]+)\s*;/m.exec(text)?.[1];
  const out: Class[] = [];
  const classRe = /^(?:final\s+|abstract\s+|readonly\s+)*class\s+(\w+)/gm;
  const heads = [...text.matchAll(classRe)];
  heads.forEach((h, i) => {
    const body = text.slice(h.index, heads[i + 1]?.index ?? text.length);
    const members: Member[] = [];
    const fnRe = /^\s*public\s+(static\s+)?function\s+(\w+)\s*\(([\s\S]*?)\)\s*(?::\s*([^\n{]+?))?\s*(?:\{|$)/gm;
    for (const f of body.matchAll(fnRe)) {
      const name = f[2]!;
      members.push({
        name,
        kind: name === "constructor" ? "constructor" : f[1] ? "static" : "instance",
        returns: (f[4] ?? "void").trim(),
        params: countParams(f[3]!),
      });
    }
    out.push({ name: ns ? `${ns}\\${h[1]}` : h[1]!, members, judgeClassName: true });
  });
  return out;
}

/** The first-party packages read from source rather than from the binary. */
export const PACKAGE_SOURCES = ["packages/rest/src"];

function packageClasses(root: string): Class[] {
  const out: Class[] = [];
  for (const dir of PACKAGE_SOURCES) {
    for (const f of readdirSync(join(root, dir)).sort()) {
      if (f.endsWith(".nvs")) out.push(...fromSource(readFileSync(join(root, dir, f), "utf8")));
    }
  }
  return out;
}

export async function run(args: string[]): Promise<number> {
  let check = false;
  let nvs = process.env.NVS_BIN || newestNvs() || "nvs";
  for (let i = 0; i < args.length; i++) {
    const a = args[i];
    if (a === "--check") check = true;
    else if (a === "--nvs" && args[i + 1] !== undefined) nvs = args[++i]!;
    else {
      console.error(`names: unknown argument ${a}\n${summary}`);
      return 2;
    }
  }
  const out = await proc([nvs, "meta", "--json"], { cwd: ROOT, timeoutMs: 120_000, env: { NVS_FOOTPRINT_LOG: "" } });
  if (out.code !== 0) {
    console.error(`names: \`${nvs} meta --json\` failed:\n${out.stderr.trim()}`);
    return 2;
  }
  const classes = [...fromMeta(JSON.parse(out.stdout)), ...packageClasses(ROOT)];
  const findings = judge(classes, loadTables());
  for (const f of findings) console.log(`${f.where}: ${f.part}: ${f.message}`);
  const byPart = new Map<string, number>();
  for (const f of findings) byPart.set(f.part, (byPart.get(f.part) ?? 0) + 1);
  const counts = [...byPart].map(([p, n]) => `${p}: ${n}`).join(", ");
  console.log(findings.length === 0 ? "names: every name follows the naming rules" : `names: ${findings.length} name(s) break the naming rules (${counts})`);
  return check && findings.length > 0 ? 1 : 0;
}
