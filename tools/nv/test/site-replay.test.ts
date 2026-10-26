import { afterEach, describe, expect, test } from "bun:test";
import { existsSync } from "node:fs";
import { join } from "node:path";
import { replayProblems, snippetReplay } from "../cmd/site.ts";
import { ROOT } from "../lib/paths.ts";
import { parseReplay, replaySession } from "../lib/replay.ts";
import { scratch, type Scratch } from "./scratch.ts";

// The real server, so a recording here is what `nvs lsp` returns: the build the acceptance runner names
// when it is built, or the debug build `nv verify` makes.
const NVS = [process.env.NVS_BIN, join(ROOT, `target/debug/nvs${process.platform === "win32" ? ".exe" : ""}`)].find((p) => p !== undefined && existsSync(p)) ?? "nvs";

let s: Scratch;
afterEach(() => s?.cleanup());

const P = "website/snippets/guides/a/01-key.nvs";

/** A scratch site with the snippet `text` at `P`, its `.replay`, and its `.replay.out` when given. */
function site(text: string, replay: string, out?: string): Scratch {
  s?.cleanup();
  s = scratch();
  s.put(P, text);
  s.put(P.replace(/\.nvs$/, ".replay"), replay);
  if (out !== undefined) s.put(P.replace(/\.nvs$/, ".replay.out"), out);
  return s;
}

const CLASSES = "<?nvs\nclass Person {\n    public string $name = \"ada\";\n    protected string $note = \"kept\";\n}\nclass User extends Person {\n    public int $age = 3;\n    private string $secret = \"hidden\";\n}\n\n";
const KEY = `${CLASSES}property<User> $key = 'name' as property<User>;\n`;
const KEY_REPLAY = `--START--\n${CLASSES}property<User> $key = '<|>' as property<User>;\n--STEPS--\ntype na\naccept name\n`;
const KEY_OUT = "completion 1 11:25\nage\tproperty\tint\nname\tproperty\tstring\n";

// One fixture per step that is not a list or a diagnostic: the snippet, its `.replay` and its `.replay.out`.
const ACTION = "<?nvs\nint $n = 3;\nint $m = $n as int;\n";
const ACTION_REPLAY = "--START--\n<?nvs\nint $n = 3;\nint $m = (i<|>nt)$n;\n--STEPS--\naction rewrite as `$n as int`\n";
const ACTION_OUT = 'action 1 3:12\n  rewrite as `$n as int`\nedit 3:10-3:17 "$n as int"\n';
const CART = "<?nvs\nclass Cart\n{\n    public function total(): int\n    {\n        var $sum = 2;\n        return $sum;\n    }\n}\n";
const RENAME = CART.replaceAll("$sum", "$total");
const RENAME_REPLAY = `--START--\n${CART.replace("return $sum", "return $s<|>um")}--STEPS--\nrename total\n`;
const RENAME_OUT = 'rename 1 7:18 0\nedit 6:13-6:17 "$total"\nedit 7:16-7:20 "$total"\n';
const LENS = "<?nvs\nclass Cart {\n    private int $items;\n}\nvar $a = new Cart();\nvar $b = new Cart();\n";
const LENS_REPLAY = `--START--\n${LENS.replace("$b = new Cart", "$b = new C<|>art")}--STEPS--\nlens\n`;
const LENS_OUT = "lens 1\n  2:7 2 references\n  3:17 no references\n";
const CALLS = "<?nvs\nclass Cart\n{\n    public function total(): int\n    {\n        return 2;\n    }\n\n    public function report(): int\n    {\n        return $this->total();\n    }\n}\n";
const CALLS_REPLAY = `--START--\n${CALLS.replace("function total", "function to<|>tal")}--STEPS--\ncalls\n`;
const CALLS_OUT = "calls 1 4:23 Cart::total\n  in Cart::report 1\n";
const CARD = '<?nvs\necho Core\\Str::length("abc");\n';
const CARD_REPLAY = `--START--\n${CARD.replace("::length", "::len<|>gth")}--STEPS--\ncard\n`;
const CARD_OUT = [
  "card 1 2:19",
  "  ```nvs",
  "  Core\\Str::length(string $s): uint",
  "  ```",
  "",
  "  Counts the graphemes in `$s`. A grapheme is what a person counts as one character, so a letter with a combining accent counts once. Every `Core\\Str` function counts in graphemes. The result is never a byte count.",
  "",
  "  **Parameters**",
  "",
  "  - `$s` — The string to measure.",
  "",
  "  **Returns** The number of graphemes. The empty string gives `0`.",
  "",
].join("\n");

describe("a .replay", () => {
  test("is two sections, with the cursor once in the start text and one step per line", () => {
    expect(parseReplay("--START--\n<?nvs\necho <|>;\n--STEPS--\ntype a\\nb\\\\\naccept name\n\nhover\n")).toEqual({
      start: "<?nvs\necho ;\n",
      cursor: 11,
      steps: [{ kind: "type", text: "a\nb\\" }, { kind: "accept", label: "name" }, { kind: "hover" }],
    });
    expect(parseReplay("<?nvs\n")).toBe("the first line is not `--START--`");
    expect(parseReplay("--START--\n<?nvs\n--STEPS--\nhover\n")).toBe("the start text has `<|>` nowhere");
    expect(parseReplay("--START--\n<|><|>\n--STEPS--\nhover\n")).toBe("the start text has `<|>` more than once");
    expect(parseReplay("--START--\n<|>\n--STEPS--\npress Enter\n")).toBe("line 4: `press Enter` is not a step: `type TEXT`, `accept LABEL`, `rename NAME`, `action TITLE`, `hover`, `card`, `lens`, `calls`");
    expect(parseReplay("--START--\n<|>\n--STEPS--\ntype \\t\n")).toBe("line 4: only `\\n` and `\\\\` are escapes");
  });

  test("has a rename, an action, a lens, the calls and a hover card as steps", () => {
    expect(parseReplay("--START--\n<|>\n--STEPS--\nrename total\naction rewrite as `$n as int`\nlens\ncalls\ncard\n")).toEqual({
      start: "\n",
      cursor: 0,
      steps: [{ kind: "rename", name: "total" }, { kind: "action", title: "rewrite as `$n as int`" }, { kind: "lens" }, { kind: "calls" }, { kind: "card" }],
    });
    expect(parseReplay("--START--\n<|>\n--STEPS--\nrename two words\n")).toStartWith("line 4: `rename two words` is not a step");
  });

  test("plays each new step's answer as frames, and makes a rename's or an action's edits", async () => {
    const { replayOf, panelHtml, codeHtml, frameLenses } = (await import(join(ROOT, "website/config/replay-frames.mjs"))) as {
      replayOf(o: { replay: string; recording: string; where: string }): {
        asks: { kind: string }[];
        frames: { edit: [number, number, string] | null; panel: { ask: number; done?: boolean } | null }[];
        poster: number;
        steps: string[];
        last: string;
      };
      panelHtml(ask: object, panel: object): string;
      codeHtml(text: string, cursor: number | null, squiggle: null, colours: null, lenses: object[]): string;
      frameLenses(asks: object[], panel: object | null): object[];
    };
    const action = replayOf({ replay: ACTION_REPLAY, recording: ACTION_OUT, where: "fixture" });
    expect(action.last).toBe(ACTION);
    expect(action.frames.map((f) => [f.edit, f.panel === null])).toEqual([
      [null, true],
      [null, false],
      [[27, 7, "$n as int"], true],
    ]);
    expect(action.poster).toBe(1);
    expect(panelHtml(action.asks[0]!, action.frames[1]!.panel!)).toContain('<li class="nv-row nv-selected"><span class="nv-label">rewrite as `$n as int`</span></li>');
    expect(action.steps).toEqual(["Press Ctrl+. to show the code actions here. The menu has \"rewrite as `$n as int`\". Choose \"rewrite as `$n as int`\", and the editor changes the code."]);

    const rename = replayOf({ replay: RENAME_REPLAY, recording: RENAME_OUT.replace("rename 1 7:18 0", "rename 1 7:18 2"), where: "fixture" });
    expect(rename.last).toBe(RENAME);
    expect(rename.steps).toEqual(["Press F2 and type `total`. The editor renames `$sum` in 2 places in this file and in 2 other files."]);
    expect(panelHtml(rename.asks[0]!, rename.frames[2]!.panel!)).toContain("Also renamed in 2 other files.");

    const lens = replayOf({ replay: LENS_REPLAY, recording: LENS_OUT, where: "fixture" });
    expect(lens.steps).toEqual(["The editor shows 2 code lenses, short notes above the code: \"2 references\" and \"no references\"."]);
    const lensed = codeHtml(LENS, null, null, null, frameLenses(lens.asks, lens.frames[1]!.panel));
    expect(lensed).toContain('<div class="nv-line nv-lens"><span class="nv-ln"></span><span class="nv-text">    no references</span></div>');

    const calls = replayOf({ replay: CALLS_REPLAY, recording: CALLS_OUT, where: "fixture" });
    expect(calls.steps).toEqual(["Open the call hierarchy of `Cart::total`. It is called by `Cart::report`. It calls no other method."]);

    const card = replayOf({ replay: CARD_REPLAY, recording: CARD_OUT, where: "fixture" });
    expect(card.steps).toEqual(["Point at the code. A card shows `Core\\Str::length(string $s): uint` and its documentation."]);
    expect(panelHtml(card.asks[0]!, card.frames[1]!.panel!)).toContain("<p><strong>Parameters</strong></p><ul><li><code>$s</code> — The string to measure.</li></ul>");

    expect(() => replayOf({ replay: ACTION_REPLAY.replace("action rewrite", "action change"), recording: ACTION_OUT, where: "fixture" })).toThrow(
      'fixture: step 1: the code actions are "rewrite as `$n as int`", not "change as `$n as int`"',
    );
  });

  test("is coloured from the runs it is given, and the cursor and the underline nest between them", async () => {
    type Paint = { styles: string[]; lines: [number, number][][]; frames: number[][] };
    const { codeHtml, replayPaint, frameColours } = (await import(join(ROOT, "website/config/replay-frames.mjs"))) as {
      codeHtml(text: string, cursor: number | null, squiggle: object | null, colours: object | null): string;
      replayPaint(texts: string[], colours: (text: string) => [string, string][][], where: string): Paint;
      frameColours(paint: Paint, index: number): object;
    };
    const runs: Record<string, [string, string][][]> = {
      "echo $ab;\n": [[["echo", "--0:#a"], [" ", ""], ["$ab", "--0:#b"], [";", ""]]],
      "echo $abc;\n": [[["echo", "--0:#a"], [" ", ""], ["$abc", "--0:#b"], [";", ""]]],
    };
    const paint = replayPaint(Object.keys(runs), (text) => runs[text]!, "fixture");
    expect(paint.styles).toEqual(["", "--0:#a", "--0:#b"]);
    expect(paint.frames).toEqual([[0, 1], [2, 1]]);
    expect(codeHtml("echo $ab;\n", 7, { line: 0, col: 5, length: 3 }, frameColours(paint, 0))).toBe(
      '<div class="nv-line"><span class="nv-ln">1</span><span class="nv-text"><span style="--0:#a">echo</span> ' +
        '<span class="nv-squiggle"><span style="--0:#b">$a</span><span class="nv-caret"></span><span style="--0:#b">b</span></span>;</span></div>',
    );
    expect(() => replayPaint(["echo;\n"], () => [[["echo", ""]]], "fixture")).toThrow("fixture: the colours of line 1 are not its text `echo;`");
  });

  test("asks for a list after a trigger character and at a word's first letter, and not inside a word", () => {
    const replay = parseReplay("--START--\n<?nvs\n<|>\n--STEPS--\ntype $ab cd\n");
    if (typeof replay === "string") throw new Error(replay);
    const asks = replaySession("file:///a.nvs", replay, { triggers: ["$"], inserts: {}, edits: {}, items: {} }).asks;
    expect(asks.map((a) => [a.line + 1, a.character + 1])).toEqual([
      [2, 2],
      [2, 6],
    ]);
  });
});

describe.skipIf(!existsSync(NVS))("nv site --check snippets: a replay against `nvs lsp`", () => {
  test("a completion step records the list the server returned", async () => {
    site(KEY, KEY_REPLAY, KEY_OUT);
    expect(await snippetReplay(P, NVS, s.root)).toEqual({ recording: KEY_OUT, last: KEY });
    expect(await replayProblems(P, NVS, s.root)).toEqual([]);
  }, 60_000);

  test("a diagnostic step records the error the server published", async () => {
    site("<?nvs\necho $nope;\n", "--START--\n<?nvs\necho <|>;\n--STEPS--\ntype $nope\nhover\n");
    const got = await snippetReplay(P, NVS, s.root);
    // `$` is a trigger character, so the list it opens is recorded too, and it is empty.
    expect(got).toEqual({ recording: "completion 1 2:7\n\ndiagnostic 2 2:6+5 E0301\n  `$nope` is not declared\n", last: "<?nvs\necho $nope;\n" });
  }, 60_000);

  test.each([
    ["an action makes the edits of the action it names", ACTION, ACTION_REPLAY, ACTION_OUT],
    ["a rename makes the server's edits in the open file", RENAME, RENAME_REPLAY, RENAME_OUT],
    ["a lens records each code lens of the file", LENS, LENS_REPLAY, LENS_OUT],
    ["calls records the method's callers and the methods it calls", CALLS, CALLS_REPLAY, CALLS_OUT],
    ["a card records the hover card's Markdown", CARD, CARD_REPLAY, CARD_OUT],
  ])("%s", async (_, text, replay, out) => {
    site(text, replay, out);
    expect(await snippetReplay(P, NVS, s.root)).toEqual({ recording: out, last: text });
    expect(await replayProblems(P, NVS, s.root)).toEqual([]);
  }, 60_000);

  test("an action with another title, and a rename the server refuses, are named", async () => {
    site(ACTION, ACTION_REPLAY.replace("action rewrite", "action change"));
    expect((await replayProblems(P, NVS, s.root))[0]).toStartWith(`${P.replace(/\.nvs$/, ".replay")}: step 1: the code actions at 3:12 are "rewrite as \`$n as int\`", not "change as \`$n as int\`"`);
    site(CART, `--START--\n${CART.replace("return $sum", "return $s<|>um")}--STEPS--\nrename two-words\n`);
    expect((await replayProblems(P, NVS, s.root))[0]).toStartWith(`${P.replace(/\.nvs$/, ".replay")}: step 1: \`nvs lsp\` answered request 2 with an error: \`two-words\` is not a valid name.`);
  }, 60_000);

  test("a stale recording is named by the check", async () => {
    site(KEY, KEY_REPLAY, KEY_OUT.replace("age\tproperty\tint\n", ""));
    expect(await replayProblems(P, NVS, s.root)).toEqual([`${P}: its .replay.out is not what \`nvs lsp\` returns for its .replay now`]);
  }, 60_000);

  test("a missing recording, one with no replay, a wrong accept and steps that end elsewhere are named", async () => {
    site(KEY, KEY_REPLAY);
    expect(await replayProblems(P, NVS, s.root)).toEqual([`${P}: no .replay.out beside its .replay`]);
    site(KEY.replace("'name'", "'age'"), KEY_REPLAY, KEY_OUT);
    expect(await replayProblems(P, NVS, s.root)).toEqual([`${P.replace(/\.nvs$/, ".replay")}: its steps end on other text than the snippet`]);
    site(KEY, KEY_REPLAY.replace("accept name", "accept age"), KEY_OUT);
    expect((await replayProblems(P, NVS, s.root))[0]).toStartWith(`${P.replace(/\.nvs$/, ".replay")}: step 2: after \`na\` the list selects \`name\`, not \`age\``);
    s.cleanup();
    s = scratch();
    s.put(P, KEY);
    s.put(P.replace(/\.nvs$/, ".replay.out"), KEY_OUT);
    expect(await replayProblems(P, NVS, s.root)).toEqual([`${P}: a .replay.out beside it and no .replay`]);
  }, 120_000);
});
