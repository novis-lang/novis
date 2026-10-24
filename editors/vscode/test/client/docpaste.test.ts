// *Paste as doc comment*, held without an editor.
//
// `src/docpaste.ts` imports no `vscode`, so it runs in the headless tier the unattended loop gates on
// (`rule:ide/headless-gates-the-loop-the-host-run-gates-the-milestone`). What is pinned is the frozen
// kind id, where the paste is offered, and what each pasted line becomes.

import * as assert from "node:assert/strict";

import { DOC_COMMENT_KIND, asDocComment } from "../../src/docpaste";

describe("paste as doc comment", () => {
  it("is offered under a frozen kind id", () => {
    assert.equal(DOC_COMMENT_KIND, "text.docComment.nvs");
  });

  it("gives every later line the run's indentation and marker", () => {
    assert.equal(
      asDocComment("    /// ", "Returns the total.\nThrows when empty.", "\n"),
      "Returns the total.\n    /// Throws when empty.",
    );
  });

  it("writes an empty line as a bare marker", () => {
    assert.equal(asDocComment("/// ", "One.\n\n  \nTwo.", "\n"), "One.\n///\n///\n/// Two.");
  });

  it("writes the document's line ending", () => {
    assert.equal(asDocComment("\t/// ", "a\r\nb", "\r\n"), "a\r\n\t/// b");
  });

  it("drops one line ending at the end of the text", () => {
    assert.equal(asDocComment("/// ", "a\nb\n", "\n"), "a\n/// b");
  });

  it("keeps the text of a line as it is pasted", () => {
    assert.equal(asDocComment("/// ", "```\n    $x = 1;\n```", "\n"), "```\n///     $x = 1;\n/// ```");
  });

  it("is not offered for a single line", () => {
    assert.equal(asDocComment("/// ", "one line", "\n"), undefined);
    assert.equal(asDocComment("/// ", "one line\n", "\n"), undefined);
  });

  it("is not offered outside a run", () => {
    assert.equal(asDocComment("", "a\nb", "\n"), undefined);
    assert.equal(asDocComment("    ", "a\nb", "\n"), undefined);
    assert.equal(asDocComment("// ", "a\nb", "\n"), undefined);
    assert.equal(asDocComment("//// ", "a\nb", "\n"), undefined);
    assert.equal(asDocComment("$x = 1; /// ", "a\nb", "\n"), undefined);
  });

  it("is not offered before the marker", () => {
    assert.equal(asDocComment("  /", "a\nb", "\n"), undefined);
  });
});
