// The reserved table, the type keywords and the qualifiers — and what is only a keyword in context.
//
// `crates/nvs-syntax/src/lexer.rs:650` looks a name up exactly as it is written, so the whole of
// `rule:classes/reserved-spellings-are-lower-case` reduces to one property this suite can check: a
// mis-cased spelling is an identifier, and `class IF extends TRUE {}` is a class declaration rather
// than four keywords. The same holds one step out — a name reached through `->` or `::` is a member,
// and colouring `$order->list` as a keyword would say the language is about to do something it is not.
//
// The contextual spellings are the other half. `crates/nvs-syntax/src/parser/mod.rs:609` recognises
// them by text one position at a time, so each is coloured only beside the token that makes it a
// construct, and `rule:ide/highlighting-is-two-layers` names the ones that must reach the grammar at
// all: `spawn script`, `autoload`, `type`, `by`-delegation and the property hooks.

import * as assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";

import { FIXTURES, Span, tokenize } from "./tokenize";

const CONTROL = "keyword.control.nvs";
const OPERATOR = "keyword.operator.nvs";
const OTHER = "keyword.other.nvs";
const TYPE = "storage.type.nvs";
const CLASS = "storage.type.class.nvs";
const FUNCTION = "storage.type.function.nvs";
const MODIFIER = "storage.modifier.nvs";
const LITERAL = "constant.language.nvs";
const LANGUAGE = "variable.language.nvs";

let spans: Span[];

before(async () => {
  spans = await tokenize(readFileSync(join(FIXTURES, "keywords.nvs"), "utf8"));
});

/** Every span whose bytes are exactly `text`, of which there is at least one. */
function all(text: string): Span[] {
  const found = spans.filter((s) => s.text === text);
  assert.ok(found.length > 0, `no span reads exactly ${JSON.stringify(text)}`);
  return found;
}

/** Asserts `scope` is on every span reading `text` — a word repeats, and each one is coloured. */
function coloured(text: string, scope: string): void {
  for (const found of all(text)) {
    assert.ok(found.scopes.includes(scope), `${text} is ${found.scopes.join(" ")}, wanted ${scope}`);
  }
}

/** Asserts nothing reading `text` is coloured as a construct of any kind. */
function plain(text: string): void {
  for (const found of all(text)) {
    for (const scope of found.scopes) {
      assert.ok(!/^(keyword|storage|constant|variable|support)\./.test(scope),
                `${JSON.stringify(text)} is coloured ${scope}`);
    }
  }
}

describe("the reserved table", () => {
  it("colours control flow", () => {
    for (const word of ["if", "elseif", "else", "foreach", "continue", "return"]) {
      coloured(word, CONTROL);
    }
  });

  it("colours the word operators", () => {
    for (const word of ["as", "is"]) {
      coloured(word, OPERATOR);
    }
  });

  it("colours the statement keywords", () => {
    coloured("echo", OTHER);
    coloured("autoload", OTHER);
  });

  it("colours a declaration by what it declares", () => {
    for (const word of ["class", "interface", "enum"]) {
      coloured(word, CLASS);
    }
    for (const word of ["function", "fn"]) {
      coloured(word, FUNCTION);
    }
  });

  it("colours decimal as a scalar type beside int and float", () => {
    for (const word of ["int", "uint", "float", "decimal", "string", "bool", "const"]) {
      coloured(word, TYPE);
    }
  });

  it("colours the modifiers, and the two qualifiers among them", () => {
    for (const word of ["final", "public", "private", "protected", "static", "readonly",
                        "extends", "implements", "tainted", "secret"]) {
      coloured(word, MODIFIER);
    }
  });

  it("colours the literals and the language variables", () => {
    coloured("null", LITERAL);
    coloured("true", LITERAL);
    coloured("self", LANGUAGE);
    coloured("parent", LANGUAGE);
  });
});

describe("what is a keyword only where it is written", () => {
  it("reads a mis-cased spelling as the identifier it is", () => {
    // `class IF extends TRUE {}` is a class declaration, and both names are legal ones.
    plain(" IF ");
    plain(" TRUE {}");
  });

  it("reads a name after -> or :: as a member rather than a construct", () => {
    plain("$member = $order->list;");
    plain("$constant = Order::class;");
  });

  it("reads a capitalized type name as a name", () => {
    plain(" Iterable ");
  });
});

describe("the contextual spellings, each beside what makes it one", () => {
  it("colours spawn only where script follows it", () => {
    coloured("spawn", CONTROL);
    coloured("script", CONTROL);
  });

  it("colours a type alias, a delegation and the autoload roots", () => {
    coloured("type", TYPE);
    coloured("by", OTHER);
    coloured("from", OTHER);
    coloured("discover", OTHER);
  });

  it("colours a property hook and the visibility that names one", () => {
    coloured("get", OTHER);
    coloured("set", OTHER);
  });

  it("colours await before an operand", () => {
    coloured("await", CONTROL);
  });
});
