// `bun nv ext-templates` and the CI job that runs it: the job's platforms, what the guide's example
// leg copies, and the C leg's two outcomes without wasi-sdk.

import { expect, test } from "bun:test";
import { readFileSync, readdirSync } from "node:fs";
import { join, relative } from "node:path";
import { EXAMPLE, exampleOwns } from "../cmd/ext-templates.ts";
import { ROOT } from "../lib/paths.ts";

/** Every file under `dir`, repository-relative with `/` separators. */
function filesUnder(dir: string): string[] {
  return readdirSync(join(ROOT, dir), { recursive: true, withFileTypes: true })
    .filter((e) => e.isFile())
    .map((e) => relative(ROOT, join(e.parentPath, e.name)).replaceAll("\\", "/"));
}

test("the CI job runs both templates on Linux, Windows and macOS", () => {
  const yml = Bun.YAML.parse(readFileSync(join(ROOT, ".github/workflows/ci.yml"), "utf8")) as {
    jobs: Record<string, { strategy?: { matrix?: { include?: { os: string }[] } }; steps: { run?: string }[] }>;
  };
  const job = yml.jobs["ext-templates"];
  expect(job).toBeDefined();
  const oses = (job!.strategy?.matrix?.include ?? []).map((row) => row.os).sort();
  expect(oses).toEqual(["macos-latest", "ubuntu-26.04", "windows-latest"]);
  const runs = job!.steps.map((s) => s.run?.trim()).filter(Boolean);
  expect(runs).toContain("bun nv ext-templates --lang rust");
  expect(runs).toContain("bun nv ext-templates --lang c");
  expect(runs).toContain("bun nv ext-templates --example");
});

test("the example leg copies every source of the guide's example and none of its build output", () => {
  const owned = filesUnder(EXAMPLE).filter((path) => exampleOwns(relative(EXAMPLE, path)));
  expect(owned).toContain(`${EXAMPLE}/nvsx.toml`);
  expect(owned).toContain(`${EXAMPLE}/demo/index.nvs`);
  expect(exampleOwns("target/wasm32-wasip2/release/highlight.wasm")).toBe(false);
  expect(exampleOwns("highlighter.nvsx")).toBe(false);
  expect(exampleOwns("demo/nvs.toml")).toBe(false);
  expect(exampleOwns("src/lib.rs")).toBe(true);
});

test("the C leg is skipped without wasi-sdk and fails under CI without it", () => {
  const leg = (ci: string) => {
    const r = Bun.spawnSync(["bun", "tools/nv/main.ts", "ext-templates", "--lang", "c"], {
      cwd: ROOT,
      env: { ...process.env, CI: ci, WASI_SDK_PATH: join(ROOT, ".agent-tmp", "no-such-wasi-sdk") },
    });
    return { code: r.exitCode, out: r.stdout.toString() };
  };
  const local = leg("");
  expect(local.out).toContain("c template: skipped");
  expect(local.code).toBe(0);
  const ci = leg("true");
  expect(ci.out).toContain("c template: FAILED");
  expect(ci.code).toBe(1);
});
