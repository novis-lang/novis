// What a push builds beside its tests: the release binaries through `build.yml`, and the `dev` image
// through `image.yml` once every other job in `ci.yml` is green. The release calls the same two files,
// and its call of `ci.yml` grants every permission a job there asks for, which GitHub checks when the
// run starts.

import { expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { ROOT } from "../lib/paths.ts";

interface Job {
  needs?: string | string[];
  if?: string;
  uses?: string;
  with?: Record<string, unknown>;
  permissions?: Record<string, string>;
}

function jobs(file: string): Record<string, Job> {
  const yml = Bun.YAML.parse(readFileSync(join(ROOT, ".github/workflows", file), "utf8")) as { jobs: Record<string, Job> };
  return yml.jobs;
}

const needs = (job: Job): string[] => (job.needs === undefined ? [] : [job.needs].flat());

test("the dev image waits for every other job in ci.yml", () => {
  const ci = jobs("ci.yml");
  const image = ci["dev-image"];
  expect(image).toBeDefined();
  const others = Object.keys(ci).filter((name) => name !== "dev-image").sort();
  expect(needs(image!).sort()).toEqual(others);
});

test("the dev image runs on a green push to main alone, and every push builds the binaries", () => {
  const ci = jobs("ci.yml");
  const image = ci["dev-image"]!;
  for (const part of ["!cancelled()", "github.event_name == 'push'", "github.ref == 'refs/heads/main'", "'failure'", "'cancelled'"]) {
    expect(image.if).toContain(part);
  }
  expect(image.uses).toBe("./.github/workflows/image.yml");
  expect(image.with?.version ?? "").toBe("");
  expect(ci.binaries!.if).toBe("github.event_name == 'push'");
  expect(ci.binaries!.uses).toBe("./.github/workflows/build.yml");
  expect(ci.binaries!.with?.version ?? "").toBe("");
});

test("the release builds through the same two files, with its version", () => {
  const release = jobs("release.yml");
  expect(release.build!.uses).toBe("./.github/workflows/build.yml");
  expect(release.build!.with?.version).toBe("${{ needs.plan.outputs.version }}");
  expect(release.docker!.uses).toBe("./.github/workflows/image.yml");
  expect(release.docker!.with?.version).toBe("${{ needs.plan.outputs.version }}");
});

test("the release's call of ci.yml grants every permission a job there asks for", () => {
  const granted = jobs("release.yml").test!.permissions ?? {};
  const level = { none: 0, read: 1, write: 2 } as Record<string, number>;
  const short: string[] = [];
  for (const [name, job] of Object.entries(jobs("ci.yml"))) {
    for (const [scope, wanted] of Object.entries(job.permissions ?? {})) {
      if ((level[granted[scope] ?? "none"] ?? 0) < (level[wanted] ?? 0)) short.push(`${name}: ${scope}: ${wanted}`);
    }
  }
  expect(short).toEqual([]);
});
