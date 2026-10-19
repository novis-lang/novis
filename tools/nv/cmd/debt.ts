// `bun nv debt`: pays the floor debt, every atom of it, and is red when any of it stays red or owed.
//
//     bun nv debt [--goal <slug>]
//
// It runs as the floor check `floor-debt` of `docs/goals/README.md`, which sets `reach: true`, so looper
// judges it once per goal: last, and only once every stage is done and every other check is green. No
// session pays it, and nothing runs it between two sessions. The nightly CI run pays the same atoms
// again on its own runners.
//
// The debt is what the selection store holds that no session runs: every proof program, case, test
// executable, tools test file, plan check and Linux leg that is owed, and every proof program and leg
// that is red, since `nv verify` runs neither. `nv verify` and `bun nv affected --run` mark owed each atom
// a change reached that they did not run (`select/record.ts` `advance`). The atoms go by their last run,
// oldest first (`debtTurns`). One runner takes each turn, and the turns go in the order of their first
// atoms: `select/full.ts` `fullRun` with `only` for every kind but the legs, and `cmd/legs.ts`
// `legsOver` for the legs. The legs run
// over `--goal`'s goal, else the one `LOOPER_GOAL` names, else the current one (`lib/chain.ts`
// `namedPlan`). A goal that names no fixture gives the legs nothing to run, so an owed or red leg is not
// that goal's debt: it stays in the store as it is, and the first goal with a fixture pays it (`legsWait`).
// Recording it green would lose a change to code only Linux compiles. Each atom's run is recorded the
// moment it ends, so a run the check's timeout kills loses
// only the atoms still running. It prints each turn with the time it took, and then what it paid and what
// debt is left, by kind.
//
// **The benches are not debt.** One measured run paid 1093 owed benches in 70 minutes, then 2763 proof
// programs in 49 minutes, so a goal's debt with its benches took past two hours. An owed bench waits for a
// release: the release gate's `proofs` job is red while a perf record is stale, and `bun nv benches
// --owed` pays them. What it costs: a bench whose growth a goal broke is found at the release, or by
// nightly CI's `proofs` job, and not at the goal's end.
//
// Under looper, which names an event file in `LOOPER_EVENTS`, the run writes looper's progress events
// (`lib/events.ts`), and outside it writes none. It declares the debt's item, `DEBT_ITEM`, and under it
// one item per debt atom in the order the turns pay them, with the atom's id and the debt kind, then
// `planned`. A runner tells it of each atom it starts on its own: a test executable, a tools test file, a
// plan check or a bench. The Linux legs start with their turn, and a case or a proof program, which runs
// in a batch, only ends. An atom's item ends when its run is recorded (`store.ts` `onRecorded`), red when
// it ran red. One paid with no recorded run ends with its verdict once the turns are over. The run ends
// with `done`. What it spends: one appended line per item event.
//
// It exits 0 when every turn was green and no debt is left, 1 when a turn found a red or debt is left,
// and 2 on a usage error.

import { PROGRAM_KINDS } from "../accept/accept.ts";
import { namedPlan } from "../lib/chain.ts";
import { type EndFields, ev } from "../lib/events.ts";
import { LEGS, legId } from "../select/checks.ts";
import { fullRun } from "../select/full.ts";
import { type AtomRow, onRecorded, SelectStore } from "../select/store.ts";
import { type LegsCmdSeams, legsOver } from "./legs.ts";

/** The progress item of the debt; each debt atom's item is its atom id, under it. */
export const DEBT_ITEM = "debt";

export const summary = "the floor debt, every owed atom but the benches, paid once per goal as the floor check `floor-debt`: nv debt [--goal <slug>]";

const USAGE = "usage: bun nv debt [--goal <slug>]";

/** A kind of floor debt: an atom kind, or `leg` for the atom of a Linux leg. */
export type DebtKind = "proof" | "case" | "test" | "nvtest" | "check" | "nv" | "leg";

/** One atom of the floor debt, owed or red. */
export interface DebtAtom {
  id: string;
  kind: DebtKind;
  /** When it last ran, in milliseconds since the epoch; 0 for never. */
  lastRun: number;
}

/** What runs a turn of the debt: `fullRun` or `legsOver`. */
export type Runner = "full" | "leg";

/** The atoms one runner pays, in the order they run. */
export interface DebtTurn {
  runner: Runner;
  ids: string[];
}

/** How the command reports itself: the command prints it, and a test silences it. */
export interface DebtVoice {
  /** A step of the run. */
  step(line: string): void;
  /** A line under a step. */
  note(line: string): void;
}

/** Everything the command reads or starts, replaced under test. Each one left out is the real one. */
export interface DebtSeams {
  /** The clock each turn is timed on, in milliseconds since the epoch. */
  now(): number;
  /** The floor debt the store holds now. */
  debt(): DebtAtom[];
  /** Why the legs have nothing to run over `slug`'s goal, or "" when they have. */
  legsWait(slug: string | undefined): string;
  /** Runs one turn: a red line, or "". `slug` is the goal the legs run over, `namedPlan`'s when unset.
   * `onStart` is told of each atom as the runner starts it. */
  pay(turn: DebtTurn, slug: string | undefined, note: (line: string) => void, onStart?: (id: string) => void): Promise<string>;
  voice: DebtVoice;
}

/** What the debt run did. */
export interface DebtOutcome {
  /** The atoms of the debt that ran. */
  paid: DebtAtom[];
  /** The debt the store holds after the run. */
  left: DebtAtom[];
  /** Each red line a turn returned. */
  reds: string[];
}

// ---- the debt ----------------------------------------------------------------------------------------

const LEG_IDS = new Set<string>(LEGS.map(legId));

/**
 * Each atom of `rows` that is floor debt: an owed atom of any kind but a bench, and a red proof program
 * or leg, since `nv verify` runs neither and so never runs a red one again.
 */
export function debtOf(rows: Pick<AtomRow, "id" | "kind" | "verdict" | "lastRun">[]): DebtAtom[] {
  const out: DebtAtom[] = [];
  for (const a of rows) {
    let kind: DebtKind | null = null;
    if (a.kind === "heavy") kind = LEG_IDS.has(a.id) ? "leg" : null;
    else if (a.kind === "proof" || a.kind === "case" || a.kind === "test" || a.kind === "nvtest" || a.kind === "check" || a.kind === "nv") kind = a.kind;
    if (kind === null) continue;
    if (a.verdict === "owed" || (a.verdict === "red" && (kind === "proof" || kind === "leg"))) out.push({ id: a.id, kind, lastRun: a.lastRun });
  }
  return out;
}

/**
 * The turns that pay `debt`, ordered by last run, oldest first, then by id. The atoms of one runner form
 * one turn, and the turns go in the order of their first atoms.
 */
export function debtTurns(debt: DebtAtom[]): DebtTurn[] {
  const byAge = (a: DebtAtom, b: DebtAtom) => a.lastRun - b.lastRun || (a.id < b.id ? -1 : a.id > b.id ? 1 : 0);
  const byRunner = new Map<Runner, string[]>();
  for (const a of [...debt].sort(byAge)) {
    const r: Runner = a.kind === "leg" ? "leg" : "full";
    (byRunner.get(r) ?? byRunner.set(r, []).get(r)!).push(a.id);
  }
  return [...byRunner].map(([runner, ids]) => ({ runner, ids }));
}

/** The atoms of `before` that ran: gone from the debt, or with a newer run. */
export function paidOf(before: DebtAtom[], after: DebtAtom[]): DebtAtom[] {
  const now = new Map(after.map((a) => [a.id, a]));
  return before.filter((a) => {
    const b = now.get(a.id);
    return b === undefined || b.lastRun !== a.lastRun;
  });
}

/** `atoms` counted by kind, as `3 proof, 1 bench`, most first. */
export function byKind(atoms: DebtAtom[]): string {
  const n = new Map<DebtKind, number>();
  for (const a of atoms) n.set(a.kind, (n.get(a.kind) ?? 0) + 1);
  return [...n].sort((a, b) => b[1] - a[1] || (a[0] < b[0] ? -1 : 1)).map(([k, c]) => `${c} ${k}`).join(", ");
}

function minutes(ms: number): string {
  const s = Math.max(0, Math.round(ms / 1000));
  return `${Math.floor(s / 60)}m${String(s % 60).padStart(2, "0")}s`;
}

/** Pays every atom of the floor debt, a turn per runner, and says what each turn took. A leg waits, and
 * is neither paid nor left, when `legsWait` gives a reason. */
export async function payDebt(slug: string | undefined, s: DebtSeams): Promise<DebtOutcome> {
  const wait = s.legsWait(slug);
  const ours = (debt: DebtAtom[]) => (wait === "" ? debt : debt.filter((a) => a.kind !== "leg"));
  const all = s.debt();
  const before = ours(all);
  const waiting = all.length - before.length;
  if (waiting > 0) s.voice.step(`floor debt: ${waiting} leg(s) wait in the store for a goal that names a fixture -- ${wait}`);
  const started = s.now();
  const reds: string[] = [];
  const turns = debtTurns(before);
  const planned = new Set(turns.flatMap((t) => t.ids));
  const kindOf = new Map(before.map((a) => [a.id, a.kind]));
  ev.add(DEBT_ITEM, { name: "floor debt", kind: "step" });
  for (const id of planned) ev.add(id, { parent: DEBT_ITEM, name: id, kind: kindOf.get(id) ?? "atom" });
  ev.planned();
  ev.start(DEBT_ITEM);
  // An atom's item ends when its run is recorded, whichever runner records it.
  const ended = new Set<string>();
  const end = (id: string, f: EndFields) => {
    if (!planned.has(id) || ended.has(id)) return;
    ended.add(id);
    ev.end(id, f);
  };
  onRecorded((id, verdict) => end(id, verdict === "red" ? { result: "red", why: "ran red" } : {}));
  const onStart = (id: string) => {
    if (planned.has(id) && !ended.has(id)) ev.start(id);
  };
  try {
    if (before.length === 0) s.voice.step("floor debt: none");
    else {
      s.voice.step(`floor debt: ${before.length} atom(s), ${byKind(before)}; the oldest first`);
      for (const turn of turns) {
        const kinds = byKind(before.filter((a) => turn.ids.includes(a.id)));
        const at = s.now();
        reds.push(await s.pay(turn, slug, s.voice.note, onStart));
        s.voice.step(`floor debt: paid ${kinds} in ${minutes(s.now() - at)}, by ${turn.runner === "full" ? "the full run's runners" : "the Linux legs"}`);
      }
    }
  } finally {
    onRecorded(null);
  }
  const left = before.length === 0 ? [] : ours(s.debt());
  const paid = paidOf(before, left);
  if (before.length > 0) {
    s.voice.step(`floor debt: paid ${paid.length}${paid.length > 0 ? ` (${byKind(paid)})` : ""} in ${minutes(s.now() - started)}; left ${left.length}${left.length > 0 ? ` (${byKind(left)})` : ""}`);
  }
  // An atom paid without a recorded run ends now, and one still owed ends skipped.
  const after = new Set(left.map((a) => a.id));
  for (const id of planned) end(id, after.has(id) ? { result: "skipped", why: "its runner did not run it, so it stays owed" } : {});
  const found = reds.filter((r) => r !== "");
  ev.end(DEBT_ITEM, found.length > 0 ? { result: "red", why: found[0]!.split("\n")[0]! } : {});
  return { paid, left, reds: found };
}

// ---- the real runners --------------------------------------------------------------------------------

function storeDebt(): DebtAtom[] {
  const store = new SelectStore();
  try {
    return debtOf(store.atoms());
  } finally {
    store.close();
  }
}

/**
 * The atoms of `ids` that are not legs, run by `fullRun` in their order, each recorded.
 * Returns a red line naming the atoms that ran red, or the covws build that failed, and "" when there is
 * neither.
 */
async function fullTurn(ids: string[], note: (line: string) => void, onStart: (id: string) => void): Promise<string> {
  const store = new SelectStore();
  try {
    const r = await fullRun(store, { only: ids, say: note, onStart });
    if (r.buildFailed) return "owed atoms: not run, the covws build failed";
    const red = [...r.ran].filter(([, v]) => v === "red").map(([id]) => id);
    note(`owed atoms: ${r.ran.size} ran, ${red.length} red, in ${r.seconds}s`);
    if (red.length === 0) return "";
    return `owed atoms: ${red.length} ran red: ${red.slice(0, 5).join(", ")}${red.length > 5 ? ", ..." : ""}`;
  } finally {
    store.close();
  }
}

/**
 * Why the Linux legs have nothing to run over `namedPlan`'s goal: "" when it names a fixture, and when no
 * goal is found, since `legsTurn` reports that as red.
 */
export function legsWait(slug: string | undefined, plan: typeof namedPlan = namedPlan): string {
  const found = plan(slug);
  if (found === null) return "";
  return (found.goal.checks as { kind: string }[]).some((c) => PROGRAM_KINDS.has(c.kind)) ? "" : `\`${found.slug}\` names no fixture`;
}

/**
 * The Linux legs over `namedPlan`'s goal, or a red line saying no goal is there to read them from. With
 * `full`, both legs run whatever changed; without it, a leg runs when the store selects it, owed or red
 * among the reasons. `over` replaces what `legsOver` reads or starts, under test.
 */
export async function legsTurn(slug: string | undefined, note: (line: string) => void, full: boolean, over: Partial<LegsCmdSeams> = {}): Promise<string> {
  const found = (over.plan ?? namedPlan)(slug);
  if (found === null) return `Linux legs: not run, ${slug ? `no goal record for \`${slug}\`` : "no goal is named or current"}`;
  return legsOver(found.goal, full, { say: note, ...over });
}

async function payTurn(turn: DebtTurn, slug: string | undefined, note: (line: string) => void, onStart: (id: string) => void = () => {}): Promise<string> {
  if (turn.runner === "leg") {
    // The legs run inside one call, which says nothing of each, so each starts with it.
    for (const id of turn.ids) onStart(id);
    return legsTurn(slug, note, false);
  }
  return fullTurn(turn.ids, note, onStart);
}

const REAL: DebtSeams = {
  now: () => Date.now(),
  debt: storeDebt,
  legsWait: (slug) => legsWait(slug),
  pay: payTurn,
  voice: { step: (l) => console.log(l), note: (l) => console.log(`   ${l}`) },
};

// ---- the command -------------------------------------------------------------------------------------

/** Parses and pays the debt: 0 when nothing is red or left, 1 when something is, 2 on a usage error. */
export async function debt(args: string[], over: Partial<DebtSeams> = {}): Promise<number> {
  const s: DebtSeams = { ...REAL, ...over };
  let slug: string | undefined;
  for (let i = 0; i < args.length; i++) {
    const a = args[i]!;
    if (a === "--goal" && args[i + 1] !== undefined) slug = args[++i];
    else {
      console.error(`${USAGE}\nnv debt: error: ${a === "--goal" ? "--goal needs a value" : `unexpected argument ${a}`}`);
      return 2;
    }
  }
  const out = await payDebt(slug, s);
  for (const red of out.reds) s.voice.step(`RED: ${red}`);
  const red = out.reds.length > 0 || out.left.length > 0;
  s.voice.step(red ? `RED: the floor debt is not paid${out.left.length > 0 ? `, ${out.left.length} atom(s) left` : ""}` : "GREEN: the floor debt is paid");
  ev.done(red ? "red" : "green");
  return red ? 1 : 0;
}

export async function run(args: string[]): Promise<number> {
  if (args.includes("--help") || args.includes("-h")) {
    console.log(`${USAGE}\n\n${summary}`);
    return 0;
  }
  return debt(args);
}
