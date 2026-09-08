// The reveal state machine, asserted without an editor.
//
// `src/concealment.ts` imports no `vscode`, which is what lets this run in the headless tier the
// unattended loop gates on (`rule:ide/headless-gates-the-loop-the-host-run-gates-the-milestone`).
// What is left in `src/redactions.ts` once this file has run is position conversion and a call to
// `setDecorations`, and the extension's README § *Decided here* records why no tier drives that.
//
// The two security properties under test are `rule:security/reveal-is-explicit-and-window-local` —
// one range, this window, gone when the editor closes — and the fail direction of
// `rule:security/redaction-ranges-come-from-the-server`: the absence of an answer conceals what
// was concealed before, and only an answer changes it.

import * as assert from "node:assert/strict";

import { Concealment, Position, Range, Redaction } from "../../src/concealment";

const URI = "file:///secrets.nvs";
const OTHER = "file:///other.nvs";

// The ranges `nvs lsp` answers for the two-secret document the protocol suite opens: the literal on
// line 1 and the literal on line 2, each without the `$key` naming it.
const KEY: Range = { start: { line: 1, character: 21 }, end: { line: 1, character: 37 } };
const SPARE: Range = { start: { line: 2, character: 23 }, end: { line: 2, character: 39 } };

const ANSWER: Redaction[] = [
  { range: KEY, kind: "secretLiteral" },
  { range: SPARE, kind: "secretLiteral" },
];

// A `tainted` declaration's name, which the server answers on the same list under the other kind.
const DIRTY: Range = { start: { line: 3, character: 15 }, end: { line: 3, character: 21 } };
const IN_DIRTY: Position = { line: 3, character: 17 };

const IN_KEY: Position = { line: 1, character: 25 };
const IN_SPARE: Position = { line: 2, character: 30 };
const IN_NEITHER: Position = { line: 1, character: 4 };

function opened(): Concealment {
  const concealment = new Concealment();
  concealment.hold(URI, ANSWER);
  return concealment;
}

describe("what the client conceals", () => {
  it("conceals every range the server answered", () => {
    assert.deepEqual(opened().concealed(URI), [KEY, SPARE]);
  });

  it("conceals nothing for a document nobody has answered for", () => {
    const concealment = opened();
    assert.deepEqual(concealment.concealed(OTHER), []);
    assert.equal(concealment.knows(OTHER), false);
  });

  it("tells an empty answer apart from no answer at all", () => {
    // The distinction the server's `[]`-not-`null` exists for: an answer of nothing clears the
    // concealment, and never being answered leaves the last one drawn.
    const concealment = opened();
    concealment.hold(URI, []);
    assert.deepEqual(concealment.concealed(URI), []);
    assert.equal(concealment.knows(URI), true);
  });

  it("keeps the last answer when the next one never comes", () => {
    // What an error, a cancellation or a stopped server look like from here: nothing is called, so
    // nothing changes. A client that cleared on failure would uncover the bytes exactly when the
    // analysis is going wrong.
    const concealment = opened();
    assert.deepEqual(concealment.concealed(URI), [KEY, SPARE]);
    assert.deepEqual(concealment.concealed(URI), [KEY, SPARE]);
  });
});

describe("what the client marks rather than conceals", () => {
  // `taintedDeclaration` is a name, and concealing a name is what
  // `rule:security/redaction-covers-bytes-only` refuses: a black bar over `$dirty` would hide no
  // bytes of any secret and make the file unreadable for the developer whose editor it is.
  function mixed(): Concealment {
    const concealment = new Concealment();
    concealment.hold(URI, [...ANSWER, { range: DIRTY, kind: "taintedDeclaration" }]);
    return concealment;
  }

  it("conceals the secret ranges and marks the tainted one", () => {
    assert.deepEqual(mixed().concealed(URI), [KEY, SPARE]);
    assert.deepEqual(mixed().marked(URI), [DIRTY]);
  });

  it("reveals nothing at a marked range, because nothing there is covered", () => {
    const concealment = mixed();
    assert.equal(concealment.reveal(URI, IN_DIRTY), false);
    assert.deepEqual(concealment.revealed(URI), []);
    assert.deepEqual(concealment.marked(URI), [DIRTY]);
  });

  it("conceals a kind it has never heard of", () => {
    // The safe direction, and the one this client is behind its server in: a spelling added to
    // `crates/nvs-lsp/src/redactions.rs` before it is added here covers bytes that needed no
    // covering, rather than leaving bytes uncovered that did.
    const concealment = new Concealment();
    concealment.hold(URI, [{ range: KEY, kind: "somethingLaterThanThis" }]);
    assert.deepEqual(concealment.concealed(URI), [KEY]);
    assert.deepEqual(concealment.marked(URI), []);
  });
});

describe("what a reveal uncovers", () => {
  it("reveals the one range under the cursor and leaves the other concealed", () => {
    const concealment = opened();
    assert.equal(concealment.reveal(URI, IN_KEY), true);
    assert.deepEqual(concealment.revealed(URI), [KEY]);
    assert.deepEqual(concealment.concealed(URI), [SPARE]);
  });

  it("reveals nothing when the cursor is outside every range", () => {
    const concealment = opened();
    assert.equal(concealment.reveal(URI, IN_NEITHER), false);
    assert.deepEqual(concealment.concealed(URI), [KEY, SPARE]);
  });

  it("reveals a range whose closing edge the cursor is against", () => {
    const concealment = opened();
    assert.equal(concealment.reveal(URI, KEY.end), true);
    assert.deepEqual(concealment.revealed(URI), [KEY]);
  });

  it("reveals in one document without touching another", () => {
    const concealment = opened();
    concealment.hold(OTHER, ANSWER);
    concealment.reveal(URI, IN_KEY);
    assert.deepEqual(concealment.concealed(OTHER), [KEY, SPARE]);
  });

  it("re-conceals everything the window revealed", () => {
    // `nvs.hideSecrets`, which is the whole window and not one document: the user is putting the
    // screen back, not editing one file.
    const concealment = opened();
    concealment.hold(OTHER, ANSWER);
    concealment.reveal(URI, IN_KEY);
    concealment.reveal(OTHER, IN_SPARE);
    concealment.hide();
    assert.deepEqual(concealment.concealed(URI), [KEY, SPARE]);
    assert.deepEqual(concealment.concealed(OTHER), [KEY, SPARE]);
  });

  it("forgets every reveal when the editor closes", () => {
    // A reveal does not survive the document being closed and reopened, which is the whole of
    // `rule:security/reveal-is-explicit-and-window-local`: nothing is written down, so there is
    // nothing for a reload to read back.
    const concealment = opened();
    concealment.reveal(URI, IN_KEY);
    concealment.forget(URI);
    assert.equal(concealment.knows(URI), false);
    concealment.hold(URI, ANSWER);
    assert.deepEqual(concealment.concealed(URI), [KEY, SPARE]);
  });
});

describe("what a reveal survives", () => {
  it("survives an answer that still names the range", () => {
    // Every keystroke re-asks, so a reveal that did not survive a re-answer would last one
    // character.
    const concealment = opened();
    concealment.reveal(URI, IN_KEY);
    concealment.hold(URI, ANSWER);
    assert.deepEqual(concealment.concealed(URI), [SPARE]);
  });

  it("does not survive an answer that moved the range", () => {
    // An edit above the literal shifts it down a line, and the reveal is dropped: re-concealing
    // bytes the user is looking at is recoverable in one keypress, and leaving a decoration off a
    // range nobody has looked at is not.
    const concealment = opened();
    concealment.reveal(URI, IN_KEY);
    const moved: Range = { start: { line: 2, character: 21 }, end: { line: 2, character: 37 } };
    concealment.hold(URI, [{ range: moved, kind: "secretLiteral" }]);
    assert.deepEqual(concealment.concealed(URI), [moved]);
    assert.deepEqual(concealment.revealed(URI), []);
  });
});
