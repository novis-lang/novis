// The constructs a PHP grammar colours and Novis refuses.
//
// `rule:ide/rejected-syntax-gets-no-colour` is the whole of this suite: a grammar that colours what
// the language rejects confirms the mistake in the editor before the server contradicts it, which is
// worse than leaving it plain. Four of them are named there, and each is refused by a rule of its own
// — `===` and `!==` by `rule:expressions/one-equality-operator`, `(int)$x` by
// `rule:types/no-legacy-cast`, and the alternative colon syntax by not being syntax at all.
//
// Three of the four need no pattern: the grammar colours no operator and knows no `endif`, so they
// are plain because nothing claims them, and this suite is what keeps them that way when the operator
// family lands. The legacy cast is the one that needs refusing on purpose, because `int` is a type
// keyword the grammar does colour — everywhere it means one.
//
// `|>` is not here as a refusal. It is a Novis operator (`rule:expressions/pipeline-substitution`),
// and what the record refuses is reading it as PHP 8.5's; a grammar that colours no operator says
// nothing either way, so the assertion is that it stays that way rather than borrowing the meaning.

import * as assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";

import { FIXTURES, Span, span, tokenize } from "./tokenize";

const TYPE = "storage.type.nvs";
const CONTROL = "keyword.control.nvs";

let spans: Span[];

before(async () => {
  spans = await tokenize(readFileSync(join(FIXTURES, "rejected.nvs"), "utf8"));
});

/** Asserts every span whose bytes hold `text` is coloured as no construct of any kind. */
function refused(text: string): void {
  const found = spans.filter((s) => s.text.includes(text));
  assert.ok(found.length > 0, `no span holds ${JSON.stringify(text)}`);
  for (const s of found) {
    for (const scope of s.scopes) {
      assert.ok(!/^(entity|keyword|storage|constant|variable|support)\./.test(scope),
                `${JSON.stringify(s.text)} is coloured ${scope}`);
    }
  }
}

describe("the operators Novis has no spelling of", () => {
  it("colours neither === nor !== nor <>", () => {
    for (const operator of [" === ", " !== ", " <> "]) {
      refused(operator);
    }
  });

  it("colours |> as nothing, so it borrows no meaning from PHP's", () => {
    refused("|>");
  });
});

describe("the legacy cast, which the parser refuses by name", () => {
  it("colours the keyword in none of the seven spellings", () => {
    for (const cast of ["(int)", "(string)", "(bool)"]) {
      refused(cast);
    }
  });

  it("leaves the same keyword its colour where it is a type", () => {
    // `$raw as int` is the one conversion spelling, and `int` there means what it always means.
    assert.ok(span(spans, "int").scopes.includes(TYPE), "as int lost the type keyword's colour");
  });
});

describe("the alternative colon syntax, which is no syntax at all", () => {
  it("colours neither the colon nor the word that closes the block", () => {
    for (const text of ["):", "endif;", "endforeach;"]) {
      refused(text);
    }
  });

  it("colours the keyword that opens it, which is a keyword either way", () => {
    // `if` and `foreach` are the language's own; only the body spelling after them is not.
    for (const word of ["if", "foreach"]) {
      assert.ok(span(spans, word).scopes.includes(CONTROL), `${word} is not a keyword here`);
    }
  });
});
