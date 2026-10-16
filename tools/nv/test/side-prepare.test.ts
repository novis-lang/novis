// `bun nv side --prepare`, in its `<slug>` form and its `--hook` form, over a scratch repository with one
// side goal committed on `main`: the disk check, the package install and the environment are handed in,
// and git runs for real.

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
  tmp.put("main/docs/goals/s.md", "# s\n");
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
    expect(existsSync(join(dir, "docs/goals/s.md"))).toBe(true);
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

/** A lane as looper leaves it: a worktree of `main` on branch `side/s`, with nothing the tools put there. */
async function lane(main: string, dir: string): Promise<void> {
  expect((await git(main, "worktree", "add", "-q", "-b", "side/s", dir, "main")).code).toBe(0);
}

/** Runs `bun nv side --prepare --hook` in `cwd` under `env`, counting the installs it starts. The main tree is left for it to find. */
function hookIn(cwd: string, env: Record<string, string | undefined>, installs: string[]) {
  return side(["--prepare", "--hook"], {
    cwd,
    env,
    disk: () => "the hook form checks no disk",
    install: async (at) => {
      installs.push(at);
      mkdirSync(join(at, "node_modules"));
      return "";
    },
  });
}

describe("bun nv side --prepare --hook", () => {
  test("nv side --prepare --hook does nothing for a loop that is not a side loop", async () => {
    const { main, dir } = await repo();
    await lane(main, dir);
    const installs: string[] = [];
    expect(await hookIn(dir, { LOOPER_LOOP_KIND: "main", LOOPER_GOAL: "s" }, installs)).toBe(0);
    expect(await hookIn(dir, { LOOPER_GOAL: "s" }, installs)).toBe(0);
    expect(installs).toEqual([]);
    for (const d of [".loop/machine.json", ".agent-tmp", "fuzz/corpus"]) expect(existsSync(join(dir, d))).toBe(false);
  });

  test("nv side --prepare --hook readies the cwd's worktree and makes no branch or worktree", async () => {
    const { main, dir } = await repo();
    await lane(main, dir);
    mkdirSync(join(dir, "docs/goals/sub"));
    const worktrees = (await git(main, "worktree", "list", "--porcelain")).stdout;
    const branches = (await git(main, "branch", "--list")).stdout;
    const installs: string[] = [];
    expect(await hookIn(join(dir, "docs/goals/sub"), { LOOPER_LOOP_KIND: "side", LOOPER_GOAL: "s" }, installs)).toBe(0);
    expect((await git(main, "worktree", "list", "--porcelain")).stdout).toBe(worktrees);
    expect((await git(main, "branch", "--list")).stdout).toBe(branches);
    expect(existsSync(join(main, ".agent-tmp/worktrees/side/s"))).toBe(false);
    expect(readFileSync(join(dir, ".loop/machine.json"), "utf8")).toBe('{"cores":4}\n');
    for (const d of [".agent-tmp", "fuzz/corpus", "node_modules"]) expect(existsSync(join(dir, d))).toBe(true);
    expect(installs).toHaveLength(1);
    expect(existsSync(join(installs[0]!, "node_modules"))).toBe(true);
    expect(await hookIn(dir, { LOOPER_LOOP_KIND: "side", LOOPER_GOAL: "s" }, installs)).toBe(0);
    expect(installs).toHaveLength(1);
  });

  test("nv side --prepare --hook refuses a goal the branch does not have, and arguments it does not read", async () => {
    const { main, dir } = await repo();
    await lane(main, dir);
    expect(await hookIn(dir, { LOOPER_LOOP_KIND: "side", LOOPER_GOAL: "other" }, [])).toBe(1);
    expect(await hookIn(dir, { LOOPER_LOOP_KIND: "side" }, [])).toBe(2);
    expect(await side(["--prepare", "--hook", "s"])).toBe(2);
    expect(await side(["--prepare", "--hook", "--dir", dir])).toBe(2);
  });
});
