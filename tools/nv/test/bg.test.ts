import { afterAll, describe, expect, test } from "bun:test";
import { rmSync } from "node:fs";
import { join } from "node:path";
import { JOBS } from "../cmd/bg.ts";
import { ROOT } from "../lib/paths.ts";
import { run } from "../lib/proc.ts";

const MAIN = join(ROOT, "tools", "nv", "main.ts");
const started: string[] = [];
afterAll(() => {
  for (const id of started) rmSync(join(JOBS, id), { recursive: true, force: true });
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
    const id = await startJob("await Bun.sleep(1500)");
    expect((await nv("--list")).stdout).toContain(id);
    expect((await nv("--wait", id)).code).toBe(0);
    expect((await nv("--list")).stdout).not.toContain(id);
  });

  test("an id that names no job is a bad argument", async () => {
    const r = await nv("--wait", "no-such-job");
    expect(r.code).toBe(2);
  });
});
