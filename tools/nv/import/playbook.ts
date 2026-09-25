// The playbook's Markdown home: one bullet per file under `docs/agent/playbook/<section>/`, each
// becoming `data/playbook/<section>/<slug>.json`. A section's title and order are its record,
// `data/playbook/<section>.json`, which has no Markdown source, so it is read as it stands and passed
// through.
//
// A bullet's `lead` is its bold opening and `body` the rest before the trailer, each unwrapped to one
// line, since the rendered file wraps them again. `files` is what `anchors` finds: the tree paths the
// bullet names in backticks that exist, and a `gone` or `exists` trailer's.

import { statSync } from "node:fs";
import { join } from "node:path";
import { load, pathOf } from "../lib/store.ts";
import { playbookBullet, playbookSection } from "../schema/playbook.ts";
import { exists, list, text, type Importer, type ImportResult } from "./lib.ts";

const DIR = "docs/agent/playbook";
const TREE_DIRS = ["crates/", "tools/", "docs/", "tests/", "benches/", "examples/", "fuzz/", ".github/"];
const PATH_TRIM = /(:re:.*|:@[\w:.-]+|:\d+([-+]\d+)?|[.,;:)\]'"]+)$/;
const BRACES = /^([^{}]*)\{([^{}]+)\}([^{}]*)$/;
const EXPIRY = /\[until:\s*(test|exists|gone|rule)\s+([^\]]+?)\s*\]\s*$/;

function isFile(root: string, path: string): boolean {
  try {
    return statSync(join(root, path)).isFile();
  } catch {
    return false;
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
  const out = namedPaths(bullet).filter((p) => exists(root, p));
  for (const [, raw] of bullet.matchAll(/`([^`\s/]+\.[A-Za-z]+)`/g)) if (isFile(root, raw!)) out.push(raw!);
  if (until && (until.kind === "gone" || until.kind === "exists")) {
    const path = until.arg.split(":")[0]!.trim().replace(/\\/g, "/");
    if (path !== "" && exists(root, path)) out.push(path);
  }
  return [...new Set(out)];
}

const unwrap = (s: string) => s.replace(/\s*\n\s*/g, " ").trim();

/**
 * One bullet file's text as its record's value, or why it cannot be one. The lead ends at the first
 * `**` outside a code span, so a lead that quotes bold Markdown in backticks is read whole.
 */
export function bulletValue(root: string, src: string): { value: { lead: string; body: string; files: string[]; until: { kind: string; arg: string } } } | { reason: string } {
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

export const playbook: Importer = {
  name: "playbook",
  read(root) {
    const out: ImportResult = { records: [], unread: [], files: 0 };
    const sections = load(playbookSection, root)
      .filter((r) => r.issues.length === 0)
      .map((r) => ({ id: r.id, value: r.value as { title: string; order: number } }))
      .sort((a, b) => a.value.order - b.value.order);
    for (const s of sections) {
      out.files++;
      out.records.push({ type: playbookSection, id: s.id, value: s.value, from: pathOf(playbookSection, s.id) });
    }
    const known = new Set(sections.map((s) => s.id));
    for (const dir of list(root, DIR)) {
      if (dir.endsWith(".md")) continue;
      if (!known.has(dir)) {
        out.unread.push({ path: `${DIR}/${dir}`, reason: "has no section record under `data/playbook/`" });
        continue;
      }
      for (const name of list(root, `${DIR}/${dir}`)) {
        const path = `${DIR}/${dir}/${name}`;
        if (!name.endsWith(".md")) continue;
        out.files++;
        const src = text(root, path).trim();
        if (/^- \*\*/.test(src) && /\n- /.test(src)) out.unread.push({ path, reason: "holds more than one bullet" });
        const got = bulletValue(root, src);
        if ("reason" in got) {
          out.unread.push({ path, reason: got.reason });
          continue;
        }
        out.records.push({ type: playbookBullet, id: `${dir}/${name.slice(0, -".md".length)}`, value: got.value, from: path });
      }
    }
    return out;
  },
};
