// The `Core` class table: `crates/nvs-stdlib/src/classes.rs`, which `registry.rs` re-exports as
// `CLASSES`. Each class is a `CoreClass` const in its own domain module, and the line directly above
// the const, past any attributes, is `// rank: N`. The table lists every ranked const by rank and
// then by path, so adding a class edits its own file and no shared one, and the order is the order of
// the website's `Core` index and of `nvs meta --json`. Ranks are spaced by ten, so a class can be put
// between two others without moving either.
//
// A const's path is its file's module path, `crate::a::b` for `src/a/b.rs` or `src/a/b/mod.rs`, with
// a segment dropped when the parent module re-exports it whole (`use self::b::*;`), so the path is the
// one the rest of the crate writes. Only a const at the top level of its file counts. A `CoreClass`
// const with no rank fails the render unless `UNRANKED` names it with its reason, so a class is never
// left out of the table by accident.

import { existsSync, readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { MARKER, type Output, type Renderer } from "../lib/render.ts";

export const CORE_CLASSES = "crates/nvs-stdlib/src/classes.rs";
const SRC = "crates/nvs-stdlib/src";

/** The `CoreClass` consts that are not in the table, by path, each with why. */
export const UNRANKED: Record<string, string> = {
  "crate::cursor::CLASS": "a runtime layout no program names, so it has no registry row (`crate::cursor`'s module doc)",
};

const CONST_RE = /^(?:pub(?:\([a-z]+\))? )?const ([A-Z][A-Z0-9_]*): CoreClass = /;
const RANK_RE = /^\/\/ rank: (\d+)$/;

export interface RankedClass {
  path: string;
  rank: number;
}

/** Every `.rs` file under `dir`, repo-relative with forward slashes, the table itself aside. */
function rustFiles(root: string, dir: string): string[] {
  const out: string[] = [];
  for (const e of readdirSync(join(root, dir), { withFileTypes: true })) {
    const path = `${dir}/${e.name}`;
    if (e.isDirectory()) out.push(...rustFiles(root, path));
    else if (e.name.endsWith(".rs") && path !== CORE_CLASSES) out.push(path);
  }
  return out.sort();
}

/** The module path `crate::…` the rest of the crate writes for the items of `file`. */
export function modulePath(root: string, file: string): string {
  const segments = file.slice(SRC.length + 1).replace(/\.rs$/, "").split("/");
  if (segments.at(-1) === "mod" || segments.at(-1) === "lib") segments.pop();
  for (let i = segments.length - 1; i >= 1; i--) {
    const parent = segments.slice(0, i).join("/");
    const parentFile = [`${SRC}/${parent}/mod.rs`, `${SRC}/${parent}.rs`].find((p) => existsSync(join(root, p)));
    if (!parentFile) continue;
    const reexport = new RegExp(`^(?:pub(?:\\([a-z]+\\))? )?use self::${segments[i]}::\\*;$`, "m");
    if (reexport.test(readFileSync(join(root, parentFile), "utf8"))) segments.splice(i, 1);
  }
  return ["crate", ...segments].join("::");
}

/** Every ranked class, sorted. Throws naming each `CoreClass` const that has no rank and no reason. */
export function rankedClasses(root: string): RankedClass[] {
  const ranked: RankedClass[] = [];
  const missing: string[] = [];
  for (const file of rustFiles(root, SRC)) {
    const lines = readFileSync(join(root, file), "utf8").split(/\r?\n/);
    let modPath: string | undefined;
    lines.forEach((line, at) => {
      const m = CONST_RE.exec(line);
      if (!m) return;
      modPath ??= modulePath(root, file);
      const path = `${modPath}::${m[1]}`;
      let above = at - 1;
      while (above >= 0 && lines[above]!.startsWith("#[")) above--;
      const rank = above >= 0 ? RANK_RE.exec(lines[above]!) : null;
      if (rank) ranked.push({ path, rank: Number(rank[1]) });
      else if (!(path in UNRANKED)) missing.push(`${path} (${file}:${at + 1})`);
    });
  }
  if (missing.length > 0) {
    throw new Error(
      `${missing.length} \`CoreClass\` const(s) have no \`// rank: N\` line above them: ${missing.join(", ")}. ` +
        `Give each a rank, or name it in UNRANKED with the reason it is not a registered class.`,
    );
  }
  return ranked.sort((a, b) => a.rank - b.rank || (a.path < b.path ? -1 : a.path > b.path ? 1 : 0));
}

export function renderCoreClasses(root: string): Output[] {
  const rows = rankedClasses(root).map((c) => `    ${c.path},`);
  const text = `// ${MARKER}. \`tools/nv/renderers/core-classes.ts\` writes it from the
// \`// rank: N\` line above each \`CoreClass\` const in this crate.

use crate::registry::CoreClass;

/// Every \`Core\` class the compiler knows, by rank and then by path.
///
/// The single home for "does \`Core\\X::y\` exist": a class not listed here is
/// not a class the compiler resolves. Each domain module declares its own
/// \`CoreClass\` const beside the implementations it registers, with a
/// \`// rank: N\` line above it that places it here, and \`bun nv render\`
/// writes this list. So no line here is written by hand. The order
/// is the order of the website's \`Core\` index and of \`nvs meta --json\`.
pub const CLASSES: &[CoreClass] = &[
${rows.join("\n")}
];
`;
  return [{ path: CORE_CLASSES, text }];
}

export const coreClasses: Renderer = {
  name: "core-classes",
  render: (root) => renderCoreClasses(root),
};
