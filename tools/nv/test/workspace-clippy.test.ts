// Every workspace with a `Cargo.lock` of its own is linted twice: by `ci.yml`'s `clippy-workspaces` job
// and by `bun nv ci --check`'s `PRE_PUSH`, both from `workspaceClippy` in `tools/nv/cmd/ci.ts`. The
// root's `cargo clippy --workspace` does not reach these, so a workspace this list misses is linted by
// nobody, and its lints pile up until somebody runs clippy there by hand.

import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { LOCKFILE_DIRS, PRE_PUSH, workspaceClippy } from "../cmd/ci.ts";
import { ROOT } from "../lib/paths.ts";

/** One step of a workflow, as far as this reads it. */
interface Step {
  name?: string;
  uses?: string;
  run?: string;
  with?: Record<string, string>;
}

/** The steps of `ci.yml`'s `clippy-workspaces` job. */
function jobSteps(): Step[] {
  const ci = Bun.YAML.parse(readFileSync(join(ROOT, ".github/workflows/ci.yml"), "utf8")) as {
    jobs: Record<string, { steps?: Step[] }>;
  };
  const job = ci.jobs["clippy-workspaces"];
  if (job === undefined) throw new Error("ci.yml has no `clippy-workspaces` job");
  return job.steps ?? [];
}

/** The directory of every tracked `Cargo.lock` but the root's. */
function lockDirs(): string[] {
  const out = Bun.spawnSync(["git", "ls-files", "--", "*/Cargo.lock"], { cwd: ROOT });
  if (out.exitCode !== 0) throw new Error(`git ls-files exited ${out.exitCode}`);
  return out.stdout
    .toString()
    .split("\n")
    .filter((l) => l !== "")
    .map((l) => l.slice(0, -"/Cargo.lock".length));
}

describe("every separate workspace is linted", () => {
  test("LOCKFILE_DIRS names every tracked Cargo.lock but the root's", () => {
    expect([...LOCKFILE_DIRS].sort()).toEqual(lockDirs().sort());
  });

  test("ci.yml's clippy-workspaces job runs workspaceClippy's commands, each as a step of its name", () => {
    const steps = jobSteps().filter((s) => s.run !== undefined && s.name?.startsWith("clippy (") === true);
    expect(steps.map((s) => [s.name, s.run?.trim()])).toEqual(workspaceClippy().map((c) => [c.name, c.argv.join(" ")]));
  });

  test("the job's cache holds the target of every workspace it lints", () => {
    const cache = jobSteps().find((s) => s.uses?.startsWith("Swatinem/rust-cache@") === true);
    const listed = (cache?.with?.workspaces ?? "").split("\n").map((l) => l.trim()).filter((l) => l !== "");
    expect(listed).toEqual(LOCKFILE_DIRS.map((d) => `${d} -> target`));
  });

  test("PRE_PUSH runs every one of them", () => {
    const names = new Set(PRE_PUSH.map((c) => c.name));
    expect(workspaceClippy().filter((c) => !names.has(c.name))).toEqual([]);
  });
});
