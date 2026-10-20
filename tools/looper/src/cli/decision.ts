// `decision`: a loop's decisions for the user, asked, answered, closed and listed in its folder.
// `../loop/decisions.ts` is the home of the record and its bounds.
//
// - `decision ask <question> --option <text>… --recommend <n> (--parked | --built-on <n>) [--item
//   <anchor>]` records an open decision, and prints its id.
// - `decision answer <id> (--option <n> | --text <text>) [--note <text>]` records the user's answer and
//   says it to the loop. Looper's UI runs it for its answer.
// - `decision close <id>` drops an answered decision, once a session has recorded the answer under the
//   goal's `## Standing decisions`.
// - `decision list` prints every decision that stands.
//
// Each verb takes `--loop <name>`, which defaults to `LOOPER_LOOP`, then `main`. The loop's folder is in
// the project `LOOPER_ROOT` names, or else the current directory's. So a session in a side loop's lane
// reaches its loop's folder in the root, which the lane's own tree does not hold. A project with no such
// loop is refused.
//
// With `--json`, `ask`, `answer` and `close` print `{"loop", "decision"}`, and `list` `{"loop",
// "decisions"}`.

import { existsSync } from "node:fs";
import { join } from "node:path";
import { Refused } from "../goal/model.ts";
import { loopsDir } from "../loop/lanes.ts";
import { type Answered, answer, ask, close, type Decision, readDecisions } from "../loop/decisions.ts";
import { asker } from "./claims.ts";
import { type Command, Refusal, type Report, usage } from "./cli.ts";
import { flag, loopName, noMore, option, whole } from "./loop.ts";

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
  return [`${d.id}  ${state}, ${stance}${d.item === null ? "" : `, ${d.item}`}`, `  ${d.question}`, ...options].join("\n");
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

function closeVerb(dir: string, args: string[]): Promise<Decision> {
  const [id, ...more] = args;
  if (id === undefined) throw usage("`decision close` takes the decision's id");
  noMore(more, "decision close");
  return close(dir, id);
}

async function run(argv: string[]): Promise<Report> {
  const { name, rest } = chosenLoop(argv);
  const [verb, ...args] = rest;
  if (verb !== "ask" && verb !== "answer" && verb !== "close" && verb !== "list") throw usage("`decision` takes ask, answer, close or list");
  const dir = folder(name);
  try {
    if (verb === "list") {
      noMore(args, "decision list");
      const { decisions } = readDecisions(dir);
      return { doc: { loop: name, decisions }, text: decisions.length === 0 ? `loop ${name} has no decisions` : decisions.map(line).join("\n") };
    }
    const d = await (verb === "ask" ? askVerb : verb === "answer" ? answerVerb : closeVerb)(dir, args);
    const text = verb === "ask" ? d.id : `decision ${d.id} ${verb === "answer" ? "answered, and said to the loop" : "closed"}`;
    return { doc: { loop: name, decision: d }, text };
  } catch (e) {
    throw e instanceof Refused ? new Refusal(e.message) : e;
  }
}

export const decisionCommand: Command = {
  name: "decision",
  about: "Asks the user a decision while the loop runs on, and answers, closes and lists them: `decision ask|answer|close|list`.",
  json: true,
  run,
};
