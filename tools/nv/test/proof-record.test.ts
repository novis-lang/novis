import { describe, expect, test } from "bun:test";
import { existsSync, readFileSync, rmSync } from "node:fs";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import { covwsNvs } from "../lib/covws.ts";
import { abs } from "../lib/paths.ts";
import { run } from "../lib/proc.ts";
import { ENV as READS_ENV, readModules } from "../lib/reads.ts";
import { divergence, HANG_FACTOR, hostileLimitMs, PROOF_BINARY, PROOF_BUILD, RECORD_ENV, type Ran, proofRecording, recordingLimitMs, recordName, type Result, spawnProof, timedOut } from "../proofs/run.ts";
import { judgeThenRecord, longestFirst, recordProgram } from "../proofs/select.ts";
import { proofId } from "../select/atoms.ts";
import { Recorder } from "../select/record.ts";
import { SelectStore } from "../select/store.ts";
import { scratch } from "./scratch.ts";

describe("recording a proof program", () => {
  test("a record name is the path with its slashes as `~` and every other unsafe byte escaped", () => {
    expect(recordName("tests/conformance/a b.nvst")).toBe("tests~conformance~a@20b.nvst");
    expect(recordName("tests\\conformance\\a b.nvst")).toBe("tests~conformance~a@20b.nvst");
    expect(recordName("50%.nvs")).toBe("50@25.nvs");
    expect(recordName("é.nvs")).toBe("@c3@a9.nvs");
    expect(recordName("a~b.nvs")).not.toBe(recordName("a/b.nvs"));
    // A long name is cut and ends in the hash of the whole, as `nvs_test::record_name` does.
    const long = "tests/conformance/reject/a-binding-refuses-a-second-declaration-an-undeclared-assignment-a-changed-type.nvst";
    expect(recordName(long)).toBe("tests~conformance~reject~a-binding-refuses-a-second-declaration-@43cc62f86b7c4528");
    expect(recordName("x".repeat(96))).toBe("x".repeat(96));
  });

  test("nothing is recorded unless a directory is named, and a named one gets absolute paths", () => {
    expect(proofRecording("docs/examples/core/Math/sqrt/01-square-roots.nvs", undefined)).toEqual({});
    const vars = proofRecording("docs/examples/core/Math/sqrt/01-square-roots.nvs", ".cache/records");
    const stem = "docs~examples~core~Math~sqrt~01-square-roots.nvs";
    expect(vars.LLVM_PROFILE_FILE).toBe(join(abs(".cache/records"), `${stem}-%p.profraw`));
    expect(vars.NVS_FOOTPRINT_LOG).toBe(join(abs(".cache/records"), `${stem}.log`));
    expect(vars.NOVIS_NO_FILE_CACHE).toBe("1");
  });

  test("a recorded run leaves the program's footprint log under its name", async () => {
    const nvs = covwsNvs();
    if (!existsSync(nvs)) throw new Error(`${nvs} is built by \`bun nv verify\` before this test runs`);
    const proof = "docs/examples/core/Math/sqrt/01-square-roots.nvs";
    const s = scratch();
    const before = process.env[RECORD_ENV];
    process.env[RECORD_ENV] = s.root;
    try {
      const ran = await spawnProof([nvs, "run", proof], proof, 60_000);
      expect(ran.code).toBe(0);
      const log = readFileSync(join(s.root, `${recordName(proof)}.log`), "utf8");
      expect(log.split("\n")).toContain("class\tCore\\Math");
      expect(log).toContain(`file\t${abs(proof).replaceAll("\\", "/")}`);
    } finally {
      if (before === undefined) delete process.env[RECORD_ENV];
      else process.env[RECORD_ENV] = before;
      s.cleanup();
    }
  }, 120_000);
});

describe("the modules a recorded `nv proofs` names", () => {
  const LOG = join(abs(".cache"), `proof-modules-${process.pid}.ndjson`);
  // Code that judges a program, gates a feature or blesses an output: a change to any of it reaches
  // every proofs check.
  const JUDGING = ["tools/nv/cmd/proofs.ts", "tools/nv/proofs/run.ts", "tools/nv/proofs/select.ts", "tools/nv/proofs/collect.ts", "tools/nv/proofs/roster.ts", "tools/nv/proofs/markers.ts", "tools/nv/proofs/ledger.ts", "tools/nv/proofs/perf.ts"];
  // The selection engine, the crate graph, the recorder and the driver, which decide what runs and what
  // the store remembers.
  const ENGINE = ["tools/nv/select/select.ts", "tools/nv/select/items.ts", "tools/nv/select/change.ts", "tools/nv/select/record.ts", "tools/nv/select/extract.ts", "tools/nv/select/atoms.ts", "tools/nv/keys/graph.ts", "tools/nv/driver/accept.ts", "tools/nv/driver/runner.ts"];

  test("the command names the judging modules and none of the engine, and loading the engine for a recording names none of it", async () => {
    const script = [
      `import { install } from ${JSON.stringify(pathToFileURL(abs("tools/nv/lib/reads.ts")).href)};`,
      `install(process.env.${READS_ENV});`,
      `await import(${JSON.stringify(pathToFileURL(abs("tools/nv/cmd/proofs.ts")).href)});`,
      `const { recordProgram } = await import(${JSON.stringify(pathToFileURL(abs("tools/nv/proofs/select.ts")).href)});`,
      // A program skipped on this host is recorded without a run, which loads the engine and nothing else.
      'const rec = { record() {} };',
      'await recordProgram(rec, "", "", "examples", "docs/examples/core/Math/sqrt/01-square-roots.nvs", undefined);',
    ].join("\n");
    rmSync(LOG, { force: true });
    try {
      const r = await run([process.execPath, "-e", script], { cwd: abs("."), env: { [READS_ENV]: LOG } });
      expect(r.stderr).toBe("");
      const mods = readModules(LOG);
      for (const m of JUDGING) expect(mods).toContain(m);
      for (const m of ENGINE) expect(mods).not.toContain(m);
    } finally {
      rmSync(LOG, { force: true });
    }
  });
});

describe("the order programs are judged and recorded in", () => {
  type P = { what: "examples" | "hostile"; path: string };
  const ex = (path: string): P => ({ what: "examples", path });
  const at = (path: string): P => ({ what: "hostile", path });
  const done = (ms: number): Ran => ({ code: 0, stdout: "", stderr: "", timedOut: false, ms });

  test("known programs go longest first, after every unknown one, an attack before an example, ties by path", () => {
    const ms: Record<string, number> = { a: 10, b: 500, c: 40, d: 40 };
    const order = longestFirst([ex("a"), ex("b"), ex("c"), ex("new-ex"), ex("d"), at("new-at")], (p) => ms[p.path], (p) => (p.what === "hostile" ? 1 : 0));
    expect(order.map((p) => p.path)).toEqual(["new-at", "new-ex", "b", "c", "d", "a"]);
  });

  test("every program is judged before any is recorded, and each pool goes longest first by its own time", async () => {
    const programs = [ex("quick"), at("slow-attack"), ex("slow-to-record"), ex("never-recorded")];
    const took: Record<string, { judged: number; recorded: number }> = {
      quick: { judged: 5, recorded: 50 },
      "slow-attack": { judged: 900, recorded: 1000 },
      "slow-to-record": { judged: 20, recorded: 5000 },
      "never-recorded": { judged: 30, recorded: 0 },
    };
    const events: string[] = [];
    const pass = await judgeThenRecord(
      programs,
      (p) => took[p.path],
      async (ordered) => {
        const results = new Map<string, Result>();
        for (const p of ordered) {
          events.push(`judge ${p.path}`);
          results.set(`${p.what}:${p.path}`, { verdict: "ok", why: "", cached: false, ran: done(took[p.path]!.judged) });
        }
        return { results, width: 3, seconds: 1 };
      },
      async (width, ordered, body) => {
        expect(width).toBe(3);
        await Promise.all(ordered.map(body));
      },
      async (p, result) => {
        expect(result?.ran?.ms).toBe(took[p.path]!.judged);
        events.push(`record ${p.path}`);
      },
    );
    expect(events).toEqual([
      "judge slow-attack",
      "judge never-recorded",
      "judge slow-to-record",
      "judge quick",
      "record never-recorded",
      "record slow-to-record",
      "record slow-attack",
      "record quick",
    ]);
    expect(pass.results.size).toBe(4);
  });

  test("no programs judge and record nothing", async () => {
    const pass = await judgeThenRecord([] as P[], () => undefined, () => Promise.reject(new Error("judged")), () => Promise.reject(new Error("pooled")), () => Promise.reject(new Error("recorded")));
    expect(pass.results.size).toBe(0);
  });
});

describe("the judged run and the recording run", () => {
  const SQRT = "docs/examples/core/Math/sqrt/01-square-roots.nvs";
  const ran = (code: number, stdout: string, timedOut = false): Ran => ({ code, stdout, stderr: "", timedOut, ms: 1000 });

  test("a verdict is taken on the uninstrumented proof profile, built as plain cargo builds it", () => {
    expect(PROOF_BINARY).toBe(`target/proof/${process.platform === "win32" ? "nvs.exe" : "nvs"}`);
    expect(PROOF_BUILD.argv).toEqual(["cargo", "build", "--profile", "proof", "--bin", "nvs"]);
    expect(PROOF_BUILD.path).toBe(PROOF_BINARY);
  });

  test("an unlogged judged run drops the footprint log it would inherit, and a recorded run keeps its own", async () => {
    const before = { log: process.env.NVS_FOOTPRINT_LOG, record: process.env[RECORD_ENV] };
    process.env.NVS_FOOTPRINT_LOG = "inherited.log";
    delete process.env[RECORD_ENV];
    const argv = [process.execPath, "-e", "console.log(process.env.NVS_FOOTPRINT_LOG ?? 'none')"];
    try {
      expect((await spawnProof(argv, SQRT, 60_000)).stdout.trim()).toBe("inherited.log");
      expect((await spawnProof(argv, SQRT, 60_000, { unlogged: true })).stdout.trim()).toBe("none");
      const s = scratch();
      try {
        const own = (await spawnProof(argv, SQRT, 60_000, { dir: s.root, unlogged: true })).stdout.trim();
        expect(own).toBe(join(s.root, `${recordName(SQRT)}.log`));
      } finally {
        s.cleanup();
      }
    } finally {
      if (before.log === undefined) delete process.env.NVS_FOOTPRINT_LOG;
      else process.env.NVS_FOOTPRINT_LOG = before.log;
      if (before.record !== undefined) process.env[RECORD_ENV] = before.record;
    }
  });

  test("a recording run may take many times its program's limit, and its own limit after a judged timeout", () => {
    const attack = "tests/hostile/core/Attributes/all/01-one-roster-read-back-at-many-places.nvs";
    expect(recordingLimitMs("hostile", attack)).toBe(hostileLimitMs(readFileSync(abs(attack), "utf8")) * HANG_FACTOR);
    expect(recordingLimitMs("examples", SQRT, ran(0, "", true))).toBe(60_000);
  });

  test("a recording run diverges on another exit status or other standard output, and never after a judged timeout", () => {
    expect(divergence(ran(0, "a\nb\n"), ran(0, "a\r\nb"))).toBeNull();
    expect(divergence(ran(0, "a\n"), ran(101, "a\n"))).toBe("exit 101 on the recording run, 0 on the judged run");
    expect(divergence(ran(0, "a\nb\n"), ran(0, "a\nc\n"))).toBe("stdout differs from the judged run's at line 2");
    expect(divergence(ran(0, "a\n"), ran(1, "", true))).toBe("the recording run was still running after 1s");
    expect(divergence(ran(1, "", true), ran(0, "x"))).toBeNull();
  });

  test("only a program whose judged run ran out of time is run again alone", () => {
    const results = new Map<string, Result>([
      ["hostile:slow.nvs", { verdict: "fail", why: "still running after 10s", cached: false, ran: ran(1, "", true) }],
      ["hostile:crashed.nvs", { verdict: "fail", why: "crash-shaped exit status -1", cached: false, ran: ran(-1, "") }],
      ["examples:ok.nvs", { verdict: "ok", why: "", cached: false, ran: ran(0, "a\n") }],
      ["examples:cached.nvs", { verdict: "ok", why: "", cached: true }],
    ]);
    expect(timedOut(results)).toEqual(["hostile:slow.nvs"]);
  });

  test("a recording run records on the covws build, and marks or clears the program's divergence", async () => {
    const nvs = covwsNvs();
    if (!existsSync(nvs)) throw new Error(`${nvs} is built by \`bun nv verify\` before this test runs`);
    const store = new SelectStore(":memory:", "test-os");
    const rec = await Recorder.open(store, new Map(), null, "proof-record-test");
    try {
      const out = readFileSync(abs(SQRT.replace(/\.nvs$/, ".out")), "utf8");
      const dir = join(rec.dir, "proofs");
      const diverged = await recordProgram(rec, nvs, dir, "examples", SQRT, { verdict: "ok", why: "", cached: false, ran: ran(0, `${out}extra\n`) });
      expect(diverged).toMatch(/^stdout differs from the judged run's at line /);
      expect([...store.divergences().keys()]).toEqual([proofId(SQRT)]);
      expect(store.atom(proofId(SQRT))?.verdict).toBe("green");
      expect(store.durations("proof").get(proofId(SQRT))?.judged).toBe(1000);
      expect(await recordProgram(rec, nvs, dir, "examples", SQRT, { verdict: "ok", why: "", cached: false, ran: ran(0, out) })).toBeNull();
      expect(store.divergences().size).toBe(0);
      // A program skipped on this host is not run again, and holds its own file.
      expect(await recordProgram(rec, nvs, dir, "examples", SQRT, { verdict: "skip", why: "", cached: false })).toBeNull();
      expect(store.footprint(proofId(SQRT))).toContain(`file:${SQRT}`);
    } finally {
      rec.close();
      store.close();
    }
  }, 240_000);
});
