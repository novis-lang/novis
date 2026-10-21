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
// - **A blocker is a decision that holds the loop.** The driver marks a `BLOCKED` session's open parked
//   decisions `blocking` with `block`. The first answer that leaves no blocker open appends a `resume`
//   after its `say`, by the answer's asker. A file without the field reads as not blocking.
// - **Every decision carries a discussion,** its `thread` of messages by the user or by `advisor`.
//   `discuss` is `post` then `reply`. `post` appends the user's message and writes the file, so a reader
//   sees it at once. `reply` runs the advisor outside the mutex, and appends its reply under it. A
//   detached process may run `reply` alone. A file without the field reads as an empty thread.
// - **One advisor runs per decision.** `discuss` refuses while the thread's last message is the user's,
//   unless it is older than any run can be, because a killed `discuss` left it unanswered. A run that
//   fails or outlasts `ADVISOR_MS` leaves a reply that starts `The advisor could not answer:`.
// - **A reply may end with a fenced `options` block,** `{"options": [...], "recommend": n}`. It revises
//   an open parked decision under `ask`'s bounds. On a decision built on an option it may change only
//   `recommend`, so `builtOn` keeps its meaning. A block that fails its checks is dropped, and the reply
//   says why.
// - **A file that does not parse is never overwritten.** Every verb refuses until a person repairs or
//   deletes it.
// - **The driver reads the record twice a session.** `packSection` lists each answered decision and its
//   thread in the next pack, and `standing` says why a `DONE` claim waits while any decision stands.
//
// # The bounds it keeps
//
// - At most `DECISIONS` stand per loop, open or answered. `ask` refuses one more with a sentence that
//   sends the session back to the priority ordering.
// - A question takes at most `QUESTION_BYTES`, and an option, an item, an own answer and a note each
//   `TEXT_BYTES`.
// - A thread holds at most `THREAD` messages. `discuss` refuses a message that leaves no room for its
//   reply, with a sentence that says to answer. A user's message takes at most `MESSAGE_BYTES`, and a
//   reply is cut at `REPLY_BYTES` with a marker. The advisor's stdout is read up to `ADVISOR_OUT`.
// - So the file stays under `FILE_BYTES`, and a larger one is refused as not the kit's.

import { statSync } from "node:fs";
import { join } from "node:path";
import { Refused } from "../goal/model.ts";
import { killTree } from "../os/index.ts";
import { readState, writeState } from "../store/state.ts";
import { type AgentCommand, firstLine, SUMMARY_CHARS } from "./adapter.ts";
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
/** The most messages a decision's thread holds, the user's and the advisor's. */
export const THREAD = 12;
/** The longest message of the user's, in UTF-8 bytes. */
export const MESSAGE_BYTES = 1024;
/** The longest reply of the advisor's, in UTF-8 bytes, marker included. Its prompt asks for 1.5 KiB. */
export const REPLY_BYTES = 2048;
/** How long one advisor run may take, in ms, before its whole tree is killed. */
export const ADVISOR_MS = 300_000;
/** How long past `ADVISOR_MS` an unanswered message still refuses the next, in ms. A run that died with its process left none. */
const STALE_MS = 60_000;
/** The most bytes of the advisor's stdout read. */
export const ADVISOR_OUT = 1024 * 1024;
/** The largest file a reader takes. Eight decisions with full threads come to about 200 KiB. */
export const FILE_BYTES = 256 * 1024;

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
  /** The decision holds the loop until it is answered. */
  blocking: boolean;
  /** The discussion, oldest message first. */
  thread: Message[];
}

/** One message of a decision's discussion. */
export interface Message {
  /** In ms since the Unix epoch. */
  at: number;
  /** The asker who wrote it, or `advisor`. */
  by: string;
  text: string;
}

/** What `discuss` records of the user. */
export interface Discussed {
  text: string;
  by: string;
  at?: number;
}

/** Runs the advisor on `decision`, whose thread ends with the user's message, and gives its reply. Throws why it could not. */
export type Advisor = (decision: Decision) => Promise<string>;

/** Who writes the advisor's replies. */
export const ADVISOR = "advisor";

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
  for (const x of doc.decisions) {
    x.blocking = x.blocking === true;
    if (!Array.isArray(x.thread)) x.thread = [];
  }
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
    const decision: Decision = { id: `d${d.next}`, at: a.at ?? Date.now(), by: a.by, question, options, recommend, builtOn, item, answer: null, blocking: false, thread: [] };
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

/**
 * Records the user's answer to decision `id` of the loop whose folder is `dir`, says it to the loop, and
 * returns the decision. A second answer replaces the first. The first answer to a blocker that leaves
 * none open also resumes the loop. A resume the standing pause refuses stays on record as refused.
 */
export async function answer(dir: string, id: string, a: Answered): Promise<Decision> {
  if ((a.option === undefined) === (a.text === undefined)) throw new Refused("an answer is one option or a text of its own");
  const text = a.text === undefined ? null : bounded("the answer", a.text, TEXT_BYTES);
  const note = a.note === undefined || a.note.trim() === "" ? null : bounded("the note", a.note, TEXT_BYTES);
  let resume = false;
  const decision = await update(dir, (d) => {
    const found = find(d, id);
    const option = a.option === undefined ? null : numbered("the answer", a.option, found.options.length);
    const first = found.answer === null;
    found.answer = { at: a.at ?? Date.now(), by: a.by, option, text, note };
    resume = first && found.blocking && !d.decisions.some((x) => x.blocking && x.answer === null);
    return found;
  });
  await appendControl(dir, { by: a.by, action: "say", payload: sayText(decision) });
  if (resume) {
    try {
      await appendControl(dir, { by: a.by, action: "resume" });
    } catch (e) {
      if (!(e instanceof Refused)) throw e;
    }
  }
  return decision;
}

/**
 * Marks the open decisions `ids` of the loop whose folder is `dir` as blockers, under the controls mutex,
 * and returns those it marked. An id that is gone or answered is passed over.
 */
export async function block(dir: string, ids: string[]): Promise<Decision[]> {
  return update(dir, (d) => {
    const marked = d.decisions.filter((x) => ids.includes(x.id) && x.answer === null);
    for (const x of marked) x.blocking = true;
    return marked;
  });
}

/**
 * Adds the user's message to decision `id`'s thread in the loop whose folder is `dir`, runs `advise` on
 * it, and adds the reply. Gives the decision as it then stands. The head comment has the rules.
 */
export async function discuss(dir: string, id: string, said: Discussed, advise: Advisor): Promise<Decision> {
  await post(dir, id, said);
  return reply(dir, id, advise);
}

/** Adds the user's message to decision `id`'s thread, and gives the decision. `reply` then answers it. */
export async function post(dir: string, id: string, said: Discussed): Promise<Decision> {
  const text = bounded("the message", said.text, MESSAGE_BYTES);
  return update(dir, (d) => {
    const found = find(d, id);
    if (found.thread.length + 2 > THREAD) {
      throw new Refused(`decision ${id}'s discussion holds ${found.thread.length} of ${THREAD} messages: pick an option or write an answer of your own`);
    }
    const last = found.thread.at(-1);
    if (last !== undefined && last.by !== ADVISOR && Date.now() - last.at < ADVISOR_MS + STALE_MS) {
      throw new Refused(`the advisor is still answering the last message on decision ${id}`);
    }
    found.thread.push({ at: said.at ?? Date.now(), by: said.by, text });
    return found;
  });
}

/** Runs `advise` on decision `id`, whose thread ends with the user's message, and adds its reply. */
export async function reply(dir: string, id: string, advise: Advisor): Promise<Decision> {
  const asked = find(readDecisions(dir), id);
  if ((asked.thread.at(-1)?.by ?? ADVISOR) === ADVISOR) throw new Refused(`decision ${id} has no message waiting for the advisor`);
  let said: string;
  try {
    said = await advise(asked);
  } catch (e) {
    said = `The advisor could not answer: ${(e as Error).message}`;
  }
  return update(dir, (d) => {
    const found = find(d, id);
    const { said: kept, block } = splitBlock(said);
    const why = block === null ? undefined : revise(found, block);
    const note =
      why === undefined ? "" : why === null ? "\n\n(looper revised the decision as this reply proposes.)" : `\n\n(looper dropped this reply's options block: ${why}.)`;
    const body = kept.trim() === "" ? "The advisor gave no reply." : kept.trim();
    found.thread.push({ at: Date.now(), by: ADVISOR, text: cut(cut(body, REPLY_BYTES - Buffer.byteLength(note)) + note, REPLY_BYTES) });
    return found;
  });
}

/** The fence that opens a reply's `options` block. */
const BLOCK_FENCE = /```options[ \t]*\r?\n([\s\S]*?)\r?\n?```\s*$/;

/** `reply` without the `options` block it ends with, and that block's text, or null without one. */
function splitBlock(reply: string): { said: string; block: string | null } {
  const m = BLOCK_FENCE.exec(reply);
  if (m === null) return { said: reply, block: null };
  return { said: reply.slice(0, m.index), block: m[1] ?? "" };
}

/** Applies the `options` block `block` to `decision`. Gives why it was dropped, or null once applied. */
function revise(decision: Decision, block: string): string | null {
  let v: unknown;
  try {
    v = JSON.parse(block);
  } catch {
    return "it is not JSON";
  }
  if (typeof v !== "object" || v === null || Array.isArray(v)) return "it is not a JSON object";
  const { options, recommend, ...rest } = v as Record<string, unknown>;
  if (Object.keys(rest).length > 0) return `it holds only \`options\` and \`recommend\`, not \`${Object.keys(rest)[0]}\``;
  if (decision.answer !== null) return "the decision is answered already";
  let list = decision.options;
  try {
    if (options !== undefined) {
      if (!Array.isArray(options) || options.some((o) => typeof o !== "string")) return "`options` is a list of texts";
      if (decision.builtOn !== null && JSON.stringify(options) !== JSON.stringify(decision.options)) {
        return `the session built on option ${decision.builtOn}, so only \`recommend\` may change`;
      }
      if (options.length < OPTIONS[0] || options.length > OPTIONS[1]) return `a decision lists ${OPTIONS[0]} to ${OPTIONS[1]} options`;
      list = options.map((o, i) => bounded(`option ${i + 1}`, o as string, TEXT_BYTES));
    }
    const pick = recommend === undefined ? decision.recommend : numbered("`recommend`", recommend as number, list.length);
    decision.options = list;
    decision.recommend = pick;
  } catch (e) {
    if (!(e instanceof Refused)) throw e;
    return e.message;
  }
  return null;
}

/** `text` cut to at most `max` UTF-8 bytes, a marker included, on a character's edge. */
function cut(text: string, max: number): string {
  if (Buffer.byteLength(text) <= max) return text;
  const marker = `\n[cut at ${max} bytes]`;
  const kept = Buffer.from(text).subarray(0, max - Buffer.byteLength(marker)).toString("utf8");
  return `${kept.replace(/�+$/, "")}${marker}`;
}

/** What the advisor reads of `decision`: the question, the options, the session's stance and the thread. */
export function adviceRequest(decision: Decision): string {
  const stance = decision.builtOn === null ? "It parked the work that hangs on the answer." : `It built on option ${decision.builtOn}.`;
  const out = [`======== THE DECISION: ${decision.id} ========`, "", decision.question, ""];
  decision.options.forEach((o, i) => out.push(`${i + 1}. ${o}${i + 1 === decision.recommend ? " (the session recommends it)" : ""}`));
  out.push("", `The session asked it as ${decision.by}. ${stance}${decision.item === null ? "" : ` The work is ${decision.item}.`}`);
  if (decision.blocking) out.push("The loop is held until the user answers it.");
  out.push("", "======== THE DISCUSSION ========", "", ...threadLines(decision.thread, ""));
  return out.join("\n");
}

/** `thread` as list lines, each indented by `indent`. */
function threadLines(thread: Message[], indent: string): string[] {
  return thread.map((m) => `${indent}- ${m.by === ADVISOR ? "The advisor" : `The user (${m.by})`}: ${m.text.replaceAll("\n", `\n${indent}  `)}`);
}

/**
 * Runs the advisor `cmd` with `input` on its stdin, and gives its stdout. After `ms` its whole tree is
 * killed. Throws why it failed: it could not start, timed out, or exited non-zero.
 */
export async function runAdvisor(cmd: AgentCommand, input: string, ms = ADVISOR_MS): Promise<string> {
  let child: Bun.Subprocess<"pipe", "pipe", "pipe">;
  try {
    const env = { ...process.env, ...cmd.env };
    child = Bun.spawn(cmd.argv, { cwd: cmd.cwd, env, stdin: "pipe", stdout: "pipe", stderr: "pipe", windowsHide: true });
  } catch (e) {
    throw new Error(`it could not start: ${(e as Error).message}`);
  }
  child.stdin.write(input);
  child.stdin.end();
  let timer: Timer | undefined;
  const timeout = new Promise<"timeout">((done) => {
    timer = setTimeout(() => done("timeout"), ms);
  });
  const ran = await Promise.race([Promise.all([readUpTo(child.stdout), readUpTo(child.stderr), child.exited]), timeout]);
  clearTimeout(timer);
  if (ran === "timeout") {
    killTree(child.pid);
    child.kill();
    await child.exited;
    throw new Error(`it ran past ${Math.round(ms / 1000)} s, so it was stopped`);
  }
  const [out, err, code] = ran;
  if (code !== 0) {
    const why = firstLine(err.trim() || out.trim(), SUMMARY_CHARS);
    throw new Error(`it exited ${code}${why === "" ? "" : `: ${why}`}`);
  }
  return out;
}

/** Reads `stream` to its end, keeping its first `ADVISOR_OUT` bytes as text. */
async function readUpTo(stream: ReadableStream<Uint8Array>): Promise<string> {
  const kept: Uint8Array[] = [];
  let used = 0;
  try {
    for await (const chunk of stream) {
      const into = Math.min(ADVISOR_OUT - used, chunk.length);
      if (into > 0) kept.push(chunk.subarray(0, into));
      used += into;
    }
  } catch {
    // A read that fails ends the output.
  }
  return Buffer.concat(kept).toString("utf8");
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
  for (const x of d.decisions.filter((x) => x.answer !== null)) {
    out.push(`- ${sayText(x)}`);
    if (x.thread.length > 0) out.push("  Their discussion with the advisor, before they answered:", ...threadLines(x.thread, "  "));
  }
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
