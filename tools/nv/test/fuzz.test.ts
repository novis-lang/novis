// `bun nv fuzz`: which argument lists it takes, and the `bash` line it runs in the WSL copy.

import { describe, expect, test } from "bun:test";
import { ALL_SECONDS, allTargets, fuzzLine, parsed, savedLine } from "../cmd/fuzz.ts";

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

  test("the corpus, the seeds, the artifacts and the dictionary are passed to the run", () => {
    const line = fuzzLine("/var/tmp/copy", "request_head", "-runs=9", "", "fuzz/dicts/request_head.dict");
    expect(line).toContain("cd /var/tmp/copy && cargo +nightly fuzz run --target-dir /var/tmp/nvs-target-wsl-fuzz request_head");
    expect(line).toContain('request_head /var/tmp/nvs-fuzz-corpus/request_head -- -fork="$j" -runs=9');
    expect(line).toContain("-artifact_prefix=/var/tmp/nvs-fuzz-artifacts/request_head/ -dict=fuzz/dicts/request_head.dict");
    const seeded = fuzzLine("/c", "ast", "-runs=1", "fuzz/seeds/parse", "");
    expect(seeded).toContain('ast /var/tmp/nvs-fuzz-corpus/ast fuzz/seeds/parse -- -fork="$j" -runs=1 ');
    expect(seeded).toContain("mkdir -p /var/tmp/nvs-fuzz-corpus/ast /var/tmp/nvs-fuzz-artifacts/ast/ && ");
  });

  test("the run forks jobs and still stops on a timeout or an out-of-memory input", () => {
    const line = fuzzLine("/c", "markdown", "-runs=1", "", "");
    expect(line.startsWith("j=$(( $(nproc) / 2 )); ")).toBe(true);
    expect(line).toContain("-ignore_timeouts=0 -ignore_ooms=0");
  });

  test("--all is taken alone, and keeps going after a crash, a timeout or an out-of-memory input", () => {
    expect(parsed(["--all"])).toEqual({ all: true });
    expect(typeof parsed(["--all", "--seconds", "5"])).toBe("string");
    expect(typeof parsed(["markdown", "--all"])).toBe("string");
    const line = fuzzLine("/c", "json", "-max_total_time=120", "", "", true);
    expect(line).toContain("-max_total_time=120 -ignore_crashes=1 -ignore_timeouts=1 -ignore_ooms=1 -artifact_prefix=");
    expect(line).not.toContain("-ignore_ooms=0");
  });

  test("--all fuzzes every target file, sorted, but the hand-run ones, each for 120 seconds", () => {
    expect(allTargets(["xml.rs", "ast.rs", "markdown_comrak.rs", "README.md", "json.rs"])).toEqual(["ast", "json", "xml"]);
    expect(allTargets([])).toEqual([]);
    expect(ALL_SECONDS).toBe(120);
  });

  test("the saved inputs are listed with their base64", () => {
    const line = savedLine();
    expect(line).toContain("find /var/tmp/nvs-fuzz-artifacts -type f -newer /var/tmp/nvs-fuzz-all.stamp");
    expect(line).toContain('echo "saved: $f"; base64 -w0 "$f"');
  });

  test("the job count is half the cores, capped by the free GiB, and at least one", () => {
    const jobs = (cores: number, freeKib: number): string => {
      const shell = fuzzLine("/c", "x", "-runs=1", "", "").split("mkdir -p")[0]!
        .replace("$(nproc)", String(cores)).replace(/\$\(awk [^)]*\)/, String(freeKib));
      const out = Bun.spawnSync(["bash", "-c", shell]);
      return out.stdout.toString().trim();
    };
    expect(jobs(16, 64 * 1048576)).toBe("fuzz: 8 job(s)");
    expect(jobs(16, 3 * 1048576 + 5)).toBe("fuzz: 3 job(s)");
    expect(jobs(1, 64 * 1048576)).toBe("fuzz: 1 job(s)");
    expect(jobs(16, 1000)).toBe("fuzz: 1 job(s)");
  });
});
