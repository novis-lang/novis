import { afterEach, describe, expect, test } from "bun:test";
import { existsSync } from "node:fs";
import { join } from "node:path";
import { dapProblems, snippetDap } from "../cmd/site.ts";
import { parseDap } from "../lib/dap-replay.ts";
import { ROOT } from "../lib/paths.ts";
import { scratch, type Scratch } from "./scratch.ts";

// The real adapter, so a recording here is what `nvs dap` returns: the build the acceptance runner names
// when it is built, or the debug build `nv verify` makes.
const NVS = [process.env.NVS_BIN, join(ROOT, `target/debug/nvs${process.platform === "win32" ? ".exe" : ""}`)].find((p) => p !== undefined && existsSync(p)) ?? "nvs";

const frames = (await import(join(ROOT, "website/config/replay-frames.mjs"))) as {
  dapOf(files: { dap: string; recording: string; text: string; title: string; where: string }): {
    frames: { panel: { ask: number; line: number; col: number } | null }[];
    asks: { kind: string }[];
    steps: string[];
    poster: number;
    breaks: number[];
  };
  dapMarkup(files: { dap: string; recording: string; text: string; title: string; install: string; where: string }): string;
};

let s: Scratch;
afterEach(() => s?.cleanup());

const P = "website/snippets/tools/debugger/01-shop.nvs";

/** A scratch site with the snippet `text` at `P`, its `.dap`, and its `.dap.out` when given. */
function site(text: string, dap: string, out?: string): Scratch {
  s?.cleanup();
  s = scratch();
  s.put(P, text);
  s.put(P.replace(/\.nvs$/, ".dap"), dap);
  if (out !== undefined) s.put(P.replace(/\.nvs$/, ".dap.out"), out);
  return s;
}

const SHOP = "<?nvs\nclass Shop {\n    public static function total(int $price, int $count): int {\n        int $sum = $price * $count;\n        int $withTax = $sum + 2;\n        return $withTax;\n    }\n}\nint $order = Shop::total(4, 3);\necho $order, \"\\n\";\n";
const SHOP_DAP = "break 9\nin\nnext\nout\ncontinue\n";
const SHOP_OUT = [
  "stop 0 breakpoint\n  frame 01-shop.nvs:9 <script>\n",
  "stop 1 step\n  frame 01-shop.nvs:4 Shop::total\n  frame 01-shop.nvs:9 <script>\n  var $count\tint\t3\n  var $price\tint\t4\n",
  "stop 2 step\n  frame 01-shop.nvs:5 Shop::total\n  frame 01-shop.nvs:9 <script>\n  var $count\tint\t3\n  var $price\tint\t4\n  var $sum\tint\t12\n",
  "stop 3 step\n  frame 01-shop.nvs:10 <script>\n  var Shop::total() returned\tint\t14\n  var $order\tint\t14\n",
  "end 4 0\n",
].join("\n");

describe("a .dap", () => {
  test("is the breakpoints and then one step per line", () => {
    expect(parseDap("break 9\nbreak 10\n\nin\nnext\nout\ncontinue\n")).toEqual({ breaks: [9, 10], steps: ["in", "next", "out", "continue"] });
    expect(parseDap("in\n")).toBe("no `break` line");
    expect(parseDap("break 9\nnext\nbreak 10\n")).toBe("line 3: a `break` after a step; list every breakpoint first");
    expect(parseDap("break 9\njump\n")).toStartWith("line 2: `jump` is not `break LINE` or a step");
  });

  test("records each stop's call stack and variables, and the program's end", async () => {
    site(SHOP, SHOP_DAP);
    const got = await snippetDap(P, NVS, s.root);
    expect(got).toEqual({ recording: SHOP_OUT });
  }, 60_000);

  test("a fresh recording passes the check, and a stale one is named", async () => {
    site(SHOP, SHOP_DAP, SHOP_OUT);
    expect(await dapProblems(P, NVS, s.root)).toEqual([]);
    site(SHOP, SHOP_DAP, SHOP_OUT.replace("var $sum\tint\t12", "var $sum\tint\t13"));
    expect(await dapProblems(P, NVS, s.root)).toEqual([`${P}: its .dap.out is not what \`nvs dap\` returns for its .dap now`]);
  }, 60_000);

  test("a missing recording, one with no .dap, and a step after the end are named", async () => {
    site(SHOP, SHOP_DAP);
    expect(await dapProblems(P, NVS, s.root)).toEqual([`${P}: no .dap.out beside its .dap`]);
    site(SHOP, `${SHOP_DAP}next\n`, SHOP_OUT);
    expect((await dapProblems(P, NVS, s.root))[0]).toStartWith(`${P.replace(/\.nvs$/, ".dap")}: step 5: \`next\` after the program ended`);
    s.cleanup();
    s = scratch();
    s.put(P, SHOP);
    s.put(P.replace(/\.nvs$/, ".dap.out"), SHOP_OUT);
    expect(await dapProblems(P, NVS, s.root)).toEqual([`${P}: a .dap.out beside it and no .dap`]);
  }, 60_000);

  test("plays one frame per stop, with the breakpoint, the stopped line and the panel", () => {
    const data = frames.dapOf({ dap: SHOP_DAP, recording: SHOP_OUT, text: SHOP, title: "01-shop.nvs", where: P });
    expect(data.breaks).toEqual([8]);
    expect(data.frames.map((f) => f.panel && [f.panel.line, f.panel.col])).toEqual([null, [8, 0], [3, 8], [4, 8], [9, 0], [9, 0]]);
    expect(data.asks.map((a) => a.kind)).toEqual(["stop", "stop", "stop", "stop", "end"]);
    expect(data.poster).toBe(4);
    expect(data.steps[0]).toBe("Set a breakpoint on line 9 and start debugging. The program stops on line 9.");
    expect(data.steps[1]).toBe("Press F11 to step into the call. The program stops on line 4 in `Shop::total`. `$count` is `3` and `$price` is `4`.");
    expect(data.steps[4]).toBe("Press F5 to continue. The program runs to its end. It ends with exit code `0`.");
    const html = frames.dapMarkup({ dap: SHOP_DAP, recording: SHOP_OUT, text: SHOP, title: "01-shop.nvs", install: "/install/editor/", where: P });
    expect(html).toContain('<div class="nv-line nv-break"><span class="nv-ln">9</span>');
    expect(html).toContain("How it looks in your debugger");
    expect(() => frames.dapOf({ dap: `${SHOP_DAP}next\n`, recording: SHOP_OUT, text: SHOP, title: "01-shop.nvs", where: P })).toThrow("step 5: the program has already ended");
    expect(() => frames.dapOf({ dap: SHOP_DAP, recording: SHOP_OUT, text: SHOP, title: "other.nvs", where: P })).toThrow("the program stops in `01-shop.nvs`");
  });
});
