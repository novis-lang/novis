// The release binaries and the container images are built by `build.yml` and `image.yml`, which the
// release calls with its version and the dev build calls with none, at the commit CI
// tested. The release's call of `ci.yml` grants every permission a job there asks for, which GitHub
// checks when the run starts.

import { expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { ROOT } from "../lib/paths.ts";

interface Job {
  uses?: string;
  with?: Record<string, unknown>;
  permissions?: Record<string, string>;
}

interface Workflow {
  on: { workflow_call?: { inputs?: Record<string, unknown> } };
  jobs: Record<string, Job & { steps?: { uses?: string; with?: Record<string, unknown> }[] }>;
}

function workflow(file: string): Workflow {
  return Bun.YAML.parse(readFileSync(join(ROOT, ".github/workflows", file), "utf8")) as Workflow;
}

test("the release builds through build.yml and image.yml, with its version", () => {
  const release = workflow("release.yml").jobs;
  expect(release.build!.uses).toBe("./.github/workflows/build.yml");
  expect(release.build!.with?.version).toBe("${{ needs.plan.outputs.version }}");
  expect(release.docker!.uses).toBe("./.github/workflows/image.yml");
  expect(release.docker!.with?.version).toBe("${{ needs.plan.outputs.version }}");
});

test("ci.yml builds no release binaries and pushes no image", () => {
  const ci = workflow("ci.yml").jobs;
  const called = Object.values(ci).map((job) => job.uses).filter(Boolean);
  expect(called).toEqual([]);
  const writes = Object.entries(ci).filter(([, job]) => Object.values(job.permissions ?? {}).includes("write"));
  expect(writes.map(([name]) => name)).toEqual([]);
});

test("build.yml and image.yml check out the commit the dev build names, in every job", () => {
  for (const file of ["build.yml", "image.yml"]) {
    const wf = workflow(file);
    expect(Object.keys(wf.on.workflow_call?.inputs ?? {})).toContain("ref");
    for (const [name, job] of Object.entries(wf.jobs)) {
      const checkout = (job.steps ?? []).find((step) => step.uses?.startsWith("actions/checkout@"));
      expect(`${file} ${name}: ${checkout?.with?.ref}`).toBe(`${file} ${name}: \${{ inputs.ref }}`);
    }
  }
});

test("the release's call of ci.yml grants every permission a job there asks for", () => {
  const granted = workflow("release.yml").jobs.test!.permissions ?? {};
  const level = { none: 0, read: 1, write: 2 } as Record<string, number>;
  const short: string[] = [];
  for (const [name, job] of Object.entries(workflow("ci.yml").jobs)) {
    for (const [scope, wanted] of Object.entries(job.permissions ?? {})) {
      if ((level[granted[scope] ?? "none"] ?? 0) < (level[wanted] ?? 0)) short.push(`${name}: ${scope}: ${wanted}`);
    }
  }
  expect(short).toEqual([]);
});
