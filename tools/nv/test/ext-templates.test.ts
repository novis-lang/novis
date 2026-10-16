// `bun nv ext-templates` and the CI job that runs it: the job's platforms and gate, the `ext` lane's
// reach over what the templates are built from, and the C leg's two outcomes without wasi-sdk.

import { expect, test } from "bun:test";
import { readFileSync, readdirSync } from "node:fs";
import { dirname, join, relative, resolve } from "node:path";
import { LANES, lanesFor } from "../cmd/ci-changes.ts";
import { ROOT } from "../lib/paths.ts";

/** Every file under `dir`, repository-relative with `/` separators. */
function filesUnder(dir: string): string[] {
  return readdirSync(join(ROOT, dir), { recursive: true, withFileTypes: true })
    .filter((e) => e.isFile())
    .map((e) => relative(ROOT, join(e.parentPath, e.name)).replaceAll("\\", "/"));
}

/** The files `rs` names in an `include_str!` or `include_bytes!`, repository-relative. */
function included(rs: string): string[] {
  const text = readFileSync(join(ROOT, rs), "utf8");
  return [...text.matchAll(/include_(?:str|bytes)!\(\s*"([^"]+)"\s*\)/g)].map((m) =>
    relative(ROOT, resolve(join(ROOT, dirname(rs)), m[1]!)).replaceAll("\\", "/"),
  );
}

test("the CI job runs both templates on Linux, Windows and macOS", () => {
  const yml = Bun.YAML.parse(readFileSync(join(ROOT, ".github/workflows/ci.yml"), "utf8")) as {
    jobs: Record<string, { if?: string; strategy?: { matrix?: { include?: { os: string }[] } }; steps: { run?: string }[] }>;
  };
  const job = yml.jobs["ext-templates"];
  expect(job).toBeDefined();
  expect(job!.if).toBe("needs.changes.outputs.ext == 'true'");
  const oses = (job!.strategy?.matrix?.include ?? []).map((row) => row.os).sort();
  expect(oses).toEqual(["macos-latest", "ubuntu-latest", "windows-latest"]);
  const runs = job!.steps.map((s) => s.run?.trim()).filter(Boolean);
  expect(runs).toContain("bun nv ext-templates --lang rust");
  expect(runs).toContain("bun nv ext-templates --lang c");
});

test("the ext lane covers every file the templates are built from", () => {
  const sources = [
    ...filesUnder("crates/nvs-cli/templates/ext"),
    ...filesUnder("crates/nvs-cli/src/ext"),
    "crates/nvs-cli/src/ext.rs",
    ...included("crates/nvs-cli/src/ext/new.rs"),
    ...filesUnder("crates/nvs-ext/src"),
    ...filesUnder("wit/nvs-ext"),
    "tools/nv/cmd/ext-templates.ts",
    "rust-toolchain.toml",
  ];
  expect(sources.length).toBeGreaterThan(20);
  const uncovered = sources.filter((path) => !lanesFor({ ext: LANES.ext! }, [path])[0]![1]);
  expect(uncovered).toEqual([]);
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
