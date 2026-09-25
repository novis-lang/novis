import { describe, expect, test } from "bun:test";
import { checkOf } from "../cmd/loop.ts";
import { claudeArgs } from "../driver/launch.ts";
import { apiErrorStatus, nthWait, OVERLOAD_BACKOFF, RESUME_BACKOFF, streamDropped } from "../driver/sweep.ts";

describe("the exits that are not crashes", () => {
  test("an overload is the terminal event's status, never a number in its text", () => {
    expect(apiErrorStatus({ type: "result", is_error: true, api_error_status: 529 })).toBe(529);
    expect(apiErrorStatus({ type: "result", is_error: true, result: "the API said 529" })).toBe(0);
    expect(apiErrorStatus(null)).toBe(0);
  });

  test("a dropped stream is an error ended on api_error with no status, and a refusal is not one", () => {
    expect(streamDropped({ type: "result", is_error: true, terminal_reason: "api_error", api_error_status: null })).toBe(true);
    expect(streamDropped({ type: "result", is_error: true, terminal_reason: "api_error", api_error_status: 529 })).toBe(false);
    expect(streamDropped({ type: "result", is_error: false, terminal_reason: "api_error" })).toBe(false);
    expect(streamDropped({ type: "result", is_error: true, terminal_reason: "max_turns" })).toBe(false);
    expect(streamDropped(null)).toBe(false);
  });

  test("a backoff table's last entry repeats", () => {
    expect([1, 2, 3, 4, 5, 9].map((n) => nthWait(OVERLOAD_BACKOFF, n))).toEqual([60, 120, 300, 600, 600, 600]);
    expect([0, 1, 3, 4].map((n) => nthWait(RESUME_BACKOFF, n))).toEqual([15, 15, 60, 60]);
  });

  test("a rejoined session's command line carries --resume and its id", () => {
    const args = claudeArgs({ model: "opus", permissionMode: "bypassPermissions", resume: "abc-123" });
    expect(args.slice(args.indexOf("--resume"), args.indexOf("--resume") + 2)).toEqual(["--resume", "abc-123"]);
    expect(claudeArgs({ model: "opus", permissionMode: "bypassPermissions" })).not.toContain("--resume");
  });
});

describe("checkOf", () => {
  test("two sweeps red on one check compare equal whatever the detail after the label", () => {
    expect(checkOf("the parser reads a file [3 the parser]: exit 1 -- no such test")).toBe("the parser reads a file [3 the parser]");
    expect(checkOf("the parser reads a file [3 the parser]: timed out")).toBe("the parser reads a file [3 the parser]");
    expect(checkOf("the native build failed -- error[E0425]\n  also red: x")).toBe("the native build failed -- error[E0425]");
  });
});
