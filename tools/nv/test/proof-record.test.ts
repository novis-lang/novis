import { describe, expect, test } from "bun:test";
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { abs } from "../lib/paths.ts";
import { RECORD_ENV, proofRecording, recordName, spawnProof } from "../proofs/run.ts";
import { scratch } from "./scratch.ts";

describe("recording a proof program", () => {
  test("a record name is the path with its slashes as `~` and every other unsafe byte escaped", () => {
    expect(recordName("tests/conformance/a b.nvst")).toBe("tests~conformance~a@20b.nvst");
    expect(recordName("tests\\conformance\\a b.nvst")).toBe("tests~conformance~a@20b.nvst");
    expect(recordName("50%.nvs")).toBe("50@25.nvs");
    expect(recordName("é.nvs")).toBe("@c3@a9.nvs");
    expect(recordName("a~b.nvs")).not.toBe(recordName("a/b.nvs"));
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
    const nvs = abs(`target/debug/nvs${process.platform === "win32" ? ".exe" : ""}`);
    if (!existsSync(nvs)) throw new Error(`${nvs} is built by \`cargo build\` before this test runs`);
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
