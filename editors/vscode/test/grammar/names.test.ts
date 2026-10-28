// Every name the grammar colours, and the token beside it that made it one.
//
// A name carries no colour of its own: `Order` is a class name because `class` stands in front of it,
// `subtotal` is a member because `->` does, `plus` is a method because a `(` follows it, `Ledger` is
// a type because `::` follows it, and `$order` is a variable because `$` does. So every assertion
// here is about a pair, and the pair is what keeps the grammar inside
// `rule:ide/highlighting-is-two-layers` — it colours a name for the accessor or the declaration it is
// written beside, and leaves to layer two everything a regex cannot see: whether a member nobody
// calls is a property, a constant or an enum case, and whether the value in a variable is `tainted`
// or `secret`.
//
// `rule:ide/novis-ships-names-not-colours` is why the scopes are the standard ones and the allowlist
// suite is where that is enforced. A member keeps the scope the string family already uses for
// `$a->b`, so an interpolated member and a member in code colour alike, and a method takes the scope
// VS Code maps layer two's `method` to, so a theme colours it the same once the server answers.

import * as assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";

import { FIXTURES, Span, tokenize } from "./tokenize";

const CLASS_NAME = "entity.name.type.class.nvs";
const INTERFACE_NAME = "entity.name.type.interface.nvs";
const ENUM_NAME = "entity.name.type.enum.nvs";
const FUNCTION_NAME = "entity.name.function.nvs";
const METHOD_NAME = "entity.name.function.member.nvs";
const NAMESPACE_NAME = "entity.name.namespace.nvs";
const VARIABLE = "variable.other.nvs";
const LANGUAGE = "variable.language.nvs";
const PROPERTY = "variable.other.property.nvs";
const ACCESSOR = "punctuation.accessor.nvs";
const SLOT = "meta.embedded.line.nvs";
const CLASS_KEYWORD = "storage.type.class.nvs";
const FUNCTION_KEYWORD = "storage.type.function.nvs";
const OTHER = "keyword.other.nvs";
const MODIFIER = "storage.modifier.nvs";
const TYPE_NAME = "entity.name.type.nvs";
const INHERITED = "entity.other.inherited-class.nvs";

let spans: Span[];

before(async () => {
  spans = await tokenize(readFileSync(join(FIXTURES, "names.nvs"), "utf8"));
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

describe("the name a declaration introduces", () => {
  it("colours it by what declares it, and the keyword beside it as itself", () => {
    coloured("Order", CLASS_NAME);
    coloured("Reader", INTERFACE_NAME);
    coloured("Suit", ENUM_NAME);
    coloured("total", FUNCTION_NAME);
    coloured("checkout", FUNCTION_NAME);
    coloured("App\\Orders", NAMESPACE_NAME);
    for (const word of ["class", "interface", "enum"]) {
      assert.ok(all(word).some((s) => s.scopes.includes(CLASS_KEYWORD)),
                `${word} lost its own scope to the name it introduces`);
    }
    coloured("function", FUNCTION_KEYWORD);
  });

  it("colours an enum a class body declares as one declared at file scope", () => {
    // `rule:enums/class-scoped-enum`: the class body has no scope of its own, so the one pattern
    // that colours `enum Suit` colours `enum Status` inside `Order` too.
    assert.deepEqual(carrying(ENUM_NAME), ["Status", "Suit"]);
  });

  it("colours a name as a declaration only where something declares it", () => {
    assert.deepEqual(carrying(CLASS_NAME), ["Order"]);
    assert.deepEqual(carrying(FUNCTION_NAME), ["total", "checkout"]);
  });

  it("colours no name where an anonymous function introduces none", () => {
    // `function (` and `fn(` declare nothing, so the reserved table colours the keyword alone and
    // what follows it is a parameter rather than the name of anything.
    coloured("fn", FUNCTION_KEYWORD);
    coloured("$amount", VARIABLE);
    assert.equal(carrying(FUNCTION_NAME).length, 2, "an anonymous function was read as a declaration");
  });
});

describe("the names a declaration builds on, and the path an import names", () => {
  it("colours every name after extends or implements, over as many lines as the list takes", () => {
    // Both words are reserved and stand nowhere else, so the word before the list says what it is.
    assert.deepEqual(carrying(INHERITED),
                     ["Base", "\\App\\Shared\\Priced", "Feed", "Countable", "Sized"]);
    for (const word of ["extends", "implements"]) {
      for (const found of all(word)) {
        assert.ok(found.scopes.includes(MODIFIER), `${word} lost its own scope to the list`);
      }
    }
  });

  it("colours a type argument as a type, and a delegated field as a property", () => {
    coloured("Line", TYPE_NAME);
    coloured("by", OTHER);
    coloured("$feed", PROPERTY);
  });

  it("colours an import's whole path as a type", () => {
    coloured("App\\Shared\\Base", TYPE_NAME);
    assert.ok(all("use").every((s) => s.scopes.includes(OTHER)), "use lost its keyword scope");
  });
});

describe("the name an accessor reaches", () => {
  it("colours a member the same whichever accessor reaches it", () => {
    for (const member of ["subtotal", "RATE", "LIMIT", "rounded", "label"]) {
      coloured(member, PROPERTY);
    }
    for (const accessor of ["->", "::", "?->"]) {
      coloured(accessor, ACCESSOR);
    }
  });

  it("colours a member that is called as a method, and every lowercase name after :: too", () => {
    // `->convert<Line>(` is a call with a type argument before the `(`, and `Ledger::compare` is a
    // method reference: the parser reads a lowercase name after `::` as a method, called or not.
    assert.deepEqual(carrying(METHOD_NAME), ["of", "convert", "compare", "reset", "plus"]);
    for (const method of carrying(METHOD_NAME)) {
      assert.equal(all(method).some((s) => s.scopes.includes(PROPERTY)), false,
                   `${method} is also coloured as a property`);
    }
    coloured("Line", TYPE_NAME);
  });

  it("colours the name before :: as a type, and leaves self and parent to the reserved table", () => {
    assert.deepEqual(carrying(TYPE_NAME).filter((name) => name !== "Line" && !name.includes("Base")),
                     ["App\\Shared\\Money", "Ledger", "Ledger"]);
    coloured("self", LANGUAGE);
    coloured("parent", LANGUAGE);
  });

  it("reads ::class as the class-name constant rather than a member", () => {
    // `$obj::class` answers the class the receiver is, so it is a construct and not a name at all.
    const constant = all("class").filter((s) => s.scopes.includes(OTHER));
    assert.equal(constant.length, 1, "the class-name constant is coloured as a member");
    assert.equal(constant[0].scopes.includes(PROPERTY), false);
  });
});

describe("the variables", () => {
  it("colours the receiver as the language's own and every other as a local", () => {
    coloured("$this", LANGUAGE);
    assert.equal(all("$this")[0].scopes.includes(VARIABLE), false, "$this is an ordinary local");
    for (const name of ["$label", "$sum", "$rate", "$order", "$anon", "$x"]) {
      coloured(name, VARIABLE);
    }
  });

  it("reaches inside an interpolation slot, which holds code", () => {
    const inside = spans.filter((s) => s.text === "$order" && s.scopes.includes(SLOT));
    assert.equal(inside.length, 1, "the slot in the double-quoted string holds no variable");
    assert.ok(inside[0].scopes.includes(VARIABLE));
    assert.ok(inside[0].scopes.includes("string.quoted.double.nvs"), "the slot escaped its string");
  });
});
