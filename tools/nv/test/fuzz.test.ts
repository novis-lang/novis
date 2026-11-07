// `bun nv fuzz`: which argument lists it takes, and the `bash` line it runs in the WSL copy.

import { describe, expect, test } from "bun:test";
import { fuzzLine, parsed } from "../cmd/fuzz.ts";

describe("bun nv fuzz", () => {
  test("a target and a number of runs or seconds is taken", () => {
    expect(parsed(["markdown", "--runs", "1000000"])).toEqual({ target: "markdown", limit: "-runs=1000000" });
    expect(parsed(["xml", "--seconds", "300"])).toEqual({ target: "xml", limit: "-max_total_time=300" });
  });

  test("a run with no limit, a bad number or a stray word is refused", () => {
    for (const args of [["markdown"], ["markdown", "--runs"], ["markdown", "--runs", "0"], ["markdown", "--runs", "x"],
      ["markdown", "--runs", "5", "more"], ["markdown", "--for", "5"], ["../x", "--runs", "5"]]) {
      expect(typeof parsed(args)).toBe("string");
    }
  });

  test("the corpus, the seeds, the artifacts and the dictionary are passed as CI passes them", () => {
    const line = fuzzLine("/var/tmp/copy", "request_head", "-runs=9", "", "fuzz/dicts/request_head.dict");
    expect(line).toContain("cd /var/tmp/copy && cargo +nightly fuzz run --target-dir /var/tmp/nvs-target-wsl-fuzz request_head");
    expect(line).toContain("request_head /var/tmp/nvs-fuzz-corpus/request_head -- -runs=9");
    expect(line).toContain("-artifact_prefix=/var/tmp/nvs-fuzz-artifacts/request_head/ -dict=fuzz/dicts/request_head.dict");
    const seeded = fuzzLine("/c", "ast", "-runs=1", "fuzz/seeds/parse", "");
    expect(seeded).toContain("ast /var/tmp/nvs-fuzz-corpus/ast fuzz/seeds/parse -- -runs=1 -artifact_prefix=");
    expect(seeded.startsWith("mkdir -p /var/tmp/nvs-fuzz-corpus/ast /var/tmp/nvs-fuzz-artifacts/ast/ && ")).toBe(true);
  });
});
