// A session's wrap file, parsed: its `## kind: arg` sections and the `--- old` / `--- new` pairs of a
// `## plan-edit:`. `bun nv plan --apply` reads the file through this module and applies its plan sections.

import { pyRepr } from "./py.ts";

/** One `## kind: arg` section of a wrap file, with its 1-based heading line. */
export interface Section {
  kind: string;
  arg: string;
  body: string;
  line: number;
  /** The paths the wrap added to a `## commit:` because it writes them and no section named them. */
  added?: string[];
  /** A `## playbook:` section's bullets, each with the fragment file it is written to; see `playbookTargets`. */
  targets?: [string, string][];
}

/** The section kinds, in the order a wrap applies them. */
export const KNOWN = ["plan", "plan-edit", "milestone", "playbook", "handoff", "commit", "status"];
/** The kinds `bun nv plan --apply` takes. */
export const PLAN_KINDS = ["plan", "plan-edit", "milestone"];
/** `## kind: arg`. The argument is `[^\n]*` so a CR left by a CRLF file lands in it and is trimmed. */
const DIRECTIVE = /^##\s+([A-Za-z][A-Za-z-]*)\s*:?\s*([^\n]*)$/;

/** The first `n` characters of `s`, counted as Python counts them. */
export function head(s: string, n: number): string {
  return [...s].slice(0, n).join("");
}

/** How many times `sub` occurs in `s` without overlapping, as Python's `str.count`. */
export function occurrences(s: string, sub: string): number {
  return s.split(sub).length - 1;
}

export function nbytes(s: string): number {
  return Buffer.byteLength(s, "utf8");
}

export const thousands = (n: number) => n.toLocaleString("en-US");

/** `sections` in the order a wrap applies them, `KNOWN`'s. */
export function inOrder(sections: Section[]): Section[] {
  return KNOWN.flatMap((kind) => sections.filter((s) => s.kind === kind));
}

/**
 * The `## kind: arg` headings and their bodies, in file order, and what is malformed. The handoff body
 * is markdown with its own `## ` headings, so inside `## handoff` only a known directive starts a new
 * section, and `## State` stays in the body.
 */
export function parseWrap(text: string): { sections: Section[]; errors: string[] } {
  const sections: Section[] = [];
  const errors: string[] = [];
  let cur: Section | null = null;
  let buf: string[] = [];
  const flush = () => {
    if (cur !== null) {
      cur.body = buf.join("\n").replace(/^\n+|\n+$/g, "");
      sections.push(cur);
    }
    cur = null;
    buf = [];
  };
  text.split("\n").forEach((raw, i) => {
    const m = DIRECTIVE.exec(raw);
    const kind = m ? m[1]!.toLowerCase() : "";
    if (m && KNOWN.includes(kind)) {
      if (cur?.kind === "handoff" && kind === "handoff") errors.push(`line ${i + 1}: a second \`## handoff\` section`);
      flush();
      cur = { kind, arg: m[2]!.trim(), body: "", line: i + 1 };
      return;
    }
    if (cur === null) {
      if (raw.trim()) errors.push(`line ${i + 1}: text before the first \`## \` directive: ${pyRepr(head(raw.trim(), 60))}`);
      return;
    }
    buf.push(raw);
  });
  flush();

  for (const s of sections) {
    if (["plan", "plan-edit", "milestone", "playbook", "commit"].includes(s.kind) && !s.arg) {
      errors.push(`line ${s.line}: \`## ${s.kind}:\` needs an argument`);
    }
    if ((s.kind === "handoff" || s.kind === "status") && s.arg) {
      errors.push(`line ${s.line}: \`## ${s.kind}\` takes no argument, got ${pyRepr(s.arg)}`);
    }
    if (!s.body.trim()) errors.push(`line ${s.line}: \`## ${s.kind}\` has an empty body`);
  }
  return { sections, errors };
}

/** A field's text as the plan stores it: one paragraph, single-spaced. */
export function normalize(text: string): string {
  return text.split(/\s+/).filter(Boolean).join(" ");
}

/** The `--- old` / `--- new` fragment pairs of a `## plan-edit:` body, and what is malformed. */
export function parseEdits(body: string): { pairs: [string, string][]; errors: string[] } {
  const pairs: [string, string][] = [];
  const errors: string[] = [];
  let slot: "old" | "new" | null = null;
  const buf = { old: [] as string[], new: [] as string[] };
  const flush = () => {
    if (buf.old.length || buf.new.length) pairs.push([normalize(buf.old.join("\n")), normalize(buf.new.join("\n"))]);
    buf.old = [];
    buf.new = [];
  };
  for (const raw of body.split("\n")) {
    const m = /^---\s*(old|new)\s*$/i.exec(raw.trim());
    if (m) {
      const next = m[1]!.toLowerCase() as "old" | "new";
      if (next === "old") flush();
      else if (buf.old.length === 0) errors.push("a `--- new` fragment with no `--- old` in front of it");
      slot = next;
      continue;
    }
    if (slot === null) {
      if (raw.trim()) errors.push(`text before the first \`--- old\`: ${pyRepr(head(raw.trim(), 60))}`);
      continue;
    }
    buf[slot].push(raw);
  }
  flush();
  if (pairs.length === 0) errors.push("no `--- old` / `--- new` fragment pair");
  for (const [old] of pairs) {
    if (!old) errors.push("an empty `--- old` fragment -- it must quote what is there now");
  }
  return { pairs, errors };
}
