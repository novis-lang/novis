// The old driver's writes to its own goal data: `data/chain.json`, `data/goals/` and the goal prose under
// `docs/agent/goals/`. Only the old driver calls these. `lib/chain.ts` reads looper's goal files instead.

import { existsSync, rmSync } from "node:fs";
import { join } from "node:path";
import { ROOT } from "../lib/paths.ts";
import { load, write } from "../lib/store.ts";
import { chain as chainType } from "../schema/chain.ts";

/** `data/chain.json`, or an empty chain when it is absent. */
function chainRecord(root: string): { id: string; live: string | null; goals: string[] } {
  const got = load(chainType, root)[0];
  return { id: got?.id ?? "chain", live: got?.value.live ?? null, goals: got?.value.goals ?? [] };
}

/** The slug `data/chain.json` names live, or null when it names none on its chain. */
export function oldLive(root: string = ROOT): string | null {
  const c = chainRecord(root);
  return c.live !== null && c.goals.includes(c.live) ? c.live : null;
}

/** Makes `slug` the live goal. The caller commits `data/chain.json`, which is the whole of a goal switch on disk. */
export function setLive(slug: string, root: string = ROOT): string {
  const c = chainRecord(root);
  if (!c.goals.includes(slug)) throw new Error(`${slug} is not on the chain`);
  return write(chainType, c.id, { live: slug, goals: c.goals }, root).path;
}

/** A goal's three files, repo-relative: its prose, its record and its handoff record. */
export function goalFiles(slug: string): string[] {
  return [`docs/agent/goals/${slug}.md`, `data/goals/${slug}.json`, `data/goals/${slug}.handoff.json`];
}

/**
 * The goal switch on disk, once goal `from` is reached: its three files are deleted, its slug leaves
 * `goals`, and `to` becomes `live`. Returns every path it changed, repo-relative, for the caller to
 * commit. Throws when either goal is not on the chain, and changes nothing then.
 */
export function leaveGoal(from: string, to: string, root: string = ROOT): string[] {
  const c = chainRecord(root);
  for (const slug of [from, to]) if (!c.goals.includes(slug)) throw new Error(`${slug} is not on the chain`);
  const gone = goalFiles(from).filter((p) => existsSync(join(root, p)));
  for (const p of gone) rmSync(join(root, p));
  const chain = write(chainType, c.id, { live: to, goals: c.goals.filter((s) => s !== from) }, root).path;
  return [chain, ...gone];
}
