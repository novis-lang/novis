// The case grammar: what a `.nvst` or `.lspt` file looks like when it opens.
//
// `rule:ide/case-files-have-their-own-grammar` is what this suite pins. The claim is narrow — the
// section headers are coloured, the four sections that hold a program are Novis, and every other
// section is literal bytes with only its delimiter coloured — and the one thing that is easy to get
// wrong is where a program stops. A case almost never closes with `?>`, so a code-mode rule that
// ended only there would colour the expectation, the oracle and every section after them as code;
// the fixture is written with an unclosed `--FILE--` on purpose, so that failure would show up here
// rather than in an editor.

import * as assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";

import { FIXTURES, Span, tokenizeCase } from "./tokenize";

const PUNCTUATION = "punctuation.definition.section.nvst";
const NAME = "entity.name.section.nvst";
const PATH = "string.unquoted.path.nvst";
const ESCAPE = "constant.character.escape.nvst";
const EMBEDDED = "meta.embedded.block.nvs";
const OPEN = "punctuation.section.embedded.begin.nvs";
const CLOSE = "punctuation.section.embedded.end.nvs";
const PHP = "meta.embedded.block.php";
const PHP_OPEN = "punctuation.section.embedded.begin.php";
const PHP_CLOSE = "punctuation.section.embedded.end.php";

let spans: Span[] = [];

before(async () => {
  spans = await tokenizeCase(readFileSync(join(FIXTURES, "case.nvst"), "utf8"));
});

/** The bytes of every span carrying `scope`, in the order the file writes them. */
function carrying(scope: string): string[] {
  return spans.filter((s) => s.scopes.includes(scope)).map((s) => s.text);
}

/** Every span reading exactly `text` from inside a program, and every one from outside every program. */
function inProgram(text: string): Span[] {
  return spans.filter((s) => s.text === text && s.scopes.includes(EMBEDDED));
}

function outsideProgram(text: string): Span[] {
  return spans.filter((s) => s.text === text && !s.scopes.includes(EMBEDDED));
}

describe("the sections a case is made of", () => {
  it("names every header, in the order the file writes them", () => {
    assert.deepEqual(carrying(NAME), [
      "TEST",
      "SKIPIF",
      "FILE",
      "FILE",
      "ARGS",
      "ENV",
      "EXPECT",
      "EXPECTF",
      "ORACLE",
      "CLEAN",
    ]);
  });

  it("colours the delimiter of each and nothing else on its line", () => {
    const delimiters = carrying(PUNCTUATION);
    assert.equal(delimiters.length, 20, "ten headers, opened and closed");
    for (const text of delimiters) {
      assert.equal(text, "--");
    }
  });

  it("reads the path a --FILE <path>-- header carries", () => {
    assert.deepEqual(carrying(PATH), ["src/Bootstrap.nvs"]);
  });

  it("reads a line beginning -- inside a section as its content", () => {
    // `--ARGS--`'s body is `--first`, which is an argument and not a header: the roster is closed and
    // a section ends at `--` followed by a capital, so a lower-case one is bytes like any other.
    const found = outsideProgram("--first");
    assert.equal(found.length, 1);
    assert.deepEqual(found[0].scopes, ["source.nvst"]);
  });
});

describe("the four sections that hold a program", () => {
  it("opens code mode in each of them", () => {
    assert.deepEqual(carrying(OPEN), ["<?nvs", "<?nvs", "<?nvs", "<?nvs"]);
  });

  it("colours what is inside with the language's own patterns", () => {
    const table: [string, string][] = [
      ["class", "storage.type.class.nvs"],
      ["function", "storage.type.function.nvs"],
      ["boot", "entity.name.function.nvs"],
    ];
    for (const [text, scope] of table) {
      const found = inProgram(text);
      assert.equal(found.length, 1, `${found.length} spans read ${text} inside a program`);
      assert.ok(found[0].scopes.includes(scope), `${text} carries ${found[0].scopes.join(" ")}`);
    }
  });

  it("reads 41 as a number in a program and as bytes in an expectation", () => {
    assert.ok(inProgram("41")[0].scopes.includes("constant.numeric.nvs"));
    assert.deepEqual(outsideProgram("41")[0].scopes, ["source.nvst"]);
  });

  it("leaves every other section literal", () => {
    // Outside a program, the only thing carrying a scope of its own is a header: an expectation is
    // compared byte for byte, and colouring part of one would say the runner reads it as something.
    const outside = new Set<string>();
    for (const found of spans) {
      if (!found.scopes.includes(EMBEDDED) && !found.scopes.includes(PHP)) {
        for (const scope of found.scopes) {
          outside.add(scope);
        }
      }
    }
    assert.deepEqual([...outside].sort(), [NAME, PUNCTUATION, PATH, ESCAPE, "source.nvst"].sort());
  });
});

describe("the two sections that are neither a program nor literal bytes", () => {
  it("colours the escapes an --EXPECTF-- may write, and the % that stands for itself", () => {
    // `crates/nvs-test/src/expect.rs` is the roster: twelve spellings, `%%` among them, and nothing
    // else in the section means anything.
    assert.deepEqual(carrying(ESCAPE), ["%f", "%%", "%s"]);
  });

  it("reads an --ORACLE-- as PHP, opened and closed by PHP's own names", () => {
    assert.deepEqual(carrying(PHP_OPEN), ["<?php"]);
    assert.deepEqual(carrying(PHP_CLOSE), ["?>"]);
    // And the body reaches `source.php` itself. The registry here holds a stub of it — one keyword —
    // because a rule whose every pattern is an include nothing resolves is dropped along with the
    // rule that included it, which would take the whole section rather than just its colour.
    const echo = spans.filter((s) => s.text === "echo" && s.scopes.includes(PHP));
    assert.equal(echo.length, 1);
    assert.ok(echo[0].scopes.includes("keyword.other.php"), echo[0].scopes.join(" "));
  });

  it("keeps the oracle's tag out of the Novis openers", () => {
    for (const found of spans) {
      assert.ok(!(found.scopes.includes(PHP) && found.scopes.includes(EMBEDDED)),
                `${found.text} is inside both grammars at once`);
    }
  });
});

describe("where a program stops", () => {
  it("ends at the next header when no closing tag is written", () => {
    // `--FILE--` in the fixture never writes `?>`, so this is the assertion that the section after it
    // is a section rather than more code.
    for (const found of spans) {
      assert.ok(!(found.scopes.includes(NAME) && found.scopes.includes(EMBEDDED)),
                `${found.text} is a header read from inside a program`);
    }
  });

  it("ends at the closing tag where one is written", () => {
    assert.deepEqual(carrying(CLOSE), ["?>"]);
  });
});
