// The test distro (`lib/wsl.ts`, `cmd/wsl.ts`): every WSL call names it, and `--setup` finds every version
// it installs in the file CI reads it from.

import { describe, expect, test } from "bun:test";
import { readdirSync, readFileSync } from "node:fs";
import { join, relative } from "node:path";
import { provisionArgv, versions } from "../cmd/wsl.ts";
import { ROOT } from "../lib/paths.ts";
import { DISTRO, wslArgv, wslExec } from "../lib/wsl.ts";

/** Every TypeScript file under `tools/nv`, tests left out. */
function sources(dir: string = join(ROOT, "tools", "nv")): string[] {
  return readdirSync(dir, { withFileTypes: true }).flatMap((e) => {
    const p = join(dir, e.name);
    if (e.isDirectory()) return e.name === "test" || e.name === "node_modules" ? [] : sources(p);
    return e.name.endsWith(".ts") ? [p] : [];
  });
}

describe("the test distro", () => {
  test("every argv names the distro, and the shell and user are the ones asked for", () => {
    expect(wslArgv("true")).toEqual(["wsl.exe", "-d", DISTRO, "--exec", "bash", "-lc", "true"]);
    expect(wslArgv("true", { login: false, root: true })).toEqual(["wsl.exe", "-d", DISTRO, "-u", "root", "--exec", "bash", "-c", "true"]);
    expect(wslExec(["valgrind", "x"])).toEqual(["wsl.exe", "-d", DISTRO, "--exec", "valgrind", "x"]);
    expect(provisionArgv("system").slice(0, 5)).toEqual(["wsl.exe", "-d", DISTRO, "-u", "root"]);
  });

  test("no tool starts wsl.exe with a flag before -d, so none runs in the default distro", () => {
    // `"wsl.exe", "--exec"` or `"wsl.exe", "--"` runs in whatever distro is the default, which can be any
    // distro at all. `lib/wsl.ts` builds every argv, and `cmd/wsl.ts` runs WSL's own management commands.
    const allowed = new Set(["tools/nv/lib/wsl.ts", "tools/nv/cmd/wsl.ts"]);
    const offenders = sources()
      .map((f) => relative(ROOT, f).replaceAll("\\", "/"))
      .filter((f) => !allowed.has(f) && /"wsl\.exe",\s*"-(?!d")/.test(readFileSync(join(ROOT, f), "utf8")));
    expect(offenders).toEqual([]);
  });

  test("every version --setup installs is found where CI reads it", () => {
    const v = versions();
    expect(Number(v.NVS_NODE_MAJOR)).toBeGreaterThanOrEqual(20);
    for (const key of ["NVS_BUN", "NVS_NEXTEST", "NVS_FUZZ"]) expect(v[key]).toMatch(/^\d+\.\d+\.\d+$/);
  });
});
