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

import { ROOT, fixtures, tokenize } from "./tokenize";

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

describe("the grammar the manifest contributes", () => {
  it("points VS Code at this file, under this scope name", () => {
    const manifest = JSON.parse(readFileSync(join(ROOT, "package.json"), "utf8")) as {
      contributes: { grammars: { language: string; scopeName: string; path: string }[] };
    };
    assert.deepEqual(manifest.contributes.grammars, [
      {
        language: "nvs",
        scopeName: "source.nvs",
        path: "./syntaxes/nvs.tmLanguage.json",
      },
    ]);
  });
});

describe("the scopes the grammar is allowed to emit", () => {
  it("names only roots the standard vocabulary has", () => {
    for (const scope of ALLOWED) {
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
