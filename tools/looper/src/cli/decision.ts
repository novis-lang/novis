// `decision`: a loop's decisions for the user, asked, answered, closed and listed in its folder.
// `../loop/decisions.ts` is the home of the record and its bounds.
//
// - `decision ask <question> --option <text>… --recommend <n> (--parked | --built-on <n>) [--item
//   <anchor>]` records an open decision, and prints its id.
// - `decision answer <id> (--option <n> | --text <text>) [--note <text>]` records the user's answer and
//   says it to the loop. Looper's UI runs it for its answer.
// - `decision discuss <id> --text <message>` adds the user's message to the decision's thread, runs its
//   read-only advisor in the loop's tree, and prints the reply. The usage wall on the loop's account
//   refuses it, and the advisor takes no heavy-job slot.
//   - Its hidden `--detach` returns once the message is written, and a detached `discuss <id> --reply`
//     runs the advisor. Looper's service runs it so, and so never waits on an advisor.
//   - Its hidden `--advisor <script>` runs that script with Bun in place of `claude`, for a test.
// - `decision close <id>` drops an answered decision, once a session has recorded the answer under the
//   goal's `## Standing decisions`.
// - `decision list` prints every decision that stands.
//
// Each verb takes `--loop <name>`, which defaults to `LOOPER_LOOP`, then `main`. The loop's folder is in
// the project `LOOPER_ROOT` names, or else the current directory's. So a session in a side loop's lane
// reaches its loop's folder in the root, which the lane's own tree does not hold. A project with no such
// loop is refused.
//
// With `--json`, `ask`, `answer`, `discuss` and `close` print `{"loop", "decision"}`, and `list` `{"loop",
// "decisions"}`.

import { existsSync } from "node:fs";
import { basename, join, resolve } from "node:path";
import { loadConfig } from "../goal/config.ts";
import { Refused, SIDE_GOAL_VAR } from "../goal/model.ts";
import { pack } from "../goal/orient.ts";
import { looperHome } from "../jobs/slots.ts";
import { spawnDetached } from "../os/index.ts";
import { adviceText, adviseLine } from "../loop/claude.ts";
import { type Advisor, adviceRequest, type Answered, answer, ask, close, type Decision, discuss, post, readDecisions, reply, runAdvisor } from "../loop/decisions.ts";
import { ADVISOR_PROMPT } from "../loop/driver.ts";
import { loopRecord, loopsDir } from "../loop/lanes.ts";
import { clearsAt, Walls } from "../loop/wall.ts";
import { asker } from "./claims.ts";
import { extra as packExtra } from "./orient.ts";
import { type Command, Refusal, type Report, usage } from "./cli.ts";
import { flag, loopName, noMore, option, whole } from "./loop.ts";

/** The kit's entry, which a detached reply runs. */
const MAIN = join(import.meta.dir, "..", "..", "main.ts");

/** The loop the verb works on: `--loop`, then `LOOPER_LOOP`, then `main`. */
function chosenLoop(argv: string[]): { name: string; rest: string[] } {
  const { name, named, rest } = loopName(argv);
  if (named) return { name, rest };
  const env = process.env.LOOPER_LOOP ?? "";
  return { name: /^[A-Za-z0-9][A-Za-z0-9_-]*$/.test(env) ? env : "main", rest };
}

/** The folder of loop `name` in the project `LOOPER_ROOT` names, or the current directory's. */
function folder(name: string): string {
  const root = process.env.LOOPER_ROOT || process.cwd();
  const dir = join(loopsDir(root), name);
  if (!existsSync(dir)) throw new Refusal(`this project has no loop ${name}: \`loop add\` adds its main loop`);
  return dir;
}

/** One decision as `list` prints it. */
function line(d: Decision): string {
  const stance = d.builtOn === null ? "parked" : `built on ${d.builtOn}`;
  const state = d.answer === null ? "open" : d.answer.option !== null ? `answered: option ${d.answer.option}` : `answered: "${d.answer.text}"`;
  const options = d.options.map((o, i) => `  ${i + 1}. ${o}${i + 1 === d.recommend ? " (recommended)" : ""}`);
  const head = `${d.id}  ${state}, ${stance}${d.blocking ? ", blocking the loop" : ""}${d.item === null ? "" : `, ${d.item}`}`;
  const thread = d.thread.length === 0 ? [] : [`  ${d.thread.length} message(s) in its discussion`];
  return [head, `  ${d.question}`, ...options, ...thread].join("\n");
}

function askVerb(dir: string, args: string[]): Promise<Decision> {
  const options: string[] = [];
  for (let o = option(args, "--option"); o !== undefined; o = option(args, "--option")) options.push(o);
  const recommend = option(args, "--recommend");
  if (recommend === undefined) throw usage("`decision ask` takes `--recommend <n>`");
  const parked = flag(args, "--parked");
  const builtOn = option(args, "--built-on");
  if (parked === (builtOn !== undefined)) throw usage("`decision ask` takes one of `--parked` and `--built-on <n>`");
  const item = option(args, "--item");
  const [question, ...more] = args;
  if (question === undefined) throw usage("`decision ask` takes the question");
  noMore(more, "decision ask");
  return ask(dir, {
    question,
    options,
    recommend: whole("--recommend", recommend, 1),
    builtOn: builtOn === undefined ? null : whole("--built-on", builtOn, 1),
    item,
    by: asker(),
  });
}

function answerVerb(dir: string, args: string[]): Promise<Decision> {
  const picked = option(args, "--option");
  const text = option(args, "--text");
  const note = option(args, "--note");
  const [id, ...more] = args;
  if (id === undefined) throw usage("`decision answer` takes the decision's id");
  noMore(more, "decision answer");
  const a: Answered = { text, note, by: asker() };
  if (picked !== undefined) a.option = whole("--option", picked, 1);
  return answer(dir, id, a);
}

/** Refuses while the usage wall on `account` stands, with the wall's sentence. */
async function wallRefuses(root: string, account: string): Promise<void> {
  const walls = new Walls(join(root, ".looper", "walls.json"), looperHome());
  await walls.pull(account, Date.now());
  const wall = walls.standing(account, Date.now());
  if (wall === null) return;
  const kind = wall.kind === "" ? "usage" : wall.kind;
  throw new Refused(`the ${kind} wall on ${account} stands until ${new Date(clearsAt(wall)).toISOString()}: send the message once it clears`);
}

/**
 * `decision discuss`: the user's message, then the advisor's reply in the loop's tree. The usage wall
 * refuses it. With `--detach` a detached `--reply` run of this verb answers, and the verb returns once
 * the message is written. `--reply` runs the advisor on the message waiting for it.
 */
async function discussVerb(dir: string, args: string[]): Promise<Decision> {
  const text = option(args, "--text");
  const script = option(args, "--advisor");
  const detach = flag(args, "--detach");
  const replying = flag(args, "--reply");
  const [id, ...more] = args;
  if (id === undefined) throw usage("`decision discuss` takes the decision's id");
  if ((text === undefined) === !replying) throw usage("`decision discuss` takes `--text <message>`, or `--reply` alone");
  noMore(more, "decision discuss");
  const root = resolve(dir, "..", "..", "..");
  const record = loopRecord(root, dir);
  const tree = record.worktree === "" ? root : record.worktree;
  const cfg = loadConfig(tree);
  // A reply answers a message the wall let through.
  if (text !== undefined) await wallRefuses(root, cfg.agent.adapter);
  // A side loop in the root reads its own goal as the live one, as its sessions do.
  if (record.kind === "side" && record.inRoot === true && record.goal !== null) process.env[SIDE_GOAL_VAR] = record.goal;
  const advise: Advisor = async (d) => {
    const sections = await packExtra(tree, cfg, undefined, false);
    const prompt = `${ADVISOR_PROMPT}\n\n${adviceRequest(d)}\n${pack(tree, cfg, false, undefined, sections)}`;
    const program = script === undefined ? ["claude"] : [process.execPath, script];
    return adviceText(await runAdvisor(adviseLine(program, cfg.agent.model, tree), prompt));
  };
  if (text === undefined) return reply(dir, id, advise);
  if (!detach) return discuss(dir, id, { text, by: asker() }, advise);
  const posted = await post(dir, id, { text, by: asker() });
  const argv = [process.execPath, MAIN, "decision", `--loop=${basename(dir)}`, "discuss", id, "--reply"];
  // One argument with `=`, so a script path is never read as a flag.
  if (script !== undefined) argv.push(`--advisor=${script}`);
  try {
    spawnDetached(argv, { cwd: root });
  } catch (e) {
    return reply(dir, id, () => Promise.reject(e));
  }
  return posted;
}

function closeVerb(dir: string, args: string[]): Promise<Decision> {
  const [id, ...more] = args;
  if (id === undefined) throw usage("`decision close` takes the decision's id");
  noMore(more, "decision close");
  return close(dir, id);
}

async function run(argv: string[]): Promise<Report> {
  const { name, rest } = chosenLoop(argv);
  const [verb, ...args] = rest;
  const verbs = ["ask", "answer", "discuss", "close", "list"];
  if (verb === undefined || !verbs.includes(verb)) throw usage("`decision` takes ask, answer, discuss, close or list");
  const dir = folder(name);
  try {
    if (verb === "list") {
      noMore(args, "decision list");
      const { decisions } = readDecisions(dir);
      return { doc: { loop: name, decisions }, text: decisions.length === 0 ? `loop ${name} has no decisions` : decisions.map(line).join("\n") };
    }
    const act = { ask: askVerb, answer: answerVerb, discuss: discussVerb, close: closeVerb }[verb as "ask" | "answer" | "discuss" | "close"];
    const d = await act(dir, args);
    const said = { ask: d.id, answer: `decision ${d.id} answered, and said to the loop`, close: `decision ${d.id} closed` } as Record<string, string>;
    const text = said[verb] ?? d.thread.at(-1)?.text ?? "";
    return { doc: { loop: name, decision: d }, text };
  } catch (e) {
    throw e instanceof Refused ? new Refusal(e.message) : e;
  }
}

export const decisionCommand: Command = {
  name: "decision",
  about: "Asks the user a decision while the loop runs on, and answers, discusses, closes and lists them: `decision ask|answer|discuss|close|list`.",
  json: true,
  run,
};
