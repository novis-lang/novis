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
    // The name, the parentheses and the string literal are each a span of their own.
    for (const text of ["#[", "Route", "(", "/orders", ")", "]"]) {
      const found = span(spans, text);
      assert.deepEqual(commentScopes(found), [], `${text} is coloured as a comment`);
      assert.ok(found.scopes.includes(ATTRIBUTE), `${text} is outside the attribute`);
    }
  });

  it("ends the attribute rather than running to the end of the line", () => {
    // The failure this catches is the whole rest of the file joining the attribute.
    assert.equal(span(spans, "class").scopes.includes(ATTRIBUTE), false);
  });
});

describe("the names inside an attribute", () => {
  const TYPE_NAME = "entity.name.type.nvs";
  const PROPERTY = "variable.other.property.nvs";
  let spans: Span[];

  before(async () => {
    spans = await tokenize(readFileSync(join(FIXTURES, "attributes.nvs"), "utf8"));
  });

  it("colours each attribute class as a type, the member form's alias too", () => {
    for (const text of ["Core\\Route", "Cached", "Shop\\Owner", "Alias", "Access"]) {
      assert.ok(span(spans, text).scopes.includes(TYPE_NAME), `${text} is not a type`);
    }
  });

  it("colours each argument's name as the property it sets, on its own line too", () => {
    for (const text of ["path", "tags", "method", "allow", "reason"]) {
      assert.ok(span(spans, text).scopes.includes(PROPERTY), `${text} is not a property`);
    }
  });

  it("leaves a ternary's branch alone, and an argument's value to the code patterns", () => {
    // `first :` is a ternary's branch: nothing but `?` stands before it.
    const branch = spans.filter((s) => s.text.includes("first"));
    assert.equal(branch.length, 1);
    assert.equal(branch[0].scopes.includes(PROPERTY), false, "a ternary's branch is a property");
    assert.ok(span(spans, "Get").scopes.includes(PROPERTY), "a value lost its member colour");
    assert.ok(span(spans, "/orders").scopes.includes("string.quoted.single.nvs"));
  });

  it("does not end the attribute at the ] of an array inside its arguments", () => {
    assert.ok(span(spans, "Cached").scopes.includes(ATTRIBUTE), "the attribute ended at []");
    assert.equal(span(spans, "public").scopes.includes(ATTRIBUTE), false);
  });
});
