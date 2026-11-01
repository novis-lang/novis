// Every `Cargo.lock` the repository tracks is audited twice: by `ci.yml`'s `supply-chain` job and by
// `bun nv ci --check`'s `PRE_PUSH`. The root's lockfile gets the full `cargo deny check`, and every
// other one gets its advisories leg, under the root's `deny.toml`
// (`rule:packaging/a-dependency-move-proves-two-things`). A lockfile neither names is audited by
// nobody, so this fails naming it. It also fails on a target `build.yml` releases that `deny.toml`'s
// `[graph] targets` leaves out, since cargo-deny reads only the dependencies of the targets it lists.

import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { PRE_PUSH, type PrePush } from "../cmd/ci.ts";
import { ROOT } from "../lib/paths.ts";

/** One step of a workflow, as far as this reads it. */
interface Step {
  uses?: string;
  with?: Record<string, string>;
}

const read = (path: string): string => readFileSync(join(ROOT, path), "utf8");

/** The directory of every tracked `Cargo.lock`, with `.` for the root's. */
function lockDirs(): string[] {
  const out = Bun.spawnSync(["git", "ls-files", "--", "Cargo.lock", "*/Cargo.lock"], { cwd: ROOT });
  if (out.exitCode !== 0) throw new Error(`git ls-files exited ${out.exitCode}`);
  return out.stdout
    .toString()
    .split("\n")
    .filter((l) => l !== "")
    .map((l) => (l === "Cargo.lock" ? "." : l.slice(0, -"/Cargo.lock".length)));
}

/** The cargo-deny steps of `ci.yml`'s `supply-chain` job. */
function denySteps(): Step[] {
  const ci = Bun.YAML.parse(read(".github/workflows/ci.yml")) as { jobs: Record<string, { steps?: Step[] }> };
  const job = ci.jobs["supply-chain"];
  if (job === undefined) throw new Error("ci.yml has no `supply-chain` job");
  return (job.steps ?? []).filter((s) => s.uses?.startsWith("EmbarkStudios/cargo-deny-action@") === true);
}

/** Each lockfile directory neither `ci.yml`'s cargo-deny steps nor `PRE_PUSH` covers, with what misses it. */
function uncovered(dirs: string[], steps: Step[], prePush: PrePush[]): string[] {
  const gaps: string[] = [];
  for (const d of dirs) {
    if (d === ".") {
      const ci = steps.some((s) => s.with?.["manifest-path"] === undefined && s.with?.command === "check" && s.with?.["command-arguments"] === undefined);
      const push = prePush.some((p) => p.argv.join(" ") === "cargo deny check advisories");
      if (!ci) gaps.push(". (ci.yml has no full `cargo deny check` of the root)");
      if (!push) gaps.push(". (PRE_PUSH has no `cargo deny check advisories`)");
      continue;
    }
    const manifest = `${d}/Cargo.toml`;
    const ci = steps.some((s) => s.with?.["manifest-path"] === manifest && s.with?.command === "check");
    const push = prePush.some((p) => {
      const at = p.argv.indexOf("--manifest-path");
      return p.argv[0] === "cargo" && p.argv[1] === "deny" && at >= 0 && p.argv[at + 1] === manifest && p.argv.includes("check");
    });
    if (!ci) gaps.push(`${d} (ci.yml's supply-chain job does not run cargo-deny on ${manifest})`);
    if (!push) gaps.push(`${d} (PRE_PUSH does not run cargo deny on ${manifest})`);
  }
  return gaps;
}

/** Every `target:` a matrix row of `build.yml` names. */
function releaseTargets(): string[] {
  const build = Bun.YAML.parse(read(".github/workflows/build.yml")) as {
    jobs: Record<string, { strategy?: { matrix?: { include?: { target?: string }[] } } }>;
  };
  const targets = new Set<string>();
  for (const job of Object.values(build.jobs)) {
    for (const row of job.strategy?.matrix?.include ?? []) if (row.target !== undefined) targets.add(row.target);
  }
  return [...targets].sort();
}

describe("the supply-chain checks cover the tree", () => {
  test("every tracked Cargo.lock is audited by ci.yml and by PRE_PUSH", () => {
    const dirs = lockDirs();
    expect(dirs).toContain(".");
    expect(uncovered(dirs, denySteps(), PRE_PUSH)).toEqual([]);
  });

  test("a lockfile no step names is reported by its directory", () => {
    const steps: Step[] = [{ uses: "EmbarkStudios/cargo-deny-action@x", with: { command: "check" } }];
    const prePush: PrePush[] = [{ name: "advisories", argv: ["cargo", "deny", "check", "advisories"] }];
    expect(uncovered([".", "extensions/new"], steps, prePush)).toEqual([
      "extensions/new (ci.yml's supply-chain job does not run cargo-deny on extensions/new/Cargo.toml)",
      "extensions/new (PRE_PUSH does not run cargo deny on extensions/new/Cargo.toml)",
    ]);
  });

  test("deny.toml lists every target build.yml releases", () => {
    const deny = Bun.TOML.parse(read("deny.toml")) as { graph?: { targets?: string[] } };
    const listed = new Set(deny.graph?.targets ?? []);
    const released = releaseTargets();
    expect(released.length).toBeGreaterThan(0);
    expect(released.filter((t) => !listed.has(t))).toEqual([]);
  });
});
