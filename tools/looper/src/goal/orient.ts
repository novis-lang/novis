// The orientation pack, as `crates/looper-core/src/orient.rs` builds it: everything a session needs to
// start on the live goal, in one message. Its order and wording are pinned byte for byte by the golden
// packs in `kit/fixtures/orient/`, and the head comment of `crates/looper-core/src/orient.rs` names the
// sections.
//
// - `loadPack` reads the sections into a `Pack`, one field each, and `packText` renders the text. The
//   `--json` document of `orient` is the `Pack`, with looper's field names.
// - The project's sections, the output of `[orient] extra`, come in as `extra`. `../cli/orient.ts`
//   runs those commands, because this module starts no process.
// - `harvested` answers the driver's other question of the tree: whether a reached goal's findings are
//   written.
//
// The bounds are looper's: at most `MAX_ANCHORS` code windows of `WINDOW` lines either side of their
// anchor, at most `HEAD_CAP` UTF-16 units of each context file's head, and at most `[orient] traps`
// bullets. The handoff, the stages and the playbook's head are printed whole.

import { readFileSync } from "node:fs";
import { extname, join } from "node:path";
import type { Config } from "./config.ts";
import { expand as expandGlob, isGlob } from "./glob.ts";
import {
  type Chain,
  type Check,
  type Goal,
  gateStage,
  liveGoal,
  liveStage,
  loadChain,
  loadGoal,
  sideGoalOf,
  type Stage,
} from "./model.ts";
import { bulletPaths, loadPlaybook, type Playbook } from "./playbook.ts";
import { anchors as anchorsIn, items, names, section, sectionsOf } from "./sections.ts";

/** Lines of code printed either side of an anchor. */
export const WINDOW = 12;
/** Anchors whose code the pack prints, the first ones in `### Next`. */
export const MAX_ANCHORS = 12;
/** Lines of a Markdown file's head. */
export const MD_HEAD = 40;
/** UTF-16 units of one context file's head, at most. */
export const HEAD_CAP = 6000;
/** Lines of a Rust file searched for its `//!` doc. */
const RUST_HEAD = 400;
/** The driver's run directory under the project root. */
export const RUN_DIR = ".loop";

/**
 * The run directory of the loop this process belongs to in the checkout at `root`: `RUN_DIR`, or
 * `RUN_DIR/side-<slug>` for a side loop in the project root, so it never reads or clears the main loop's
 * reports.
 */
export function runDir(root: string): string {
  const own = sideGoalOf();
  return own === undefined ? join(root, RUN_DIR) : join(root, RUN_DIR, `side-${own}`);
}
/** The command prefix the pack tells a session to lint, check and verify with. */
export const TOOL = "looper";
/** The line a `usize` parse of an anchor's line gives when it overflows, as looper prints it. */
const LINE_MAX = 18446744073709551615n;

/** A file's text with LF line endings, or `undefined` when it cannot be read. */
function read(path: string): string | undefined {
  try {
    return readFileSync(path).toString("utf8").replaceAll("\r\n", "\n");
  } catch {
    return undefined;
  }
}

/** A file's head: a Rust `//!` block, a leading `//` block, or a Markdown file's first lines. */
export function headOf(root: string, rel: string): string {
  const text = read(join(root, rel));
  if (text === undefined) return "(does not exist yet)";
  const lines = text.split("\n");
  const head: string[] = [];
  const extension = extname(rel).slice(1);
  if (extension === "rs") {
    for (const l of lines.slice(0, RUST_HEAD)) {
      if (l.startsWith("//!")) head.push(l);
      else if (!(l.trim() === "" || l.startsWith("#!"))) break;
    }
  } else if (["ts", "tsx", "js", "wgsl", "css"].includes(extension)) {
    for (const l of lines) {
      if (l.startsWith("//")) head.push(l);
      else if (head.length > 0 || l.trim() !== "") break;
    }
  } else {
    head.push(...lines.slice(0, MD_HEAD));
  }
  const out = head.join("\n");
  if (out === "") return `(no leading doc; ${lines.length} lines)`;
  if (out.length <= HEAD_CAP) return out;
  // A surrogate pair that straddles the cap is cut whole, as looper cuts before its char.
  const high = out.charCodeAt(HEAD_CAP - 1);
  const at = high >= 0xd800 && high <= 0xdbff ? HEAD_CAP - 1 : HEAD_CAP;
  return `${out.slice(0, at)}\n[...]`;
}

/**
 * The paths a context list names, each glob replaced by the files it matches (or kept when it matches
 * none), each path once.
 */
function expand(root: string, patterns: string[]): string[] {
  const out: string[] = [];
  for (const p of patterns) {
    const found = isGlob(p) ? expandGlob(root, p) : [];
    for (const f of found.length === 0 ? [p] : found) if (!out.includes(f)) out.push(f);
  }
  return out;
}

/** An anchor of `### Next`: its path, and its line as looper's `usize` parse reads it. */
export interface Anchor {
  path: string;
  line: bigint;
}

/** The `path:line` anchors in the open items of the handoff's `### Next`, in order. */
export function anchors(goal: Goal): Anchor[] {
  const handoff = section(sectionsOf(goal.body, 2), "Handoff")?.text ?? "";
  const next = section(sectionsOf(handoff, 3), "Next")?.text ?? "";
  const out: Anchor[] = [];
  for (const item of items(next).filter((i) => !i.done)) {
    for (const m of anchorsIn(item.text)) {
      const colon = m.lastIndexOf(":");
      if (colon < 0) continue;
      const line = BigInt(m.slice(colon + 1));
      out.push({ path: m.slice(0, colon), line: line > LINE_MAX ? LINE_MAX : line });
    }
  }
  return out;
}

/** The lines either side of `line`, numbered, under a `path:from-to` heading. */
function codeWindow(root: string, path: string, line: bigint): string {
  const text = read(join(root, path));
  if (text === undefined) return `${path}:${line} (does not exist yet)`;
  const lines = text.split("\n");
  const w = BigInt(WINDOW);
  const below = line > w ? line - w : 0n;
  const from = below < 1n ? 1n : below;
  const above = line + w > LINE_MAX ? LINE_MAX : line + w;
  const to = BigInt(lines.length) < above ? BigInt(lines.length) : above;
  const body =
    from <= to ? lines.slice(Number(from) - 1, Number(to)).map((l, i) => `${String(Number(from) + i).padStart(5)}  ${l}`) : [];
  return `${path}:${from}-${to}\n${body.join("\n")}`;
}

/** The `### <id>. <title>` section of `## Stages`, heading included, or nothing. */
function stageSection(goal: Goal, id: number): string {
  const stages = section(sectionsOf(goal.body, 2), "Stages")?.text ?? "";
  const subs = sectionsOf(stages, 3);
  const name = names(subs).find((n) => n.startsWith(`${id}. `));
  const s = name === undefined ? undefined : section(subs, name);
  return s === undefined ? "" : `### ${s.name}\n\n${s.text}`;
}

/** One acceptance check as the pack lists it. */
function checkLine(c: Check): string {
  const stage = c.stage === undefined ? "-" : String(c.stage);
  const reach = c.reach === true ? ", reach" : "";
  const tests = c.tests === undefined ? "" : ` must run: ${c.tests.join(", ")}`;
  return `- \`${c.id}\` (stage ${stage}${reach}): ${c.name ?? ""} → \`${c.run.join(" ")}\`${tests}`;
}

/** What the driver left in the run directory: an interrupted session, the last acceptance sweep and the operator's notes. */
function runSection(root: string): string {
  const out: string[] = [];
  const run = runDir(root);
  const interrupted = nonEmpty(read(join(run, "interrupted.json")), (t) => t);
  const last = nonEmpty(read(join(run, "last-sweep.md")), (t) => t);
  const notes = nonEmpty(read(join(run, "notes.md")), (t) => t.trim());
  if (interrupted !== undefined) {
    out.push(
      `The last session was cut off before it wrapped. The driver committed its own unfinished paths as WIP (not verified). Read that commit first, then continue, amend or revert it:\n${interrupted}`,
    );
  }
  if (last !== undefined) out.push(`What the acceptance sweep after the last session found. Fix these first:\n${last}`);
  if (notes !== undefined) out.push(`Notes from the operator, given while no session could receive them:\n${notes}`);
  if (interrupted === undefined && last === undefined && notes === undefined) out.push("Nothing to report from the last session.");
  return out.join("\n");
}

/** The text, unless `seen` of it is empty. */
function nonEmpty(text: string | undefined, seen: (t: string) => string): string | undefined {
  return text === undefined || seen(text) === "" ? undefined : text;
}

/** Where the live goal `live` stands: the goals ahead of it on the chain, or off the chain on its branch or in the root. */
function place(chain: Chain, live: string): string {
  if (sideGoalOf() === live) return "\nA side goal, live for its own loop in the project root and off the chain.";
  if (chain.chain[0] !== live) return "\nA side goal, live on its own branch and off the chain.";
  const ahead = chain.chain.slice(1);
  return `\n${ahead.length} goal(s) ahead on the chain: ${ahead.length === 0 ? "none" : ahead.join(", ")}.`;
}

/** The pack, one field per section, each holding the body its text prints. */
export interface Pack {
  /** RUN. */
  run: string;
  /** Every section after RUN, or null when the chain is empty. */
  live: Live | null;
}

/** The sections of a pack whose chain has a live goal, in looper's field order. */
export interface Live {
  slug: string;
  /** The goal's file from the root. */
  file: string;
  /** THE LIVE GOAL: its file, title, summary, stages and place on the chain. */
  goal: string;
  /** WHERE THE WORK STANDS. */
  handoff: string;
  /** THE CODE YOUR NEXT ITEMS ANCHOR, one window per anchor. */
  code: string[];
  /** STANDING DECISIONS. */
  decisions: string;
  /** The stage the pack shows as live, or null when every stage is done. */
  stage: { id: number; title: string } | null;
  /** Whether `stages` holds every stage. */
  full: boolean;
  /** THE LIVE STAGE's section, or ALL STAGES with `full`. */
  stages: string | null;
  /**
   * The line under the stages that says how the live stage is checked: a build alone, or `looper verify`
   * in the gate stage. Null when the goal has no gate stage or no stage is live.
   */
  check: string | null;
  /** CONTEXT, the head of each file. */
  context: { path: string; head: string }[];
  /** ACCEPTANCE, one line per check of the goal. */
  acceptance: string[];
  /** How many checks the floor holds. */
  floor: number;
  /** TRAPS, or null when the playbook is empty. */
  traps: string | null;
  /** The project's sections. */
  extra: string[];
  /** WHEN YOU ARE DONE. */
  done: string;
}

/**
 * The pack for the live goal of the project at `root`. `full` prints every stage and every stage's
 * context instead of the live stage's. `stage` names the stage the pack shows as live in place of the
 * first open one; one the goal does not have is ignored. `extra` holds the project's sections.
 */
export function loadPack(root: string, cfg: Config, full: boolean, stage: number | undefined, extra: string[]): Pack {
  const dir = join(root, cfg.project.goals_dir);
  const chain = loadChain(dir);
  const run = runSection(root);
  const slug = liveGoal(root, dir, chain);
  if (slug === undefined) return { run, live: null };
  const goal = loadGoal(dir, slug);
  const front = goal.front;
  const live: Stage | undefined = (stage === undefined ? undefined : front.stages.find((s) => s.id === stage)) ?? liveStage(goal);
  const secs = sectionsOf(goal.body, 2);
  const text = (name: string) => section(secs, name)?.text;
  const file = `${cfg.project.goals_dir}/${goal.slug}.md`;

  const head = [`File: ${file} (edit it in place; \`${TOOL} lint\` must pass)`, `Title: ${front.title}\nReached when: ${front.summary}\n`];
  for (const s of front.stages) {
    const mark = s.status === "done" ? "x" : s === live ? ">" : " ";
    head.push(`  [${mark}] ${s.id}. ${s.title}`);
  }
  head.push(place(chain, slug));

  const marks = anchors(goal);
  const stages = full ? (text("Stages") ?? "") : live === undefined ? null : stageSection(goal, live.id);
  const gate = gateStage(goal);
  const atGate = live !== undefined && live === gate;
  const stageContext = full ? front.stages.flatMap((s) => s.context ?? []) : (live?.context ?? []);
  const playbook = loadPlaybook(root, cfg);
  const traps =
    playbook !== undefined && (playbook.head !== "" || playbook.sections.length > 0) ? trapsOf(playbook, marks, cfg.orient.traps, cfg) : null;
  return {
    run,
    live: {
      slug: goal.slug,
      file,
      goal: head.join("\n"),
      handoff: text("Handoff") ?? "(no handoff yet: start at the live stage's first open item)",
      code: marks.slice(0, MAX_ANCHORS).map((a) => codeWindow(root, a.path, a.line)),
      decisions: text("Standing decisions") ?? "(none)",
      stage: live === undefined ? null : { id: live.id, title: live.title },
      full,
      stages,
      check: live === undefined || gate === undefined ? null : checkLineOf(atGate, gate.id),
      context: expand(root, [...front.context, ...stageContext]).map((path) => ({ path, head: headOf(root, path) })),
      acceptance: front.checks.map(checkLine),
      floor: chain.floor.length,
      traps,
      extra: [...extra],
      done: doneSteps(file, cfg.project.scratch, atGate),
    },
  };
}

/** The pack's text, as the driver pipes it in. */
export function packText(pack: Pack): string {
  const out: string[] = [];
  const h = (title: string) => out.push(`\n======== ${title} ========\n`);

  h("RUN");
  out.push(pack.run);
  const live = pack.live;
  if (live === null) {
    out.push("\nThe chain is empty: there is no live goal.");
    return out.join("\n");
  }

  h(`THE LIVE GOAL: ${live.slug}`);
  out.push(live.goal);

  h("WHERE THE WORK STANDS (the goal's ## Handoff)");
  out.push(live.handoff);

  if (live.code.length > 0) {
    h("THE CODE YOUR NEXT ITEMS ANCHOR");
    for (const window of live.code) out.push(window, "");
  }

  h("STANDING DECISIONS");
  out.push(live.decisions);

  if (live.stages !== null && live.full) {
    h("ALL STAGES");
    out.push(live.stages);
  } else if (live.stage !== null && live.stages !== null) {
    h(`THE LIVE STAGE: ${live.stage.id}. ${live.stage.title}`);
    out.push(live.stages);
  }
  if (live.check !== null) out.push(`\n${live.check}`);

  if (live.context.length > 0) {
    h("CONTEXT (the head of each file the goal names)");
    for (const c of live.context) out.push(`--- ${c.path}\n${c.head}\n`);
  }

  h(`ACCEPTANCE (the driver runs these when a session claims DONE; you run one with \`${TOOL} check --name <text>\`)`);
  out.push(...live.acceptance);
  if (live.floor > 0) out.push(`\nThe floor (${live.floor} checks from reached goals) also runs and must stay green.`);

  if (live.traps !== null) {
    h("TRAPS (the playbook)");
    out.push(live.traps);
  }

  for (const s of live.extra) out.push(`\n${s}`);

  h("WHEN YOU ARE DONE");
  out.push(live.done);
  return out.join("\n");
}

/** The pack's text for the live goal of the project at `root`, with the arguments `loadPack` takes. */
export function pack(root: string, cfg: Config, full: boolean, stage: number | undefined, extra: string[]): string {
  return packText(loadPack(root, cfg, full, stage, extra));
}

/**
 * How near the path a bullet names stands to the anchor's file `anchor`: 3 for the file itself, 2 in
 * its directory, 1 under its top-level directory, else 0.
 */
function nearness(path: string, anchor: string): number {
  const under = (dir: string) => path === dir || path.startsWith(`${dir}/`);
  if (path === anchor) return 3;
  const slash = anchor.lastIndexOf("/");
  if (slash >= 0 && under(anchor.slice(0, slash))) return 2;
  const top = anchor.indexOf("/");
  return top >= 0 && under(anchor.slice(0, top)) ? 1 : 0;
}

/**
 * The TRAPS section's body, or `(empty)`: the bullet lines of the head, then at most `bound` bullets
 * whole, each under its section's heading when it differs from the one before. The bullets go in the
 * order of their best `nearness` to the anchors' files, ties in the playbook's order. A last line names
 * how many more there are and where they stand.
 */
function trapsOf(playbook: Playbook, marks: Anchor[], bound: number, cfg: Config): string {
  const out = playbook.head.split("\n").filter((l) => l.startsWith("- "));
  const ranked: { score: number; name: string; text: string }[] = [];
  for (const s of playbook.sections) {
    for (const b of s.bullets) {
      const scores = bulletPaths(b).flatMap((p) => marks.map((a) => nearness(p, a.path)));
      ranked.push({ score: Math.max(0, ...scores), name: s.name, text: b.text });
    }
  }
  ranked.sort((a, b) => b.score - a.score);
  let last: string | undefined;
  for (const r of ranked.slice(0, bound)) {
    if (last !== r.name) {
      out.push(`## ${r.name}`);
      last = r.name;
    }
    out.push(r.text);
  }
  const more = Math.max(0, ranked.length - bound);
  if (more > 0) {
    const file = `\`${cfg.project.playbook.replaceAll("\\", "/")}\``;
    const dir = cfg.project.playbook_dir;
    const homes = dir === undefined ? file : `${file} and \`${dir.replaceAll("\\", "/").replace(/\/+$/, "")}/\``;
    out.push(`(${more} more bullet(s), ranked lower, stand in ${homes}.)`);
  }
  return out.length === 0 ? "(empty)" : out.join("\n");
}

/** The line that says how the live stage is checked: `atGate` when it is the gate stage `gateId`. */
function checkLineOf(atGate: boolean, gateId: number): string {
  return atGate
    ? `This is the goal's gate stage: run \`${TOOL} verify\` over the whole goal.`
    : `This stage checks with a build alone; stage ${gateId} verifies the goal.`;
}

/**
 * The steps that finish a session on the goal file `file`, with the commit message in the project's
 * scratch directory `scratch`. `atGate` says the live stage is the goal's gate stage.
 */
function doneSteps(file: string, scratch: string, atGate: boolean): string {
  return [
    atGate
      ? `1. \`${TOOL} verify\` over the whole goal; fix every failure it names in one pass, then run it once more.`
      : "1. A build alone checks the work. No test and no verify runs, except a test you wrote, run once by name, and a run an item names.",
    `2. Overwrite ## Handoff in ${file} (### State, ### Next with anchors, ### Backlog), naming the tests you expect to go red; tick finished items; set a stage's status to done in the front matter when its items are done and its checks pass.`,
    "3. A playbook bullet if a trap cost you time.",
    `4. One commit per slice, once its build is green${atGate ? " and verify is green" : ""}: \`git add <the slice's paths>\` then \`git commit -F ${scratch}/msg.txt\`.`,
    `5. Write .loop/status.txt with one line: CONTINUE <what landed> | DONE <goal reached> | BLOCKED <what the loop cannot do itself>. A design call never blocks, however large: pick the option the project's priorities favour, record it and why in the goal's Standing decisions, and build on it. Ask the user nothing you can decide; only a call the project leaves open goes to \`${TOOL} decision ask --built-on <n>\`, with its options and your recommendation. BLOCKED is only for a call your permissions refuse, a credential you do not have, or an act that publishes outside the repository or costs money; before it, file that with \`${TOOL} decision ask --parked\`. Never ask a decision about decisions already open: the loop holds on them, and the user's answers resume it. Then stop.`,
  ].join("\n");
}

/**
 * Whether the harvest file holds a non-empty section for the goal: the heading of `looper.toml`'s
 * `[project.harvest].section` with `{slug}` filled in, at that heading's level. With `harvest = false`
 * every goal counts as harvested.
 */
export function harvested(root: string, cfg: Config, slug: string): boolean {
  const harvest = cfg.project.harvest;
  if (harvest === null) return true;
  const text = read(join(root, harvest.file));
  if (text === undefined) return false;
  const heading = harvest.section.replace("{slug}", () => slug);
  const name = heading.replace(/^#+/, "");
  const level = heading.length - name.length || 2;
  return (section(sectionsOf(text, level), name.trimStart())?.text.trim() ?? "") !== "";
}
