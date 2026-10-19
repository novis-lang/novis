import { afterEach, describe, expect, spyOn, test } from "bun:test";
import { join } from "node:path";
import { readFileSync } from "node:fs";
import { softWarning } from "../cmd/bench.ts";
import { scratch } from "./scratch.ts";

describe("the warm-start soft bound", () => {
  const saved = { actions: process.env.GITHUB_ACTIONS, summary: process.env.GITHUB_STEP_SUMMARY };
  afterEach(() => {
    for (const [key, value] of [["GITHUB_ACTIONS", saved.actions], ["GITHUB_STEP_SUMMARY", saved.summary]] as const) {
      if (value === undefined) delete process.env[key];
      else process.env[key] = value;
    }
  });

  test("a miss off GitHub Actions is one log line", () => {
    delete process.env.GITHUB_ACTIONS;
    const log = spyOn(console, "log").mockImplementation(() => {});
    softWarning("7.0 ms of work is over the 6.0 ms soft bound");
    expect(log.mock.calls.map((c) => c[0])).toEqual(["  warning: 7.0 ms of work is over the 6.0 ms soft bound"]);
    log.mockRestore();
  });

  test("a miss on GitHub Actions is also an annotation and a step-summary line", () => {
    const s = scratch();
    const summary = join(s.root, "summary.md");
    process.env.GITHUB_ACTIONS = "true";
    process.env.GITHUB_STEP_SUMMARY = summary;
    const log = spyOn(console, "log").mockImplementation(() => {});
    softWarning("7.0 ms of work is over the 6.0 ms soft bound");
    expect(log.mock.calls.map((c) => c[0])).toEqual([
      "  warning: 7.0 ms of work is over the 6.0 ms soft bound",
      "::warning title=warm start::7.0 ms of work is over the 6.0 ms soft bound",
    ]);
    log.mockRestore();
    expect(readFileSync(summary, "utf8")).toBe("- warm start: 7.0 ms of work is over the 6.0 ms soft bound\n");
    s.cleanup();
  });
});
