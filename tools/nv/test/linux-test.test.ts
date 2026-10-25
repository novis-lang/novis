// `bun nv linux-test`: which argument lists it takes, and the `bash` line it runs in the WSL copy.

import { describe, expect, test } from "bun:test";
import { refusal, testLine } from "../cmd/linux-test.ts";

describe("bun nv linux-test", () => {
  test("a run narrowed by --test, --lib or a filter is taken", () => {
    expect(refusal(["--test", "env_context", "a_queued_job"])).toBe("");
    expect(refusal(["--lib", "io::tests::within"])).toBe("");
    expect(refusal(["sends_to_a_peer"])).toBe("");
  });

  test("-p and --package are refused, as every debug cargo command refuses them", () => {
    for (const args of [["-p", "nvs-cli"], ["-pnvs-cli"], ["--package", "nvs-cli"], ["--package=nvs-cli"]]) {
      expect(refusal(args)).toContain("never takes -p");
    }
  });

  test("a run of the whole suite is refused", () => {
    expect(refusal(["--no-fail-fast"])).toContain("the whole suite is CI's");
    expect(refusal(["--", "--nocapture"])).toContain("the whole suite is CI's");
  });

  test("the line runs cargo test in the copy, into the WSL target directory, with each argument quoted", () => {
    expect(testLine("/var/tmp/nvs-target-wsl-src/mnt-e-mwl", ["--test", "env_context", "a b"])).toBe(
      "cd /var/tmp/nvs-target-wsl-src/mnt-e-mwl && CARGO_TARGET_DIR=/var/tmp/nvs-target-wsl cargo test --test env_context 'a b'",
    );
  });
});
