import { afterAll, describe, expect, test } from "bun:test";
import { rmSync } from "node:fs";
import { join } from "node:path";
import { CACHE, ROOT } from "../lib/paths.ts";
import { run } from "../lib/proc.ts";
import { ENV, readLog } from "../lib/reads.ts";

const LOG = join(CACHE, `registers-reads-${process.pid}.ndjson`);
afterAll(() => rmSync(LOG, { force: true }));

/** Every path under `data/playbook/` that `bun nv <args>` read, listed or tested for. */
async function playbookReads(args: string[]): Promise<string[]> {
  rmSync(LOG, { force: true });
  await run([process.execPath, join(ROOT, "tools", "nv", "main.ts"), ...args], { env: { [ENV]: LOG, NO_COLOR: "1" } });
  const got = readLog(LOG)!;
  return [...got.files, ...got.dirs, ...got.exists].filter((p) => p.startsWith("data/playbook"));
}

describe("a register is read only by a command whose answer it can change", () => {
  test("`plan --past` counts owners per milestone, and the playbook names none, so it is not read", async () => {
    expect(await playbookReads(["plan", "--past"])).toEqual([]);
  });

  test("the owners gate is the gap records' alone, and reads no playbook bullet", async () => {
    expect(await playbookReads(["owners", "--check"])).toEqual([]);
    expect(await playbookReads(["owners", "--closes", "no-such-goal"])).toEqual([]);
  });

  test("the roster counts every register, the playbook among them", async () => {
    expect((await playbookReads(["owners", "--registers"])).length).toBeGreaterThan(0);
  });
});
