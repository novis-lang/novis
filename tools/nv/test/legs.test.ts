import { describe, expect, test } from "bun:test";
import { type Check, GreenMemo, type Outcome } from "../driver/accept.ts";
import { ROOT } from "../lib/paths.ts";
import { type LegName, type LegsOptions, type LegsSeams, legSpec, linuxLegs, q, startWslBuild, valgrindFailLine, valgrindLine, wslPath } from "../driver/legs.ts";
import { CARRIED, carryLine, mirrorPath, targetLine } from "../driver/mirror.ts";
import { mkdirSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const label = (n: number) => (n === 1 ? "1 floor" : String(n));

/** The copy of this checkout the WSL leg builds and runs from, for the target directory `options` names. */
const MIRROR = mirrorPath("/var/tmp/nvs-target-wsl", wslPath(ROOT));

function program(file: string, stage = 5, extra: Partial<Check> = {}): Check {
  return { id: `run ${file}`, kind: "exact", stage, file, want: ["ok"], ...extra };
}

const suite: Check = { id: "suite conformance", kind: "nvs-suite", stage: 3, name: "conformance", args: ["test", "tests/conformance"] };

interface Fake {
  lines: string[];
  said: string[];
  origins: number;
  closed: number;
  /** Every copy synced, in order, and the repository each origin was started from. */
  synced: string[];
  originRepos: string[];
  seams: Partial<LegsSeams>;
}

/** A WSL machine whose every process is `answer(line)`; green unless told otherwise. */
function fake(answer: (line: string) => Outcome | undefined = () => undefined, platform: NodeJS.Platform = "win32"): Fake {
  const f: Fake = { lines: [], said: [], origins: 0, closed: 0, synced: [], originRepos: [], seams: {} };
  let clock = 0;
  f.seams = {
    platform,
    hasWsl: () => true,
    hasValgrind: () => true,
    shell: async (_where, line) => {
      f.lines.push(line);
      clock += 1000;
      const got = answer(line);
      if (got !== undefined) return got;
      if (line.includes("nvs-suite") || line.endsWith("test tests/conformance")) return { code: 0, out: "12 passed, 0 failed\n", err: "" };
      return { code: 0, out: "ok\n", err: "" };
    },
    sync: async (mirror) => {
      f.synced.push(mirror);
      return "";
    },
    origin: async (_where, _binary, repo) => {
      f.origins++;
      f.originRepos.push(repo);
      return { line: "wsl origin: listening", close: () => void f.closed++ };
    },
    profile: () => ({ cores: 4 }),
    jobs: (_c, ceiling) => Math.min(2, ceiling),
    remember: () => {},
    now: () => clock,
  };
  return f;
}

function options(f: Fake, over: Partial<LegsOptions> = {}): LegsOptions {
  return {
    programs: [program("examples/a.nvs"), program("examples/b.nvs")],
    suites: [suite],
    files: ["examples/a.nvs", "examples/b.nvs", "examples/c.nvs"],
    valgrindSkip: [],
    wslTarget: "/var/tmp/nvs-target-wsl",
    gateOpen: true,
    memo: new GreenMemo(),
    key: (leg: LegName) => `key-${leg}`,
    full: false,
    label,
    onRun: (what) => f.said.push(what),
    seams: f.seams,
    ...over,
  };
}

const valgrindRuns = (f: Fake) => f.lines.filter((l) => l.includes("valgrind --error-exitcode"));
const builds = (f: Fake) => f.lines.filter((l) => l.includes("cargo build"));

describe("linuxLegs", () => {
  test("a green WSL leg and a green valgrind sweep are remembered when the floor gate is open", async () => {
    const f = fake();
    const o = options(f);
    expect(await linuxLegs(o)).toBe("");
    expect(f.synced).toEqual([MIRROR]);
    expect(builds(f)).toEqual([`cd ${q(MIRROR)} && CARGO_TARGET_DIR=/var/tmp/nvs-target-wsl cargo build --quiet`]);
    expect(f.lines.filter((l) => l.includes(" run ") && !l.includes("valgrind"))).toHaveLength(2);
    // Every fixture and valgrind run is in the copy; only the origin, whose program is written here, is on the mount.
    expect(f.lines.every((l) => l.startsWith(`cd ${q(MIRROR)} && `))).toBe(true);
    expect(f.originRepos).toEqual([wslPath(ROOT)]);
    expect(f.lines.some((l) => l.endsWith("/var/tmp/nvs-target-wsl/debug/nvs test tests/conformance"))).toBe(true);
    expect(valgrindRuns(f)).toHaveLength(2);
    expect(f.origins).toBe(1);
    expect(f.closed).toBe(1);
    expect(o.memo.answers(legSpec("wsl leg"), "key-wsl leg")).toBe(true);
    expect(o.memo.answers(legSpec("valgrind sweep"), "key-valgrind sweep")).toBe(true);
  });

  test("with the floor gate shut, a green leg is not remembered", async () => {
    const f = fake();
    const o = options(f, { gateOpen: false });
    expect(await linuxLegs(o)).toBe("");
    expect(o.memo.answers(legSpec("wsl leg"), "key-wsl leg")).toBe(false);
    expect(o.memo.answers(legSpec("valgrind sweep"), "key-valgrind sweep")).toBe(false);
  });

  test("a memo that answers both legs skips the build and every run", async () => {
    const f = fake();
    const memo = new GreenMemo({ "wsl leg": "key-wsl leg", "valgrind sweep": "key-valgrind sweep" });
    expect(await linuxLegs(options(f, { memo }))).toBe("");
    expect(f.lines).toEqual([]);
    expect(f.synced).toEqual([]);
    expect(f.origins).toBe(0);
    // `full` consults no memo.
    expect(await linuxLegs(options(f, { memo, full: true }))).toBe("");
    expect(builds(f)).toHaveLength(1);
  });

  test("a memo that answers one leg still builds, and runs only the other", async () => {
    const f = fake();
    const memo = new GreenMemo({ "wsl leg": "key-wsl leg" });
    expect(await linuxLegs(options(f, { memo }))).toBe("");
    expect(builds(f)).toHaveLength(1);
    expect(f.lines.filter((l) => l.includes(" run ") && !l.includes("valgrind"))).toHaveLength(0);
    expect(valgrindRuns(f)).toHaveLength(2);
  });

  test("a fixture red on the WSL leg is the programFailLine, earliest stage first, and is not remembered", async () => {
    const f = fake((line) => (line.includes("valgrind") ? undefined : line.endsWith("run examples/b.nvs") || line.endsWith("run examples/a.nvs") ? { code: 0, out: "wrong\n", err: "" } : undefined));
    const o = options(f, { programs: [program("examples/a.nvs", 5), program("examples/b.nvs", 1)] });
    const got = await linuxLegs(o);
    expect(got.split("\n")[0]).toBe("wsl examples/b.nvs [1 floor]: stdout was [wrong], wanted [ok]");
    expect(got).toContain("(and 1 later fixture(s) red, in stage order: examples/a.nvs [5])");
    expect(o.memo.answers(legSpec("wsl leg"), "key-wsl leg")).toBe(false);
    // The valgrind sweep still ran and is green.
    expect(o.memo.answers(legSpec("valgrind sweep"), "key-valgrind sweep")).toBe(true);
  });

  test("a red suite on the WSL leg is reported", async () => {
    const f = fake((line) => (line.endsWith("test tests/conformance") ? { code: 0, out: "10 passed, 2 failed\n", err: "" } : undefined));
    expect(await linuxLegs(options(f))).toBe("wsl conformance [3]: 2 case(s) failed");
  });

  test("checks that name one command line share one run on the WSL leg, and each judges it", async () => {
    const f = fake((line) => (line.endsWith("test tests/conformance") ? { code: 0, out: "10 passed, 2 failed\n", err: "" } : undefined));
    const other: Check = { ...suite, id: "suite conformance again", stage: 1, name: "conformance again" };
    const got = await linuxLegs(options(f, { programs: [program("examples/a.nvs"), program("examples/a.nvs", 1, { id: "run a again" })], suites: [suite, other] }));
    expect(f.lines.filter((l) => l.endsWith("test tests/conformance"))).toHaveLength(1);
    expect(f.lines.filter((l) => l.endsWith(" run examples/a.nvs") && !l.includes("valgrind"))).toHaveLength(1);
    expect(got.split("\n")[0]).toBe("wsl conformance again [1 floor]: 2 case(s) failed");
    expect(got).toContain("in stage order: conformance [3]");
  });

  test("two leaking fixtures give the `(and 1 more: ...)` line, in the plan's order", async () => {
    const leak = { code: 97, out: "", err: "==1== 8 bytes in 1 blocks are definitely lost\n" };
    const f = fake((line) => (line.includes("valgrind") ? leak : undefined));
    const o = options(f);
    expect(await linuxLegs(o)).toBe(
      "valgrind examples/a.nvs: exit 97 -- ==1== 8 bytes in 1 blocks are definitely lost  (and 1 more: valgrind examples/b.nvs)",
    );
    expect(o.memo.answers(legSpec("valgrind sweep"), "key-valgrind sweep")).toBe(false);
    expect(o.memo.answers(legSpec("wsl leg"), "key-wsl leg")).toBe(true);
  });

  test("a fixture's own non-zero exit through valgrind is not a leak", async () => {
    const f = fake((line) => (line.includes("valgrind") ? { code: 1, out: "", err: "FATAL: by design" } : undefined));
    expect(await linuxLegs(options(f))).toBe("");
  });

  test("a WSL red and a valgrind red are both reported", async () => {
    const f = fake((line) =>
      line.includes("valgrind") ? { code: 97, out: "", err: "lost" } : line.endsWith("run examples/a.nvs") ? { code: 3, out: "", err: "boom" } : undefined,
    );
    const got = await linuxLegs(options(f, { programs: [program("examples/a.nvs")], files: ["examples/a.nvs"] }));
    expect(got).toBe("wsl examples/a.nvs [5]: exit 3 -- boom\n       also red: valgrind examples/a.nvs: exit 97 -- lost");
  });

  test("the skip list and files the sweep did not reach are not swept", async () => {
    const f = fake();
    await linuxLegs(options(f, { valgrindSkip: ["examples/b.nvs"] }));
    expect(valgrindRuns(f).map((l) => l.split(" ").pop())).toEqual(["examples/a.nvs"]);
  });

  test("examples/limits.nvs gets the two --config flags", async () => {
    const f = fake();
    await linuxLegs(options(f, { programs: [program("examples/limits.nvs", 5, { exit: "nonzero" })], files: ["examples/limits.nvs"] }));
    const [line] = valgrindRuns(f);
    expect(line).toEndWith("/var/tmp/nvs-target-wsl/debug/nvs --config nvs.toml --config tools/valgrind-limits.toml run examples/limits.nvs");
    expect(line).toContain("--suppressions=tools/valgrind.supp");
    expect(valgrindLine("/r", "/b/nvs", "examples/a.nvs")).toEndWith("/b/nvs run examples/a.nvs");
  });

  test("a fixture that needs af-unix runs on the WSL leg, and an unknown need is red", async () => {
    const f = fake();
    await linuxLegs(options(f, { programs: [program("examples/unix.nvs", 5, { needs: "af-unix" })], files: [] }));
    expect(f.lines.some((l) => l.endsWith("run examples/unix.nvs"))).toBe(true);
    const g = fake();
    const got = await linuxLegs(options(g, { programs: [program("examples/x.nvs", 5, { needs: "gpu" })], files: [] }));
    expect(got).toStartWith('wsl examples/x.nvs [5]: `needs` is "gpu", which no leg is asked');
  });

  test("the longest fixture last time is submitted first", async () => {
    const f = fake();
    f.seams.profile = () => ({ cores: 4, fixture_s: { "examples/a.nvs": 1, "examples/b.nvs": 9 } });
    f.seams.jobs = () => 1;
    await linuxLegs(options(f, { files: ["examples/a.nvs", "examples/b.nvs"] }));
    expect(valgrindRuns(f).map((l) => l.split(" ").pop())).toEqual(["examples/b.nvs", "examples/a.nvs"]);
  });

  test("a failed WSL build is the whole line", async () => {
    const f = fake((line) => (line.includes("cargo build") ? { code: 101, out: "", err: "error[E0425]: cannot find value" } : undefined));
    expect(await linuxLegs(options(f))).toBe("the wsl build failed -- error[E0425]: cannot find value");
    expect(f.origins).toBe(0);
  });

  test("no wsl.exe, no target directory or no fixture skips both without failing", async () => {
    const f = fake();
    f.seams.hasWsl = () => false;
    expect(await linuxLegs(options(f))).toBe("");
    expect(await linuxLegs(options(fake(), { wslTarget: null }))).toBe("");
    expect(await linuxLegs(options(fake(), { programs: [] }))).toBe("");
    expect(f.lines).toEqual([]);
    expect(f.said.some((l) => l.includes("no wsl.exe"))).toBe(true);
  });

  test("off Windows the valgrind sweep runs natively, and is skipped without valgrind", async () => {
    const f = fake(undefined, "linux");
    const o = options(f);
    expect(await linuxLegs(o)).toBe("");
    expect(f.lines.filter((l) => l.includes(" run ") && !l.includes("valgrind"))).toHaveLength(0);
    expect(valgrindRuns(f)).toHaveLength(2);
    expect(valgrindRuns(f)[0]).toMatch(/[\\/]target[\\/]debug[\\/]nvs'? run examples\/a\.nvs$/);
    expect(f.lines.filter((l) => l.includes("cargo build"))).toHaveLength(1);
    expect(o.memo.answers(legSpec("valgrind sweep"), "key-valgrind sweep")).toBe(true);
    const g = fake(undefined, "linux");
    g.seams.hasValgrind = () => false;
    expect(await linuxLegs(options(g))).toBe("");
    expect(g.lines).toEqual([]);
    expect(g.said).toContain("valgrind sweep skipped -- no valgrind on this platform");
  });

  test("a build started early is the one linuxLegs waits for", async () => {
    const f = fake();
    const o = options(f);
    expect(startWslBuild(o)).toBe(true);
    expect(await linuxLegs(o)).toBe("");
    expect(f.synced).toHaveLength(1);
    expect(builds(f)).toHaveLength(1);
    expect(startWslBuild(options(fake(), { gateOpen: false }))).toBe(false);
  });

  test("a copy that cannot be synced is red, and nothing is built or run from it", async () => {
    const f = fake();
    f.seams.sync = async () => "git add -A: fatal: unable to write new index file";
    const got = await linuxLegs(options(f));
    expect(got).toBe("the wsl copy of the tree could not be synced -- git add -A: fatal: unable to write new index file");
    expect(f.lines).toEqual([]);
    expect(f.origins).toBe(0);
  });
});

describe("the leg helpers", () => {
  test("wslPath maps a Windows drive path to its mount", () => {
    expect(wslPath("D:\\mwl")).toBe("/mnt/d/mwl");
    expect(wslPath("C:/Users/x/repo/")).toBe("/mnt/c/Users/x/repo");
  });

  test("mirrorPath gives each checkout its own copy beside the target directory", () => {
    expect(mirrorPath("/var/tmp/nvs-target-wsl", "/mnt/d/mwl")).toBe("/var/tmp/nvs-target-wsl-src/mnt-d-mwl");
    expect(mirrorPath("/var/tmp/nvs-target-wsl", "/mnt/d/mwl/.agent-tmp/worktrees/x")).toBe(
      "/var/tmp/nvs-target-wsl-src/mnt-d-mwl-.agent-tmp-worktrees-x",
    );
  });

  test("carryLine copies an ignored file the configuration names, and only when the checkout has it", () => {
    const root = join(ROOT, ".agent-tmp", "legs-test-carry");
    try {
      mkdirSync(join(root, "tests", "db"), { recursive: true });
      expect(carryLine(root)).toBe("");
      writeFileSync(join(root, "tests", "db", "ca.crt"), "x");
      expect(carryLine(root)).toBe(
        ` && mkdir -p tests/db && cp ${wslPath(root)}/tests/db/ca.crt tests/db/ca.crt`,
      );
      expect(CARRIED).toContain("tests/db/ca.crt");
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  });

  test("targetLine links the copy's target to the target directory the copy sits beside", () => {
    const mirror = mirrorPath("/var/tmp/nvs-target-wsl", "/mnt/d/mwl");
    expect(targetLine(mirror)).toBe(" && ln -sfn /var/tmp/nvs-target-wsl target");
  });

  test("q quotes a word only when bash would split or expand it", () => {
    expect(q("examples/a.nvs")).toBe("examples/a.nvs");
    expect(q("--flag=1")).toBe("--flag=1");
    expect(q("two words")).toBe("'two words'");
    expect(q("it's")).toBe("'it'\\''s'");
    expect(q("")).toBe("''");
  });

  test("valgrindFailLine names every leak after the first", () => {
    expect(valgrindFailLine([])).toBe("");
    expect(valgrindFailLine(["valgrind a: exit 97 -- x"])).toBe("valgrind a: exit 97 -- x");
    expect(valgrindFailLine(["valgrind a: exit 97 -- x", "valgrind b: exit 97 -- y", "valgrind c: exit 97 -- z"])).toBe(
      "valgrind a: exit 97 -- x  (and 2 more: valgrind b, valgrind c)",
    );
  });
});
