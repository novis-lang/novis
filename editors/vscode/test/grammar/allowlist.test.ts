// Every scope the grammar emits, frozen as a list of standard names.
//
// `rule:ide/novis-ships-names-not-colours` is what this suite enforces. Both highlighting layers ship
// names and no colours, and a theme styles only the names it already recognises, so an invented scope
// like `keyword.nvs.spawn` renders the construct as unstyled body text — a grammar that is technically
// correct and visibly broken. The two assertions below are the two halves of that failure: a name
// whose root is outside the standard vocabulary, and a name that does not carry the `.nvs` suffix a
// theme's Novis-specific rule would select on.
//
// The list is exhaustive on purpose, and a scope in the grammar that no fixture produces fails here
// too: a construct family lands as a pattern, a fixture and a row in `ALLOWED`, in one commit.

import * as assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";

import { ROOT, caseFixtures, fixtures, tokenize, tokenizeCase } from "./tokenize";

// TextMate's standard top-level names, plus the two root scopes a document itself carries.
const STANDARD_ROOTS = [
  "comment",
  "constant",
  "entity",
  "invalid",
  "keyword",
  "markup",
  "meta",
  "punctuation",
  "source",
  "storage",
  "string",
  "support",
  "text",
  "variable",
];

const ALLOWED = [
  "source.nvs",
  "meta.embedded.block.nvs",
  "punctuation.section.embedded.begin.nvs",
  "punctuation.section.embedded.end.nvs",
  "invalid.illegal.open-tag.nvs",
  "comment.line.number-sign.shebang.nvs",
  "comment.line.double-slash.nvs",
  "comment.line.documentation.nvs",
  "comment.line.number-sign.nvs",
  "comment.block.nvs",
  "meta.attribute.nvs",
  "punctuation.definition.attribute.begin.nvs",
  "punctuation.definition.attribute.end.nvs",
  "string.quoted.single.nvs",
  "string.quoted.double.nvs",
  "string.unquoted.heredoc.nvs",
  "string.unquoted.nowdoc.nvs",
  "string.quoted.other.markup.nvs",
  "punctuation.definition.string.begin.nvs",
  "punctuation.definition.string.end.nvs",
  "constant.character.escape.nvs",
  "invalid.illegal.escape.nvs",
  "meta.embedded.line.nvs",
  "variable.other.nvs",
  "variable.other.property.nvs",
  "punctuation.accessor.nvs",
  "constant.other.nvs",
  "entity.name.type.nvs",
  "entity.name.type.class.nvs",
  "entity.name.type.interface.nvs",
  "entity.name.type.enum.nvs",
  "entity.name.function.nvs",
  "entity.name.namespace.nvs",
  "keyword.control.nvs",
  "keyword.operator.nvs",
  "keyword.other.nvs",
  "storage.type.nvs",
  "storage.type.class.nvs",
  "storage.type.function.nvs",
  "storage.modifier.nvs",
  "constant.language.nvs",
  "variable.language.nvs",
  "constant.numeric.nvs",
];

// The case grammar's own names, suffixed `.nvst` for the same reason the list above is suffixed
// `.nvs`. Everything else it emits comes from the Novis grammar embedded in a case's programs, and is
// checked against `ALLOWED` — a case is meant to look exactly like the file it holds.
const CASE_ALLOWED = [
  "source.nvst",
  "punctuation.definition.section.nvst",
  "entity.name.section.nvst",
  "string.unquoted.path.nvst",
  "constant.character.escape.nvst",
];

// The three names the case grammar borrows from PHP's, to open `--ORACLE--` with. A twin is PHP, so
// it carries what PHP carries everywhere else in the editor; the names are `text.html.php`'s own.
const CASE_BORROWED = [
  "meta.embedded.block.php",
  "punctuation.section.embedded.begin.php",
  "punctuation.section.embedded.end.php",
];

describe("the grammars the manifest contributes", () => {
  it("points VS Code at these two files, under these scope names", () => {
    const manifest = JSON.parse(readFileSync(join(ROOT, "package.json"), "utf8")) as {
      contributes: { grammars: unknown[] };
    };
    assert.deepEqual(manifest.contributes.grammars, [
      {
        language: "nvs",
        scopeName: "source.nvs",
        path: "./syntaxes/nvs.tmLanguage.json",
      },
      {
        // The embedded mapping is what gives a `--FILE--` body Novis's own comment toggling and
        // bracket behaviour; the oracle's gets none, since naming php here would claim the file
        // type `rule:ide/the-extension-claims-nvs-only` refuses.
        language: "nvst",
        scopeName: "source.nvst",
        path: "./syntaxes/nvst.tmLanguage.json",
        embeddedLanguages: { "meta.embedded.block.nvs": "nvs" },
      },
    ]);
  });
});

describe("the scopes the grammar is allowed to emit", () => {
  it("names only roots the standard vocabulary has", () => {
    for (const scope of [...ALLOWED, ...CASE_ALLOWED, ...CASE_BORROWED]) {
      const root = scope.split(".")[0];
      assert.ok(STANDARD_ROOTS.includes(root),
                `${scope} begins with ${root}, which no theme styles`);
    }
  });

  it("suffixes every one of them .nvs", () => {
    for (const scope of ALLOWED) {
      assert.ok(scope.endsWith(".nvs"), `${scope} carries no .nvs suffix`);
    }
  });

  it("suffixes the case grammar's own names .nvst", () => {
    for (const scope of CASE_ALLOWED) {
      assert.ok(scope.endsWith(".nvst"), `${scope} carries no .nvst suffix`);
    }
  });

  it("suffixes the names it borrows for an oracle .php", () => {
    for (const scope of CASE_BORROWED) {
      assert.ok(scope.endsWith(".php"), `${scope} is not a name PHP already carries`);
    }
  });
});

describe("the scopes the grammar does emit", () => {
  const emitted = new Set<string>();

  before(async () => {
    for (const [, content] of fixtures()) {
      for (const { scopes } of await tokenize(content)) {
        for (const scope of scopes) {
          emitted.add(scope);
        }
      }
    }
  });

  it("emits nothing the allowlist does not name", () => {
    for (const scope of [...emitted].sort()) {
      assert.ok(ALLOWED.includes(scope),
                `the grammar emits ${scope}, which is on no allowlist and in no theme`);
    }
  });

  it("emits every name the allowlist carries", () => {
    // A row no fixture reaches is a scope nothing checks, which is how a family lands half-written.
    for (const scope of ALLOWED) {
      assert.ok(emitted.has(scope), `no fixture under test/grammar/fixtures produces ${scope}`);
    }
  });

  it("reads every fixture as at least two spans", () => {
    // A grammar that fails to load tokenizes each line as one span carrying the root scope alone,
    // which would satisfy every assertion above by emitting nothing at all.
    for (const [name, content] of fixtures()) {
      assert.ok(content.length > 0, `${name} is empty`);
    }
    assert.ok(emitted.size > 1, "the grammar emitted only the root scope; did it load?");
  });
});

describe("the scopes the case grammar emits", () => {
  const emitted = new Set<string>();

  before(async () => {
    for (const [, content] of caseFixtures()) {
      for (const { scopes } of await tokenizeCase(content)) {
        for (const scope of scopes) {
          emitted.add(scope);
        }
      }
    }
  });

  it("emits its own names, and otherwise the Novis grammar's or PHP's", () => {
    for (const scope of [...emitted].sort()) {
      // What PHP's own grammar emits inside an oracle is PHP's business and is on no list of ours;
      // the three names below that carry `.php` are the ones this grammar writes itself.
      assert.ok(CASE_ALLOWED.includes(scope) || ALLOWED.includes(scope) || scope.endsWith(".php"),
                `the case grammar emits ${scope}, which is on no allowlist and in no theme`);
    }
  });

  it("emits every name its own allowlist carries", () => {
    for (const scope of [...CASE_ALLOWED, ...CASE_BORROWED]) {
      assert.ok(emitted.has(scope), `no fixture under test/grammar/fixtures produces ${scope}`);
    }
  });

  it("reaches the embedded grammar at all", () => {
    // `source.nvst` includes `source.nvs#code`, and an include of a grammar a registry cannot find
    // resolves to nothing rather than failing: without this, a case whose program was never coloured
    // would pass every assertion above by emitting nothing.
    assert.ok(caseFixtures().length > 0, "no .nvst fixture under test/grammar/fixtures");
    assert.ok(emitted.has("meta.embedded.block.nvs"),
              "no case fixture's program opened code mode; did source.nvs load?");
    assert.ok(emitted.has("storage.type.class.nvs"),
              "code mode opened but nothing inside it was coloured");
  });
});
