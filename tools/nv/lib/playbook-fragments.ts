// The playbook's Markdown home: one bullet per file under `docs/agent/playbook/<section>/`, each with
// its record at `data/playbook/<section>/<slug>.json`. `bun nv session --wrap` writes both, and
// `bun nv orient` prints the bullets from the fragment files. A section's title and order are its
// record, `data/playbook/<section>.json`, which has no Markdown source.
//
// A bullet's `lead` is its bold opening and `body` the rest before the trailer, each unwrapped to one
// line, since the rendered file wraps them again. `files` is what `anchors` finds: the tree paths the
// bullet names in backticks that exist, and a `gone` or `exists` trailer's.

import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";
import { load } from "./store.ts";
import { playbookSection } from "../schema/playbook.ts";

export const PLAYBOOK_FRAGMENTS = "docs/agent/playbook";
const TREE_DIRS = ["crates/", "tools/", "docs/", "tests/", "benches/", "examples/", "fuzz/", ".github/"];
const PATH_TRIM = /(:re:.*|:@[\w:.-]+|:\d+([-+]\d+)?|[.,;:)\]'"]+)$/;
const BRACES = /^([^{}]*)\{([^{}]+)\}([^{}]*)$/;
const EXPIRY = /\[until:\s*(test|exists|gone|rule)\s+([^\]]+?)\s*\]\s*$/;

export interface BulletValue {
  lead: string;
  body: string;
  files: string[];
  until: { kind: string; arg: string };
}

function isFile(root: string, path: string): boolean {
  try {
    return statSync(join(root, path)).isFile();
  } catch {
    return false;
  }
}

/** The names in the repo-relative directory `dir`, sorted, or none when it does not exist. */
export function list(root: string, dir: string): string[] {
  try {
    return readdirSync(join(root, dir)).sort();
  } catch {
    return [];
  }
}

/**
 * Every tree path a bullet names in backticks, braces expanded, whether or not it exists. `*`, `<` and
 * an elision are how a bullet writes a path it never claimed exists, so those are skipped.
 */
export function namedPaths(bullet: string): string[] {
  const out: string[] = [];
  for (const [, raw] of bullet.matchAll(/`([^`]+)`/g)) {
    const first = raw!.trim().split(/\s+/)[0] ?? "";
    const cand = first.replace(PATH_TRIM, "");
    if (!TREE_DIRS.some((d) => cand.startsWith(d)) || ["*", "<", "…", "..."].some((m) => cand.includes(m))) continue;
    const m = BRACES.exec(cand);
    out.push(...(m ? m[2]!.split(",").map((p) => p.trim()).filter((p) => p !== "").map((p) => `${m[1]}${p}${m[3]}`) : [cand]));
  }
  return out;
}

/** The files in the tree a bullet names: its backticked paths that exist, and its trailer's path. */
export function anchors(root: string, bullet: string, until: { kind: string; arg: string } | null): string[] {
  const out = namedPaths(bullet).filter((p) => existsSync(join(root, p)));
  for (const [, raw] of bullet.matchAll(/`([^`\s/]+\.[A-Za-z]+)`/g)) if (isFile(root, raw!)) out.push(raw!);
  if (until && (until.kind === "gone" || until.kind === "exists")) {
    const path = until.arg.split(":")[0]!.trim().replace(/\\/g, "/");
    if (path !== "" && existsSync(join(root, path))) out.push(path);
  }
  return [...new Set(out)];
}

const unwrap = (s: string) => s.replace(/\s*\n\s*/g, " ").trim();

/**
 * One bullet file's text as its record's value, or why it cannot be one. The lead ends at the first
 * `**` outside a code span, so a lead that quotes bold Markdown in backticks is read whole.
 */
export function bulletValue(root: string, src: string): { value: BulletValue } | { reason: string } {
  const m = /^- \*\*((?:`[^`]*`|[^`*]|\*(?!\*))+?)\*\*([\s\S]*)$/.exec(src);
  if (!m) return { reason: "does not open on `- **`" };
  const trailer = EXPIRY.exec(m[2]!);
  if (!trailer) return { reason: "does not end on an `[until: <kind> <arg>]` trailer" };
  const until = { kind: trailer[1]!, arg: trailer[2]! };
  return {
    value: {
      lead: unwrap(m[1]!),
      body: unwrap(m[2]!.slice(0, trailer.index)),
      files: anchors(root, src, until),
      until,
    },
  };
}

export interface PlaybookSection {
  id: string;
  title: string;
  order: number;
}

export interface Fragment {
  /** `<section>/<slug>`, the id of its record. */
  id: string;
  value: BulletValue;
  /** The repo-relative fragment file. */
  from: string;
}

/**
 * The playbook's sections, in order, from their records, and every bullet read from its fragment file
 * under a section that has a record, in file-name order. A fragment that is not one well-formed bullet
 * is left out.
 */
export function readPlaybook(root: string): { sections: PlaybookSection[]; bullets: Fragment[] } {
  const sections = load(playbookSection, root)
    .filter((r) => r.issues.length === 0)
    .map((r) => ({ id: r.id, ...(r.value as { title: string; order: number }) }))
    .sort((a, b) => a.order - b.order);
  const bullets: Fragment[] = [];
  for (const s of sections) {
    for (const name of list(root, `${PLAYBOOK_FRAGMENTS}/${s.id}`)) {
      if (!name.endsWith(".md")) continue;
      const path = `${PLAYBOOK_FRAGMENTS}/${s.id}/${name}`;
      const got = bulletValue(root, readFileSync(join(root, path), "utf8").replace(/\r\n?/g, "\n").trim());
      if ("value" in got) bullets.push({ id: `${s.id}/${name.slice(0, -".md".length)}`, value: got.value, from: path });
    }
  }
  return { sections, bullets };
}
