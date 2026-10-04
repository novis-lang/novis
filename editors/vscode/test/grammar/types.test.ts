// Every slot that carries a written type, and the token beside it that says a type is what stands there.
//
// `rule:types/declaration` makes an annotation mandatory at every binding site, and PHP has syntax for
// almost none of them — which is why `rule:ide/highlighting-is-two-layers` names this family as the
// single largest divergence from any borrowed PHP grammar. The scalar words already colour from the
// reserved table wherever they stand, so what this suite is about is the *positions*: a bare name is a
// type because a `$` follows it, because a `<` opened an argument list in front of it, because a
// second name and an `=` follow it, or because the region it is in began at a signature's `):`.
//
// Two of the slots are deliberately narrower than the grammar, and both are pinned below. A ternary's
// colon and a return type's are one spelling, so the return type is read only inside a signature; an
// anonymous object and an inline shape (`rule:types/object-top`) are one spelling too, so a shape is
// read only where a region has already established a type. Colouring either of them by the colon alone
// would confirm a construct that is not there, which is `rule:ide/rejected-syntax-gets-no-colour`'s
// claim applied to a position rather than to a spelling.

import * as assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";

import { FIXTURES, Span, tokenize } from "./tokenize";

const TYPE_NAME = "entity.name.type.nvs";
const FUNCTION_NAME = "entity.name.function.nvs";
const PROPERTY = "variable.other.property.nvs";
const VARIABLE = "variable.other.nvs";
const TYPE = "storage.type.nvs";
const MODIFIER = "storage.modifier.nvs";

let spans: Span[];

before(async () => {
  spans = await tokenize(readFileSync(join(FIXTURES, "types.nvs"), "utf8"));
});

/** Every span whose bytes are exactly `text`, of which there is at least one. */
function all(text: string): Span[] {
  const found = spans.filter((s) => s.text === text);
  assert.ok(found.length > 0, `no span reads exactly ${JSON.stringify(text)}`);
  return found;
}

/** Asserts `scope` is on every span reading `text`. */
function coloured(text: string, scope: string): void {
  for (const found of all(text)) {
    assert.ok(found.scopes.includes(scope), `${text} is ${found.scopes.join(" ")}, wanted ${scope}`);
  }
}

/** The bytes of every span carrying `scope`, in file order. */
function carrying(scope: string): string[] {
  return spans.filter((s) => s.scopes.includes(scope)).map((s) => s.text);
}

/** Asserts no span reads exactly `text` — an uncoloured word is part of a longer plain run. */
function uncoloured(...texts: string[]): void {
  for (const text of texts) {
    assert.equal(spans.filter((s) => s.text === text).length, 0,
                 `${JSON.stringify(text)} was coloured as something`);
  }
}

describe("the name a type slot holds", () => {
  it("colours one in every slot, and nothing that is not in one", () => {
    // In file order: a class constant's type, a property's, a nullable property's, a type argument,
    // two parameters and a return, a typed local, a `foreach` binding, a union member, a shape field,
    // a generic return and its argument, the name an alias introduces and the type it stands for, and
    // an anonymous function's parameter and return.
    assert.deepEqual(carrying(TYPE_NAME), [
      "Currency", "Token", "Customer", "Line",
      "Money", "Card", "Receipt",
      "Ledger", "Line",
      "Rounding", "Money",
      "Iterator", "Row",
      "Line", "Money", "Money",
      "Totals", "Money",
      "Line", "Money",
    ]);
  });

  it("leaves the reserved table to the scalars standing in the same slots", () => {
    // `int` in `const int SCALE` and `decimal` in a return and a shape field are types too; they are
    // the table's, because `rule:classes/reserved-spellings-are-lower-case` already decided the word.
    for (const word of ["int", "decimal", "array"]) {
      coloured(word, TYPE);
    }
    coloured("secret", MODIFIER);
  });

  it("colours the type a class constant declares, not the class it is initialised from", () => {
    // `public const Currency DEFAULT = Currency::Eur;` writes the same name twice, and only the first
    // of them is in a type slot: the second is a receiver, and what it names is layer two's to say.
    // So the name is one span and not two — the receiver stays inside the plain run around it, which
    // is how every word this grammar leaves alone reaches the theme.
    const written = all("Currency");
    assert.equal(written.length, 1, "the receiver of `::` was coloured as a type");
    assert.ok(written[0].scopes.includes(TYPE_NAME));
    uncoloured("DEFAULT", "SCALE");
  });

  it("colours the name an alias introduces, and reads its right-hand side as a type", () => {
    // `type Totals = {gross: Money, fee: decimal};` — `type` is a contextual spelling and the name
    // and the `=` are what make this one a declaration, so the alias carries both and the shape
    // after it colours exactly as the same spelling does in the return position two blocks above.
    coloured("type", TYPE);
    coloured("Totals", TYPE_NAME);
    coloured("gross", PROPERTY);
  });

  it("colours the name a signature declares, and none where an anonymous function declares none", () => {
    assert.deepEqual(carrying(FUNCTION_NAME), ["charge", "rate", "shape", "rows", "map"]);
    coloured("$line", VARIABLE);
  });

  it("gives a parameter back its variable where the type before it is a signature", () => {
    // `callable(Line): Money $fn` closes a paren and writes a colon in the middle of a parameter
    // list, so the return-type region opens on a type that is not one. It stops at the `$` — a
    // return type holds no variable — and the binding is read as the code it is rather than swallowed.
    coloured("$fn", VARIABLE);
  });
});

describe("the two slots a regex cannot have", () => {
  it("reads a ternary's colon as the operator it is", () => {
    // `$ready ? first($rows) : rest($rows)` puts a `)` and a `:` side by side outside any signature,
    // which is the shape a return type anchored on the colon alone would colour as a type name.
    uncoloured("first", "rest");
  });

  it("reads an anonymous object's fields as the values they are", () => {
    // `{sum: $this->net, cut: 0}` is written exactly as the shape type two lines above it, so the
    // shape is recognised only inside a region that has already established a type stands there.
    uncoloured("sum", "cut");
    coloured("net", PROPERTY);
    coloured("tax", PROPERTY);
  });

  it("colours no name that a declaration only mentions", () => {
    // `implements Iterable` names a type this file does not introduce, and resolving it is layer
    // two's, which is the same call `#names` makes for the `extends` clause.
    uncoloured("Iterable");
  });
});
