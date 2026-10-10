// `lib/progress.ts` shows what a command is doing without adding a byte to what it prints; `proc.run`
// hands a caller each line as it arrives.

import { afterAll, describe, expect, test } from "bun:test";
import { join } from "node:path";
import { run } from "../lib/proc.ts";
import { cargoStatus } from "../lib/progress.ts";
import { scratch } from "./scratch.ts";

const PROGRESS = join(import.meta.dir, "../lib/progress.ts").replace(/\\/g, "/");
const dir = scratch();
afterAll(() => dir.cleanup());

/** A child that says `text` as its progress, then prints `done` and exits once `release` exists. */
function child(text: string, release: string): string {
  return [
    `import { progress } from ${JSON.stringify(PROGRESS)};`,
    "import { existsSync } from 'node:fs';",
    `progress(${JSON.stringify(text)});`,
    `while (!existsSync(${JSON.stringify(release)})) await Bun.sleep(20);`,
    "console.log('done');",
  ].join("\n");
}

describe("progress", () => {
  test("cargo's status lines are read as what the build is doing, and nothing else is", () => {
    const status = cargoStatus();
    expect(status("   Compiling nvs-syntax v0.1.0 (D:\\mwl\\crates\\nvs-syntax)")).toBe("compiling nvs-syntax (1 crate)");
    expect(status("   Compiling nvs-ir v0.1.0 (D:\\mwl\\crates\\nvs-ir)")).toBe("compiling nvs-ir (2 crates)");
    expect(status('{"reason":"compiler-artifact"}')).toBeNull();
    expect(status("warning: unused variable")).toBeNull();
    expect(status("    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3.21s")).toBe("finished");
  });

  test("with stderr a pipe, a command prints exactly what it would without progress", async () => {
    const script = join(dir.root, "none.ts");
    const release = join(dir.root, "none.go");
    dir.put("none.ts", child("building", release));
    dir.put("none.go", "");
    const r = await run([process.execPath, script]);
    expect(r.code).toBe(0);
    expect(r.stdout.replace(/\r\n/g, "\n")).toBe("done\n");
    expect(r.stderr).toBe("");
  });

  test("proc.run passes each line as it arrives and still returns the whole output", async () => {
    const lines: string[] = [];
    const r = await run([process.execPath, "-e", "console.log('a'); console.error('b'); process.stdout.write('c')"], {
      onLine: (line, stream) => lines.push(`${stream}:${line}`),
    });
    expect(r.stdout.replace(/\r\n/g, "\n")).toBe("a\nc");
    expect(lines.sort()).toEqual(["stderr:b", "stdout:a", "stdout:c"]);
  });
});
