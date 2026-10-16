// The round trip: every request the client routes, against the real binary, over a real pipe.
//
// These assertions are deliberately thin on *content* — what a hover says and where a definition
// points is frozen by the `.lspt` corpus under `tests/lsp/`, which is the server's own suite and
// the home of that question (`rule:ide/an-lsp-answer-is-frozen-as-an-lspt-case`). What is only
// visible here is that a client sending LSP's wire shapes to `nvs lsp` gets LSP's wire shapes back:
// the answer is the right kind of thing, in the document it asked about.

import * as assert from "node:assert/strict";

import { Diagnostic, Position, Range, Session } from "./session";

const CART_URI = "file:///cart.nvs";
const CART = [
  "<?nvs", //                                     0
  "class Cart", //                                1
  "{", //                                         2
  "    public const int LIMIT = 10;", //          3
  "", //                                          4
  '    public string $label = "cart";', //        5
  "", //                                          6
  "    public function total(int $qty): int", //  7
  "    {", //                                     8
  "        int $sum = $qty * Cart::LIMIT;", //     9
  "        return $sum;", //                     10
  "    }", //                                    11
  "", //                                         12
  "    public function name(): string", //       13
  "    {", //                                    14
  "        return $this->label;", //             15
  "    }", //                                    16
  "}", //                                        17
  "",
].join("\n");

// The read this file asks most of its questions about: `$this->label` on line 15, whose property is
// declared on line 5.
const PROPERTY_READ: Position = { line: 15, character: 24 };
const AFTER_ARROW: Position = { line: 15, character: 22 };

const CASING_URI = "file:///casing.nvs";
const CASING = ["<?nvs", "class user_account {}", ""].join("\n");

// Two secrets on two lines, which is the shape the reveal state machine is written against: one
// range is revealed and the other stays concealed.
const SECRETS_URI = "file:///secrets.nvs";
const SECRETS = [
  "<?nvs", //                                     0
  'secret string $key = "sk-live-abcdef";', //    1
  'secret string $spare = "sk-live-999999";', //  2
  "",
].join("\n");

// A file in a namespace of its own with one import: the shape a selection is copied out of. Lines 4
// and 5 write `Str`, which the `use` reaches, and `Basket`, which the namespace reaches.
const SHOP_URI = "file:///shop.nvs";
const SHOP = [
  "<?nvs", //                                     0
  "namespace Shop;", //                           1
  "", //                                          2
  "use Core\\Str;", //                            3
  'var $n = Str::length("a");', //                4
  "var $b = new Basket();", //                    5
  "class Basket {}", //                           6
  "",
].join("\n");

interface Import {
  symbol: string;
  written: string;
}

interface TextEdit {
  range: Range;
  newText: string;
}

interface Hover {
  contents: { kind: string; value: string };
  range?: Range;
}

interface Location {
  uri: string;
  range: Range;
}

interface CompletionItem {
  label: string;
  detail?: string;
}

interface DocumentSymbol {
  name: string;
  kind: number;
  range: Range;
  children?: DocumentSymbol[];
}

interface FoldingRange {
  startLine: number;
  endLine: number;
}

interface SelectionRange {
  range: Range;
  parent?: SelectionRange;
}

interface CodeAction {
  title: string;
  kind?: string;
  edit?: unknown;
}

interface Redaction {
  range: Range;
  kind: string;
}

describe("the requests the client routes to nvs lsp", function () {
  this.timeout(60_000);

  let session: Session;
  let opened: Diagnostic[];

  const document = { textDocument: { uri: CART_URI } };
  const at = (position: Position) => ({ ...document, position });

  before(async () => {
    ({ session } = await Session.start());
    session.open(CART_URI, CART);
    opened = await session.diagnostics(CART_URI);
  });

  after(() => {
    session?.kill();
  });

  it("publishes diagnostics for a document the moment it opens", () => {
    // Publication is unrequested, and the empty list is the interesting half: a client that only
    // learned about diagnostics when it asked would never clear the ones it drew.
    assert.deepEqual(opened, []);
  });

  it("answers a hover with the declared type, ranged over the read", async () => {
    const hover = await session.request<Hover | null>("textDocument/hover", at(PROPERTY_READ));
    assert.ok(hover, "the property read answered no hover");
    assert.equal(hover.contents.kind, "markdown");
    assert.ok(hover.contents.value.includes("string"), hover.contents.value);
    assert.equal(hover.range?.start.line, 15);
  });

  it("answers a definition in the document that declares it", async () => {
    const answer = await session.request<Location | Location[] | null>(
      "textDocument/definition",
      at(PROPERTY_READ),
    );
    assert.ok(answer, "the property read answered no definition");
    const location = Array.isArray(answer) ? answer[0] : answer;
    assert.equal(location.uri, CART_URI);
    assert.equal(location.range.start.line, 5);
  });

  it("answers a completion after -> with the members of the class", async () => {
    const items = await session.request<CompletionItem[]>("textDocument/completion", at(AFTER_ARROW));
    assert.deepEqual(items.map((item) => item.label).sort(), ["label", "name", "total"]);
  });

  it("answers the outline as one class with its members under it", async () => {
    // A property is outlined as it is written, `$label`, and completed as what follows the arrow,
    // `label`: an outline names a declaration and a completion is text about to be inserted.
    const symbols = await session.request<DocumentSymbol[]>("textDocument/documentSymbol", document);
    assert.equal(symbols.length, 1);
    assert.equal(symbols[0].name, "Cart");
    assert.deepEqual(symbols[0].children?.map((child) => child.name).sort(),
                     ["$label", "LIMIT", "name", "total"]);
  });

  it("answers a folding range for the class and for each method body", async () => {
    const folds = await session.request<FoldingRange[]>("textDocument/foldingRange", document);
    assert.ok(folds.some((fold) => fold.startLine === 1 && fold.endLine === 16), JSON.stringify(folds));
    assert.ok(folds.length >= 3, JSON.stringify(folds));
  });

  it("answers a selection range that grows outwards from the cursor", async () => {
    const ranges = await session.request<SelectionRange[]>("textDocument/selectionRange", {
      ...document,
      positions: [PROPERTY_READ],
    });
    assert.equal(ranges.length, 1);
    let outer = ranges[0];
    let depth = 1;
    while (outer.parent !== undefined) {
      const parent = outer.parent;
      assert.ok(parent.range.start.line <= outer.range.start.line, "a parent starts after its child");
      outer = parent;
      depth += 1;
    }
    assert.ok(depth >= 2, "the cursor expanded to nothing");
    assert.equal(outer.range.start.line, 1, "the outermost range is not the class");
  });

  it("answers semantic tokens as five-number rows the declared legend decodes", async () => {
    const tokens = await session.request<{ data: number[] }>("textDocument/semanticTokens/full", document);
    assert.ok(tokens.data.length > 0, "the document produced no semantic tokens");
    assert.equal(tokens.data.length % 5, 0, "a semantic token is five numbers");
    // The first token is the class name on line 1, six columns in: `deltaLine`, `deltaStart`,
    // `length`, then the index into the legend's types.
    assert.deepEqual(tokens.data.slice(0, 3), [1, 6, 4]);
  });

  it("answers a document link list for a document that requires nothing", async () => {
    assert.deepEqual(await session.request("textDocument/documentLink", document), []);
  });

  it("offers the fix its own diagnostic carried, and nothing else", async () => {
    // `rule:ide/a-code-action-ships-only-a-fix-a-diagnostic-already-knows`: the action is a
    // translation of a suggestion the diagnostic already published, so the client sends the
    // diagnostic back in the request's context and the server computes nothing new.
    session.open(CASING_URI, CASING);
    const published = await session.diagnostics(CASING_URI);
    assert.equal(published.length, 1, JSON.stringify(published));
    assert.equal(published[0].source, "nvs");
    assert.equal(published[0].severity, 1);

    const actions = await session.request<CodeAction[]>("textDocument/codeAction", {
      textDocument: { uri: CASING_URI },
      range: published[0].range,
      context: { diagnostics: published },
    });
    assert.equal(actions.length, 1, JSON.stringify(actions));
    assert.equal(actions[0].kind, "quickfix");
    assert.ok(actions[0].title.includes("UserAccount"), actions[0].title);
    assert.ok(actions[0].edit, "the action carries no edit");
  });

  it("answers nvs/redactions with the bytes of every secret literal", async () => {
    // The one request of Novis's own, and the only source a client has for which bytes it conceals
    // (`rule:ide/redaction-ranges-come-from-the-server`). Which ranges are answered for which
    // construct is frozen in `tests/lsp/redactions/`; what is only visible here is that a client
    // sending a `TextDocumentIdentifier` gets `{range, kind}` back, over the wire, from the binary.
    session.open(SECRETS_URI, SECRETS);
    await session.diagnostics(SECRETS_URI);

    const answered = await session.request<Redaction[]>("nvs/redactions", { uri: SECRETS_URI });
    assert.equal(answered.length, 2, JSON.stringify(answered));
    const lines = SECRETS.split("\n");
    const covered = answered.map((redaction) => {
      assert.equal(redaction.kind, "secretLiteral");
      assert.equal(redaction.range.start.line, redaction.range.end.line, JSON.stringify(redaction));
      return lines[redaction.range.start.line]
        .slice(redaction.range.start.character, redaction.range.end.character);
    });
    // The literal token and nothing around it: never the `$key` naming it and never the
    // `secret string` declaring it (`rule:ide/redaction-covers-bytes-only`).
    assert.deepEqual(covered, ['"sk-live-abcdef"', '"sk-live-999999"']);
  });

  it("answers nvs/imports with what a range resolved, and nvs/importEdits with the use lines a paste lacks", async () => {
    // The two requests behind paste-with-imports (`rule:ide/a-pasted-type-carries-its-use-line`).
    // Which names a range carries and where a `use` line lands are held by `crates/nvs-lsp/tests/
    // imports.rs`; what is only visible here is the round trip: a range goes in and `{symbol,
    // written}` pairs come back, and those pairs go into another document and come back as edits.
    session.open(SHOP_URI, SHOP);
    await session.diagnostics(SHOP_URI);

    const carried = await session.request<Import[]>("nvs/imports", {
      textDocument: { uri: SHOP_URI },
      range: { start: { line: 4, character: 0 }, end: { line: 5, character: 20 } },
    });
    assert.deepEqual(carried, [
      { symbol: "Core\\Str", written: "Str" },
      { symbol: "Shop\\Basket", written: "Basket" },
    ]);

    // Pasted into the cart file: `Str` has no import there and gets one after the open tag, and
    // `Basket` is imported beside it. Both in one edit, so the group stays together.
    const edits = await session.request<TextEdit[]>("nvs/importEdits", {
      textDocument: { uri: CART_URI },
      position: { line: 18, character: 0 },
      imports: carried,
    });
    assert.equal(edits.length, 1, JSON.stringify(edits));
    assert.deepEqual(edits[0].range, { start: { line: 0, character: 5 }, end: { line: 0, character: 5 } });
    assert.equal(edits[0].newText, "\nuse Core\\Str;\nuse Shop\\Basket;");
  });

  it("answers an empty list for a document with nothing to conceal", async () => {
    // An answer of nothing, not the absence of an answer: the client holds its last concealment
    // when none arrives, so the two must be distinguishable on the wire.
    assert.deepEqual(await session.request("nvs/redactions", document.textDocument), []);
  });

  it("writes nothing to stderr while it does all that", () => {
    // `rule:ide/stdout-belongs-to-the-protocol` gives stdout to the framing; a server logging
    // anywhere near it desynchronises the stream rather than printing something a user sees, and
    // every frame above having parsed is the other half of that check.
    assert.equal(session.errors(), "");
  });
});
