// The canonical writer: a goal's front matter, the chain's front matter, and the README body generated
// from the chain and the goal files, written as `crates/looper-core/src/render.rs` writes them.
//
// The kit writes YAML with its own writer, so the canonical form is fixed here. The form:
// - keys in the order of `docs/goal-format.md`;
// - block style for maps and lists of maps, and flow style for lists of scalars, so an argv stays on one line;
// - plain scalars where YAML reads them back as the same string, and JSON-quoted strings otherwise.
//
// Only front matter and the README body are rewritten. Prose is not.

import {
  type Chain,
  type Check,
  FORMAT,
  type Goal,
  type GoalFront,
  goalSlugs,
  liveStage,
  loadGoal,
  type Mapping,
  type Stage,
} from "./model.ts";

/** The command the README's generated-body comment names. */
export const RENDER_COMMAND = "looper render";

/** A YAML value in the order it is written. A scalar is already rendered. */
export type Node = { scalar: string } | { list: Node[] } | { map: [string, Node][] };

export function strNode(s: string): Node {
  return { scalar: scalar(s) };
}

export function strsNode(list: string[]): Node {
  return { list: list.map(strNode) };
}

function raw(s: string): Node {
  return { scalar: s };
}

const PLAIN = /^[A-Za-z0-9_./@-][A-Za-z0-9_ ./@()-]*$/;

/** Plain text that YAML would read as something other than this string: a boolean, a null or a number. */
const AMBIGUOUS =
  /^(true|false|null|yes|no|~|-?[0-9][0-9.]*|-?(\.[0-9]+|[0-9]+(\.[0-9]*)?)(e[-+]?[0-9]+)?|0x[0-9a-f]+|0o[0-7]+|-?\.inf|\.nan)$/i;

/**
 * A string as a YAML scalar: plain when YAML reads it back unchanged, otherwise JSON-quoted, which is
 * valid YAML. A plain scalar never starts with `@` or a `- ` sequence entry, which YAML reserves.
 */
export function scalar(s: string): string {
  const reserved = s === "-" || s.startsWith("- ") || s.startsWith("@");
  return PLAIN.test(s) && !AMBIGUOUS.test(s) && !s.endsWith(" ") && !reserved ? s : jsonQuote(s);
}

/** The string as a JSON string literal. */
export function jsonQuote(s: string): string {
  return JSON.stringify(s);
}

/**
 * The node as block YAML at `indent`. A map's text starts with a newline, so the caller puts it after
 * its key. A list of scalars and a scalar start with their value.
 */
export function toYaml(node: Node, indent: string): string {
  if ("scalar" in node) return node.scalar;
  const inner = `${indent}  `;
  if ("list" in node) {
    if (node.list.length === 0) return " []";
    if (node.list.every((x) => "scalar" in x)) {
      return ` [${node.list.map((x) => (x as { scalar: string }).scalar).join(", ")}]`;
    }
    return node.list.map((x) => `\n${indent}- ${stripLead(toYaml(x, inner))}`).join("");
  }
  if (node.map.length === 0) return " {}";
  const lines = node.map.map(([k, x]) => {
    const v = toYaml(x, inner);
    return v.startsWith("\n") || v.startsWith(" [") || v.startsWith(" {") ? `${indent}${k}:${v}` : `${indent}${k}: ${v.trimStart()}`;
  });
  return `\n${lines.join("\n")}`;
}

/** The text without a leading newline and the indent after it. */
function stripLead(s: string): string {
  return s.startsWith("\n") ? s.slice(1).trimStart() : s;
}

/** A map that leaves out the fields that are absent. */
function mapNode(entries: [string, Node | undefined][]): Node {
  return { map: entries.filter((e): e is [string, Node] => e[1] !== undefined) };
}

function optStrs(list: string[] | undefined): Node | undefined {
  return list === undefined ? undefined : strsNode(list);
}

function optStr(s: string | undefined): Node | undefined {
  return s === undefined ? undefined : strNode(s);
}

function optRaw(v: number | boolean | undefined): Node | undefined {
  return v === undefined ? undefined : raw(String(v));
}

function stageNode(s: Stage): Node {
  return mapNode([
    ["id", raw(String(s.id))],
    ["title", strNode(s.title)],
    ["status", raw(s.status)],
    ["gate", optRaw(s.gate)],
    ["context", optStrs(s.context)],
    ["extra", s.extra === undefined ? undefined : valueNode(s.extra)],
  ]);
}

export function checkNode(c: Check): Node {
  const env = c.env;
  return mapNode([
    ["id", strNode(c.id)],
    ["stage", optRaw(c.stage)],
    ["name", optStr(c.name)],
    ["run", c.run.length > 0 || c.file === undefined ? strsNode(c.run) : undefined],
    ["cwd", optStr(c.cwd)],
    ["exit", c.exit === undefined ? undefined : raw(c.exit === "zero" ? "0" : "nonzero")],
    ["stdout_contains", optStrs(c.stdout_contains)],
    ["stderr_contains", optStrs(c.stderr_contains)],
    ["stdout_exact", optStr(c.stdout_exact)],
    ["stdout_ordered", optStrs(c.stdout_ordered)],
    ["output_contains", optStrs(c.output_contains)],
    ["output_exact", optStr(c.output_exact)],
    ["output_ordered", optStrs(c.output_ordered)],
    ["tests", optStrs(c.tests)],
    ["file", optStr(c.file)],
    ["min_bytes", optRaw(c.min_bytes)],
    ["timeout", optRaw(c.timeout)],
    [
      "env",
      env === undefined
        ? undefined
        : { map: Object.keys(env).sort(byBytes).map((k): [string, Node] => [scalar(k), strNode(env[k] as string)]) },
    ],
    ["every", optRaw(c.every)],
    ["inputs", optStrs(c.inputs)],
    ["reach", optRaw(c.reach)],
    ["floor", optRaw(c.floor)],
    ["from", optStr(c.from)],
  ]);
}

/** The order of a `BTreeMap<String, _>`: by the strings' UTF-8 bytes. */
function byBytes(a: string, b: string): number {
  return Buffer.compare(Buffer.from(a), Buffer.from(b));
}

/** A free-form YAML value, a goal's `settings` or an `extra`, in its own key order. */
function valueNode(v: unknown): Node {
  if (v === null || v === undefined) return raw("null");
  if (typeof v === "boolean") return raw(String(v));
  if (typeof v === "number") return raw(numberText(v));
  if (typeof v === "string") return strNode(v);
  if (Array.isArray(v)) return { list: v.map(valueNode) };
  return { map: Object.entries(v as Mapping).map(([k, x]): [string, Node] => [scalar(k), valueNode(x)]) };
}

/** A number as YAML writes it, with YAML's names for the infinities and NaN. */
function numberText(n: number): string {
  if (Number.isNaN(n)) return ".nan";
  if (n === Number.POSITIVE_INFINITY) return ".inf";
  if (n === Number.NEGATIVE_INFINITY) return "-.inf";
  return String(n);
}

export function goalFrontNode(f: GoalFront): Node {
  return mapNode([
    ["looper", raw(String(f.looper))],
    ["title", strNode(f.title)],
    ["summary", strNode(f.summary)],
    ["context", strsNode(f.context)],
    ["stages", { list: f.stages.map(stageNode) }],
    ["checks", { list: f.checks.map(checkNode) }],
    ["settings", f.settings === undefined ? undefined : valueNode(f.settings)],
    ["extra", f.extra === undefined ? undefined : valueNode(f.extra)],
  ]);
}

export function chainFrontNode(c: Chain): Node {
  return mapNode([
    ["looper", raw(String(c.looper))],
    ["chain", strsNode(c.chain)],
    ["floor", { list: c.floor.map(checkNode) }],
  ]);
}

/** `---`, the node as YAML, `---`, each on its own line. */
export function frontMatter(node: Node): string {
  return `---${toYaml(node, "")}\n---\n`;
}

/** The goal file's whole text: canonical front matter, then the body exactly as it was read. */
export function goalText(g: Goal): string {
  return frontMatter(goalFrontNode(g.front)) + g.body;
}

function stageLine(g: Goal): string {
  const live = liveStage(g)?.id;
  return g.front.stages
    .map((s) => {
      if (s.status === "done") return `${s.id} ✓ ${s.title}`;
      if (s.id === live) return `**${s.id} ${s.title}**`;
      return `${s.id} ${s.title}`;
    })
    .join(" · ");
}

/**
 * The whole README: the chain's front matter, then the overview generated from the goal files in
 * `goalsDir`. A slug on the chain whose file is missing or does not parse is listed as having no file,
 * and lint reports why.
 */
export function renderReadme(c: Chain, goalsDir: string): string {
  let out = frontMatter(chainFrontNode({ ...c, looper: FORMAT }));
  out += `<!-- Generated by \`${RENDER_COMMAND}\` from the front matter above and the goal files. Edit those, not this. -->\n\n`;
  out += "# Goals\n\n";
  const live = c.chain[0];
  if (live === undefined) {
    out += "The chain is empty.\n";
  } else {
    out += `The chain holds ${c.chain.length} goal(s). \`${live}\` is live.\n\n`;
    out += "| # | Goal | Reached when | Stages |\n|---|---|---|---|\n";
    c.chain.forEach((slug, i) => {
      const name = `[\`${slug}\`](${slug}.md)${i === 0 ? " **live**" : ""}`;
      let g: Goal | undefined;
      try {
        g = loadGoal(goalsDir, slug);
      } catch {
        g = undefined;
      }
      out +=
        g === undefined
          ? `| ${i + 1} | ${name} | *(no file)* | |\n`
          : `| ${i + 1} | ${name} | ${g.front.summary.replaceAll("|", "\\|")} | ${stageLine(g)} |\n`;
    });
  }
  const side = goalSlugs(goalsDir)
    .filter((s) => !c.chain.includes(s))
    .map((s) => `[\`${s}\`](${s}.md)`);
  if (side.length > 0) out += `\nOff the chain (side goals): ${side.join(", ")}.\n`;
  out += `\nThe floor holds ${c.floor.length} check(s) carried from reached goals; every goal keeps them green.\n`;
  out += "The format is `docs/goal-format.md`.\n";
  return out;
}
