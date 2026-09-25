// `bun nv goal context --add <path>`: a session widens the live goal's manifest, the `context` that
// `orient` prints, by one module. The goal is the one `data/chain.json` names live, and the path goes
// into its record's `context.modules`.
//
// A path that is not on disk, or not inside the repository, is refused. A path already listed is left
// alone. Exits 0 on success or a no-op, 1 on a refusal, and 2 on a bad argument.

import { existsSync } from "node:fs";
import { isAbsolute, join, relative, resolve, sep } from "node:path";
import { liveGoal } from "../lib/chain.ts";
import { ROOT } from "../lib/paths.ts";
import { load, pathOf, write } from "../lib/store.ts";
import { goal as goalType } from "../schema/goal.ts";

export const summary = "the live goal's record: nv goal context --add <path>";

const USAGE = "bun nv goal context --add <path>";

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

/** Adds `path` to the live goal's `context.modules`. */
export function addModule(path: string, root: string = ROOT): Added {
  const rel = repoPath(path, root);
  if (!existsSync(join(root, rel))) throw new Error(`${rel} is not on disk`);
  const live = liveGoal(undefined, root);
  if (!live) throw new Error("no goal is live: data/chain.json names none on the chain");
  const record = load(goalType, root).find((g) => g.id === live.slug);
  if (!record) throw new Error(`the live goal \`${live.slug}\` has no record at ${pathOf(goalType, live.slug)}`);
  if (goalType.schema.validate(record.value).length > 0) throw new Error(`${record.path} fails its schema; \`bun nv check\` names how`);

  const written: string[] = [];
  const modules = record.value.context.modules ?? [];
  if (!modules.includes(rel)) {
    write(goalType, live.slug, { ...record.value, context: { ...record.value.context, modules: [...modules, rel] } }, root);
    written.push(record.path);
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
