// The boundary the loop driver's removal needs: no file outside `tools/nv/driver/` and the loop commands
// imports either, and the code they shared with the rest of `tools/nv` lives outside them. Each test
// fails naming the file and the specifier, or the moved file or function, that breaks the boundary.

import { describe, expect, test } from "bun:test";
import { existsSync, readdirSync, readFileSync } from "node:fs";
import { dirname, join, relative, resolve } from "node:path";
import { ROOT } from "../lib/paths.ts";

const DRIVER = "tools/nv/driver/";
const LOOP_COMMANDS = ["loop", "chain", "goal", "session", "orient", "loop-stats", "playbook"].map((c) => `tools/nv/cmd/${c}.ts`);
const SKIP = new Set(["node_modules", "target", "__pycache__"]);

const rel = (abs: string) => relative(ROOT, abs).replace(/\\/g, "/");
const inside = (path: string) => path.startsWith(DRIVER) || LOOP_COMMANDS.includes(path);

/** Every `*.ts` under `tools/`, repo-relative. */
function tsFiles(dir: string, out: string[] = []): string[] {
  for (const e of readdirSync(join(ROOT, dir), { withFileTypes: true })) {
    const p = `${dir}/${e.name}`;
    if (e.isDirectory()) {
      if (!SKIP.has(e.name)) tsFiles(p, out);
    } else if (e.name.endsWith(".ts")) {
      out.push(p);
    }
  }
  return out;
}

/** Each static `from "…"` and dynamic `import("…")` specifier in a file, with whether it is dynamic. */
function specifiers(text: string): { spec: string; dynamic: boolean }[] {
  const out: { spec: string; dynamic: boolean }[] = [];
  for (const m of text.matchAll(/\bfrom\s+["']([^"']+)["']/g)) out.push({ spec: m[1]!, dynamic: false });
  for (const m of text.matchAll(/\bimport\(\s*["']([^"']+)["']\s*\)/g)) out.push({ spec: m[1]!, dynamic: true });
  return out;
}

describe("the loop driver's boundary", () => {
  test("nothing outside the driver and the loop commands imports them", () => {
    const files = tsFiles("tools").filter((f) => !inside(f));
    const crossings: string[] = [];
    for (const file of files) {
      for (const { spec, dynamic } of specifiers(readFileSync(join(ROOT, file), "utf8"))) {
        if (!spec.startsWith(".")) continue;
        const target = rel(resolve(join(ROOT, dirname(file)), spec));
        if (!inside(target)) continue;
        // `main.ts` names every command in `COMMANDS`, and that is the one way in that stays.
        if (file === "tools/nv/main.ts" && dynamic && LOOP_COMMANDS.includes(target)) continue;
        crossings.push(`${file}: ${spec}`);
      }
    }
    expect(files.length).toBeGreaterThanOrEqual(100);
    expect(crossings).toEqual([]);
  });

  test("the shared modules live outside the driver", () => {
    const moved: [string, string][] = [
      ["tools/nv/driver/accept.ts", "tools/nv/accept/accept.ts"],
      ["tools/nv/driver/runner.ts", "tools/nv/accept/runner.ts"],
      ["tools/nv/driver/origin.ts", "tools/nv/accept/origin.ts"],
      ["tools/nv/driver/sweep-lock.ts", "tools/nv/accept/sweep-lock.ts"],
      ["tools/nv/driver/legs.ts", "tools/nv/accept/legs.ts"],
      ["tools/nv/driver/mirror.ts", "tools/nv/accept/mirror.ts"],
      ["tools/nv/driver/proctree.ts", "tools/nv/lib/proctree.ts"],
    ];
    const left = moved.filter(([from, to]) => existsSync(join(ROOT, from)) || !existsSync(join(ROOT, to))).map(([from]) => from);

    const symbols: [string, string, string][] = [
      ["sweepOver", "tools/nv/cmd/loop.ts", "tools/nv/accept/sweep.ts"],
      ["sweepIds", "tools/nv/cmd/loop.ts", "tools/nv/accept/sweep.ts"],
      ["featureMatches", "tools/nv/cmd/loop.ts", "tools/nv/accept/accept.ts"],
      ["NUMBER_CITE", "tools/nv/cmd/chain.ts", "tools/nv/lib/prose.ts"],
    ];
    for (const [name, from, to] of symbols) {
      if (defines(from, name) || !defines(to, name)) left.push(`${name} in ${from}`);
    }
    expect(left).toEqual([]);
  });

  test("the check runners have one home", () => {
    const left: string[] = [];
    for (const name of ["preflight", "bringUp"]) {
      for (const file of tsFiles("tools/nv/driver")) if (defines(file, name)) left.push(`${name} in ${file}`);
      if (!defines("tools/nv/cmd/services.ts", name)) left.push(`${name} not in tools/nv/cmd/services.ts`);
    }
    for (const file of ["tools/nv/accept/sweep.ts", "tools/nv/cmd/legs.ts"]) if (!/\blegsOptions\(/.test(read(file))) left.push(`${file} builds the legs' options without legsOptions`);
    left.push(...unregistered(["suite", "legs", "services"]));
    expect(left).toEqual([]);
  });

  test("the session hooks have one home", () => {
    const left: string[] = [];
    const homes: [string, string][] = [
      ["prepare", "tools/nv/cmd/side.ts"],
      ["seedStore", "tools/nv/cmd/side.ts"],
      ["AllowWatch", "tools/nv/cmd/allow.ts"],
      ["settle", "tools/nv/cmd/allow.ts"],
      ["benchTurn", "tools/nv/cmd/benches.ts"],
      ["afterSession", "tools/nv/cmd/sweep.ts"],
    ];
    const driven = [...tsFiles("tools/nv/driver"), ...LOOP_COMMANDS];
    for (const [name, home] of homes) {
      for (const file of driven) if (defines(file, name) || definesLocal(file, name)) left.push(`${name} in ${file}`);
      if (!defines(home, name)) left.push(`${name} not in ${home}`);
    }
    if (/["']--benches["']/.test(read("tools/nv/cmd/loop.ts"))) left.push("tools/nv/cmd/loop.ts still parses --benches");
    if (!/\bafterSession\(/.test(read("tools/nv/cmd/loop.ts"))) left.push("tools/nv/cmd/loop.ts sweeps without afterSession");
    left.push(...unregistered(["sweep", "benches", "side", "allow"]));
    expect(left).toEqual([]);
  });
});

const read = (path: string) => readFileSync(join(ROOT, path), "utf8");

/** Whether `path` exports a function, constant or class `name`. */
function defines(path: string, name: string): boolean {
  return new RegExp(`^export\\s+(?:async\\s+)?(?:function\\*?|const|let|class)\\s+${name}\\b`, "m").test(read(path));
}

/** Whether `path` defines a function, constant or class `name` it does not export. */
function definesLocal(path: string, name: string): boolean {
  return new RegExp(`^(?:async\\s+)?(?:function\\*?|const|let|class)\\s+${name}\\b`, "m").test(read(path));
}

/** Each of `commands` that `main.ts`'s `COMMANDS` does not map to its own module. */
function unregistered(commands: string[]): string[] {
  const main = read("tools/nv/main.ts");
  return commands.filter((c) => !new RegExp(`^\\s+"?${c}"?: \\(\\) => import\\("\\./cmd/${c}\\.ts"\\)`, "m").test(main)).map((c) => `${c} is not registered in tools/nv/main.ts`);
}
