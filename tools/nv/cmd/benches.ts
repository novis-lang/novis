// `bun nv benches`: the bench turn by hand.
//
//     bun nv benches [--owed]
//
// Every bench the selection picks, owed, red, new or reached, is judged on the release build and
// recorded (`runBenches`, `proofs/select.ts`), under the sweep lock, since a bench's growth is read off
// the cores. It is how a person pays the whole bench debt the sessions' `nv verify` and `bun nv affected
// --run` leave, where `bun nv sweep` pays it a budget at a time, and it resumes a pass that was stopped,
// since each bench is recorded the moment it is judged. `--owed` narrows it to the benches selected for
// being owed; a bench never recorded stays unrecorded, and the next pass without `--owed` picks it up.
// `bun nv sweep` calls `benchTurn` in-process with the benches it pays and its deadline, and `nv guard`
// refuses this command as a heavy run.
//
// It exits 0 when no bench is red, 1 when one is or the release build failed, and 2 on a usage error.

import { takeSweepLock } from "../accept/sweep-lock.ts";

export const summary = "the bench turn by hand: judges and records every bench the selection picks on the release build: nv benches [--owed]";

const USAGE = "usage: bun nv benches [--owed]";

/** The benches `bun nv sweep` pays, by `bench:` id in the order they run, and when none may start. */
export interface BenchDebt {
  ids: string[];
  deadline: number;
  /** Told of each bench as it starts. */
  onStart?: (id: string) => void;
}

/**
 * Every bench the selection picks, owed, red, new or reached, judged and recorded by `runBenches`, under
 * the sweep lock. With `owedOnly`, only the benches selected for being owed run. With `debt`, only its
 * benches run, in its order, and none starts past its deadline. Returns a red line naming the red
 * benches, or the release build that failed, and "" when there is neither.
 */
export async function benchTurn(note: (line: string) => void, owedOnly = false, debt?: BenchDebt): Promise<string> {
  const { runBenches } = await import("../proofs/select.ts");
  const lock = await takeSweepLock({ note });
  let got: Awaited<ReturnType<typeof runBenches>>;
  try {
    got = debt
      ? await runBenches(note, new Set(debt.ids), { order: debt.ids, deadline: debt.deadline, ...(debt.onStart && { onStart: debt.onStart }) })
      : await runBenches(note, owedOnly ? (s) => s.why === "owed" : undefined);
  } finally {
    lock.release();
  }
  if (typeof got === "string") return `benches: the release build failed: ${got.split("\n")[0]}`;
  for (const r of got.red) note(`benches: RED ${r.path}: ${r.findings.join("; ")}`);
  const left = got.left > 0 ? `, ${got.left} left for the next run` : "";
  note(`benches: ${got.reached} selected, ${got.ran} judged and recorded, ${got.skipped.size} skipped as proven, ${got.red.length} red${left}`);
  if (got.red.length === 0) return "";
  const named = got.red.slice(0, 5).map((r) => r.path).join(", ");
  return `benches: ${got.red.length} red bench(es), their growth failed: ${named}${got.red.length > 5 ? ", ..." : ""}`;
}

/** Parses, runs `turn` and prints; returns the exit code. `turn` is `benchTurn` except under test. */
export async function benches(args: string[], turn: typeof benchTurn = benchTurn): Promise<number> {
  const bad = args.find((a) => a !== "--owed");
  if (bad !== undefined || args.length > 1) {
    console.error(`${USAGE}\nnv benches: error: unexpected argument ${bad ?? args[1]}`);
    return 2;
  }
  const owed = args.length === 1;
  console.log(`benches: judging every ${owed ? "owed bench" : "bench the selection picks, owed, red, new or reached"} on the release build, then recording it`);
  const red = await turn((l) => console.log(`  ${l}`), owed);
  if (red) console.log(red);
  return red ? 1 : 0;
}

export async function run(args: string[]): Promise<number> {
  if (args.includes("--help") || args.includes("-h")) {
    console.log(`${USAGE}\n\n${summary}`);
    return 0;
  }
  return benches(args);
}
