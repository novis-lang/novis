import { afterAll, describe, expect, test } from "bun:test";
import { rmSync } from "node:fs";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import { CACHE, ROOT } from "../lib/paths.ts";
import { run } from "../lib/proc.ts";
import { ENV, readLog } from "../lib/reads.ts";
import { LEDGER, LEDGER_WHOLE, ledgerMoved, ledgerSigns, PERF_ANY, perfKey } from "../proofs/ledger.ts";
import { computeChange, query } from "../select/select.ts";
import { SelectStore } from "../select/store.ts";
import { scratch } from "./scratch.ts";

const LOG = join(CACHE, `ledger-reads-${process.pid}.ndjson`);
afterAll(() => rmSync(LOG, { force: true }));

const row = (id: string, ns: number) => JSON.stringify({ id, machine: "m1", impl_hash: "h", ns_per_op: ns });
const LEDGER_TEXT = [row("Core\\A::run", 1), row("Core\\B::run", 2), row("Core\\A::run", 3)].join("\n") + "\n";

describe("the perf ledger is keyed by the features whose rows a reader uses", () => {
  test("a change moves only the features whose rows differ", () => {
    const before = ledgerSigns(LEDGER_TEXT);
    expect(ledgerMoved(before, ledgerSigns(LEDGER_TEXT + row("Core\\B::run", 4) + "\n"))).toEqual([perfKey("Core\\B::run")]);
    expect(ledgerMoved(before, ledgerSigns(LEDGER_TEXT.replace('"ns_per_op":3', '"ns_per_op":5')))).toEqual([perfKey("Core\\A::run")]);
    expect(ledgerMoved(before, ledgerSigns(`# a comment\n\n${LEDGER_TEXT}`))).toEqual([]);
    expect(ledgerMoved(before, ledgerSigns(null))).toEqual([perfKey("Core\\A::run"), perfKey("Core\\B::run")]);
  });

  test("`nv proofs` reads the ledger unrecorded and names a key per feature, and a whole reader names the ledger key", async () => {
    const collect = JSON.stringify(pathToFileURL(join(ROOT, "tools", "nv", "proofs", "collect.ts")).href);
    const reads = JSON.stringify(pathToFileURL(join(ROOT, "tools", "nv", "lib", "reads.ts")).href);
    const probe = async (call: string) => {
      rmSync(LOG, { force: true });
      // The reader is loaded after `install`, the way `main.ts` loads a command, so its file reads are noted.
      const script = [`import { install } from ${reads};`, `install(process.env.${ENV});`, `const { ledgerRecords } = await import(${collect});`, call].join("\n");
      const r = await run([process.execPath, "-e", script], { cwd: ROOT, env: { [ENV]: LOG } });
      expect(r.code).toBe(0);
      return readLog(LOG)!;
    };
    const keyed = await probe("ledgerRecords(true);");
    expect(keyed.files).not.toContain(LEDGER);
    expect(keyed.keys ?? []).not.toContain(LEDGER_WHOLE);
    const whole = await probe("ledgerRecords();");
    expect(whole.files).not.toContain(LEDGER);
    expect(whole.keys).toContain(LEDGER_WHOLE);
  });

  test("a ledger edit selects the checks of the edited feature and whole readers, and moves no path key of the ledger", async () => {
    const t = scratch();
    const git = (...args: string[]) => {
      const r = Bun.spawnSync(["git", ...args], { cwd: t.root, stdout: "pipe", stderr: "pipe" });
      if (r.exitCode !== 0) throw new Error(`git ${args.join(" ")}: ${r.stderr.toString()}`);
      return r.stdout.toString().trim();
    };
    const s = new SelectStore(":memory:", "test-os");
    try {
      git("init", "-q");
      git("config", "user.email", "test@example.com");
      git("config", "user.name", "test");
      git("config", "core.hooksPath", ".no-hooks");
      t.put(LEDGER, LEDGER_TEXT);
      git("add", ".");
      git("commit", "-q", "-m", "start");
      s.setBase(git("rev-parse", "HEAD"));
      const rec = (id: string, keys: string[]) => s.recordRun(id, { def: "", verdict: "green", keys: new Map(keys.map((k) => [k, ""])) });
      rec("nv:proofs-a", [perfKey("Core\\A::run")]);
      rec("nv:proofs-b", [perfKey("Core\\B::run")]);
      rec("nv:report", [LEDGER_WHOLE]);
      rec("nv:stale", [`file:${LEDGER}`]);

      t.put(LEDGER, LEDGER_TEXT + row("Core\\A::run", 9) + "\n");
      const c = await computeChange(s, { paths: [LEDGER], graph: null, root: t.root });
      expect(c.moved.has(`file:${LEDGER}`)).toBe(false);
      expect(c.moved.has(perfKey("Core\\A::run"))).toBe(true);
      expect(c.moved.has(perfKey("Core\\B::run"))).toBe(false);
      expect(c.moved.has(PERF_ANY)).toBe(false);
      const sel = query(s, c);
      const keyed = [...sel.selected.values()].filter((x) => x.why === "key").map((x) => x.id).sort();
      expect(keyed).toEqual(["nv:proofs-a", "nv:report"]);
    } finally {
      s.close();
      t.cleanup();
    }
  }, 180_000);
});
