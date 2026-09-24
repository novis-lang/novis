// `bun nv ci-green`: whether the latest `ci.yml` run on `main` succeeded, and whether it ran the code
// `HEAD` holds.
//
//     bun nv ci-green           exit 0 and the sentence below, or exit 1 and what is missing
//
// Goal `ci-green`'s acceptance check. `gh run list` alone answers the first half and cannot answer the
// second: the newest run is the newest *push*, the loop never pushes, and a green run for an old
// commit proves nothing about the tree the goal is claimed on.
//
// **The run's commit need not be `HEAD` itself.** The session that sees the green run commits its
// handoff, which moves `HEAD` past the commit the run was for, and a check demanding equality could
// never be met by the session that has to meet it. So the run's commit must be an ancestor of `HEAD`,
// and every path changed since must sit under `BOOKKEEPING` -- the files a session writes to say where
// the work stands, which no CI leg builds.

import { run as runProc } from "../lib/proc.ts";

export const summary = "whether the latest ci.yml run on main is green for the code HEAD holds: nv ci-green";

/** The line goal `ci-green`'s check reads. One string, here. */
const GREEN = "the latest ci.yml run on main succeeded, and it ran the code HEAD holds";

// What a session writes after the work is done. A prefix ending in `/` matches everything beneath it.
const BOOKKEEPING = ["docs/agent/", "docs/plan/", "docs/implementation-plan.md"];

async function out(...argv: string[]): Promise<[number, string]> {
  const r = await runProc(argv);
  return [r.code, r.stdout.trim() || r.stderr.trim()];
}

interface Run {
  conclusion: string;
  status: string;
  headSha: string;
  url: string;
}

export async function run(args: string[]): Promise<number> {
  if (args.some((a) => a === "-h" || a === "--help")) {
    console.log(summary);
    return 0;
  }
  const [code, text] = await out("gh", "run", "list", "--branch", "main", "--workflow", "ci.yml", "--limit", "1", "--json", "conclusion,status,headSha,url");
  if (code) {
    console.log(`ci-green: \`gh run list\` failed -- ${text}`);
    return 1;
  }
  const runs = JSON.parse(text || "[]") as Run[];
  if (runs.length === 0) {
    console.log("ci-green: no ci.yml run on main exists");
    return 1;
  }
  const r = runs[0]!;
  const sha = r.headSha;
  if (r.status !== "completed") {
    console.log(`ci-green: the latest run is still ${r.status} -- ${r.url}`);
    return 1;
  }
  if (r.conclusion !== "success") {
    console.log(`ci-green: the latest run ended \`${r.conclusion}\` for ${sha.slice(0, 9)} -- ${r.url}`);
    return 1;
  }
  const [, head] = await out("git", "rev-parse", "HEAD");
  if ((await out("git", "merge-base", "--is-ancestor", sha, "HEAD"))[0]) {
    console.log(`ci-green: the green run is for ${sha.slice(0, 9)}, which is not an ancestor of HEAD ${head.slice(0, 9)}`);
    return 1;
  }
  const [, changed] = await out("git", "diff", "--name-only", sha, "HEAD");
  const unproven = changed
    .split(/\r?\n/)
    .filter(Boolean)
    .filter((p) => !BOOKKEEPING.some((b) => p === b || (b.endsWith("/") && p.startsWith(b))));
  if (unproven.length > 0) {
    console.log(
      `ci-green: the green run is for ${sha.slice(0, 9)}, and ${unproven.length} path(s) it never saw ` +
        `have changed since -- push ${head.slice(0, 9)} and wait for its run:`,
    );
    for (const path of unproven.slice(0, 20)) console.log(`       ${path}`);
    if (unproven.length > 20) console.log(`       ... and ${unproven.length - 20} more`);
    return 1;
  }
  console.log(`ci-green: ${GREEN} (${sha.slice(0, 9)})`);
  return 0;
}
