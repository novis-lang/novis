// The dual mode, which is the shape M1's lexer is built around and the one a borrowed PHP grammar
// gets wrong first: a file is HTML until an opener, code until `?>`, and HTML again after it.
//
// `rule:ide/highlighting-is-two-layers` names the three openers and the closer. `<?php` is one of
// them and is marked invalid rather than coloured like `<?nvs`, because
// `rule:statements/nvs-is-the-only-open-tag` has the parser report E0229 on every one it consumes,
// and `rule:ide/rejected-syntax-gets-no-colour` refuses to confirm that mistake in the editor first.

import * as assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";

import { FIXTURES, Span, span, tokenize } from "./tokenize";

const BEGIN = "punctuation.section.embedded.begin.nvs";
const END = "punctuation.section.embedded.end.nvs";
const CODE = "meta.embedded.block.nvs";
const INVALID = "invalid.illegal.open-tag.nvs";

function fixture(name: string): string {
  return readFileSync(join(FIXTURES, name), "utf8");
}

describe("the openers, and the HTML around them", () => {
  let spans: Span[];

  before(async () => {
    // With the HTML stub, so the `<?=` in an attribute value is inside a rule that has already begun.
    spans = await tokenize(fixture("dual-mode.nvs"), true);
  });

  it("opens code mode on <?nvs and on <?=", () => {
    for (const opener of ["<?nvs", "<?="]) {
      const found = span(spans, opener);
      assert.ok(found.scopes.includes(BEGIN), `${opener} is not an embedded begin: ${found.scopes}`);
      assert.ok(found.scopes.includes(CODE), `${opener} opens no code block: ${found.scopes}`);
    }
  });

  it("closes code mode on every ?>", () => {
    const closers = spans.filter((s) => s.text === "?>");
    assert.equal(closers.length, 2);
    for (const closer of closers) {
      assert.ok(closer.scopes.includes(END), `a ?> is not an embedded end: ${closer.scopes}`);
    }
  });

  it("reads what is between them as code", () => {
    assert.ok(span(spans, "$total = 1;").scopes.includes(CODE));
  });

  it("reads what is outside them as not code", () => {
    for (const text of ["<!doctype html>", "<h1>Total</h1>", "<footer>bye</footer>"]) {
      const found = span(spans, text);
      assert.equal(found.scopes.includes(CODE), false, `${text} is coloured as code`);
      assert.equal(found.scopes[0], "source.nvs");
    }
  });

  it("opens code mode inside an HTML rule that has already begun", () => {
    // The attribute value's own rule owns every position until its closing quote, so an opener
    // reaches a pattern of ours only because the grammar injects it. Without that, a template's
    // `class="<?= ... ?>"` colours as a string and the rest of the line drifts.
    const found = span(spans, "<?=");
    assert.ok(found.scopes.includes("string.quoted.double.html"), `${found.scopes}`);
    assert.ok(found.scopes.includes(BEGIN), `${found.scopes}`);
    assert.ok(span(spans, " $total ").scopes.includes(CODE));
  });
});

describe("the tags that open code mode and are wrong doing it", () => {
  let spans: Span[];

  before(async () => {
    spans = await tokenize(fixture("rejected-openers.nvs"), true);
  });

  it("marks <?php invalid and still opens code mode", () => {
    // Both halves matter. Coloured like `<?nvs` it confirms a mistake the server is about to
    // report; not opening code mode at all collapses every line below it into inline HTML.
    const found = span(spans, "<?php");
    assert.ok(found.scopes.includes(INVALID), `${found.scopes}`);
    assert.equal(found.scopes.includes(BEGIN), false, "<?php is coloured like a valid opener");
    assert.ok(found.scopes.includes(CODE), "<?php does not open code mode");
    assert.ok(span(spans, "$legacy = 2;").scopes.includes(CODE));
  });

  it("marks a miscased <?NVS invalid and still opens code mode", () => {
    // The lexer recognises it in any case and reports the casing separately, so the grammar
    // follows it rather than leaving the file looking like one long HTML document.
    const found = span(spans, "<?NVS");
    assert.ok(found.scopes.includes(INVALID), `${found.scopes}`);
    assert.equal(found.scopes.includes(BEGIN), false, "<?NVS is coloured like a valid opener");
    assert.ok(span(spans, "$total = 1;").scopes.includes(CODE));
  });

  it("reads <?phpx as the text it is", () => {
    // The lexer wants whitespace, `?` or end of input after the tag, so this one is not a tag.
    const found = span(spans, "<p><?phpx not a tag</p>");
    for (const scope of [BEGIN, INVALID, CODE]) {
      assert.equal(found.scopes.includes(scope), false, `<?phpx was read as ${scope}`);
    }
  });
});

describe("a file that opens with #!", () => {
  let spans: Span[];

  before(async () => {
    spans = await tokenize(fixture("shebang.nvs"), true);
  });

  it("reads line one as a comment and continues in code mode", () => {
    const shebang = span(spans, "#!/usr/bin/env nvs");
    assert.ok(shebang.scopes.includes("comment.line.number-sign.shebang.nvs"), `${shebang.scopes}`);
    assert.ok(span(spans, "$total = 1;").scopes.includes(CODE));
  });

  it("switches to text mode on ?> like any other code block", () => {
    assert.ok(span(spans, "?>").scopes.includes(END));
    assert.equal(span(spans, "<footer>bye</footer>").scopes.includes(CODE), false);
  });
});
