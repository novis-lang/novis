// A loop's decisions for the user: the questions a session asks while its loop runs on, kept in
// `decisions.json` in the loop's folder, never in the goal file. `looper decision` in
// `../cli/decision.ts` runs each verb, and `docs/kit-contract.md` § *The loops* is the contract.
//
// - **A decision is open until the user answers it,** and stands until a session closes it. The session
//   lists two to four options, marks the one it recommends, and says whether it parked the work that
//   hangs on the answer (`builtOn` null) or built on one option.
// - **The file is `{"next", "decisions"}`,** rewritten whole by `writeState` under the loop's controls
//   mutex (`lock` in `controls.ts`). So a crash leaves the old file or the new one, and two writers never
//   lose each other's change. `next` numbers the ids, so an id is never reused within a loop.
// - **An answer is also a `say` to the loop** (`appendControl`), appended once the record holds it. The
//   running session reads it at its next turn, and between sessions it becomes a note.
// - **A file that does not parse is never overwritten.** Every verb refuses until a person repairs or
//   deletes it.
// - **The driver reads the record twice a session.** `packSection` lists each answered decision in the
//   next pack, and `standing` says why a `DONE` claim waits while any decision stands.
//
// # The bounds it keeps
//
// - At most `DECISIONS` stand per loop, open or answered. `ask` refuses one more with a sentence that
//   sends the session back to the priority ordering.
// - A question takes at most `QUESTION_BYTES`, and an option, an item, an own answer and a note each
//   `TEXT_BYTES`. So the file stays under `FILE_BYTES`, and a larger one is refused as not the kit's.

import { statSync } from "node:fs";
import { join } from "node:path";
import { Refused } from "../goal/model.ts";
import { readState, writeState } from "../store/state.ts";
import { appendControl, lock } from "./controls.ts";

/** The decisions file in a loop's folder. */
export const DECISIONS_FILE = "decisions.json";
/** The most decisions a loop holds, open or answered. */
export const DECISIONS = 8;
/** The longest question, in UTF-8 bytes. */
export const QUESTION_BYTES = 2048;
/** The longest option, item, own answer or note, in UTF-8 bytes. */
export const TEXT_BYTES = 200;
/** The fewest and the most options a question lists. */
export const OPTIONS = [2, 4] as const;
/** The largest file a reader takes. The bounds above keep the kit's own far below it. */
export const FILE_BYTES = 64 * 1024;

/** The user's answer: one option by its number from 1, or a text of their own, with an optional note. */
export interface Answer {
  /** In ms since the Unix epoch. */
  at: number;
  by: string;
  option: number | null;
  text: string | null;
  note: string | null;
}

/** One decision a session asked the user. */
export interface Decision {
  /** `d<n>`, unique within the loop. */
  id: string;
  /** In ms since the Unix epoch. */
  at: number;
  by: string;
  question: string;
  options: string[];
  /** The option the session recommends, from 1. */
  recommend: number;
  /** The option the session built on, from 1, or null when it parked the work. */
  builtOn: number | null;
  /** The goal item the work hangs on, as its `path:line` anchor, or null. */
  item: string | null;
  /** Null while the decision is open. */
  answer: Answer | null;
}

/** The decisions file. */
export interface Decisions {
  next: number;
  decisions: Decision[];
}

/** What `ask` records. */
export interface Asked {
  question: string;
  options: string[];
  recommend: number;
  builtOn: number | null;
  item?: string | null;
  by: string;
  at?: number;
}

/** What `answer` records: exactly one of `option` and `text`. */
export interface Answered {
  option?: number;
  text?: string;
  note?: string;
  by: string;
  at?: number;
}

/** The decisions file of the loop whose folder is `dir`. */
export function decisionsPath(dir: string): string {
  return join(dir, DECISIONS_FILE);
}

/** The decisions of the loop whose folder is `dir`, none when it has no file. Throws `Refused` on a file the kit did not write. */
export function readDecisions(dir: string): Decisions {
  const path = decisionsPath(dir);
  const foreign = new Refused(`${path} is not a decisions file the kit wrote: repair or delete it`);
  const size = statSync(path, { throwIfNoEntry: false })?.size;
  if (size === undefined) return { next: 1, decisions: [] };
  if (size > FILE_BYTES) throw foreign;
  let doc: Decisions | undefined;
  try {
    doc = readState<Decisions>(path);
  } catch {
    throw foreign;
  }
  if (doc === undefined) return { next: 1, decisions: [] };
  if (typeof doc !== "object" || doc === null || !Number.isSafeInteger(doc.next) || doc.next < 1 || !Array.isArray(doc.decisions)) throw foreign;
  return doc;
}

/** Runs `change` on the decisions of the loop whose folder is `dir` under the controls mutex, writes them back, and returns what `change` returns. */
async function update<T>(dir: string, change: (d: Decisions) => T): Promise<T> {
  const m = await lock(dir);
  try {
    const d = readDecisions(dir);
    const out = change(d);
    writeState(decisionsPath(dir), d);
    return out;
  } finally {
    m.close();
  }
}

/** Refuses `text` when it is empty or longer than `max` bytes. */
function bounded(what: string, text: string, max: number): string {
  if (text.trim() === "") throw new Refused(`${what} is empty`);
  if (Buffer.byteLength(text) > max) throw new Refused(`${what} takes at most ${max} bytes`);
  return text;
}

/** Refuses `n` unless it numbers one of `count` options. */
function numbered(what: string, n: number, count: number): number {
  if (!Number.isSafeInteger(n) || n < 1 || n > count) throw new Refused(`${what} names an option from 1 to ${count}`);
  return n;
}

/** Records a new open decision in the loop whose folder is `dir`, and returns it. Refuses past `DECISIONS`. */
export async function ask(dir: string, a: Asked): Promise<Decision> {
  const question = bounded("the question", a.question, QUESTION_BYTES);
  if (a.options.length < OPTIONS[0] || a.options.length > OPTIONS[1]) throw new Refused(`a decision lists ${OPTIONS[0]} to ${OPTIONS[1]} options`);
  const options = a.options.map((o, i) => bounded(`option ${i + 1}`, o, TEXT_BYTES));
  const recommend = numbered("the recommendation", a.recommend, options.length);
  const builtOn = a.builtOn === null ? null : numbered("the option built on", a.builtOn, options.length);
  const item = a.item == null ? null : bounded("the item", a.item, TEXT_BYTES);
  return update(dir, (d) => {
    if (d.decisions.length >= DECISIONS) {
      throw new Refused(
        `this loop holds ${DECISIONS} decisions already: decide this one by the priority ordering, and record why under ## Standing decisions`,
      );
    }
    const decision: Decision = { id: `d${d.next}`, at: a.at ?? Date.now(), by: a.by, question, options, recommend, builtOn, item, answer: null };
    d.next++;
    d.decisions.push(decision);
    return decision;
  });
}

/** The decision `id` of `d`, or a refusal. */
function find(d: Decisions, id: string): Decision {
  const found = d.decisions.find((x) => x.id === id);
  if (found === undefined) throw new Refused(`this loop has no decision ${id}`);
  return found;
}

/** The `say` a session reads for `decision`'s answer. */
export function sayText(decision: Decision): string {
  const a = decision.answer as Answer;
  const chosen = a.option !== null ? `option ${a.option}, "${decision.options[a.option - 1]}"` : `"${a.text}"`;
  const note = a.note === null ? "" : ` Their note: "${a.note}".`;
  return (
    `The user answered decision ${decision.id}, "${decision.question}": ${chosen}.${note}` +
    ` Record it under ## Standing decisions, then run \`looper decision close ${decision.id}\`.`
  );
}

/** Records the user's answer to decision `id` of the loop whose folder is `dir`, says it to the loop, and returns the decision. A second answer replaces the first. */
export async function answer(dir: string, id: string, a: Answered): Promise<Decision> {
  if ((a.option === undefined) === (a.text === undefined)) throw new Refused("an answer is one option or a text of its own");
  const text = a.text === undefined ? null : bounded("the answer", a.text, TEXT_BYTES);
  const note = a.note === undefined || a.note.trim() === "" ? null : bounded("the note", a.note, TEXT_BYTES);
  const decision = await update(dir, (d) => {
    const found = find(d, id);
    const option = a.option === undefined ? null : numbered("the answer", a.option, found.options.length);
    found.answer = { at: a.at ?? Date.now(), by: a.by, option, text, note };
    return found;
  });
  await appendControl(dir, { by: a.by, action: "say", payload: sayText(decision) });
  return decision;
}

/**
 * The pack's section on the decisions of the loop whose folder is `dir`: each answered one, with the step
 * to record it and close it, then the ids still open. Null when none stands. A file the kit did not
 * write is named instead.
 */
export function packSection(dir: string): string | null {
  let d: Decisions;
  try {
    d = readDecisions(dir);
  } catch (e) {
    return `${PACK_HEADING}\n\n${(e as Error).message}.`;
  }
  if (d.decisions.length === 0) return null;
  const out = [PACK_HEADING, ""];
  for (const x of d.decisions.filter((x) => x.answer !== null)) out.push(`- ${sayText(x)}`);
  const open = d.decisions.filter((x) => x.answer === null).map((x) => x.id);
  if (open.length > 0) {
    out.push(`- Still open, for the user to answer: ${open.join(", ")}. Work on as each one says it parked or built on a default.`);
  }
  out.push("", "DONE waits while any decision stands, open or answered.");
  return out.join("\n");
}

/** The heading of `packSection`. */
const PACK_HEADING = "======== THE USER'S DECISIONS (looper decision list) ========";

/** Why `DONE` waits on the loop whose folder is `dir`: a sentence naming each decision that stands, or null when none does. */
export function standing(dir: string): string | null {
  let d: Decisions;
  try {
    d = readDecisions(dir);
  } catch (e) {
    return (e as Error).message;
  }
  if (d.decisions.length === 0) return null;
  const ids = d.decisions.map((x) => `${x.id} (${x.answer === null ? "open" : "answered"})`);
  return `decision(s) ${ids.join(", ")} still stand: record each answer under ## Standing decisions and close it before DONE`;
}

/** Drops answered decision `id` of the loop whose folder is `dir`, once its answer is recorded in the goal, and returns it. Refuses an open one. */
export async function close(dir: string, id: string): Promise<Decision> {
  return update(dir, (d) => {
    const found = find(d, id);
    if (found.answer === null) throw new Refused(`decision ${id} has no answer yet, so it stays open for the user`);
    d.decisions = d.decisions.filter((x) => x !== found);
    return found;
  });
}
