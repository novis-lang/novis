// `bun nv side --prepare` over a scratch repository with one side goal committed on `main`: the disk
// check and the package install are handed in, and git runs for real.

import { afterEach, describe, expect, test } from "bun:test";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { side } from "../cmd/side.ts";
import { run } from "../lib/proc.ts";
import { type Scratch, scratch } from "./scratch.ts";

let tmp: Scratch | null = null;
afterEach(() => {
  tmp?.cleanup();
  tmp = null;
});

const git = (cwd: string, ...args: string[]) => run(["git", "-c", "user.name=t", "-c", "user.email=t@example.com", ...args], { cwd, timeoutMs: 60_000 });

/** A main tree with side goal `s` committed on `main`, and the seed a worktree is given. */
async function repo(): Promise<{ main: string; dir: string }> {
  tmp = scratch();
  const main = join(tmp.root, "main");
  tmp.put("main/docs/agent/goals/side/s.md", "# s\n");
  tmp.put("main/.loop/machine.json", '{"cores":4}\n');
  writeFileSync(join(main, ".gitignore"), ".loop/\n");
  expect((await git(main, "init", "-q", "-b", "main")).code).toBe(0);
  expect((await git(main, "add", ".")).code).toBe(0);
  expect((await git(main, "commit", "-q", "--no-verify", "-m", "side goal s")).code).toBe(0);
  return { main, dir: join(tmp.root, "wt") };
}

/** Runs `bun nv side --prepare s --dir <dir>`, counting the installs it starts. */
function prepareIn(main: string, dir: string, installs: string[]) {
  return side(["--prepare", "s", "--dir", dir], {
    main,
    disk: () => "",
    install: async (at) => {
      installs.push(at);
      mkdirSync(join(at, "node_modules"));
      return "";
    },
  });
}

describe("bun nv side --prepare", () => {
  test("nv side --prepare makes the worktree and its branch when they are missing", async () => {
    const { main, dir } = await repo();
    const installs: string[] = [];
    expect(await prepareIn(main, dir, installs)).toBe(0);
    expect((await git(main, "rev-parse", "--verify", "--quiet", "refs/heads/side/s")).code).toBe(0);
    expect((await git(dir, "rev-parse", "--abbrev-ref", "HEAD")).stdout.trim()).toBe("side/s");
    expect(existsSync(join(dir, "docs/agent/goals/side/s.md"))).toBe(true);
    expect(readFileSync(join(dir, ".loop/machine.json"), "utf8")).toBe('{"cores":4}\n');
    for (const d of [".agent-tmp", "fuzz/corpus", "node_modules"]) expect(existsSync(join(dir, d))).toBe(true);
    expect(installs).toEqual([dir]);
  });

  test("nv side --prepare run twice changes nothing the second time", async () => {
    const { main, dir } = await repo();
    const installs: string[] = [];
    expect(await prepareIn(main, dir, installs)).toBe(0);
    const worktrees = (await git(main, "worktree", "list", "--porcelain")).stdout;
    writeFileSync(join(dir, ".loop/machine.json"), '{"cores":8}\n');
    expect(await prepareIn(main, dir, installs)).toBe(0);
    expect((await git(main, "worktree", "list", "--porcelain")).stdout).toBe(worktrees);
    expect(readFileSync(join(dir, ".loop/machine.json"), "utf8")).toBe('{"cores":8}\n');
    expect(installs).toEqual([dir]);
  });

  test("nv side --prepare needs a slug and takes no other mode", async () => {
    expect(await side(["--prepare"])).toBe(2);
    expect(await side(["--make", "s"])).toBe(2);
    expect(await side(["--prepare", "s", "--force"])).toBe(2);
  });
});
