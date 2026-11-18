// The release binaries and the container images are built by `build.yml` and `image.yml`, which the
// release calls with its version and `ci.yml` calls with none. Every CI run builds the binaries, and
// a push to `main` publishes the `dev` image once every other job of its run is green. The jobs whose
// result gates nothing, and `build.yml`'s musl leg, run in the nightly and a dispatch alone. A nightly
// whose commit a push run already tested runs only the jobs that read the outside world, and the
// nightly-only ones. The release's call of `ci.yml` grants every permission a job there asks for, which
// GitHub checks when the run starts.

import { expect, test } from "bun:test";
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { ROOT } from "../lib/paths.ts";

interface Job {
  needs?: string | string[];
  uses?: string;
  if?: string;
  with?: Record<string, unknown>;
  permissions?: Record<string, string>;
  steps?: { uses?: string; run?: string; with?: Record<string, unknown> }[];
}

interface Workflow {
  on: { workflow_call?: { inputs?: Record<string, unknown> } | null };
  jobs: Record<string, Job>;
}

/** The `if` of a job that runs in the nightly and a dispatch alone, because its result gates nothing. */
const NIGHTLY_ONLY = "github.event_name == 'schedule' || github.event_name == 'workflow_dispatch'";
const nightlyOnly = (job: Job): boolean => job.if === NIGHTLY_ONLY;

/** The `if` of a job that a nightly on a commit a push run already tested skips. */
const SKIPS_TESTED = "${{ !cancelled() && needs.tested.outputs.tested != 'true' }}";

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

test("every CI run builds the binaries with no version, and has no dev build workflow beside it", () => {
  const ci = workflow("ci.yml").jobs;
  expect(ci.build!.uses).toBe("./.github/workflows/build.yml");
  expect(ci.build!.with).toEqual({ "musl-only": "${{ needs.tested.outputs.tested == 'true' }}" });
  expect(ci.build!.if).toBe("${{ !cancelled() }}");
  expect(existsSync(join(ROOT, ".github/workflows/dev-build.yml"))).toBe(false);
});

test("ci.yml has no lane gate and no input", () => {
  const wf = workflow("ci.yml");
  expect(wf.on.workflow_call?.inputs).toBeUndefined();
  expect(wf.jobs.changes).toBeUndefined();
  const gated = Object.entries(wf.jobs).filter(([, job]) => (job.if ?? "").includes("needs.changes"));
  expect(gated.map(([name]) => name)).toEqual([]);
});

test("only a push to main publishes the dev image, after every other job of its run", () => {
  const ci = workflow("ci.yml").jobs;
  const image = ci["dev-image"]!;
  expect(image.uses).toBe("./.github/workflows/image.yml");
  // A push run skips `tested`, which is in the chain of needs of most jobs here. An `if` with no
  // status function is read as `success()`, which a skipped job anywhere in that chain makes false,
  // so the condition starts with `!cancelled()` and checks each direct need's result itself.
  const condition = image.if ?? "";
  expect(condition.startsWith("${{ !cancelled() && github.event_name == 'push' && github.ref == 'refs/heads/main'")).toBe(true);
  for (const result of ["failure", "cancelled", "skipped"]) {
    expect(condition).toContain(`!contains(needs.*.result, '${result}')`);
  }
  const needs = [image.needs ?? []].flat().sort();
  // `tested` runs in the nightly alone, so it is not a direct need.
  const others = Object.keys(ci).filter((name) => !["dev-image", "tested"].includes(name) && !nightlyOnly(ci[name]!)).sort();
  expect(needs).toEqual(others);
  // The only job of ci.yml that writes anything.
  const writes = Object.entries(ci).filter(([, job]) => Object.values(job.permissions ?? {}).includes("write"));
  expect(writes.map(([name]) => name)).toEqual(["dev-image"]);
});

test("only the jobs whose result gates nothing run in the nightly and a dispatch alone", () => {
  const ci = workflow("ci.yml").jobs;
  const nightly = Object.keys(ci).filter((name) => nightlyOnly(ci[name]!)).sort();
  expect(nightly).toEqual(["flood", "unsafe-audit"]);
  const include = String((workflow("build.yml").jobs.build as { strategy?: { matrix?: { include?: unknown } } }).strategy?.matrix?.include);
  expect(include).toContain("github.event_name == 'schedule' || github.event_name == 'workflow_dispatch'");
  expect(include).toContain('"name":"linux-x86_64-musl"');
});

test("a nightly on a commit a push run already tested runs only what reads the outside world", () => {
  const ci = workflow("ci.yml").jobs;
  expect(ci.tested!.if).toBe("github.event_name == 'schedule'");
  const kept = Object.keys(ci).filter((name) => ci[name]!.if !== SKIPS_TESTED).sort();
  expect(kept).toEqual(
    ["asan", "build", "database", "dev-image", "docker", "flood", "miri", "miri-units", "supply-chain", "tested", "tsan", "unsafe-audit"],
  );
  // Each job that skips it needs `tested`, or its `if` cannot read the output.
  const blind = Object.keys(ci).filter((name) => ci[name]!.if === SKIPS_TESTED && ![ci[name]!.needs ?? []].flat().includes("tested"));
  expect(blind).toEqual([]);
  const include = String((workflow("build.yml").jobs.build as { strategy?: { matrix?: { include?: unknown } } }).strategy?.matrix?.include);
  expect(include).toContain("!inputs.musl-only");
  expect(workflow("build.yml").jobs.vsix!.if).toBe("${{ !inputs.musl-only }}");
});

test("each run proves the release's version step on the next patch version", () => {
  const runs = (workflow("ci.yml").jobs["release-version"]!.steps ?? []).map((s) => s.run?.trim() ?? "");
  expect(runs).toContain("bun nv release --plan patch --github-output");
  expect(runs.some((r) => r.startsWith("bun nv release --apply "))).toBe(true);
  expect(runs.some((r) => r.includes("cargo metadata --locked") && r.includes("fuzz/Cargo.toml"))).toBe(true);
});

test("build.yml and image.yml check out the commit their caller names, in every job", () => {
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
