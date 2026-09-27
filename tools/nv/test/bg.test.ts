import { afterAll, describe, expect, test } from "bun:test";
import { rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { JOBS } from "../cmd/bg.ts";
import { ROOT } from "../lib/paths.ts";
import { run } from "../lib/proc.ts";

const MAIN = join(ROOT, "tools", "nv", "main.ts");
const started: string[] = [];
const released: string[] = [];
afterAll(() => {
  for (const id of started) rmSync(join(JOBS, id), { recursive: true, force: true });
  for (const path of released) rmSync(path, { force: true });
});

const nv = (...args: string[]) => run([process.execPath, MAIN, "bg", ...args], { timeoutMs: 60_000 });

/** Starts `bun -e <script>` as a job through the command line, and returns its id. */
async function startJob(script: string): Promise<string> {
  const r = await nv("--", process.execPath, "-e", script);
  expect(r.code).toBe(0);
  const id = r.stdout.trim();
  expect(id).toMatch(/^\d{8}-\d{6}-[0-9a-f]{4}$/);
  started.push(id);
  return id;
}

describe("nv bg", () => {
  test("a job that exits 0 gives back its output and status 0", async () => {
    const id = await startJob("console.log('from stdout'); console.error('from stderr')");
    const r = await nv("--wait", id);
    expect(r.code).toBe(0);
    expect(r.stdout).toContain("from stdout");
    expect(r.stdout).toContain("from stderr");
    expect(r.stdout).toContain(`bg ${id}: exit 0`);
  });

  test("a job that fails gives back its output and its own status", async () => {
    const id = await startJob("console.log('about to fail'); process.exit(3)");
    const r = await nv("--wait", id);
    expect(r.code).toBe(3);
    expect(r.stdout).toContain("about to fail");
    expect(r.stdout).toContain(`bg ${id}: exit 3`);
  });

  test("a job is listed while it runs and not after it ends", async () => {
    // The job runs until this file exists, so a slow machine cannot end it before `--list` looks.
    const release = join(ROOT, ".agent-tmp", `bg-test-release-${process.pid}`);
    try {
      const id = await startJob(
        `const { existsSync } = require("node:fs"); while (!existsSync(${JSON.stringify(release)})) await Bun.sleep(50)`,
      );
      expect((await nv("--list")).stdout).toContain(id);
      writeFileSync(release, "");
      expect((await nv("--wait", id)).code).toBe(0);
      expect((await nv("--list")).stdout).not.toContain(id);
    } finally {
      writeFileSync(release, "");
      released.push(release);
    }
  });

  test("a start through `bun run` and a pipe returns while the job still runs", async () => {
    // `bun run`'s shell reads the script's output through pipes, and the job must not hold them open.
    const release = join(ROOT, ".agent-tmp", `bg-test-pipe-${process.pid}`);
    try {
      const script = `const { existsSync } = require("node:fs"); while (!existsSync(${JSON.stringify(release)})) await Bun.sleep(50)`;
      const r = await run([process.execPath, "run", "nv", "bg", "--", process.execPath, "-e", script], { cwd: ROOT, timeoutMs: 20_000 });
      expect(r.code).toBe(0);
      const id = r.stdout.trim().split("\n").pop()!.trim();
      expect(id).toMatch(/^\d{8}-\d{6}-[0-9a-f]{4}$/);
      started.push(id);
      expect((await nv("--list")).stdout).toContain(id);
      writeFileSync(release, "");
      expect((await nv("--wait", id)).code).toBe(0);
    } finally {
      writeFileSync(release, "");
      released.push(release);
    }
  }, 30_000);

  test("an id that names no job is a bad argument", async () => {
    const r = await nv("--wait", "no-such-job");
    expect(r.code).toBe(2);
  });
});
