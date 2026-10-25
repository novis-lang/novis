// `bun nv debt`: pays the floor debt, every atom of it, and is red when any of it stays red or owed.
//
//     bun nv debt
//
// It runs as the floor check `floor-debt` of `docs/goals/README.md`, which sets `reach: true`, so looper
// judges it once per goal: last, and only once every stage is done and every other check is green. No
// session pays it, and nothing runs it between two sessions. The nightly CI run pays the same atoms
// again on its own runners.
//
// The debt is what the selection store holds that no session runs: every proof program, case, test
// executable, tools test file and plan check that is owed, and every proof program that is red, since
// `nv verify` never runs one again. `nv verify` and `bun nv affected --run` mark owed each atom a change
// reached that they did not run (`select/record.ts` `advance`). The atoms go by their last run, oldest
// first (`debtOrder`), and `select/full.ts` `fullRun` runs them with `only`. A run moves the store's
// tree, which owes each atom the change since its last run reached and the run did not run, so after it
// the debt is read again and a further round pays what is new, up to `ROUNDS`. Each atom's run is
// recorded the moment it ends, so a run the check's timeout kills loses only the atoms still running. It
// prints each round with the time it took, and then what it paid and what debt is left, by kind.
//
// **The leak check is not debt.** valgrind runs over `examples/` in CI's nightly deep lane
// (`cmd/valgrind.ts`), and CI's Linux jobs run every test on Linux at each push. What it costs: a leak a
// goal brought in is found the next night, and not at the goal's end.
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
// `planned`. The runner tells it of each atom it starts on its own: a test executable, a tools test file
// or a plan check. A case or a proof program, which runs in a batch, only ends. An atom's item ends when
// its run is recorded (`store.ts` `onRecorded`), red when it ran red. One paid with no recorded run ends
// with its verdict once the rounds are over. The run ends with `done`. What it spends: one appended line
// per item event.
//
// It exits 0 when every turn was green and no debt is left, 1 when a turn found a red or debt is left,
// and 2 on a usage error.

import { type EndFields, ev } from "../lib/events.ts";
import { fullRun } from "../select/full.ts";
import { type AtomRow, onRecorded, SelectStore } from "../select/store.ts";

/** The progress item of the debt; each debt atom's item is its atom id, under it. */
export const DEBT_ITEM = "debt";

export const summary = "the floor debt, every owed atom but the benches, paid once per goal as the floor check `floor-debt`: nv debt";

const USAGE = "usage: bun nv debt";

/** A kind of floor debt: the kind of its atom. */
export type DebtKind = "proof" | "case" | "test" | "nvtest" | "check" | "nv";

/** One atom of the floor debt, owed or red. */
export interface DebtAtom {
  id: string;
  kind: DebtKind;
  /** When it last ran, in milliseconds since the epoch; 0 for never. */
  lastRun: number;
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
  /** Runs the atoms `ids`, in their order: a red line, or "". `onStart` is told of each atom as the
   * runner starts it. */
  pay(ids: string[], note: (line: string) => void, onStart?: (id: string) => void): Promise<string>;
  voice: DebtVoice;
}

/** What the debt run did. */
export interface DebtOutcome {
  /** The atoms of the debt that ran. */
  paid: DebtAtom[];
  /** The debt the store holds after the run. */
  left: DebtAtom[];
  /** Each red line a round returned. */
  reds: string[];
}

// ---- the debt ----------------------------------------------------------------------------------------

/** The most rounds one run pays. The second round finds the tree where the first left it, so it owes
 * nothing new unless the tree moved under the run; the third is for that. */
const ROUNDS = 3;

const DEBT_KINDS = new Set<string>(["proof", "case", "test", "nvtest", "check", "nv"]);

/**
 * Each atom of `rows` that is floor debt: an owed atom of any kind but a bench or a heavy check, and a
 * red proof program, since `nv verify` never runs one again.
 */
export function debtOf(rows: Pick<AtomRow, "id" | "kind" | "verdict" | "lastRun">[]): DebtAtom[] {
  const out: DebtAtom[] = [];
  for (const a of rows) {
    if (!DEBT_KINDS.has(a.kind)) continue;
    const kind = a.kind as DebtKind;
    if (a.verdict === "owed" || (a.verdict === "red" && kind === "proof")) out.push({ id: a.id, kind, lastRun: a.lastRun });
  }
  return out;
}

/** The ids of `debt` in the order they are paid: by last run, oldest first, then by id. */
export function debtOrder(debt: DebtAtom[]): string[] {
  const byAge = (a: DebtAtom, b: DebtAtom) => a.lastRun - b.lastRun || (a.id < b.id ? -1 : a.id > b.id ? 1 : 0);
  return [...debt].sort(byAge).map((a) => a.id);
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

/** Pays every atom of the floor debt, a round at a time, and says what each round took. */
export async function payDebt(s: DebtSeams): Promise<DebtOutcome> {
  const before = s.debt();
  const started = s.now();
  const reds: string[] = [];
  const planned = new Set<string>();
  // Every atom a round planned, as it was when planned.
  const owed: DebtAtom[] = [];
  const plan = (atoms: DebtAtom[]) => {
    for (const a of atoms) {
      planned.add(a.id);
      owed.push(a);
      ev.add(a.id, { parent: DEBT_ITEM, name: a.id, kind: a.kind });
    }
  };
  ev.add(DEBT_ITEM, { name: "floor debt", kind: "step" });
  plan(before);
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
  // A round moves the store's tree past the change since its last run, which owes every atom that change
  // reached and the round did not run. Those are debt too, so a further round pays them; one that the
  // round before planned and did not pay stays left, which ends the rounds.
  let round = before;
  let left: DebtAtom[] = [];
  try {
    if (before.length === 0) s.voice.step("floor debt: none");
    for (let n = 0; n < ROUNDS && round.length > 0; n++) {
      if (n === 0) s.voice.step(`floor debt: ${round.length} atom(s), ${byKind(round)}; the oldest first`);
      else s.voice.step(`floor debt: ${round.length} more atom(s), ${byKind(round)}, owed by the run's own tree move`);
      const at = s.now();
      reds.push(await s.pay(debtOrder(round), s.voice.note, onStart));
      s.voice.step(`floor debt: paid ${byKind(round)} in ${minutes(s.now() - at)}`);
      left = s.debt();
      round = left.filter((a) => !planned.has(a.id));
      plan(round);
    }
  } finally {
    onRecorded(null);
  }
  const paid = paidOf(owed, left);
  if (before.length > 0) {
    s.voice.step(`floor debt: paid ${paid.length}${paid.length > 0 ? ` (${byKind(paid)})` : ""} in ${minutes(s.now() - started)}; left ${left.length}${left.length > 0 ? ` (${byKind(left)})` : ""}`);
  }
  // An atom left is named, so the next step is to run it, not to query the store for it. A proof stays
  // owed when its recording run diverged, and `nv proofs --id <feature> --run` prints why.
  if (left.length > 0) s.voice.step(`floor debt: still owed: ${left.slice(0, 5).map((a) => a.id).join(", ")}${left.length > 5 ? `, and ${left.length - 5} more` : ""}`);
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
 * The atoms of `ids`, run by `fullRun` in their order, each recorded. Returns a red line naming the atoms
 * that ran red, or the covws build that failed, and "" when there is neither.
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

const REAL: DebtSeams = {
  now: () => Date.now(),
  debt: storeDebt,
  pay: (ids, note, onStart = () => {}) => fullTurn(ids, note, onStart),
  voice: { step: (l) => console.log(l), note: (l) => console.log(`   ${l}`) },
};

// ---- the command -------------------------------------------------------------------------------------

/** Parses and pays the debt: 0 when nothing is red or left, 1 when something is, 2 on a usage error. */
export async function debt(args: string[], over: Partial<DebtSeams> = {}): Promise<number> {
  const s: DebtSeams = { ...REAL, ...over };
  if (args.length > 0) {
    console.error(`${USAGE}\nnv debt: error: unexpected argument ${args[0]}`);
    return 2;
  }
  const out = await payDebt(s);
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
