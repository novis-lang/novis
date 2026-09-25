import { afterAll, describe, expect, test } from "bun:test";
import { rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { commandModules } from "../keys/modules.ts";
import { Tree } from "../keys/tree.ts";
import { CACHE, ROOT } from "../lib/paths.ts";
import { run } from "../lib/proc.ts";
import { ENV, readLog } from "../lib/reads.ts";

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
    expect(readLog(LOG)).toEqual({ files: ["a", "b"], exists: ["e"], dirs: ["d"], spawns: [["git", "ls-files"]] });
    writeFileSync(LOG, "");
    expect(readLog(LOG)).toBeNull();
  });

  test("a command loads main.ts, its imports and its own module's closure, and no other command", async () => {
    const tree = await Tree.read();
    const mods = commandModules(tree, "why")!;
    expect(mods).toContain("tools/nv/main.ts");
    expect(mods).toContain("tools/nv/lib/reads.ts");
    expect(mods).toContain("tools/nv/cmd/why.ts");
    expect(mods).toContain("tools/nv/keys/checks.ts");
    expect(mods).not.toContain("tools/nv/cmd/verify.ts");
    expect(commandModules(tree, "no-such-command")).toBeNull();
  });
});
