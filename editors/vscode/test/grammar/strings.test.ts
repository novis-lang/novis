// Which literals interpolate, and which are the text they look like.
//
// `rule:ide/highlighting-is-two-layers` names heredoc and nowdoc because one of them interpolates and
// the other does not, and a grammar that misses that colours a `$name` a nowdoc will print verbatim.
// The same bit separates `"` from `'`, and `crates/nvs-syntax/src/lexer.rs:1194` is where it is read
// off the header's quote.
//
// The escapes are the other half. A spelling `crates/nvs-types/src/string_lit.rs:231` does not decode
// stays two characters, so `\q` and a single-quoted `\n` carry no escape scope; a malformed `\u{` is
// the one spelling Novis rejects outright, and `rule:ide/rejected-syntax-gets-no-colour` is why it is
// marked invalid rather than left as text.

import * as assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";

import { FIXTURES, Span, span, tokenize } from "./tokenize";

const SINGLE = "string.quoted.single.nvs";
const DOUBLE = "string.quoted.double.nvs";
const HEREDOC = "string.unquoted.heredoc.nvs";
const NOWDOC = "string.unquoted.nowdoc.nvs";
const ESCAPE = "constant.character.escape.nvs";
const INVALID = "invalid.illegal.escape.nvs";
const VARIABLE = "variable.other.nvs";
const SLOT = "meta.embedded.line.nvs";

let spans: Span[];

before(async () => {
  spans = await tokenize(readFileSync(join(FIXTURES, "strings.nvs"), "utf8"));
});

/** Asserts that `text` is one span, inside `literal`, carrying no mark of an interpolation slot. */
function verbatim(text: string, literal: string): void {
  const found = span(spans, text);
  assert.ok(found.scopes.includes(literal), `${text} is outside ${literal}`);
  for (const scope of found.scopes) {
    assert.ok(!scope.startsWith("variable."), `${text} colours a variable`);
    assert.notEqual(scope, SLOT, `${text} opens an interpolation slot`);
    assert.notEqual(scope, ESCAPE, `${text} colours an escape`);
  }
}

describe("a single-quoted string, which interpolates nothing", () => {
  it("colours neither spelling of interpolation inside one", () => {
    verbatim("no $plain and no {$complex} in a single-quoted string", SINGLE);
  });

  it("escapes the quote and the backslash, which are the two it has", () => {
    assert.ok(span(spans, "\\'").scopes.includes(ESCAPE));
    assert.ok(span(spans, "\\\\").scopes.includes(ESCAPE));
  });

  it("leaves every other backslash literal", () => {
    verbatim("a literal \\n is two characters", SINGLE);
  });
});

describe("a double-quoted string, which interpolates both spellings", () => {
  it("reads a bare $name", () => {
    assert.ok(span(spans, "$who").scopes.includes(VARIABLE));
  });

  it("reads one level of ->prop", () => {
    assert.ok(span(spans, "$user").scopes.includes(VARIABLE));
    // The slots hold code, which reaches an accessor of its own, so this is the one outside them.
    const simple = spans.filter((s) => s.text === "->" && !s.scopes.includes(SLOT));
    assert.equal(simple.length, 1, `${simple.length} accessors outside a slot, wanted 1`);
    assert.ok(simple[0].scopes.includes("punctuation.accessor.nvs"));
    assert.ok(span(spans, "email").scopes.includes("variable.other.property.nvs"));
  });

  it("reads an offset that is a bareword, digits or another variable", () => {
    assert.ok(span(spans, "$byword").scopes.includes(VARIABLE));
    assert.ok(span(spans, "label").scopes.includes("constant.other.nvs"));
    assert.ok(span(spans, "0").scopes.includes("constant.other.nvs"));
    assert.ok(span(spans, "$key").scopes.includes(VARIABLE));
  });

  it("opens a slot on {$ and closes it on the matching }", () => {
    // The slot holds code, so the code-mode patterns read it: `$order` is a span of its own there.
    const body = span(spans, "$order");
    assert.ok(body.scopes.includes(SLOT), "the complex slot holds no embedded code");
    assert.ok(body.scopes.includes(DOUBLE), "the slot escaped the string it is in");
  });

  it("reads PHP's ${name} as no variable at all", () => {
    // Novis has one complex spelling and it is `{$`, so this one is the text it looks like.
    verbatim("${brace} names no variable in Novis", DOUBLE);
  });

  it("does not end code mode on a ?> inside the literal", () => {
    verbatim("a ?> in a string does not end code mode", DOUBLE);
  });
});

describe("an html template, which interpolates a string's two spellings and one more", () => {
  const TEMPLATE = "string.quoted.other.html-template.nvs";

  it("opens on the prefix and the backtick together, and closes on a backtick", () => {
    const openers = spans.filter((s) => s.text === "html`");
    assert.equal(openers.length, 2, "the fixture holds two templates");
    for (const opener of openers) {
      assert.ok(opener.scopes.includes("punctuation.definition.string.begin.nvs"));
      assert.ok(opener.scopes.includes(TEMPLATE));
    }
    assert.ok(span(spans, "<p>hello ").scopes.includes(TEMPLATE), "the body is outside the template");
  });

  it("reads a bare $name and a {$ slot as a double-quoted string does", () => {
    assert.ok(span(spans, "$visitor").scopes.includes(VARIABLE));
    const body = span(spans, "$cart");
    assert.ok(body.scopes.includes(SLOT), "the complex slot holds no embedded code");
    assert.ok(body.scopes.includes(TEMPLATE), "the slot escaped the template it is in");
  });

  it("opens a slot on <?= and closes it on ?>, holding code that need not begin with $", () => {
    const open = span(spans, "<?=");
    assert.ok(open.scopes.includes("punctuation.section.embedded.begin.nvs"));
    assert.ok(open.scopes.includes(TEMPLATE), "the tag slot escaped the template it is in");
    // Code mode reads `App::VERSION` as it reads it anywhere: the name, then the accessor.
    const name = spans.find((s) => s.text.trim() === "App" && s.scopes.includes(SLOT));
    assert.ok(name, "the class name inside the tag slot is code");
    assert.ok(name.scopes.includes(TEMPLATE), "the tag slot escaped the template it is in");
    const accessor = spans.find((s) => s.text === "::" && s.scopes.includes(SLOT));
    assert.ok(accessor?.scopes.includes("punctuation.accessor.nvs"), "`::` in the slot is code");
    // The file's own `?>` closes code mode; the slot's is the one inside the template.
    const close = spans.find((s) => s.text === "?>" && s.scopes.includes(SLOT));
    assert.ok(close?.scopes.includes("punctuation.section.embedded.end.nvs"), "`?>` ends the slot");
  });

  it("escapes a backtick, which is the delimiter", () => {
    assert.ok(span(spans, "\\`").scopes.includes(ESCAPE));
  });

  it("reads a brace before anything but $, and a ?> outside a slot, as one run of text", () => {
    verbatim(
      "<style>.a {color: red}</style>{App::NAME} stays text and so does a ?> outside a slot",
      TEMPLATE,
    );
  });
});

describe("the escapes a double-quoted string decodes", () => {
  it("colours each spelling that decodes to something else", () => {
    for (const text of ["\\n", "\\t", '\\"', "\\$", "\\101", "\\x41", "\\u{1F600}"]) {
      assert.ok(span(spans, text).scopes.includes(ESCAPE), `${text} carries no escape scope`);
    }
  });

  it("colours no spelling that stays two characters", () => {
    verbatim("a \\q is no escape, and ", DOUBLE);
  });

  it("marks a malformed \\u{ invalid rather than leaving it as text", () => {
    assert.ok(span(spans, "\\u{").scopes.includes(INVALID));
  });
});

describe("a heredoc, which interpolates, and a nowdoc, which does not", () => {
  it("opens on the header and closes on the label alone", () => {
    for (const [open, close] of [["<<<EOT", "EOT"], ['<<<"LOUD"', "LOUD"], ["<<<'RAW'", "RAW"]]) {
      assert.ok(span(spans, open).scopes.includes("punctuation.definition.string.begin.nvs"),
                `${open} opens nothing`);
      assert.ok(span(spans, close).scopes.includes("punctuation.definition.string.end.nvs"),
                `${close} closes nothing`);
    }
  });

  it("interpolates and escapes inside a heredoc, bare label or quoted", () => {
    assert.ok(span(spans, "$inner").scopes.includes(VARIABLE));
    assert.ok(span(spans, "$nested").scopes.includes(SLOT));
    assert.ok(span(spans, "\\r").scopes.includes(ESCAPE));
    assert.ok(span(spans, "$shout").scopes.includes(VARIABLE));
  });

  it("reads a nowdoc body as one run of text", () => {
    verbatim(
      "    a nowdoc interpolates no $never and no {$missing->total}, and \\r stays literal",
      NOWDOC,
    );
  });

  it("keeps the heredoc body out of the nowdoc's scope and back", () => {
    // The failure this catches is one label's terminator ending the other's literal, which runs the
    // rest of the file together as one string.
    assert.ok(span(spans, "$inner").scopes.includes(HEREDOC));
    assert.equal(span(spans, "$inner").scopes.includes(NOWDOC), false);
  });
});
