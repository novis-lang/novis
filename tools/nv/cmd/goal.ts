// `bun nv goal context --add <path>`: a session widens the live goal's manifest, the `context` that
// `orient` prints, by one module. The goal is the one `liveGoal` names, and the path goes into its
// record's `context.modules`.
//
// While `docs/agent/loop-goal.toml` exists the Python driver reads its `[context]` table, so the path is
// also added to that table's `modules` array. That file is the driver's, so the edit is to its text: one
// line is inserted, and every comment and blank line around it stays as it was.
//
// A path that is not on disk, or not inside the repository, is refused. A path already listed is left
// alone, in each of the two homes separately. Exits 0 on success or a no-op, 1 on a refusal, and 2 on a
// bad argument.

import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { isAbsolute, join, relative, resolve, sep } from "node:path";
import { parse as parseToml } from "smol-toml";
import { liveGoal } from "../lib/chain.ts";
import { ROOT } from "../lib/paths.ts";
import { load, pathOf, write } from "../lib/store.ts";
import { goal as goalType } from "../schema/goal.ts";

export const summary = "the live goal's record: nv goal context --add <path>";

const USAGE = "bun nv goal context --add <path>";
export const MANIFEST = "docs/agent/loop-goal.toml";

/** What `addModule` changed: the repo-relative path it was given, and each file it wrote. */
export interface Added {
  path: string;
  goal: string;
  written: string[];
}

/** `path` as the manifest spells it: repo-relative, forward slashes. Throws for one outside `root`. */
function repoPath(path: string, root: string): string {
  const rel = relative(root, resolve(root, path));
  if (rel === "" || rel.startsWith("..") || isAbsolute(rel)) throw new Error(`${path} is not inside the repository`);
  return rel.split(sep).join("/");
}

/** A value line of a multi-line array: a quoted string, then an optional comma and comment. */
const VALUE_LINE = /^(\s*"(?:[^"\\]|\\.)*")(\s*,)?(\s*(?:#.*)?)$/;

/**
 * `text`, a TOML manifest, with `path` appended to its `[context]` table's `modules` array. Null when
 * the array already lists it. The table and the key are created when absent. Throws when the edit does
 * not parse back to an array that lists `path`, so a manifest is never written half-edited.
 */
export function addToManifest(text: string, path: string): string | null {
  const listed = (parseToml(text).context as { modules?: unknown } | undefined)?.modules;
  if (Array.isArray(listed) && listed.includes(path)) return null;

  const eol = text.includes("\r\n") ? "\r\n" : "\n";
  const lines = text.split(eol);
  const quoted = JSON.stringify(path);
  let head = lines.findIndex((l) => /^\[context\]\s*(#.*)?$/.test(l));
  if (head < 0) {
    while (lines.length > 0 && lines[lines.length - 1] === "") lines.pop();
    lines.push("", "[context]");
    head = lines.length - 1;
    lines.push("");
  }
  let end = lines.findIndex((l, i) => i > head && /^\[/.test(l));
  if (end < 0) end = lines.length;
  const key = lines.findIndex((l, i) => i > head && i < end && /^modules\s*=/.test(l));

  if (key < 0) {
    lines.splice(head + 1, 0, `modules = [${quoted}]`);
  } else {
    const one = /^(modules\s*=\s*\[)(.*)(\]\s*(?:#.*)?)$/.exec(lines[key]!);
    if (one) {
      const inner = one[2]!.trim().replace(/,$/, "");
      lines[key] = `${one[1]}${inner === "" ? quoted : `${inner}, ${quoted}`}${one[3]}`;
    } else {
      const close = lines.findIndex((l, i) => i > key && /^\s*\]/.test(l));
      if (close < 0) throw new Error(`${MANIFEST}: [context] modules has no closing bracket`);
      let indent = "  ";
      for (let i = close - 1; i > key; i--) {
        const m = VALUE_LINE.exec(lines[i]!);
        if (!m) continue;
        indent = /^\s*/.exec(m[1]!)![0];
        if (!m[2]) lines[i] = `${m[1]},${m[3]}`;
        break;
      }
      lines.splice(close, 0, `${indent}${quoted},`);
    }
  }

  const out = lines.join(eol);
  const after = (parseToml(out).context as { modules?: unknown } | undefined)?.modules;
  if (!Array.isArray(after) || !after.includes(path)) throw new Error(`${MANIFEST}: the edit did not add ${path} to [context] modules`);
  return out;
}

/** Adds `path` to the live goal's `context.modules`, and to `MANIFEST`'s while that file exists. */
export function addModule(path: string, root: string = ROOT): Added {
  const rel = repoPath(path, root);
  if (!existsSync(join(root, rel))) throw new Error(`${rel} is not on disk`);
  const live = liveGoal(undefined, root);
  if (!live) throw new Error("no goal is live: the driver's pointer names none, and loop-goal.md matches no goal");
  const record = load(goalType, root).find((g) => g.id === live.slug);
  if (!record) throw new Error(`the live goal \`${live.slug}\` has no record at ${pathOf(goalType, live.slug)}`);
  if (goalType.schema.validate(record.value).length > 0) throw new Error(`${record.path} fails its schema; \`bun nv check\` names how`);

  const written: string[] = [];
  const modules = record.value.context.modules ?? [];
  if (!modules.includes(rel)) {
    write(goalType, live.slug, { ...record.value, context: { ...record.value.context, modules: [...modules, rel] } }, root);
    written.push(record.path);
  }
  const manifest = join(root, MANIFEST);
  if (existsSync(manifest)) {
    const edited = addToManifest(readFileSync(manifest, "utf8"), rel);
    if (edited !== null) {
      writeFileSync(manifest, edited);
      written.push(MANIFEST);
    }
  }
  return { path: rel, goal: live.slug, written };
}

export async function run(args: string[]): Promise<number> {
  if (args.length !== 3 || args[0] !== "context" || args[1] !== "--add") {
    console.error(`usage: ${USAGE}`);
    return 2;
  }
  try {
    const { path, goal, written } = addModule(args[2]!);
    if (written.length === 0) console.log(`goal: \`${path}\` is already in \`${goal}\`'s context`);
    else console.log(`goal: \`${path}\` added to \`${goal}\`'s context, in ${written.join(" and ")}`);
    return 0;
  } catch (e) {
    console.error(`nv goal: ${(e as Error).message}`);
    return 1;
  }
}
