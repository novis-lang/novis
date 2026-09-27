import { afterAll, describe, expect, test } from "bun:test";
import { rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { CACHE, ROOT } from "../lib/paths.ts";
import { run } from "../lib/proc.ts";
import { ENV, readLog, readModules } from "../lib/reads.ts";

const LOG = join(CACHE, `reads-${process.pid}.ndjson`);
afterAll(() => rmSync(LOG, { force: true }));

describe("what a `bun nv` process reads", () => {
  test("a recorded run notes the files its command reads, and the modules it loads are not reads", async () => {
    rmSync(LOG, { force: true });
    const r = await run([process.execPath, join(ROOT, "tools", "nv", "main.ts"), "peek", "package.json"], { env: { [ENV]: LOG } });
    expect(r.code).toBe(0);
    const got = readLog(LOG)!;
    expect(got.files).toContain("package.json");
    expect(got.files.some((f) => f.startsWith("tools/nv/"))).toBe(false);
  });

  test("the log is the union of every process that wrote a line to it", () => {
    writeFileSync(LOG, `${JSON.stringify({ files: ["a"], exists: [], dirs: ["d"], spawns: [["git", "ls-files"]] })}\n${JSON.stringify({ files: ["b", "a"], exists: ["e"], dirs: [], spawns: [["git", "ls-files"]] })}\n`);
    expect(readLog(LOG)).toEqual({ files: ["a", "b"], exists: ["e"], dirs: ["d"], spawns: [["git", "ls-files"]], keys: [] });
    writeFileSync(LOG, "");
    expect(readLog(LOG)).toBeNull();
  });

  test("a recorded run names the modules its process loaded: main.ts, its command's, and no other command's", async () => {
    rmSync(LOG, { force: true });
    const r = await run([process.execPath, join(ROOT, "tools", "nv", "main.ts"), "peek", "package.json"], { env: { [ENV]: LOG } });
    expect(r.code).toBe(0);
    const mods = readModules(LOG);
    expect(mods).toContain("tools/nv/main.ts");
    expect(mods).toContain("tools/nv/lib/reads.ts");
    expect(mods).toContain("tools/nv/cmd/peek.ts");
    expect(mods).not.toContain("tools/nv/cmd/verify.ts");
  });
});
