// `bun nv brief --where [<keyword>...]`: routes a keyword to the file that owns the topic. A keyword
// hits a rule when every word is in the rule's id or title, and the rule's chapter
// `docs/rules/<topic>.md` is the file that owns it. It hits one of `HOMES` when every word is in that
// row's words or its path: those are the homes that are not rules, because they are about the
// repository rather than the language. With no keyword, prints the chapter list and every home.
//
// The rulebook is read from the `topic` and `rule` records under `data/rules/`.

import { load } from "../lib/store.ts";
import { rule } from "../schema/rule.ts";
import { classify, collect, REFUSED } from "./owners.ts";
import { topic } from "../schema/topic.ts";

export const summary = "route a keyword to the file that owns the topic: nv brief --where [<keyword>...]";

/** The homes that are not rules: the words that hit one, the home, and what is there. */
const HOMES: [string, string, string][] = [
  ["plan milestone schedule goal", "docs/plan/ + docs/implementation-plan.md",
    "the status block and the milestone table; one file per milestone; the chain is the schedule"],
  ["tools commands scripts", "docs/agent/commands.md",
    "how the repo is driven: nv peek, nv verify, nv session, nv splice, nv plan, nv disk"],
  ["benches benchmark", "benches/ + docs/perf/",
    "the benchmark programs, and the figures with the methodology that took them"],
  ["perf performance latency throughput", "docs/perf/",
    "the measured figures, the methodology and the regressions"],
  ["comment comments changelog history prose dates docstring",
    "docs/agent/conventions.md, 'A code comment'",
    "a comment says what the code does now: no date, no volatile count, rewritten whole"],
  ["unowned owner owners ownership gap gaps register",
    "bun nv owners --registers",
    "who owns each `# Known gaps` item, derived: a milestone still ahead, and nothing else"],
  ["order ordering position band chapters",
    "docs/agent/conventions.md, 'Where a rule sits in the order'",
    "the rulebook reads ground-up, not by date: the five bands, and where a new rule is inserted"],
  ["report reports issue issues feedback triage reporter", "docs/agent/user-report.md",
    "a report from somebody using Novis: intake, classify, search the plans, ask, place, neutral names"],
];

/** The home whose answer adds a line about the gap register. */
const OWNERS_HOME = "bun nv owners --registers";

/** How much of the gap register is open, counted now by `nv owners`' own classification. */
function ownershipLine(): string {
  const kinds = classify(collect());
  const refused = REFUSED.reduce((n, k) => n + kinds[k].length, 0);
  const tagged = Object.entries(kinds).reduce((n, [k, v]) => n + (k === "untagged" ? 0 : v.length), 0);
  return `${kinds.milestone.length} of ${tagged} tagged item(s) name a milestone still ahead today, and ` +
    `${refused} name an owner the gate refuses; \`bun nv owners --check\` lists those with their anchors`;
}

/** A display cap on one answer, not on anything an author writes. */
const WHERE_CAP = 40;

interface Rule {
  id: string;
  topic: string;
  title: string;
  status: string;
}

interface Topic {
  id: string;
  title: string;
  rules: Rule[];
}

/** Every topic in chapter order, each with its rules in the order its chapter prints them. */
function rulebook(): Topic[] {
  const rules = new Map(load(rule).map((r) => [r.id, r.value]));
  return load(topic)
    .sort((a, b) => a.value.order - b.value.order || (a.id < b.id ? -1 : a.id > b.id ? 1 : 0))
    .map((t) => ({
      id: t.id,
      title: t.value.title,
      rules: t.value.rules.flatMap((id) => {
        const r = rules.get(id);
        return r ? [{ id, topic: t.id, title: r.title, status: r.status }] : [];
      }),
    }));
}

function where(terms: string[]): number {
  const topics = rulebook();
  const all = topics.flatMap((t) => t.rules);
  const out: string[] = [];
  if (terms.length === 0) {
    out.push(`Routing: ${topics.length} chapters under docs/rules/, ${all.length} rules, and ${HOMES.length} homes that are not rules.`);
    out.push("Re-run with a keyword for what matches: bun nv brief --where <keyword>", "");
    for (const t of topics) out.push(`  docs/rules/${t.id}.md  ${t.title} (${t.rules.length} rules)`);
    out.push("");
    for (const [, home, what] of HOMES) out.push(`  ${home}  ${what}`);
    console.log(out.join("\n"));
    return 0;
  }
  const needles = terms.map((t) => t.toLowerCase());
  // A home is hit through its words and its path, never its description: "plan" should not route to
  // commands.md because that line happens to name `nv plan`.
  const homes = HOMES.filter(([words, home]) => needles.every((n) => `${words} ${home}`.toLowerCase().includes(n)));
  const hits = all.filter((r) => needles.every((n) => `${r.id} ${r.title}`.toLowerCase().includes(n)));
  if (hits.length === 0 && homes.length === 0) {
    console.log(
      `nv brief --where: no rule id, rule title or home matches '${terms.join(" ")}'. Run ` +
        "`--where` with no keyword for the chapter list, or grep docs/rules/ for the words.",
    );
    return 0;
  }
  for (const [, home, what] of homes) {
    out.push(`  ${home}`, `     ${what}`);
    if (home === OWNERS_HOME) out.push(`     ${ownershipLine()}`);
  }
  for (const r of hits.slice(0, WHERE_CAP)) {
    const mark = r.status === "shipped" ? "" : "  (designed)";
    const anchor = r.id.replace("/", "-");
    out.push(`  docs/rules/${r.topic}.md#${anchor}  rule:${r.id}${mark}`, `     ${r.title}`);
  }
  if (hits.length > WHERE_CAP) out.push(`  ... and ${hits.length - WHERE_CAP} more; narrow the keyword`);
  if (hits.length > 0) {
    // The path is the generated chapter, which is the reader's link and too big to fetch whole. The
    // token beside it is a `peek` target, so the next call is the answer rather than another routing
    // step, and several tokens go in one call.
    out.push(
      "",
      "  Read one -- or several -- in one call:  bun nv peek " + hits.slice(0, 2).map((r) => `rule:${r.id}`).join(" "),
      "  (the chapter path is the link; the `rule:` token is the target)",
    );
  }
  console.log(out.join("\n"));
  return 0;
}

export async function run(args: string[]): Promise<number> {
  const at = args.indexOf("--where");
  if (at < 0) {
    console.error("usage: bun nv brief --where [<keyword>...]\n  `bun nv orient` prints the orientation, and `--full` widens its map to every module");
    return 2;
  }
  return where(args.slice(at + 1).filter((a) => !a.startsWith("--")));
}
