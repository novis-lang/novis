// `bun nv chain`: the loop's goal chain, which is `data/chain.json`: `goals`, a list of slugs, and `live`,
// the goal the driver works on. A goal's position is its place in that list, printed as `N of M`, so no
// file is renamed and no number is written. `--new`, `--move` and `--remove` edit the list in that one
// file, then render `docs/agent/goal-plan.md` again for the new order, and never touch `live`, which only
// the goal switch moves; a goal's record and prose are written by hand around it, and `--check` reports
// what is still missing.
//
// `--check` is whether the driver can walk the chain, as a query over the records.
//
// A problem is something that stops the run when the chain reaches it:
//   - the chain is empty;
//   - a goal record the chain does not name, which the driver never reaches;
//   - a chain, goal or handoff record fails its schema, a foreign key or an invariant (`nv check`'s
//     findings, narrowed to those files);
//   - a goal has no prose under `docs/agent/goals/`;
//   - a goal with checks has no handoff record;
//   - a retired goal, one whose record has no checks, stands at or behind the installed goal;
//   - a goal pinned `position: last` has an unpinned goal behind it;
//   - a side goal with checks has no prose, no `# Side goal` H1 or no handoff record;
//   - prose names a goal by its number, which is a position and moves;
//   - a goal's `context` names a shape or a playbook bullet that is not there (`nv orient`'s
//     `manifestFindings`, over every goal not retired and every side goal with checks).
// A note is something a reader misses: a goal not yet reached with no `## Why here` or no
// `## Standing decisions`, a goal with no row in the goals README, prose still carrying `TODO`, and
// a `context` entry `manifestFindings` only reports.
//
// With no problem it prints two lines, one per invariant a floor check holds it to: every goal is
// walkable and every manifest resolves, and the chain already names every goal record. Exits 0 when
// there is no problem, 1 when there is one, and 2 on a bad argument.

import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { type ChainGoal, chainGoals, h1Of, liveGoal } from "../lib/chain.ts";
import { Index } from "../lib/index.ts";
import { ROOT } from "../lib/paths.ts";
import { run as proc } from "../lib/proc.ts";
import { load, write } from "../lib/store.ts";
import { chain as chainType } from "../schema/chain.ts";
import { goal as goalType, sideGoal as sideGoalType } from "../schema/goal.ts";
import { RECORDS } from "../schema/index.ts";
import { writeGoalPlan } from "../renderers/goal-plan.ts";
import { type GoalValue, manifestFindings } from "./orient.ts";

export const summary = "the loop's goal chain, over data/chain.json: nv chain --check | --new | --move | --remove";

const GOALS = "docs/agent/goals";
const README = `${GOALS}/README.md`;

/** A goal named by its number. Every one is a defect, because the number is a position. */
export const NUMBER_CITE = /(?<![`\w])[Gg]oals?\s+\d+/g;
/** A goal's own header, the one place a number still stands until the prose is renamed by slug. */
const SHOWN_HEADER = /#\s*Loop goal \d+|#\s*Goal \d+ --|\*\*Goal \d+ —/g;
export const OWN_HEADER = /^#\s*Loop goal \d+|^#\s*Goal \d+ --|^\*\*Goal \d+ —/;
const BINARY = new Set([".png", ".jpg", ".jpeg", ".ico", ".svg", ".lock", ".woff", ".woff2"]);

function read(path: string): string | null {
  const abs = join(ROOT, path);
  return existsSync(abs) ? readFileSync(abs, "utf8").replace(/\r\n/g, "\n") : null;
}

/** Every file git would show: tracked, plus untracked and not ignored, so a goal still being written counts. */
async function shownFiles(): Promise<string[]> {
  const r = await proc(["git", "ls-files", "-z", "--cached", "--others", "--exclude-standard"], { cwd: ROOT, timeoutMs: 60_000 });
  if (r.code !== 0) throw new Error(`git ls-files: exit ${r.code}\n${r.stderr.trim()}`);
  return [...new Set(r.stdout.split("\0").filter((p) => p.length > 0))];
}

/** Every `goal 29` in a text file, as `path:line  text`. The goals README lists the chain and is exempt. */
async function numberCitations(): Promise<string[]> {
  const out: string[] = [];
  const utf8 = new TextDecoder("utf-8", { fatal: true });
  for (const path of await shownFiles()) {
    if (path === README) continue;
    const dot = path.lastIndexOf(".");
    if (dot >= 0 && BINARY.has(path.slice(dot).toLowerCase())) continue;
    let text: string;
    try {
      text = utf8.decode(readFileSync(join(ROOT, path)));
    } catch {
      continue;
    }
    if (!NUMBER_CITE.test(text)) continue;
    NUMBER_CITE.lastIndex = 0;
    text.split("\n").forEach((line, i) => {
      if (OWN_HEADER.test(line)) return;
      const shown = [...line.matchAll(SHOWN_HEADER)].map((m) => [m.index!, m.index! + m[0].length] as const);
      for (const m of line.matchAll(NUMBER_CITE)) {
        if (!shown.some(([a, b]) => a <= m.index! && m.index! < b)) out.push(`${path}:${i + 1}  ${m[0]}`);
      }
    });
  }
  return out;
}

function one<T>(rows: Record<string, unknown>[], col: string): T[] {
  return rows.map((r) => r[col] as T);
}

async function check(): Promise<number> {
  const problems: string[] = [];
  const notes: string[] = [];
  const goals = chainGoals();
  const live = liveGoal(goals);
  const where = (g: ChainGoal) => `goal \`${g.slug}\` (${g.num} of ${goals.length})`;
  const ahead = (g: ChainGoal) => live === null || g.num > live.num;
  // A feature-proof goal's target is the features it lists, so it owes no reason for its place and no
  // row in the goals README.
  const provesFeatures = (text: string) => /\n## The target\n(?:(?!\n## )[^])*rule:testing\/feature-proofs/.test(text);

  if (goals.length === 0) problems.push("data/chain.json holds no goal -- the driver has nothing to walk");

  const index = new Index({ types: RECORDS });
  let handoffs: Set<string>, pinned: Set<string>, sideGoals: string[], sideHandoffs: Set<string>;
  try {
    index.refresh();
    for (const f of index.check()) {
      if (f.path === "data/chain.json" || f.path.startsWith("data/goals/")) problems.push(`${f.path}: ${f.message}`);
    }
    handoffs = new Set(one<string>(index.query("SELECT id FROM handoff"), "id"));
    pinned = new Set(one<string>(index.query("SELECT id FROM goal WHERE position = 'last'"), "id"));
    sideGoals = one<string>(
      index.query("SELECT id FROM side_goal WHERE EXISTS (SELECT 1 FROM side_goal__checks c WHERE c.owner = side_goal.id) ORDER BY id"),
      "id",
    );
    sideHandoffs = new Set(one<string>(index.query("SELECT id FROM side_handoff"), "id"));
  } finally {
    index.close();
  }

  // The other half of walkable: the driver can run the checks, and the session gets the pack. A manifest
  // naming a heading that is not there costs nothing until the chain reaches the goal, and then costs
  // one session the section it needed.
  const audit = (value: unknown, record: string, prose: string | null) => {
    const found = manifestFindings(value as GoalValue, record, prose);
    problems.push(...found.problems);
    notes.push(...found.notes);
  };
  const records = new Map(load<any>(goalType).map((r) => [r.id as string, r.value]));
  const sideRecords = new Map(load<any>(sideGoalType).map((r) => [r.id as string, r.value]));
  const named = new Set(goals.map((g) => g.slug));
  for (const slug of [...records.keys()].sort()) {
    if (!named.has(slug)) problems.push(`data/goals/${slug}.json: a goal record data/chain.json does not name -- the driver never reaches it; \`nv chain --new\` places it`);
  }

  const readme = read(README) ?? "";
  for (const g of goals) {
    if (g.md === null) {
      problems.push(`${where(g)}: no prose under ${GOALS}/ -- a session is started on that file`);
      continue;
    }
    const text = read(g.md) ?? "";
    if (g.retired) {
      if (live !== null && g.num >= live.num) {
        problems.push(
          `${where(g)}: is retired but the run stands on \`${live.slug}\` -- only a goal the run has LEFT ` +
            "may be retired, since retiring is what says its checks are already somebody's floor",
        );
      }
      continue;
    }
    if (!handoffs.has(g.slug)) problems.push(`${where(g)}: no data/goals/${g.slug}.handoff.json -- the first session has no group to take`);
    if (records.has(g.slug)) audit(records.get(g.slug), `data/goals/${g.slug}.json`, g.md);
    if (ahead(g) && !text.includes("\n## Why here") && !provesFeatures(text)) {
      notes.push(`${g.md}: no \`## Why here\` section. The order is a dependency chain and that section is the only home of the reason this goal sits where it does`);
    }
    if (ahead(g) && !text.includes("\n## Standing decisions")) {
      notes.push(`${g.md}: no \`## Standing decisions\` section -- loop-authoring.md § 5 is where a goal pre-authorizes the calls its stages reach`);
    }
    if (text.includes("TODO")) notes.push(`${g.md}: still carries TODO markers -- a scaffold nobody has filled in yet`);
    if (!provesFeatures(text) && !readme.includes(g.slug)) {
      notes.push(`${README}: no row for \`${g.slug}\` -- the table there is what a reader reads instead of the directory`);
    }
  }

  // `position: last` holds a goal behind every unpinned one, so the pinned goals form the chain's tail.
  let tail = goals.length;
  while (tail > 0 && pinned.has(goals[tail - 1]!.slug)) tail--;
  for (const g of goals.slice(0, tail)) {
    if (pinned.has(g.slug) && ahead(g)) {
      problems.push(`${where(g)}: its record says \`position: last\` and ${goals.length - g.num} goal(s) sit behind it`);
    }
  }

  for (const slug of sideGoals) {
    const md = `${GOALS}/side/${slug}.md`;
    const text = read(md);
    if (text === null) {
      problems.push(`side goal \`${slug}\`: no ${md}`);
      continue;
    }
    if (!h1Of(text).startsWith("# Side goal ")) problems.push(`${md}: its H1 is ${JSON.stringify(h1Of(text).slice(0, 60))}; a side goal's opens \`# Side goal — \``);
    if (!sideHandoffs.has(slug)) problems.push(`side goal \`${slug}\`: no data/goals/side/${slug}.handoff.json`);
    if (sideRecords.has(slug)) audit(sideRecords.get(slug), `data/goals/side/${slug}.json`, md);
  }

  const stale = await numberCitations();
  if (stale.length > 0) {
    problems.push(
      `${stale.length} place(s) name a goal by its NUMBER. A goal is named by its slug -- \`goal \`xml-tree\`\`, ` +
        "never `goal 29` -- because a number is a position and moves whenever anything is inserted in front of it:",
    );
    for (const s of stale.slice(0, 15)) problems.push(`    ${s}`);
    if (stale.length > 15) problems.push(`    ... and ${stale.length - 15} more`);
  }

  for (const p of problems) console.log(`  ${p}`);
  for (const n of notes) console.log(`  note: ${n}`);
  if (problems.length > 0) {
    console.log("");
    console.log(`chain: ${problems.length} problem(s), ${notes.length} note(s) over ${goals.length} goals`);
    return 1;
  }
  const at = live === null ? "no goal is installed" : `the run stands on \`${live.slug}\`, ${live.num} of ${goals.length}`;
  console.log(`chain: every goal is walkable and every manifest resolves -- ${goals.length} goals, ${at}` + (notes.length > 0 ? `, ${notes.length} note(s) above` : ""));
  console.log(`chain: ${records.size} goal record(s) on disk, and data/chain.json already names every one of them -- nothing appended`);
  console.log("       `bun nv plan --check` is the other half: milestone tags and `Carried by` cells.");
  return 0;
}

const USAGE =
  "nv chain --check | --new <slug> <where> | --move <slug> <where> | --remove <slug>, " +
  "where <where> is --after <goal>, --before <goal>, --to <position>, --next or --end";

/** The value after `flag`, or undefined when the flag is absent. Throws when the flag has no value. */
function valueOf(args: string[], flag: string): string | undefined {
  const i = args.indexOf(flag);
  if (i < 0) return undefined;
  const v = args[i + 1];
  if (v === undefined || v.startsWith("--")) throw new Error(`${flag} needs a value`);
  return v;
}

/**
 * The index in `order` (with the goal being placed already taken out) that the goal lands at. A goal
 * is named by its slug, or by its position as the list stood before the edit.
 */
function landing(args: string[], order: string[], before: string[], live: string | null): number {
  const indexOf = (flag: string, name: string): number => {
    const slug = /^\d+$/.test(name) ? before[Number(name) - 1] : name;
    const i = slug === undefined ? -1 : order.indexOf(slug);
    if (i < 0) throw new Error(`${flag} ${name}: no such goal on the chain`);
    return i;
  };
  const after = valueOf(args, "--after");
  const beforeGoal = valueOf(args, "--before");
  const to = valueOf(args, "--to");
  const picked = [after, beforeGoal, to, args.includes("--next") || undefined, args.includes("--end") || undefined];
  if (picked.filter((p) => p !== undefined).length !== 1) throw new Error("say where, with exactly one of --after, --before, --to, --next and --end");
  if (after !== undefined) return indexOf("--after", after) + 1;
  if (beforeGoal !== undefined) return indexOf("--before", beforeGoal);
  if (to !== undefined) {
    const n = Number(to);
    if (!Number.isInteger(n) || n < 1 || n > order.length + 1) throw new Error(`--to ${to}: a position is 1 to ${order.length + 1}`);
    return n - 1;
  }
  if (args.includes("--next")) {
    if (live === null) throw new Error("--next: no goal is installed, so there is nothing to land after");
    return order.indexOf(live) + 1;
  }
  return order.length;
}

/**
 * Writes the new order, after refusing any edit that changes what the run has walked: a goal at or in
 * front of the installed one is its floor, and nothing lands in front of it either. A goal not pinned
 * `position: last` never lands inside the pinned tail, and `--end` puts it just in front of that tail.
 */
function edit(args: string[]): number {
  const goals = chainGoals();
  const live = liveGoal(goals);
  const before = goals.map((g) => g.slug);
  const loaded = load(chainType)[0];
  const id = loaded?.id ?? "chain";
  const pinned = new Set(load(goalType).filter((g) => g.value.position === "last").map((g) => g.id));
  const floorEnd = live === null ? 0 : live.num;

  const created = valueOf(args, "--new");
  const moved = valueOf(args, "--move");
  const removed = valueOf(args, "--remove");
  const slug = (created ?? moved ?? removed)!;
  if ([created, moved, removed].filter((x) => x !== undefined).length !== 1) throw new Error("takes one of --new, --move and --remove");

  const order = before.slice();
  const at = order.indexOf(slug);
  if (created !== undefined) {
    if (!/^[a-z0-9]+(?:-[a-z0-9]+)*$/.test(slug)) throw new Error(`--new ${slug}: a slug is lower-case words joined by hyphens`);
    if (at >= 0) throw new Error(`--new ${slug}: already on the chain, at ${at + 1} of ${order.length}`);
  } else {
    if (at < 0) throw new Error(`${moved !== undefined ? "--move" : "--remove"} ${slug}: not on the chain`);
    if (at < floorEnd) throw new Error(`${slug} is ${at + 1} of ${order.length}, at or in front of the installed goal \`${live!.slug}\`, so its checks are the floor`);
    order.splice(at, 1);
  }

  if (removed === undefined) {
    let i = landing(args, order, before, live?.slug ?? null);
    if (i < floorEnd) throw new Error(`that lands at ${i + 1}, in front of the installed goal \`${live!.slug}\`, which only a goal already walked may do`);
    let tail = order.length;
    while (tail > 0 && pinned.has(order[tail - 1]!)) tail--;
    if (!pinned.has(slug)) {
      if (args.includes("--end")) i = tail;
      else if (i > tail) throw new Error(`that lands behind \`${order[tail]}\`, whose record says \`position: last\`; give ${slug} the pin too, or land it in front`);
    }
    order.splice(i, 0, slug);
  }

  const liveSlug = loaded?.value.live ?? order[0];
  if (liveSlug === undefined) throw new Error("the chain would hold no goal");
  write(chainType, id, { live: liveSlug, goals: order });
  const record = `data/goals/${slug}.json`;
  if (removed !== undefined) {
    console.log(`chain: \`${slug}\` removed, ${order.length} goals left`);
    if (existsSync(join(ROOT, record))) console.log(`       ${record} is still on disk, and \`nv check\` reports it until it is deleted or put back`);
  } else {
    const n = order.indexOf(slug) + 1;
    console.log(`chain: \`${slug}\` is ${n} of ${order.length}` + (moved !== undefined ? `, from ${at + 1}` : ""));
    if (!existsSync(join(ROOT, record))) console.log(`       ${record} is not written yet, and \`nv check\` reports the chain until it is`);
  }
  const plan = writeGoalPlan();
  if (plan) console.log(`       ${plan} rendered again for the new order`);
  return 0;
}

export async function run(args: string[]): Promise<number> {
  if (args.length === 1 && args[0] === "--check") return check();
  if (!["--new", "--move", "--remove"].some((f) => args.includes(f))) {
    console.error(`usage: ${USAGE}`);
    return 2;
  }
  try {
    return edit(args);
  } catch (e) {
    console.error(`nv chain: ${(e as Error).message}`);
    return 2;
  }
}
