// The playbook's legacy homes: one bullet per file under `docs/agent/playbook/<section>/`, and the
// section titles and their order in `tools/playbook.py`'s `SECTIONS`. A section becomes
// `data/playbook/<section>.json` and a bullet `data/playbook/<section>/<slug>.json`.
//
// A bullet's `lead` is its bold opening and `body` the rest before the trailer, each unwrapped to one
// line, since the rendered file wraps them again. `files` is what `tools/playbook.py`'s `anchors`
// finds: the tree paths the bullet names in backticks that exist, and a `gone` or `exists` trailer's.

import { statSync } from "node:fs";
import { join } from "node:path";
import { playbookBullet, playbookSection } from "../schema/playbook.ts";
import { exists, list, text, type Importer, type ImportResult } from "./lib.ts";

const DIR = "docs/agent/playbook";
const TOOL = "tools/playbook.py";
const TREE_DIRS = ["crates/", "tools/", "docs/", "tests/", "benches/", "examples/", "fuzz/", ".github/"];
const PATH_TRIM = /(:re:.*|:@[\w:.-]+|:\d+([-+]\d+)?|[.,;:)\]'"]+)$/;
const BRACES = /^([^{}]*)\{([^{}]+)\}([^{}]*)$/;
const EXPIRY = /\[until:\s*(test|exists|gone|rule|reviewed)\s+([^\]]+?)\s*\]\s*$/;

function isFile(root: string, path: string): boolean {
  try {
    return statSync(join(root, path)).isFile();
  } catch {
    return false;
  }
}

/** The files in the tree a bullet names, as `tools/playbook.py`'s `anchors` finds them. */
export function anchors(root: string, bullet: string, until: { kind: string; arg: string } | null): string[] {
  const out: string[] = [];
  for (const [, raw] of bullet.matchAll(/`([^`]+)`/g)) {
    const first = raw!.trim().split(/\s+/)[0] ?? "";
    const cand = first.replace(PATH_TRIM, "");
    if (!TREE_DIRS.some((d) => cand.startsWith(d)) || ["*", "<", "…", "..."].some((m) => cand.includes(m))) continue;
    const m = BRACES.exec(cand);
    const paths = m ? m[2]!.split(",").map((p) => p.trim()).filter((p) => p !== "").map((p) => `${m[1]}${p}${m[3]}`) : [cand];
    out.push(...paths.filter((p) => exists(root, p)));
  }
  for (const [, raw] of bullet.matchAll(/`([^`\s/]+\.[A-Za-z]+)`/g)) if (isFile(root, raw!)) out.push(raw!);
  if (until && (until.kind === "gone" || until.kind === "exists")) {
    const path = until.arg.split(":")[0]!.trim().replace(/\\/g, "/");
    if (path !== "" && exists(root, path)) out.push(path);
  }
  return [...new Set(out)];
}

const unwrap = (s: string) => s.replace(/\s*\n\s*/g, " ").trim();

/** One bullet file's text as its record's value, or why it cannot be one. */
export function bulletValue(root: string, src: string): { value: { lead: string; body: string; files: string[]; until: { kind: string; arg: string } } } | { reason: string } {
  const m = /^- \*\*([\s\S]+?)\*\*([\s\S]*)$/.exec(src);
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
    const out: ImportResult = { records: [], unread: [], files: 1 };
    const table = /^SECTIONS = \(\n([\s\S]*?)\n\)/m.exec(text(root, TOOL));
    const sections = table ? [...table[1]!.matchAll(/\("([a-z0-9-]+)", "([^"]+)"\)/g)].map((m) => [m[1]!, m[2]!] as const) : [];
    if (sections.length === 0) out.unread.push({ path: TOOL, reason: "has no `SECTIONS` table to read the section titles from" });
    sections.forEach(([dir, title], i) => {
      out.records.push({ type: playbookSection, id: dir, value: { title, order: i + 1 }, from: TOOL });
    });
    const known = new Set(sections.map(([dir]) => dir));
    for (const dir of list(root, DIR)) {
      if (dir.endsWith(".md")) continue;
      if (!known.has(dir)) {
        out.unread.push({ path: `${DIR}/${dir}`, reason: `is no section in ${TOOL}'s \`SECTIONS\`` });
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
