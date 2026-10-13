import { describe, expect, test } from "bun:test";
import { join } from "node:path";
import { taintedRanges, taintScript, taintVerdict, type Legend } from "../cmd/site.ts";
import { lspFrame, lspMessages } from "../lib/lsp.ts";
import { ROOT } from "../lib/paths.ts";

// The plugin is plain JavaScript under website/, imported by path so `tsc` reads no `.mjs` of the site's.
interface Range {
  lineIndex: number;
  columnStart: number;
  columnEnd: number;
  title: string;
}
interface Node {
  type: string;
  tagName?: string;
  value?: string;
  properties?: Record<string, unknown>;
  children?: Node[];
}
interface Annotation {
  name: string;
  inlineRange: { columnStart: number; columnEnd: number };
  render(options: { nodesToTransform: Node[] }): Node[];
}
interface Plugin {
  baseStyles: string;
  hooks: { annotateCode(context: { codeBlock: unknown }): void };
}
const plugin = (await import(join(ROOT, "website/config/tainted-ranges.mjs"))) as {
  TAINTED_CLASS: string;
  taintRanges(text: string, lines: string[], where: string): Range[];
  taintNode(node: Node, title: string): Node;
  taintAnnotation(range: Range): Annotation;
  taintedRanges(): Plugin;
};

const CODE = ['<?nvs', 'tainted string $q = Core\\Request::query("q") ?? "";', 'echo $q, "é", $q;'].join("\n");
// Reordered on purpose: the tainted bit is found by name, so it is bit 1 here and not the server's.
const LEGEND: Legend = { tokenTypes: ["namespace", "variable"], tokenModifiers: ["secret", "tainted"] };
// `$q` tainted, `Core` plain, then `$q` twice on the next line, the second one secret too.
const DATA = [1, 15, 2, 1, 2, 0, 5, 4, 0, 0, 1, 5, 2, 1, 2, 0, 9, 2, 1, 3];
const SIDECAR = "2:16+2 variable $q\n3:6+2 variable $q\n3:15+2 variable $q\n";
const VARIABLE = "This variable is tainted: its value comes from outside the program.";

describe("nv site: a snippet's .tainted", () => {
  test("each token with the tainted bit is a line, decoded from the relative encoding through the legend", () => {
    expect(taintedRanges(CODE, LEGEND, DATA)).toBe(SIDECAR);
  });

  test("a snippet with no tainted token owes no file, and a legend with no tainted modifier is an error", () => {
    expect(taintedRanges(CODE, LEGEND, [1, 20, 4, 0, 1])).toBeNull();
    expect(() => taintedRanges(CODE, { tokenTypes: ["variable"], tokenModifiers: ["secret"] }, DATA)).toThrow("no `tainted` modifier");
  });

  test("a missing, a stale and an unneeded file are named, and a matching one or none is not", () => {
    const p = "website/snippets/a.nvs";
    expect(taintVerdict(p, SIDECAR, SIDECAR.replace(/\n/g, "\r\n"))).toBeNull();
    expect(taintVerdict(p, null, null)).toBeNull();
    expect(taintVerdict(p, SIDECAR, null)).toBe(`${p}: no .tainted beside it, and \`nvs lsp\` marks a variable in it as tainted`);
    expect(taintVerdict(p, SIDECAR, "2:16+2 variable $q\n")).toBe(`${p}: its .tainted is not where \`nvs lsp\` marks a tainted variable now`);
    expect(taintVerdict(p, null, SIDECAR)).toBe(`${p}: delete its .tainted: \`nvs lsp\` marks no variable in it as tainted`);
  });

  test("the session opens the snippet, asks for its tokens as request 2 and shuts down", () => {
    const sent = lspMessages(taintScript("file:///a.nvs", CODE));
    expect(sent.map((m) => m.method)).toEqual(["initialize", "initialized", "textDocument/didOpen", "textDocument/semanticTokens/full", "shutdown", "exit"]);
    expect(sent[3]).toMatchObject({ id: 2, params: { textDocument: { uri: "file:///a.nvs" } } });
    expect(sent[2]).toMatchObject({ params: { textDocument: { text: CODE } } });
  });

  test("messages are read by byte length, and a frame cut short is not one", () => {
    const stream = lspFrame({ id: 1, result: "é" }) + lspFrame({ id: 2, result: null });
    expect(lspMessages(stream)).toEqual([{ jsonrpc: "2.0", id: 1, result: "é" }, { jsonrpc: "2.0", id: 2, result: null }]);
    expect(lspMessages(stream.slice(0, -3))).toEqual([{ jsonrpc: "2.0", id: 1, result: "é" }]);
  });
});

describe("website: the tainted-ranges plugin", () => {
  test("a .tainted file becomes ranges from 0, each checked against the code the page shows", () => {
    const lines = CODE.split("\n");
    expect(plugin.taintRanges(SIDECAR, lines, "a.nvs")).toEqual([
      { lineIndex: 1, columnStart: 15, columnEnd: 17, title: VARIABLE },
      { lineIndex: 2, columnStart: 5, columnEnd: 7, title: VARIABLE },
      { lineIndex: 2, columnStart: 14, columnEnd: 16, title: VARIABLE },
    ]);
    expect(plugin.taintRanges("\n", lines, "a.nvs")).toEqual([]);
  });

  test("a property has its own tooltip, and a range that no longer covers its text fails the build", () => {
    expect(plugin.taintRanges("1:3+4 property city\n", ["->city"], "a.nvs")[0]!.title).toBe("This property is tainted: its value comes from outside the program.");
    expect(() => plugin.taintRanges("2:17+2 variable $q\n", CODE.split("\n"), "a.nvs")).toThrow("covers `q `");
    expect(() => plugin.taintRanges("not a range\n", [], "a.nvs")).toThrow("is not a tainted range");
  });

  test("a coloured token takes the class and the tooltip itself, and bare text is wrapped in a span", () => {
    const token: Node = { type: "element", tagName: "span", properties: { style: "--0:#123" }, children: [{ type: "text", value: "$q" }] };
    expect(plugin.taintNode(token, VARIABLE)).toEqual({ ...token, properties: { style: "--0:#123", className: [plugin.TAINTED_CLASS], title: VARIABLE } });
    expect(plugin.taintNode({ type: "text", value: "$q" }, VARIABLE)).toEqual({
      type: "element",
      tagName: "span",
      properties: { className: [plugin.TAINTED_CLASS], title: VARIABLE },
      children: [{ type: "text", value: "$q" }],
    });
  });

  test("the plugin adds one annotation per range to its line, over exactly the range, and a block with no prop gets none", () => {
    const added: Annotation[][] = [[], [], []];
    const lines = CODE.split("\n").map((text, i) => ({ text, addAnnotation: (a: Annotation) => added[i]!.push(a) }));
    const block = (tainted: string | undefined) => ({ props: { tainted }, getLines: () => lines, parentDocument: { sourceFilePath: "a.mdx" } });
    const { hooks, baseStyles } = plugin.taintedRanges();
    hooks.annotateCode({ codeBlock: block(undefined) });
    expect(added.flat()).toEqual([]);
    hooks.annotateCode({ codeBlock: block(SIDECAR) });
    expect(added.map((line) => line.map((a) => a.inlineRange))).toEqual([[], [{ columnStart: 15, columnEnd: 17 }], [{ columnStart: 5, columnEnd: 7 }, { columnStart: 14, columnEnd: 16 }]]);
    const nodes: Node[] = [{ type: "text", value: "$" }, { type: "element", tagName: "span", properties: {}, children: [] }];
    expect(added[1]![0]!.render({ nodesToTransform: nodes }).map((n) => n.properties?.title)).toEqual([VARIABLE, VARIABLE]);
    expect(baseStyles).toContain("text-decoration-style: dotted");
    expect(baseStyles).not.toMatch(/(^|[\s;{])color\s*:/);
  });
});
