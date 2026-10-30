// `bun nv sweep`, looper's hook after every session, with the disk sweep and `gh` handed in, so nothing
// is deleted and nothing is asked of GitHub: what it reads of the nightly CI run, the lines it writes into
// `.loop/last-sweep.md` beside looper's, the progress events it writes under looper, and its exit code.

import { afterAll, describe, expect, test } from "bun:test";
import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { type GhRun, HOOK_LINE, LAST_SWEEP, NIGHTLY_LINE, type SweepCmdSeams, type SweepVoice, sweep } from "../cmd/sweep.ts";
import { ROOT } from "../lib/paths.ts";

const TMP = join(ROOT, ".agent-tmp", `after-session-${process.pid}`);
afterAll(() => rmSync(TMP, { recursive: true, force: true }));

const SILENT: SweepVoice = { step() {}, note() {} };

/** What looper's own sweep wrote into the file just before the hook. */
const LOOPER_LINE = "- `light` is red: exit 1, want 0 (log: checks/light.log)";

const RUN = "https://github.com/example/novis/actions/runs";

/** `gh` answering the nightly run list with `runs`, and each run's jobs from `jobs`. */
function ghOf(runs: { databaseId: number; status: string; conclusion: string }[], jobs: Record<number, { name: string; conclusion: string }[]> = {}) {
  const asked: string[][] = [];
  const gh = async (args: string[]): Promise<GhRun> => {
    asked.push(args);
    if (args[1] === "list") return { code: 0, stdout: JSON.stringify(runs.map((r) => ({ ...r, url: `${RUN}/${r.databaseId}` }))), stderr: "" };
    return { code: 0, stdout: JSON.stringify({ jobs: jobs[Number(args[2])] ?? [] }), stderr: "" };
  };
  return { gh, asked };
}

const GREEN_NIGHTLY = ghOf([{ databaseId: 1, status: "completed", conclusion: "success" }]).gh;

/** Seams over a scratch root, with `gh` answering `nightly` and a disk sweep that frees nothing. */
function hook(name: string, nightly: SweepCmdSeams["gh"] = GREEN_NIGHTLY): SweepCmdSeams & { kept: number[] } {
  const root = join(TMP, name);
  rmSync(root, { recursive: true, force: true });
  mkdirSync(root, { recursive: true });
  const kept: number[] = [];
  return { root, kept, voice: SILENT, loopKind: "main", gh: nightly, disk: async (keepRuns) => (kept.push(keepRuns), "disk: freed nothing") };
}

const lastSweep = (root: string) => join(root, LAST_SWEEP);

/** Writes the file as looper leaves it, with `extra` lines after its own. */
function looperWrote(root: string, ...extra: string[]): void {
  mkdirSync(join(root, ".loop"), { recursive: true });
  writeFileSync(lastSweep(root), `${[LOOPER_LINE, ...extra].join("\n")}\n`);
}

describe("bun nv sweep: the disk and its own lines", () => {
  test("it sweeps the disk with `--keep-runs`, and pays no floor debt", async () => {
    const h = hook("disk");
    expect(await sweep(["--keep-runs", "3"], h)).toBe(0);
    expect(h.kept).toEqual([3]);
  });

  test("under a side loop in the root it sweeps nothing, reads no CI run and leaves the file alone", async () => {
    const nightly = ghOf([{ databaseId: 1, status: "completed", conclusion: "success" }]);
    const h = { ...hook("root-side", nightly.gh), loopKind: "root-side" };
    looperWrote(h.root);
    expect(await sweep([], h)).toBe(0);
    expect(h.kept).toEqual([]);
    expect(nightly.asked).toEqual([]);
    expect(readFileSync(lastSweep(h.root), "utf8")).toBe(`${LOOPER_LINE}\n`);
  });

  test("nv sweep exits 0 on a bad argument, says why, sweeps nothing, and still reads the nightly run", async () => {
    const failed = ghOf([{ databaseId: 7, status: "completed", conclusion: "failure" }], { 7: [{ name: "lint", conclusion: "failure" }] });
    const bad = hook("exit-bad", failed.gh);
    expect(await sweep(["--keep-runs", "many"], bad)).toBe(0);
    expect(readFileSync(lastSweep(bad.root), "utf8")).toBe(
      `${HOOK_LINE}could not run: \`bun nv sweep\` was started with a bad argument: --keep-runs needs a value\n${NIGHTLY_LINE}job \`lint\` ended \`failure\`: ${RUN}/7\n`,
    );
    expect(bad.kept).toEqual([]);

    // The next good run removes the line.
    expect(await sweep([], { ...bad, gh: GREEN_NIGHTLY })).toBe(0);
    expect(existsSync(lastSweep(bad.root))).toBe(false);
  });
});

describe("bun nv sweep: the nightly CI run", () => {
  test("it asks gh for the scheduled runs of ci.yml on main, and nothing else", async () => {
    const g = ghOf([{ databaseId: 1, status: "completed", conclusion: "success" }]);
    await sweep([], hook("nightly-ask", g.gh));
    expect(g.asked).toEqual([["run", "list", "--workflow", "ci.yml", "--branch", "main", "--event", "schedule", "--limit", "5", "--json", "databaseId,status,conclusion,url"]]);
  });

  test("a failed run writes one line per failed job with the run's URL", async () => {
    const g = ghOf([{ databaseId: 42, status: "completed", conclusion: "failure" }], {
      42: [
        { name: "docs", conclusion: "failure" },
        { name: "what changed", conclusion: "success" },
        { name: "test / linux-x86_64", conclusion: "timed_out" },
        { name: "browser", conclusion: "skipped" },
      ],
    });
    const h = hook("nightly-red", g.gh);
    looperWrote(h.root, `${NIGHTLY_LINE}job \`old\` ended \`failure\`: ${RUN}/41`);
    expect(await sweep([], h)).toBe(0);
    expect(readFileSync(lastSweep(h.root), "utf8")).toBe(
      [LOOPER_LINE, `${NIGHTLY_LINE}job \`docs\` ended \`failure\`: ${RUN}/42`, `${NIGHTLY_LINE}job \`test / linux-x86_64\` ended \`timed_out\`: ${RUN}/42`, ""].join("\n"),
    );
  });

  test("a run still in progress changes nothing: the newest completed run still decides", async () => {
    const g = ghOf(
      [
        { databaseId: 43, status: "in_progress", conclusion: "" },
        { databaseId: 42, status: "completed", conclusion: "failure" },
      ],
      { 42: [{ name: "docs", conclusion: "failure" }] },
    );
    const h = hook("nightly-running", g.gh);
    looperWrote(h.root, `${NIGHTLY_LINE}job \`docs\` ended \`failure\`: ${RUN}/42`);
    expect(await sweep([], h)).toBe(0);
    expect(readFileSync(lastSweep(h.root), "utf8")).toBe(`${LOOPER_LINE}\n${NIGHTLY_LINE}job \`docs\` ended \`failure\`: ${RUN}/42\n`);
    expect(g.asked.some((a) => a[1] === "view" && a[2] === "43")).toBe(false);
  });

  test("a newer green run removes the lines, and the file once nothing is left", async () => {
    const h = hook("nightly-green");
    looperWrote(h.root, `${NIGHTLY_LINE}job \`docs\` ended \`failure\`: ${RUN}/42`);
    expect(await sweep([], h)).toBe(0);
    expect(readFileSync(lastSweep(h.root), "utf8")).toBe(`${LOOPER_LINE}\n`);

    const alone = hook("nightly-green-alone");
    mkdirSync(join(alone.root, ".loop"), { recursive: true });
    writeFileSync(lastSweep(alone.root), `${NIGHTLY_LINE}job \`docs\` ended \`failure\`: ${RUN}/42\n`);
    expect(await sweep([], alone)).toBe(0);
    expect(existsSync(lastSweep(alone.root))).toBe(false);
  });

  test("gh that fails, is not logged in or is missing writes one line, and the hook exits 0", async () => {
    const out = hook("nightly-out", async () => ({ code: 4, stdout: "", stderr: "To get started with GitHub CLI, please run:  gh auth login\n" }));
    expect(await sweep([], out)).toBe(0);
    expect(readFileSync(lastSweep(out.root), "utf8")).toBe(`${NIGHTLY_LINE}could not be read: \`gh run list\` exited 4: To get started with GitHub CLI, please run:  gh auth login\n`);

    const missing = hook("nightly-missing", () => Promise.reject(new Error("Executable not found in $PATH: \"gh\"")));
    looperWrote(missing.root);
    expect(await sweep([], missing)).toBe(0);
    expect(readFileSync(lastSweep(missing.root), "utf8")).toBe(`${LOOPER_LINE}\n${NIGHTLY_LINE}could not be read: \`gh\` failed: Executable not found in $PATH: "gh"\n`);

    const garbled = hook("nightly-garbled", async () => ({ code: 0, stdout: "<html>", stderr: "" }));
    expect(await sweep([], garbled)).toBe(0);
    const text = readFileSync(lastSweep(garbled.root), "utf8");
    expect(text.startsWith(`${NIGHTLY_LINE}could not be read: \`gh\` failed: `)).toBe(true);
    expect(text.split("\n").filter((l) => l !== "")).toHaveLength(1);
  });
});

describe("progress events", () => {
  test("under looper, the disk sweep and the nightly read are items, and the run ends with done", async () => {
    const h = hook("events");
    const file = join(h.root, "events.ndjson");
    const was = process.env.LOOPER_EVENTS;
    process.env.LOOPER_EVENTS = file;
    try {
      expect(await sweep([], h)).toBe(0);
    } finally {
      if (was === undefined) delete process.env.LOOPER_EVENTS;
      else process.env.LOOPER_EVENTS = was;
    }
    const said = readFileSync(file, "utf8")
      .split("\n")
      .filter((l) => l !== "")
      .map((l) => JSON.parse(l) as { event: string; id?: string; result?: string })
      .map((e) => `${e.event}${e.id ? ` ${e.id}` : ""}${e.result ? ` ${e.result}` : ""}`);
    expect(said).toEqual(["add sweep:disk", "add sweep:nightly", "planned", "start sweep:disk", "end sweep:disk", "start sweep:nightly", "end sweep:nightly", "done green"]);
  });
});
