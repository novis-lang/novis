import { describe, expect, test } from "bun:test";
import { existsSync, readFileSync, rmSync } from "node:fs";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import { covwsNvs } from "../lib/covws.ts";
import { abs } from "../lib/paths.ts";
import { run } from "../lib/proc.ts";
import { ENV as READS_ENV, readModules } from "../lib/reads.ts";
import { comparableRun, divergence, HANG_FACTOR, hostileLimitMs, PROOF_BINARY, PROOF_BUILD, RECORD_ENV, type Ran, proofRecording, recordingLimitMs, recordName, type Result, composeServices, noDockerReason, requiredServices, serviceSkipReason, servicesFor, spawnProof, stoppedBy, timedOut } from "../proofs/run.ts";
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
  // The selection engine, the crate graph, the recorder and the acceptance sweep, which decide what runs
  // and what the store remembers.
  const ENGINE = ["tools/nv/select/select.ts", "tools/nv/select/items.ts", "tools/nv/select/change.ts", "tools/nv/select/record.ts", "tools/nv/select/extract.ts", "tools/nv/select/atoms.ts", "tools/nv/keys/graph.ts", "tools/nv/accept/accept.ts", "tools/nv/accept/runner.ts"];

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

  test("a program without a `requires:` service marker needs no service and never touches Docker", async () => {
    expect(await servicesFor("<?nvs\n// requires: unix\necho 1;\n")).toBeNull();
    expect(await servicesFor("<?nvs\n// Shows a samba-ad search.\necho 1;\n")).toBeNull();
  });

  test("`requires:` names any compose service, and a name the compose file does not declare fails the program", async () => {
    expect(composeServices().has("redis")).toBe(true);
    expect(composeServices().has("samba-ad")).toBe(true);
    expect(requiredServices("<?nvs\n// requires: unix\n// requires: redis\n// requires: samba-ad\n// requires: redis\n")).toEqual(["redis", "samba-ad"]);
    expect(requiredServices("<?nvs\n// requires: unimplemented\n")).toEqual([]);
    // The name is checked before Docker is reached, so this needs no Docker.
    const why = await servicesFor("<?nvs\n// requires: redis\n// requires: no-such-store\necho 1;\n", new Set(["redis", "samba-ad"]));
    expect(why).toContain("`requires: no-such-store`");
    expect(why).toContain("no service in tests/db/compose.yaml");
  });

  test("a `requires:` service program is skipped wherever Docker runs no Linux containers, a Docker that does not answer included", () => {
    expect(noDockerReason(null)).toBe("this host has no Docker");
    expect(noDockerReason({ code: 0, stdout: "windows\n", stderr: "" })).toContain("runs windows containers");
    expect(noDockerReason({ code: 0, stdout: "linux\n", stderr: "" })).toBeNull();
    const down = noDockerReason({ code: 1, stdout: "", stderr: "Client:\nerror during connect: the daemon is not running\n" });
    expect(down).toBe("this host's Docker is not answering (`docker info` exit 1: error during connect: the daemon is not running)");
    expect(noDockerReason({ code: 124, stdout: "", stderr: "" })).toBe("this host's Docker is not answering (`docker info` exit 124)");
  });

  test("a program without a service, or with one the compose file does not declare, is never a service skip", async () => {
    expect(await serviceSkipReason("<?nvs\necho 1;\n")).toBeNull();
    expect(await serviceSkipReason("<?nvs\n// requires: no-such-store\necho 1;\n", new Set(["redis"]))).toBeNull();
  });

  test("what stopped a run is the first line of its standard error that is not a warning", () => {
    expect(stoppedBy("warning: no queue worker started: x\n{\"level\":\"error\"}\n")).toBe("{\"level\":\"error\"}");
    expect(stoppedBy("\nwarning[W1023]: this condition\n")).toBe("warning[W1023]: this condition");
    expect(stoppedBy("warning: only this\n")).toBe("warning: only this");
    expect(stoppedBy("")).toBe("");
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

  test("a `proof: test` example's two reports are compared without their timings, and any other program's whole", () => {
    const flaky = "docs/examples/lang/testing/test-options/04-a-test-reported-as-flaky.nvs";
    const report = (ms: number) => `  T\n    ✓ a ${ms} ms\n\n  0 failed, 1 passed, 0 skipped, 0 flaky in ${ms} ms\n`;
    const judged = comparableRun("examples", flaky, ran(0, report(3)));
    expect(judged.stdout).toBe("  T\n    ✓ a\n\n  0 failed, 1 passed, 0 skipped, 0 flaky\n");
    expect(divergence(judged, comparableRun("examples", flaky, ran(0, report(41))))).toBeNull();
    expect(comparableRun("examples", SQRT, ran(0, report(3))).stdout).toBe(report(3));
  });

  test("with an elided `.out`, a recording run agrees when it matches the `.out`, whatever the judged run printed", () => {
    const out = "Build\n...\nHost\n";
    expect(divergence(ran(0, "Build\n  profile proof\nHost\n"), ran(0, "Build\n  profile debug\nHost\n"), out)).toBeNull();
    expect(divergence(ran(0, "Build\nHost\n"), ran(0, "Build\n"), out)).toBe("stdout on the recording run differs from the elided .out");
  });

  test("each run's own data folder is one placeholder, so only a difference elsewhere diverges", () => {
    const judgedExe = abs("target/proof/nvs.exe");
    const recordedExe = abs("target/covws/x86_64-pc-windows-msvc/debug/nvs.exe");
    const at = (exe: string) => join(exe, "..", ".nvsdata");
    const judged: Ran = { ...ran(0, `start\ndata: ${at(judgedExe)}\n`), exe: judgedExe };
    const recorded: Ran = { ...ran(0, `start\ndata: ${at(recordedExe)}\n`), exe: recordedExe };
    expect(divergence(judged, recorded)).toBeNull();
    const slashed: Ran = { ...recorded, stdout: recorded.stdout.replaceAll("\\", "/") };
    expect(divergence(judged, slashed)).toBeNull();
    if (process.platform === "win32") {
      const lower: Ran = { ...recorded, stdout: recorded.stdout.replace(/[A-Z]:/, (d) => d.toLowerCase()) };
      expect(divergence(judged, lower)).toBeNull();
    }
    const other: Ran = { ...recorded, stdout: `${recorded.stdout}extra\n` };
    expect(divergence(judged, other)).toBe("stdout differs from the judged run's at line 3");
    // The judged run printing the recording binary's folder is a real difference.
    const crossed: Ran = { ...judged, stdout: recorded.stdout };
    expect(divergence(crossed, recorded)).toBe("stdout differs from the judged run's at line 2");
  });

  test("a request's log lines are compared without their time and request id, and with every other field", () => {
    const line = (ts: string, id: string, msg: string) => `{"ts":"${ts}","level":"warn","msg":"${msg}","request_id":"${id}"}\n`;
    const judged = ran(0, line("2026-10-26T22:11:58.7316624Z", "a9e261a2ff88adb58b52867e57ea312d", "x"));
    expect(divergence(judged, ran(0, line("2026-10-26T22:11:59.1Z", "2b440d6f03c6ffcd710811a2bf52cad4", "x")))).toBeNull();
    expect(divergence(judged, ran(0, line("2026-10-26T22:11:59.1Z", "2b440d6f03c6ffcd710811a2bf52cad4", "y")))).toBe("stdout differs from the judged run's at line 1");
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
      expect(diverged?.why).toMatch(/^stdout differs from the judged run's at line /);
      expect([...store.divergences().keys()]).toEqual([proofId(SQRT)]);
      expect(store.atom(proofId(SQRT))?.verdict).toBe("green");
      expect(store.durations("proof").get(proofId(SQRT))?.judged).toBe(1000);
      expect((await recordProgram(rec, nvs, dir, "examples", SQRT, { verdict: "ok", why: "", cached: false, ran: ran(0, out) }))?.why).toBeNull();
      expect(store.divergences().size).toBe(0);
      // A program skipped on this host is not run again, and holds its own file.
      expect(await recordProgram(rec, nvs, dir, "examples", SQRT, { verdict: "skip", why: "", cached: false })).toEqual({ verdict: "green", why: null, ext: null });
      expect(store.footprint(proofId(SQRT))).toContain(`file:${SQRT}`);
    } finally {
      rec.close();
      store.close();
    }
  }, 240_000);

  test("a program whose file is gone by its recording run is neither recorded nor marked", async () => {
    const store = new SelectStore(":memory:", "test-os");
    const rec = await Recorder.open(store, new Map(), null, "proof-record-test");
    const gone = "docs/examples/core/Math/sqrt/99-renamed-after-it-was-judged.nvs";
    try {
      const dir = join(rec.dir, "proofs");
      expect(await recordProgram(rec, "no-such-nvs", dir, "examples", gone, { verdict: "ok", why: "", cached: false, ran: ran(0, "a\n") })).toBeNull();
      expect(store.atom(proofId(gone))).toBeNull();
      expect(store.verdictsWithPrefix("diverged:").size).toBe(0);
    } finally {
      rec.close();
      store.close();
    }
  });
});
