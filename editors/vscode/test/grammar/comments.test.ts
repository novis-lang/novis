// Comments, and the one thing that looks like a comment and is not.
//
// `#[` opens an attribute; the lexer's own comment branch breaks out on that exact pair, at
// `crates/nvs-syntax/src/lexer.rs:466`. A grammar that misses it colours every `#[Route]` in the file
// as a comment, which `rule:ide/highlighting-is-two-layers` names because it is what a grammar
// improvised from PHP's gets wrong.
//
// The run of slashes is the other divergence a borrowed grammar carries: `rule:tooling/doc-comment-is-three-slashes`
// documents with exactly three, so `////` is a divider and `/**` is an ordinary block nothing reads.

import * as assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";

import { FIXTURES, Span, span, tokenize } from "./tokenize";

const ATTRIBUTE = "meta.attribute.nvs";

function commentScopes(found: Span): string[] {
  return found.scopes.filter((scope) => scope.startsWith("comment."));
}

describe("what is a comment, and how long a run of slashes has to be", () => {
  let spans: Span[];

  before(async () => {
    spans = await tokenize(readFileSync(join(FIXTURES, "comments-and-attributes.nvs"), "utf8"));
  });

  it("reads // to the end of the line", () => {
    assert.deepEqual(commentScopes(span(spans, "// an ordinary line comment")),
                     ["comment.line.double-slash.nvs"]);
  });

  it("documents with exactly three slashes", () => {
    assert.deepEqual(commentScopes(span(spans, "/// documents the class below")),
                     ["comment.line.documentation.nvs"]);
  });

  it("reads a fourth slash as a divider rather than documentation", () => {
    assert.deepEqual(commentScopes(span(spans, "//// a divider, not documentation")),
                     ["comment.line.double-slash.nvs"]);
  });

  it("reads # to the end of the line too", () => {
    assert.deepEqual(commentScopes(span(spans, "# an ordinary line comment too")),
                     ["comment.line.number-sign.nvs"]);
  });

  it("carries a block comment across lines", () => {
    assert.deepEqual(commentScopes(span(spans, "/*")), ["comment.block.nvs"]);
    assert.deepEqual(commentScopes(span(spans, "*/")), ["comment.block.nvs"]);
    assert.deepEqual(commentScopes(span(spans, "   over two lines ")), ["comment.block.nvs"]);
  });
});

describe("the attribute that is not a # comment", () => {
  let spans: Span[];

  before(async () => {
    spans = await tokenize(readFileSync(join(FIXTURES, "comments-and-attributes.nvs"), "utf8"));
  });

  it("opens on #[ and closes on ]", () => {
    assert.ok(span(spans, "#[").scopes.includes("punctuation.definition.attribute.begin.nvs"));
    assert.ok(span(spans, "]").scopes.includes("punctuation.definition.attribute.end.nvs"));
  });

  it("colours no part of the attribute as a comment", () => {
    for (const text of ["#[", "Route(\"/orders\")", "]"]) {
      const found = span(spans, text);
      assert.deepEqual(commentScopes(found), [], `${text} is coloured as a comment`);
      assert.ok(found.scopes.includes(ATTRIBUTE), `${text} is outside the attribute`);
    }
  });

  it("ends the attribute rather than running to the end of the line", () => {
    // The failure this catches is the whole rest of the file joining the attribute.
    assert.equal(span(spans, "class Orders {}").scopes.includes(ATTRIBUTE), false);
  });
});
