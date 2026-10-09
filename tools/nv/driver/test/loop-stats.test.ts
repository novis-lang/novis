// `bun nv loop-stats` over one session's stream-json log: the guard's denials are counted by the rule
// that denied them.

import { describe, expect, test } from "bun:test";
import { join } from "node:path";
import { readSession } from "../../cmd/loop-stats.ts";
import { scratch } from "../../test/scratch.ts";

describe("loop-stats reads a session", () => {
  test("loop-stats --guard counts a denied call under the rule that denied it", () => {
    const t = scratch();
    try {
      const events = [
        { type: "assistant", message: { id: "m1", content: [{ type: "tool_use", id: "t1", name: "Bash", input: { command: "cat a.rs" } }] } },
        { type: "user", message: { content: [{ type: "tool_result", tool_use_id: "t1", is_error: true, content: "PreToolUse:Bash hook error: guard: whole-read: use nv peek" }] } },
        { type: "assistant", message: { id: "m2", content: [{ type: "tool_use", id: "t2", name: "Bash", input: { command: "ls" } }] } },
        { type: "user", message: { content: [{ type: "tool_result", tool_use_id: "t2", content: "a.rs" }] } },
      ];
      t.put("session.jsonl", events.map((e) => JSON.stringify(e)).join("\n"));
      expect(readSession(join(t.root, "session.jsonl"))!.guard).toEqual({ "whole-read": 1 });
    } finally {
      t.cleanup();
    }
  });
});
