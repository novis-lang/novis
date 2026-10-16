// `bun nv legs` over a fake WSL machine and a fake store: the goal is handed in, and every process the
// legs would start is answered here, so nothing builds and no distro is needed.

import { describe, expect, test } from "bun:test";
import type { Check, Outcome } from "../accept/accept.ts";
import type { LegName, LegsSeams } from "../accept/legs.ts";
import { legs } from "../cmd/legs.ts";
import type { Goal } from "../lib/chain.ts";

const TARGET = "/var/tmp/nvs-target-cmd";

const fixture = (file: string): Check => ({ id: `run-${file.replace(/\W+/g, "-")}`, kind: "exact", stage: 1, file, want: ["ok"] });

const GOAL = {
  checks: [fixture("examples/a.nvs"), fixture("examples/b.nvs")],
  files: ["examples/a.nvs", "examples/b.nvs"],
  stages: [{ number: 1, title: "floor" }],
  env: { wsl: { targetDir: TARGET }, valgrind: { skip: ["examples/b.nvs"] } },
} as unknown as Goal;

/** Runs `bun nv legs` on a WSL machine whose every process is `answer(line)`, green unless told otherwise. */
async function legsOver(answer: (line: string) => Outcome | undefined = () => undefined) {
  const lines: string[] = [];
  const said: string[] = [];
  const green = new Set<LegName>();
  const red = new Set<LegName>();
  let done = 0;
  const machine: Partial<LegsSeams> = {
    platform: "win32",
    hasWsl: () => true,
    hasValgrind: () => true,
    shell: async (_where, line) => {
      lines.push(line);
      return answer(line) ?? { code: 0, out: "ok\n", err: "" };
    },
    sync: async () => "",
    origin: async () => ({ line: "wsl origin: listening", close: () => {} }),
    profile: () => ({ cores: 4 }),
    jobs: () => 1,
    remember: () => {},
    now: () => 0,
  };
  const code = await legs(["--goal", "demo"], {
    plan: (slug) => (slug === "demo" ? { slug, goal: GOAL } : null),
    store: async () => ({ reached: () => true, green: (l) => void green.add(l), red: (l) => void red.add(l), done: () => void done++ }),
    hold: async () => () => {},
    machine,
    say: (l) => said.push(l),
  });
  return { code, lines, said, green, red, done };
}

describe("bun nv legs", () => {
  test("nv legs reads the WSL target and the valgrind skip list from the goal's env", async () => {
    const got = await legsOver();
    expect(got.lines.some((l) => l.endsWith(`CARGO_TARGET_DIR=${TARGET} cargo build --quiet`))).toBe(true);
    const valgrind = got.lines.filter((l) => l.includes("valgrind --error-exitcode"));
    expect(valgrind).toHaveLength(1);
    expect(valgrind[0]).toContain("examples/a.nvs");
    expect(got.done).toBe(1);
  });

  test("nv legs exits 1 on a red leg and 0 on a green one", async () => {
    const ok = await legsOver();
    expect(ok.code).toBe(0);
    expect(ok.green).toEqual(new Set<LegName>(["wsl leg", "valgrind sweep"]));
    expect(ok.said.at(-1)).toBe("GREEN the Linux legs of demo");

    const bad = await legsOver((line) => (line.includes("examples/b.nvs") && !line.includes("valgrind") ? { code: 0, out: "wrong\n", err: "" } : undefined));
    expect(bad.code).toBe(1);
    expect(bad.red.has("wsl leg")).toBe(true);
    expect(bad.said.at(-1)).toStartWith("RED ");
    expect(bad.said.at(-1)).toContain("examples/b.nvs");
  });
});
