import { afterEach, expect, test } from "bun:test";
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import {
  type GateConsole,
  HANDOVER_SUBJECT,
  LOOPER_ENV,
  awaitGate,
  currentPath,
  gateShut,
  looperArgv,
  putLooperFirst,
  retireChain,
  startLooper,
  waitAndHandoverProblems,
} from "../driver/handover.ts";
import { scratch, type Scratch } from "./scratch.ts";

let tmp: Scratch | undefined;
afterEach(() => {
  tmp?.cleanup();
  tmp = undefined;
});

const bun = process.execPath;
/** A program that prints its arguments and exits with the code in its first one. */
const echo = (code: number, ...words: string[]) => [bun, "-e", `console.log(${JSON.stringify(words.join(" "))}); process.exit(${code})`];

test("the wait gate is open when its command exits 0 and prints every wanted string", async () => {
  expect(await gateShut({ argv: echo(0, "imported docs/goals/compare-page.md"), want: ["compare-page.md"], everyMin: 1 })).toBe("");
  expect(await gateShut({ argv: echo(0, "anything"), everyMin: 1 })).toBe("");
});

test("the wait gate is shut on a failing command, a missing wanted string, or a command that does not start", async () => {
  expect(await gateShut({ argv: echo(3, "no such importer"), everyMin: 1 })).toContain("exited 3: no such importer");
  expect(await gateShut({ argv: echo(0, "imported docs/goals/a.md"), want: ["compare-page.md"], everyMin: 1 })).toContain('printed no "compare-page.md"');
  const lossy = { argv: echo(0, "would write docs/goals/compare-page.md\nnote: 3 bullet(s) lost their until"), want: ["compare-page.md"], absent: ["dropped", "lost"], everyMin: 1 };
  expect(await gateShut(lossy)).toContain('still printed "note: 3 bullet(s) lost their until"');
  expect(await gateShut({ ...lossy, argv: echo(0, "would write docs/goals/compare-page.md") })).toBe("");
  expect(await gateShut({ argv: ["nv-no-such-program-anywhere"], everyMin: 1 })).toContain("did not start");
});

/** A console for `awaitGate` that answers from `answers` in turn and records what the wait did. */
function fakeConsole(answers: string[], over: Partial<GateConsole> = {}) {
  const seen = { checks: 0, sleeps: [] as number[], notes: [] as string[], phases: [] as string[], parked: [] as boolean[], retries: 0 };
  let clock = 0;
  const c: GateConsole = {
    check: async () => answers[Math.min(seen.checks++, answers.length - 1)]!,
    sleep: async (seconds) => {
      seen.sleeps.push(seconds);
      clock += seconds;
    },
    stopReason: () => "",
    paused: () => false,
    hold: async () => "",
    takeRetry: () => {
      seen.retries++;
      return false;
    },
    pending: () => false,
    note: (l) => seen.notes.push(l),
    phase: (t) => seen.phases.push(t),
    now: () => clock,
    park: (on) => seen.parked.push(on),
    ...over,
  };
  return { c, seen };
}

test("a red wait gate starts no session: it idles on its cadence and says each new reason once", async () => {
  const { c, seen } = fakeConsole(["looper is too old", "looper is too old", "no compare-page", ""]);
  expect(await awaitGate("cutover", { argv: ["looper", "import"], everyMin: 15 }, c)).toBe("");
  expect(seen.checks).toBe(4);
  expect(seen.sleeps).toEqual([900, 900, 900]);
  // Two distinct reasons, then the gate opening.
  expect(seen.notes.length).toBe(3);
  expect(seen.notes[0]).toContain("waits, and no session starts: looper is too old");
  expect(seen.notes[2]).toContain("wait gate open");
  expect(seen.phases).toContain("waiting for `looper import`");
  expect(seen.phases.at(-1)).toBe("");
  expect(seen.parked).toEqual([true, false]);
});

test("a wait gate that is already open says nothing and does not sleep", async () => {
  const { c, seen } = fakeConsole([""]);
  expect(await awaitGate("cutover", { argv: ["looper"], everyMin: 15 }, c)).toBe("");
  expect(seen.sleeps).toEqual([]);
  expect(seen.notes).toEqual([]);
});

test("a stop during the wait ends the run, and a hold during it is taken before the next run of the command", async () => {
  let stops = 0;
  const stopped = fakeConsole(["red"], { stopReason: () => (stops++ > 0 ? "s was pressed at the console" : "") });
  expect(await awaitGate("cutover", { argv: ["looper"], everyMin: 1 }, stopped.c)).toContain("s was pressed at the console, while goal `cutover` waited");
  expect(stopped.seen.checks).toBe(1);

  let holds = 0;
  let paused = true;
  const held = fakeConsole(["red", ""], {
    paused: () => paused,
    hold: async () => {
      holds++;
      paused = false;
      return "";
    },
  });
  expect(await awaitGate("cutover", { argv: ["looper"], everyMin: 1 }, held.c)).toBe("");
  expect(holds).toBe(1);
  // The sleep ends early on a pending request or a hold, and a retry request is consumed after it.
  expect(held.seen.retries).toBe(1);
});

test("a goal's wait gate and handover are checked where the schema cannot", () => {
  expect(waitAndHandoverProblems("g", { waitFor: { argv: ["looper"], everyMin: 15 }, handover: "looper" }, false)).toEqual([]);
  const bad = waitAndHandoverProblems("g", { waitFor: { argv: [], want: [""], absent: [""], everyMin: 0 } }, false);
  expect(bad.length).toBe(4);
  expect(waitAndHandoverProblems("s", { handover: "looper" }, true)[0]).toContain("a side goal never hands the run over");
});

test("the looper the driver starts is NOVIS_LOOPER when set, and looper from the current Path otherwise", () => {
  expect(looperArgv({ [LOOPER_ENV]: " bun stub.ts " })).toEqual(["bun", "stub.ts"]);
  expect(looperArgv({}).length).toBe(1);
  expect(currentPath("inherited-dir").endsWith("inherited-dir")).toBe(true);
});

test("a goal's looper goes first on the turn's PATH once, and NOVIS_LOOPER leaves PATH alone", () => {
  const stubbed = { [LOOPER_ENV]: "bun stub.ts", PATH: "a" };
  putLooperFirst(stubbed);
  expect(stubbed.PATH).toBe("a");
  const env: Record<string, string | undefined> = { PATH: process.env.PATH };
  putLooperFirst(env);
  const once = env.PATH;
  putLooperFirst(env);
  expect(env.PATH).toBe(once);
});

test("a wait gate whose program is looper runs the looper the handover would start", async () => {
  tmp = scratch();
  const stub = stubLooper(tmp, {});
  expect(await gateShut({ argv: ["looper", "import", "--dry-run"], everyMin: 1 }, tmp.root, undefined, () => stub)).toBe("");
  expect(calls(tmp)).toEqual(["import --dry-run"]);
});

/** A scratch repository with a chain of `goals`, each goal with its three files, all committed. */
function repo(goals: string[]): { t: Scratch; git: (...args: string[]) => string } {
  const t = scratch();
  const git = (...args: string[]) => {
    const r = Bun.spawnSync(["git", ...args], { cwd: t.root, stdout: "pipe", stderr: "pipe" });
    if (r.exitCode !== 0) throw new Error(`git ${args.join(" ")}: ${r.stderr.toString()}`);
    return r.stdout.toString().trim();
  };
  git("init", "-q");
  git("config", "user.email", "test@example.com");
  git("config", "user.name", "test");
  git("config", "core.hooksPath", ".no-hooks");
  git("config", "commit.gpgsign", "false");
  t.put("data/chain.json", `${JSON.stringify({ live: goals[0], goals })}\n`);
  for (const g of goals) {
    t.put(`data/goals/${g}.json`, "{}\n");
    t.put(`data/goals/${g}.handoff.json`, "{}\n");
    t.put(`docs/agent/goals/${g}.md`, `# ${g}\n`);
  }
  t.put("other.txt", "kept\n");
  git("add", ".");
  git("commit", "-q", "-m", "start");
  return { t, git };
}

test("the handover deletes the goal and the chain and commits those paths alone", async () => {
  const { t, git } = repo(["cutover"]);
  tmp = t;
  // Another agent's edit in the same tree stays out of the commit.
  t.put("other.txt", "edited\n");
  expect(await retireChain("cutover", t.root)).toBe("");
  for (const p of ["data/chain.json", "data/goals/cutover.json", "data/goals/cutover.handoff.json", "docs/agent/goals/cutover.md"]) expect(existsSync(join(t.root, p))).toBe(false);
  expect(git("log", "-1", "--format=%s")).toBe(HANDOVER_SUBJECT);
  expect(git("show", "--name-only", "--format=", "HEAD").split("\n").sort()).toEqual(["data/chain.json", "data/goals/cutover.handoff.json", "data/goals/cutover.json", "docs/agent/goals/cutover.md"]);
  expect(git("status", "--porcelain")).toBe("M other.txt");
  // A second call finds nothing left to delete or commit.
  expect(await retireChain("cutover", t.root)).toBe("");
  expect(git("rev-list", "--count", "HEAD")).toBe("2");
}, 60_000);

test("the handover refuses while the chain names another goal, and deletes nothing", async () => {
  const { t } = repo(["cutover", "later"]);
  tmp = t;
  expect(await retireChain("cutover", t.root)).toContain("still names `later`");
  expect(existsSync(join(t.root, "data/chain.json"))).toBe(true);
  expect(existsSync(join(t.root, "docs/agent/goals/cutover.md"))).toBe(true);
}, 60_000);

/**
 * A stub looper under the scratch root: it logs each call, and for a call whose first two words `exits`
 * names, prints the message and exits with the code given there.
 */
function stubLooper(t: Scratch, exits: Record<string, [number, string]>): string[] {
  t.put("exits.json", JSON.stringify(exits));
  t.put(
    "stub.ts",
    [
      'import { appendFileSync, readFileSync } from "node:fs";',
      'import { join } from "node:path";',
      "const args = process.argv.slice(2);",
      'appendFileSync(join(import.meta.dir, "calls.txt"), args.join(" ") + "\\n");',
      'const [code, said] = JSON.parse(readFileSync(join(import.meta.dir, "exits.json"), "utf8"))[args.slice(0, 2).join(" ")] ?? [0, ""];',
      "if (said) console.error(said);",
      "process.exit(code);",
    ].join("\n"),
  );
  return [bun, join(t.root, "stub.ts")];
}

const calls = (t: Scratch) => readFileSync(join(t.root, "calls.txt"), "utf8").trim().split("\n");

test("the handover registers the project, adds looper's main loop and starts it", async () => {
  tmp = scratch();
  expect(await startLooper(tmp.root, stubLooper(tmp, {}))).toBe("");
  expect(calls(tmp)).toEqual(["project add .", "loop add", "loop start"]);
});

test("a main loop that already exists, or already runs, counts as started", async () => {
  tmp = scratch();
  const said: string[] = [];
  const stub = stubLooper(tmp, { "loop add": [1, "error: Project mwl has a main loop already."], "loop start": [1, "error: the loop has a worker already."] });
  expect(await startLooper(tmp.root, stub, undefined, (l) => said.push(l))).toBe("");
  expect(calls(tmp)).toEqual(["project add .", "loop add", "loop start"]);
  expect(said[1]).toContain("the start runs anyway");
  expect(said[2]).toContain("the main loop runs already");
});

test("a looper start that fails is the error the run holds on, naming what loop add said too", async () => {
  tmp = scratch();
  const failed = await startLooper(tmp.root, stubLooper(tmp, { "loop add": [1, "error: add refused"], "loop start": [2, "error: no server"] }));
  expect(failed).toContain("loop start` exited 2: error: no server");
  expect(failed).toContain("loop add` exited 1: error: add refused");
});
