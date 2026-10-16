import { afterAll, describe, expect, test } from "bun:test";
import { rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
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

  test("`unrecorded` holds across an await for what it runs, and not for work beside it", async () => {
    rmSync(LOG, { force: true });
    const reads = JSON.stringify(pathToFileURL(join(ROOT, "tools", "nv", "lib", "reads.ts")).href);
    const bun = JSON.stringify(process.execPath);
    const script = [
      `import { install, note, unrecorded } from ${reads};`,
      `install(process.env.${ENV});`,
      "let release = () => {};",
      "const gate = new Promise((r) => (release = r));",
      "const quiet = unrecorded(async () => {",
      "  await gate;",
      '  note("files", "inside-after-await.txt");',
      `  await Bun.spawn([${bun}, "--version"], { stdout: "ignore" }).exited;`,
      '  note("files", "inside-after-spawn.txt");',
      "});",
      'note("files", "beside.txt");',
      "release();",
      "await quiet;",
      'note("files", "after.txt");',
      `Bun.spawnSync([${bun}, "--revision"], { stdout: "ignore" });`,
    ].join("\n");
    const r = await run([process.execPath, "-e", script], { cwd: ROOT, env: { [ENV]: LOG } });
    expect(r.code).toBe(0);
    const got = readLog(LOG)!;
    expect(got.files).toEqual(["after.txt", "beside.txt"]);
    expect(got.spawns).toEqual([[process.execPath, "--revision"]]);
  });

  test("`inPart` notes under each part it names, across an await, and a log read for one part leaves out the others", async () => {
    rmSync(LOG, { force: true });
    const reads = JSON.stringify(pathToFileURL(join(ROOT, "tools", "nv", "lib", "reads.ts")).href);
    const script = [
      `import { install, inPart, note, noteKey, unrecorded } from ${reads};`,
      `install(process.env.${ENV});`,
      'note("files", "common.txt");',
      'await inPart(["A"], async () => {',
      "  await Promise.resolve();",
      '  note("files", "a.txt");',
      '  noteKey("card:*");',
      '  unrecorded(() => note("files", "quiet.txt"));',
      "});",
      'inPart(["B"], () => note("dirs", "b"));',
      'inPart(["A", "B"], () => note("exists", "both"));',
      'inPart([], () => note("files", "empty.txt"));',
    ].join("\n");
    const r = await run([process.execPath, "-e", script], { cwd: ROOT, env: { [ENV]: LOG } });
    expect(r.stderr).toBe("");
    expect(readLog(LOG, "A")).toEqual({ files: ["a.txt", "common.txt", "empty.txt"], exists: ["both"], dirs: [], spawns: [], keys: ["card:*"] });
    expect(readLog(LOG, "B")).toEqual({ files: ["common.txt", "empty.txt"], exists: ["both"], dirs: ["b"], spawns: [], keys: [] });
    expect(readLog(LOG, "C")).toEqual({ files: ["common.txt", "empty.txt"], exists: [], dirs: [], spawns: [], keys: [] });
    // Without a part the log is everything, as a check that ran the process alone reads it.
    expect(readLog(LOG)).toEqual({ files: ["a.txt", "common.txt", "empty.txt"], exists: ["both"], dirs: ["b"], spawns: [], keys: ["card:*"] });
  });

  test("a module `loadUnrecorded` loads first is not named, one loaded before it is, and a `bun test` process names both", async () => {
    const reads = JSON.stringify(pathToFileURL(join(ROOT, "tools", "nv", "lib", "reads.ts")).href);
    const url = (p: string) => JSON.stringify(pathToFileURL(join(ROOT, "tools", "nv", ...p.split("/"))).href);
    for (const tests of [false, true]) {
      rmSync(LOG, { force: true });
      const script = [
        `import { install, loadUnrecorded } from ${reads};`,
        `install(process.env.${ENV}, ${tests});`,
        `await import(${url("lib/py.ts")});`,
        `await loadUnrecorded(() => Promise.all([import(${url("lib/py.ts")}), import(${url("keys/graph.ts")})]));`,
      ].join("\n");
      const r = await run([process.execPath, "-e", script], { cwd: ROOT, env: { [ENV]: LOG } });
      expect(r.stderr).toBe("");
      const mods = readModules(LOG);
      expect(mods).toContain("tools/nv/lib/py.ts");
      if (tests) expect(mods).toContain("tools/nv/keys/graph.ts");
      else expect(mods).not.toContain("tools/nv/keys/graph.ts");
    }
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
