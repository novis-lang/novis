import { describe, expect, test } from "bun:test";
import { existsSync, readFileSync, rmSync } from "node:fs";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import { covwsNvs } from "../lib/covws.ts";
import { abs } from "../lib/paths.ts";
import { run } from "../lib/proc.ts";
import { ENV as READS_ENV, readModules } from "../lib/reads.ts";
import { divergence, HANG_FACTOR, hostileLimitMs, PROOF_BINARY, PROOF_BUILD, RECORD_ENV, type Ran, proofRecording, recordingLimitMs, recordName, spawnProof } from "../proofs/run.ts";
import { recordProgram } from "../proofs/select.ts";
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
