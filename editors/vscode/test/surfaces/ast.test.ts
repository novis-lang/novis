// The AST panel, and the document it renders.
//
// `rule:ide/the-ast-panel-shells-out-to-the-cli` is two claims and this file holds both. The panel
// runs `nvs ast --json` rather than parsing anything in the client, and it renders what comes back
// on the file that does not compile — which is the file it exists for, so the recording below is
// the same broken program `check.txt` is a recording of. `recorded/ast.json` is what the binary
// printed for it, byte for byte, and the assertions are what the tree view would then hold.
//
// Nothing here imports `vscode` — the headless tier has no such module (`scripts/headless.mjs`).
// The rendering decisions live in `src/nodes.ts`, which imports none either and is run directly;
// what is left in `src/ast.ts` is the process and the view, asserted as text the way the
// contributions suite asserts a registered command.

import * as assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join, resolve } from "node:path";

import { Node, detail, index, label, read, tooltip } from "../../src/nodes";

const ROOT = resolve(__dirname, "..", "..", "..");

interface Manifest {
  contributes: {
    views?: Record<string, { id: string; name: string; visibility?: string }[]>;
    viewsWelcome?: { view: string; contents: string }[];
  };
}

const manifest = JSON.parse(readFileSync(join(ROOT, "package.json"), "utf8")) as Manifest;
// The client with its prose removed. What the assertions below are about is what the panel does,
// and a comment saying which flag it deliberately never passes names that flag.
const CLIENT = readFileSync(join(ROOT, "src", "ast.ts"), "utf8")
  .replace(/\/\*[\s\S]*?\*\//g, "")
  .replace(/^\s*\/\/.*$/gm, "");

const recorded = (name: string): string =>
  readFileSync(join(ROOT, "test", "surfaces", "recorded", name), "utf8");

const DOCUMENT = recorded("ast.json");
const PROGRAM = recorded("app.nvs");
const CHECKED = recorded("check.txt");

/** The first node of `kind` anywhere under `node`, depth first, or nothing. */
function find(node: Node, kind: string): Node | undefined {
  if (node.kind === kind) {
    return node;
  }
  for (const child of node.children) {
    const found = find(child, kind);
    if (found !== undefined) {
      return found;
    }
  }
  return undefined;
}

describe("the AST panel the manifest contributes", () => {
  it("puts one tree view in the explorer, with a welcome that runs the command", () => {
    const views = manifest.contributes.views?.["explorer"] ?? [];
    assert.deepEqual(views.map((view) => view.id), ["nvs.ast"]);
    // Collapsed rather than visible: the panel is empty until somebody asks for a tree, and a
    // view that takes space in every workspace to say nothing is the wrong default.
    assert.equal(views[0].visibility, "collapsed");
    const welcome = manifest.contributes.viewsWelcome?.filter((entry) => entry.view === "nvs.ast");
    assert.equal(welcome?.length, 1);
    assert.ok(welcome?.[0].contents.includes("command:nvs.showAst"),
              "the empty panel does not say what fills it");
  });

  it("shells out to nvs ast --json, and never asks for the strict tree", () => {
    // `--strict` prints nothing once an error is reported, which is exactly the file this panel
    // is opened for (`crates/nvs-cli/src/ast.rs`). Asking for it would leave the panel empty
    // precisely when it is wanted.
    assert.ok(/"ast",\s*"--json"/.test(CLIENT), "the panel does not run nvs ast --json");
    assert.equal(/--strict/.test(CLIENT), false, "the panel asks for the strict tree");
  });

  it("draws the editor's own tree and no UI of its own", () => {
    // `rule:ide/the-extension-builds-no-ui-the-editor-already-has`: a `TreeDataProvider` feeds a
    // view VS Code draws, and a webview here would be this project rendering a tree itself.
    assert.ok(/createTreeView\(/.test(CLIENT), "the panel creates no tree view");
    assert.equal(/[Ww]ebview|<html|innerHTML/.test(CLIENT), false,
                 "the panel draws UI of its own");
  });

  it("renders what standard output printed, whatever the exit status was", () => {
    // `nvs ast` exits non-zero because diagnostics were reported and still prints the tree it
    // recovered. A client that treated the exit status as the answer would show nothing for every
    // file with an error in it.
    assert.ok(/resolve\(stdout\)/.test(CLIENT), "the panel does not read standard output");
    assert.equal(/if \(failure\b/.test(CLIENT), false,
                 "the panel refuses a tree because the process exited non-zero");
  });
});

describe("the recorded document", () => {
  const root = read(DOCUMENT);

  it("is the schema the panel understands, rooted on the whole file", () => {
    assert.ok(root !== undefined, "the recording is not a document this client reads");
    assert.equal(root.kind, "File");
    // The span is bytes, so this is also what says the recording is current: re-record it with
    // `nvs ast --json test/surfaces/recorded/app.nvs` after any edit to the program.
    assert.deepEqual(root.span, [0, Buffer.byteLength(PROGRAM.replace(/\r\n/g, "\n"), "utf8")]);
  });

  it("holds a tree for a program that does not compile", () => {
    // The two recordings are of one program: `check.txt` is three errors in it, and the document
    // is its tree all the same. That pairing is the resilience claim, and losing it would make
    // this suite assert the easy case.
    assert.ok(CHECKED.includes("aborting due to 3 errors"), "the program now compiles; re-record");
    assert.ok(find(root as Node, "Function"), "the tree lost the declaration the errors are about");
  });

  it("carries the trivia the panel is least useful without", () => {
    // A comment is a node like any other (`rule:ide/ast-json-schema-is-frozen`), sitting under the
    // innermost node whose span contains it.
    const comment = find(root as Node, "LineComment");
    assert.ok(comment, "the document carries no comment node");
    assert.deepEqual(comment.children, []);
  });

  it("labels a node by kind and dims its own scalar fields beside it", () => {
    const binary = find(root as Node, "Binary");
    const bool = find(root as Node, "Bool");
    assert.ok(binary, "the document carries no binary expression");
    assert.ok(bool, "the document carries no bool literal");
    assert.equal(label(binary), "Binary");
    // The two shapes the compiler emits — a fixed spelling out of a closed set, and a flag.
    assert.equal(detail(binary), "op=Add");
    assert.equal(detail(bool), "value=true");
    // A node with no scalar fields shows its kind and nothing else, rather than an empty pair.
    assert.equal(detail(find(root as Node, "Echo") as Node), "");
    assert.equal(tooltip(bool), `Bool  bytes ${bool.span[0]}..${bool.span[1]}`);
  });

  it("refuses a document that is not one, rather than drawing half of it", () => {
    assert.equal(read("not json"), undefined);
    assert.equal(read("{}"), undefined);
    assert.equal(read('{"kind":"File","span":[0,1],"children":[{"kind":"Echo"}]}'), undefined);
  });
});

describe("a span is bytes, and every editor position is code units", () => {
  it("is the offset itself while the text is ASCII", () => {
    assert.equal(index("echo 1 + 2;", 5), 5);
    assert.equal(index(PROGRAM, 273), 273);
  });

  it("counts a multi-byte character once, not twice", () => {
    // `ü` is two bytes and one code unit: a panel selecting the raw offset would land one column
    // past the name it was asked to highlight, for every span after the first accented letter.
    const text = "// ü\n$x";
    assert.equal(index(text, 6), 5);
    assert.equal(text.slice(index(text, 6)), "$x");
  });

  it("counts an astral character as the surrogate pair the editor holds", () => {
    const text = "🙂$x";
    assert.equal(index(text, 4), 2);
    assert.equal(text.slice(index(text, 4)), "$x");
  });
});
