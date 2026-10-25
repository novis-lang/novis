// The pipeline runs one debug build, the `covws` tree (`lib/covws.ts`), and names it to what it starts
// as `{nvs}` in a check's argv or as `NVS_BIN`. `target/debug` is what a person builds by hand: a check,
// a fixture or a tool that runs it passes on whatever stale build is on disk, and fails where there is
// none. This holds every goal file's checks, and every file the pipeline runs, to never naming it.

import { describe, expect, test } from "bun:test";
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { goalPaths, goalPlan } from "../lib/chain.ts";
import { ROOT } from "../lib/paths.ts";

/** `target/debug`, `target\debug`, or `"target", "debug"` as a path join spells it. */
const HAND_BUILD = /target(?:[\\/]+|["'`]\s*,\s*["'`])debug\b/;
const HAND_BUILD_ERE = "target([\\\\/]+|[\"'`] *, *[\"'`])debug";

/** Where the pipeline's own code and the programs it runs live. */
const PIPELINE = ["tools/nv", "examples", "tests", "docs/examples", "benches/members", "crates", "nvs.toml"];

/** A file under `PIPELINE` that names the hand build for a reason other than running it. */
const ALLOWED: Record<string, string> = {
  "tools/nv/cmd/disk.ts": "it measures and sweeps the hand build's files, and runs none of them",
  "tools/nv/keys/escape.ts": "it recognises a test that names the hand build",

  "tools/nv/cmd/garden.ts": "on Linux, it builds `target/debug` itself just before it copies the binary into the origin's image",
  "tools/nv/cmd/ci.ts": "`--push` runs `cargo build` as the step just before the checks that read `target/debug`",
  "tools/nv/lib/nvs-binary.ts": "`target/debug` is one of the builds it compares, and it reads only the newest, so a stale build loses to a fresher one",
};

/** A line that is only a comment, in any of the languages under `PIPELINE`. */
const COMMENT = /^\s*(?:\/\/|\/\*|\*|#)/;

function namesHandBuild(text: string): number[] {
  return text.split("\n").flatMap((line, i) => (HAND_BUILD.test(line) && !COMMENT.test(line) ? [i + 1] : []));
}

describe("the pipeline never runs a hand build", () => {
  test("no goal file's check names target/debug, in its argv or in the program it runs", () => {
    const found: string[] = [];
    const slugs = [...goalPaths().keys()];
    expect(slugs.length).toBeGreaterThan(0);
    for (const id of slugs) {
      for (const c of goalPlan(id)?.checks ?? []) {
        const spec = JSON.stringify({ argv: c.argv, args: c.args, file: c.file, cwd: c.cwd });
        if (HAND_BUILD.test(spec)) found.push(`${id}: check ${c.id} names it in ${spec}`);
        if (c.file && existsSync(join(ROOT, c.file))) {
          for (const n of namesHandBuild(readFileSync(join(ROOT, c.file), "utf8"))) found.push(`${id}: check ${c.id} runs ${c.file}, which names it at line ${n}`);
        }
      }
    }
    expect(found).toEqual([]);
  });

  test("no tool, fixture, case or test the pipeline runs names target/debug outside a comment", () => {
    const r = Bun.spawnSync(["git", "grep", "-n", "-I", "--untracked", "-E", HAND_BUILD_ERE, "--", ...PIPELINE], { cwd: ROOT, stdout: "pipe", stderr: "pipe" });
    // git grep exits 1 when nothing matches, which is the answer this wants.
    expect(r.exitCode === 0 || r.exitCode === 1).toBe(true);
    const found = r.stdout
      .toString()
      .split("\n")
      .filter((hit) => hit !== "")
      .filter((hit) => {
        const [file = "", , ...rest] = hit.split(":");
        const line = rest.join(":");
        return !file.startsWith("tools/nv/test/") && !(file in ALLOWED) && !COMMENT.test(line);
      });
    expect(found).toEqual([]);
  });
});
