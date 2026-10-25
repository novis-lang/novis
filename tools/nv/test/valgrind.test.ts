import { describe, expect, test } from "bun:test";
import type { Outcome } from "../accept/accept.ts";
import { besideOf, valgrind, valgrindArgv, type ValgrindSeams, verdict } from "../cmd/valgrind.ts";

interface Fake {
  runs: string[][];
  envs: Record<string, string>[];
  said: string[];
  origins: number;
  closed: number;
  seams: Partial<ValgrindSeams>;
}

/** A Linux machine with valgrind whose every run is `answer(file)`; clean unless told otherwise. */
function fake(answer: (file: string) => Outcome | undefined = () => undefined): Fake {
  const f: Fake = { runs: [], envs: [], said: [], origins: 0, closed: 0, seams: {} };
  f.seams = {
    platform: "linux",
    hasValgrind: () => true,
    exists: (path) => path.endsWith(".nvs") || path === "/b/nvs",
    examples: () => ["examples/b.nvs", "examples/a.nvs", "examples/c.nvs"],
    run: async (argv, env) => {
      f.runs.push(argv);
      f.envs.push(env);
      return answer(argv.at(-1)!) ?? { code: 0, out: "", err: "" };
    },
    origin: async () => {
      f.origins++;
      return { line: "origin: serving", close: () => void f.closed++ };
    },
    jobs: (ceiling) => Math.min(2, ceiling),
    say: (line) => void f.said.push(line),
  };
  return f;
}

const files = (f: Fake) => f.runs.map((argv) => argv.at(-1)!).sort();

describe("bun nv valgrind", () => {
  test("every example runs under memcheck with the origin up, and a clean run is green", async () => {
    const f = fake();
    expect(await valgrind(["--nvs", "/b/nvs"], f.seams)).toBe(0);
    expect(f.runs.map((argv) => argv.at(-1))).toEqual(["examples/a.nvs", "examples/b.nvs", "examples/c.nvs"]);
    expect(f.envs.every((e) => e.NVS_BIN === "/b/nvs")).toBe(true);
    expect(f.origins).toBe(1);
    expect(f.closed).toBe(1);
    expect(f.said.at(-1)).toBe("GREEN 3 program(s) under valgrind, no definite leak and no invalid access");
  });

  test("a leak and a program that did not finish are red, named in file order; a program's own exit is not", async () => {
    const f = fake((file) =>
      file === "examples/c.nvs" ? { code: 97, out: "", err: "==1== 48 bytes in 1 blocks are definitely lost" }
      : file === "examples/a.nvs" ? { code: -1, out: "", err: "timed out after 1800s" }
      : { code: 1, out: "", err: "uncaught LogicError" },
    );
    expect(await valgrind(["--nvs", "/b/nvs"], f.seams)).toBe(1);
    expect(f.said.slice(-3)).toEqual([
      "RED 2 of 3 program(s):",
      "  valgrind examples/a.nvs: did not finish -- timed out after 1800s",
      "  valgrind examples/c.nvs: exit 97 -- ==1== 48 bytes in 1 blocks are definitely lost",
    ]);
    expect(f.closed).toBe(1);
  });

  test("named files run instead of the examples", async () => {
    const f = fake();
    expect(await valgrind(["--nvs", "/b/nvs", "examples\\b.nvs"], f.seams)).toBe(0);
    expect(files(f)).toEqual(["examples/b.nvs"]);
  });

  test("off Linux, with no --nvs or no binary there, or with a missing file, it is a usage error and runs nothing", async () => {
    const off = fake();
    off.seams.platform = "win32";
    expect(await valgrind(["--nvs", "/b/nvs"], off.seams)).toBe(2);
    const unnamed = fake();
    expect(await valgrind([], unnamed.seams)).toBe(2);
    const nobin = fake();
    nobin.seams.exists = (p) => p !== "/b/nvs";
    expect(await valgrind(["--nvs", "/b/nvs"], nobin.seams)).toBe(2);
    const nofile = fake();
    nofile.seams.exists = (p) => p !== "examples/x.nvs";
    expect(await valgrind(["--nvs", "/b/nvs", "examples/x.nvs"], nofile.seams)).toBe(2);
    expect(await valgrind(["--jobs"], fake().seams)).toBe(2);
    expect([...off.runs, ...unnamed.runs, ...nobin.runs, ...nofile.runs]).toEqual([]);
  });

  test("no valgrind on PATH is red, since nothing was checked", async () => {
    const f = fake();
    f.seams.hasValgrind = () => false;
    expect(await valgrind(["--nvs", "/b/nvs"], f.seams)).toBe(1);
    expect(f.runs).toEqual([]);
  });
});

describe("the valgrind helpers", () => {
  test("examples/limits.nvs gets the two --config flags, and every run the suppressions", () => {
    expect(valgrindArgv("/b/nvs", "examples/limits.nvs").slice(-7)).toEqual(["/b/nvs", "--config", "nvs.toml", "--config", "tools/valgrind-limits.toml", "run", "examples/limits.nvs"]);
    const plain = valgrindArgv("/b/nvs", "examples/a.nvs");
    expect(plain.slice(-3)).toEqual(["/b/nvs", "run", "examples/a.nvs"]);
    expect(plain).toContain("--suppressions=tools/valgrind.supp");
    expect(plain).toContain("--error-exitcode=97");
  });

  test("a request and a configuration beside a program are passed the way the examples test passes them", () => {
    const beside = besideOf("examples/a.nvs", (p) => p === "examples/a.nvsr" || p === "examples/a.toml");
    expect(beside).toEqual({ toml: "examples/a.toml", nvsr: "examples/a.nvsr" });
    expect(valgrindArgv("/b/nvs", "examples/a.nvs", beside).slice(-7)).toEqual(["/b/nvs", "--config", "examples/a.toml", "run", "--request", "examples/a.nvsr", "examples/a.nvs"]);
    expect(besideOf("examples/b.nvs", () => false)).toEqual({});
  });

  test("only exit 97 and a run that did not finish are red", () => {
    expect(verdict("a", { code: 0, out: "", err: "" })).toBe("");
    expect(verdict("a", { code: 1, out: "", err: "boom" })).toBe("");
    expect(verdict("a", { code: 97, out: "", err: "lost" })).toBe("valgrind a: exit 97 -- lost");
  });

});
