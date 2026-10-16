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
    expect(parseReplay("--START--\n<|>\n--STEPS--\npress Enter\n")).toBe("line 4: `press Enter` is not `type TEXT`, `accept LABEL` or `hover`");
    expect(parseReplay("--START--\n<|>\n--STEPS--\ntype \\t\n")).toBe("line 4: only `\\n` and `\\\\` are escapes");
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
    const asks = replaySession("file:///a.nvs", replay, { triggers: ["$"], inserts: {} }).asks;
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
