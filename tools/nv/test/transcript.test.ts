import { describe, expect, test } from "bun:test";
import { TICKER, hms, mmss } from "../driver/console.ts";
import { Tree } from "../driver/proctree.ts";
import { DEFAULT_CAPS, Renderer } from "../driver/transcript.ts";

/** Every line `say` wrote while `fn` ran, without colour. */
function printed(fn: () => void): string[] {
  const out: string[] = [];
  const write = process.stdout.write.bind(process.stdout);
  (process.stdout as any).write = (chunk: string | Uint8Array) => {
    out.push(String(chunk));
    return true;
  };
  try {
    fn();
  } finally {
    (process.stdout as any).write = write;
  }
  return out
    .join("")
    // biome-ignore lint/suspicious/noControlCharactersInRegex: the colour codes are what is removed
    .replace(/\x1b\[[0-9;]*m/g, "")
    .split(/\r?\n/)
    .map((l) => l.replace(/^\r/, ""))
    .filter((l) => l !== "");
}

describe("the transcript", () => {
  test("a tool call shows its name and every input, and its result under it", () => {
    const r = new Renderer(DEFAULT_CAPS);
    const lines = printed(() => {
      r.event({ type: "assistant", message: { id: "m1", content: [{ type: "tool_use", id: "t1", name: "Bash", input: { command: "git status", description: "Show status" } }], usage: { input_tokens: 10, cache_read_input_tokens: 84203, output_tokens: 7 } } });
      r.event({ type: "user", message: { content: [{ type: "tool_result", tool_use_id: "t1", content: "clean" }] } });
    });
    expect(lines).toEqual(["   > Bash", "       command: git status", "       description: Show status", "     < Bash", "     | clean"]);
    expect(r.tokens()).toBe("ctx 84.2k in / 7 out");
  });

  test("a failed call is marked, and a long result says how much it hid", () => {
    const r = new Renderer({ ...DEFAULT_CAPS, maxResultLines: 2 });
    const lines = printed(() => {
      r.event({ type: "assistant", message: { content: [{ type: "tool_use", id: "t2", name: "Read", input: { file_path: "a.ts" } }] } });
      r.event({ type: "user", message: { content: [{ type: "tool_result", tool_use_id: "t2", is_error: true, content: "one\ntwo\nthree" }] } });
    });
    expect(lines.slice(2)).toEqual(["     ! Read failed", "     | one", "     | two", "     | ... 1 more line(s) -- full text in the session log"]);
  });

  test("the result event does not print the final message twice", () => {
    const r = new Renderer(DEFAULT_CAPS);
    const lines = printed(() => {
      r.event({ type: "assistant", message: { content: [{ type: "text", text: "done" }] } });
      r.event({ type: "result", subtype: "success", num_turns: 3, duration_ms: 1500, result: "done" });
    });
    expect(lines).toEqual(["   done", "   [success] 3 turns  1.5s"]);
  });
});

describe("the console's helpers", () => {
  test("durations read the way the old driver wrote them", () => {
    expect(mmss(59)).toBe("59s");
    expect(mmss(123)).toBe("2m03s");
    expect(hms(4 * 3600 + 52 * 60 + 11)).toBe("4h52m");
  });

  test("between sessions the status line's last field is the driver's phase", () => {
    TICKER.set({ phase: "acceptance sweep", total: 58, done: 0 });
    for (let i = 0; i < 31; i++) TICKER.advance();
    expect(TICKER.phase()).toBe("acceptance sweep 31/58");
    TICKER.set({ phase: "usage wall" });
    TICKER.set({ detail: `${hms(723)} left` });
    expect(TICKER.phase()).toBe("usage wall, 12m03s left");
    TICKER.set({ phase: "held" });
    expect(TICKER.phase()).toBe("held");
  });

  test("the status line's clock starts again with a new phase, and not with a new detail", async () => {
    TICKER.set({ phase: "launching" });
    await Bun.sleep(1100);
    TICKER.set({ detail: "Run the guard tests" });
    expect(TICKER.elapsed()).toBe("1s");
    TICKER.set({ phase: "closing the session" });
    expect(TICKER.elapsed()).toBe("0s");
  });
});

describe.if(process.platform === "win32")("the process tree", () => {
  test("a freeze stops the child where it stands, and a thaw lets it finish", async () => {
    const child = Bun.spawn(["cmd", "/c", "ping -n 3 127.0.0.1 >nul"], { stdout: "ignore" });
    const tree = new Tree(child.pid);
    await Bun.sleep(200);
    expect(tree.pids()).toContain(child.pid);
    const began = performance.now();
    expect(tree.freeze()).toBeGreaterThan(0);
    await Bun.sleep(2500);
    tree.thaw();
    await child.exited;
    tree.close();
    // ping -n 3 takes about two seconds; frozen for 2.5 of them, it cannot finish in under four.
    expect(performance.now() - began).toBeGreaterThan(3500);
  });
});
