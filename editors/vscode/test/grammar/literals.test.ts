// Numbers, and the durations that share their opening digits.
//
// `rule:types/duration` is one grammar with four properties a regex can carry: a duration is
// one token however many units it names, its units are lower case, they descend strictly and none
// repeats. The last three are why the spellings the lexer rejects are here beside the ones it takes —
// `30m1h` and `30S` are diagnostics, and `rule:ide/rejected-syntax-gets-no-colour` is what stops the
// editor confirming them as constants before the server says otherwise.
//
// `0x1d` and `1.5s` are the two shapes that hide a unit inside a literal that is not a duration at
// all: `rule:types/duration` reaches only a plain decimal integer.

import * as assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";

import { FIXTURES, Span, span, tokenize } from "./tokenize";

const NUMERIC = "constant.numeric.nvs";

let spans: Span[];

before(async () => {
  spans = await tokenize(readFileSync(join(FIXTURES, "literals.nvs"), "utf8"));
});

/** Asserts `text` is one whole span and carries the numeric scope. */
function literal(text: string): void {
  assert.ok(span(spans, text).scopes.includes(NUMERIC), `${text} is not a numeric literal`);
}

/** Asserts nothing reads `text` as one span, which is how a rejected spelling fails here. */
function unread(text: string): void {
  assert.equal(spans.filter((s) => s.text === text).length, 0,
               `${text} tokenized as one span, so something colours it whole`);
}

describe("a duration literal, which is one token", () => {
  it("reads a single unit and a run of them as one span each", () => {
    literal("30s");
    literal("1h30m");
    literal("250ms");
    literal("2w3d4h5m6s7ms8us9ns");
  });

  it("refuses a run whose units ascend or repeat", () => {
    unread("30m1h");
    unread("1h1h");
  });

  it("refuses an upper-case unit, which is a diagnostic and not a second spelling", () => {
    unread("30S");
  });

  it("reaches only a plain decimal integer", () => {
    // `0x1d` is one hex literal and `1.5s` is a float the lexer refuses a unit on, so neither
    // holds a `1d` or a `5s` for this pattern to find.
    literal("0x1d");
    literal("1.5");
    unread("1.5s");
  });

  it("still colours the integer a refused duration starts with", () => {
    for (const found of spans.filter((s) => s.text === "30")) {
      assert.ok(found.scopes.includes(NUMERIC), `${found.scopes.join(" ")}`);
    }
  });
});

describe("the ordinary numeric literals", () => {
  it("reads decimal, grouped, fractional and exponent forms", () => {
    literal("42");
    literal("1_000");
    literal("3.14");
    literal("1e9");
  });

  it("reads the three radix prefixes", () => {
    literal("0x1d");
    literal("0b1010");
    literal("0o777");
  });
});
