// What the editor actually shows, asserted as data: which files wake this extension, the scopes the
// grammar assigns before any server has started, and the semantic tokens that arrive once one has.
//
// Both layers are read through commands rather than pixels, per the goal's § *Standing decisions*:
// `_workbench.captureSyntaxTokens` is the command VS Code's own colorize tests use, and
// `vscode.provideDocumentSemanticTokens` is the API command for layer two. A screenshot would assert the
// theme as much as the grammar, and a theme is not what `rule:ide/highlighting-is-two-layers` is about.
//
// "Before the server answers" is the server withheld, never raced: the throwaway profile
// `scripts/host.mjs` writes starts with `nvs.lsp.enable` false, so the first tests here run against a
// client that has not spawned anything, and the third turns it on.

import * as assert from "node:assert/strict";
import * as vscode from "vscode";
import { Session } from "../protocol/session";
import { ID, fixture, open, until } from "./editor";

/** One span of `_workbench.captureSyntaxTokens`: the text it covers, and its scopes, space separated. */
interface Span {
  c: string;
  t: string;
}

/** A semantic token with its legend entries resolved, which is the form an assertion can read. */
interface Token {
  range: vscode.Range;
  type: string;
  modifiers: string[];
}

/**
 * What an editor that has not activated this extension is showing, for a wait about to give up.
 *
 * The three states that end this wait short are all here: an editor with nothing open, one holding a
 * document some other extension claimed, and one holding a Novis document under an extension that
 * stayed asleep anyway. A ledger carrying a bare deadline cannot be told which of them it was.
 */
function activation(extension: vscode.Extension<unknown>): string {
  const document = vscode.window.activeTextEditor?.document;
  if (document === undefined) {
    return "no editor is showing a document";
  }
  return `the editor holds ${document.uri.path.split("/").pop()} as language `
    + `\`${document.languageId}\`, and ${ID} is ${extension.isActive ? "active" : "asleep"}`;
}

async function captured(uri: vscode.Uri): Promise<Span[]> {
  const spans = await vscode.commands.executeCommand<Span[]>("_workbench.captureSyntaxTokens", uri);
  // A build without the command answers `undefined` rather than failing, which would otherwise read as
  // a file with no colour in it. Pinning a build that has it is `scripts/host.mjs`'s note on `VERSION`.
  assert.ok(Array.isArray(spans) && spans.length > 0,
            `_workbench.captureSyntaxTokens answered nothing for ${uri.fsPath}`);
  return spans;
}

/** That `text` is somewhere inside a span carrying `scope`, which is what "coloured as" means here. */
function coloured(spans: Span[], text: string, scope: string): void {
  const holding = spans.filter((span) => span.c.includes(text));
  assert.ok(holding.length > 0, `no span holds ${text}`);
  assert.ok(holding.some((span) => span.t.split(" ").includes(scope)),
            `${text} carries ${holding.map((span) => span.t).join(" | ")}, and not ${scope}`);
}

// The wire form is a flat run of five-number groups, each relative to the one before it
// (`crates/nvs-lsp/src/semantic.rs`). Resolving it here against the legend the editor holds is what
// makes an assertion below name a modifier rather than a bit.
function decode(tokens: vscode.SemanticTokens, legend: vscode.SemanticTokensLegend): Token[] {
  const out: Token[] = [];
  let line = 0;
  let start = 0;
  for (let at = 0; at + 4 < tokens.data.length; at += 5) {
    const [downwards, along, length, type, bits] = tokens.data.slice(at, at + 5);
    line += downwards;
    start = downwards === 0 ? start + along : along;
    out.push({
      range: new vscode.Range(line, start, line, start + length),
      type: legend.tokenTypes[type] ?? `type ${type}`,
      modifiers: legend.tokenModifiers.filter((_, bit) => (bits & (1 << bit)) !== 0)
    });
  }
  return out;
}

async function semantic(document: vscode.TextDocument): Promise<vscode.SemanticTokens | undefined> {
  return vscode.commands.executeCommand<vscode.SemanticTokens | undefined>(
    "vscode.provideDocumentSemanticTokens", document.uri);
}

describe("colour", () => {
  it("activates on a .nvs file and not on a .php file", async () => {
    const extension = vscode.extensions.getExtension(ID);
    assert.ok(extension, `${ID} is not installed in this editor`);

    // Novis parses PHP and this extension still does not claim it
    // (`rule:ide/the-extension-claims-nvs-only`): a `.php` file is somebody else's, so opening one
    // leaves us asleep and leaves the document's language alone.
    const other = await open("other.php");
    assert.notEqual(other.languageId, "nvs", "a .php file was given the nvs language");
    assert.equal(extension.isActive, false, `${ID} activated on a .php file`);

    await open("app.nvs");
    await until(`${ID} activating on a .nvs file`, async () => extension.isActive || undefined,
                () => activation(extension));
  });

  it("colours a file from the grammar before the server answers", async () => {
    assert.equal(vscode.workspace.getConfiguration("nvs").get<boolean>("lsp.enable"), false,
                 "the profile was written with the server on, so this is not the pre-server state");

    // Layer one, which is what a file looks like the instant it opens: a regex knows `secret` is a
    // qualifier from the word alone, and that `$total` is a variable from its `$`.
    const spans = await captured(fixture("app.nvs"));
    coloured(spans, "secret", "storage.modifier.nvs");
    coloured(spans, "$total", "variable.other.nvs");

    const document = await open("app.nvs");
    assert.equal(await semantic(document), undefined,
                 "semantic tokens arrived with nvs.lsp.enable false: something spawned a server");
  });

  it("adds semantic colour once the server answers", async () => {
    // The setting is respawning (`src/extension.ts:64`), so writing it starts the server; the write
    // lands in the throwaway profile's own settings file and nowhere else.
    await vscode.workspace.getConfiguration().update("nvs.lsp.enable", true, vscode.ConfigurationTarget.Global);
    const document = await open("app.nvs");
    // The two ways this wait ends short read the same from outside it and mean different things: no
    // provider is registered at all, which is a client that never spawned a server, and a provider
    // that answered an empty set, which is a server that answered the wrong document. The last
    // answer is kept so the failure can say which.
    let last = "the token provider was never asked";
    const answer = await until("nvs lsp answering semantic tokens", async () => {
      const tokens = await semantic(document);
      last = tokens === undefined
        ? "no semantic token provider is registered for this document"
        : `the provider answered ${tokens.data.length / 5} token(s)`;
      return tokens !== undefined && tokens.data.length > 0 ? tokens : undefined;
    }, () => `${last}, with nvs.lsp.enable `
             + `${vscode.workspace.getConfiguration("nvs").get<boolean>("lsp.enable")}`);

    const legend = await vscode.commands.executeCommand<vscode.SemanticTokensLegend | undefined>(
      "vscode.provideDocumentSemanticTokensLegend", document.uri);
    assert.ok(legend, "a server that answered tokens declared no legend to read them by");
    const tokens = decode(answer, legend);

    // Layer two is where the compiler colours what the regex cannot see: that `$password` *holds* a
    // secret at every use of it, and that `$total`, three lines down, does not
    // (`rule:ide/semantic-tokens-carry-the-qualifiers`).
    const password = tokens.filter((token) => document.getText(token.range) === "$password");
    assert.ok(password.length > 1, "the secret binding was tokenized at its declaration alone, or not at all");
    for (const token of password) {
      assert.ok(token.modifiers.includes("secret"),
                `$password at line ${token.range.start.line} carries [${token.modifiers.join(", ")}]`);
    }
    for (const token of tokens.filter((token) => document.getText(token.range) === "$total")) {
      assert.deepEqual(token.modifiers, [], "a plain local came back qualified");
    }
  });

  it("registers the legend the server declares", async () => {
    const document = await open("app.nvs");
    const registered = await vscode.commands.executeCommand<vscode.SemanticTokensLegend | undefined>(
      "vscode.provideDocumentSemanticTokensLegend", document.uri);
    assert.ok(registered, "no legend is registered for a .nvs document");

    // The other half is read off the wire, from a second `nvs lsp` spoken to directly: an index is
    // meaningless without the list it indexes, and a client one entry out of step colours every token
    // as its neighbour while both sides pass their own tests
    // (`rule:ide/semantic-tokens-carry-the-qualifiers`). `NVS_BIN` is pointed at the binary the profile
    // gave the client, so the comparison is between two runs of one server rather than two servers.
    process.env.NVS_BIN = process.env.NVS_HOST_NVS;
    const { session, declared } = await Session.start();
    try {
      const declaredLegend = declared.capabilities.semanticTokensProvider?.legend;
      assert.ok(declaredLegend, "the server declared no semantic tokens provider");
      assert.deepEqual(registered.tokenTypes, declaredLegend.tokenTypes);
      assert.deepEqual(registered.tokenModifiers, declaredLegend.tokenModifiers);
    } finally {
      // This session exists to read one handshake, so it is ended rather than shut down: a failed
      // assertion above must still leave no process behind.
      session.kill();
    }
  });

  it("opens a .nvst case coloured", async () => {
    // The second grammar, which is the one this repository's own authors read most
    // (`rule:ide/case-files-have-their-own-grammar`): the headers are structure, and what is inside
    // `--FILE--` is Novis, carrying the embedding scope as well as the scopes it would have on its own.
    const spans = await captured(fixture("case.nvst"));
    coloured(spans, "--", "punctuation.definition.section.nvst");
    coloured(spans, "FILE", "entity.name.section.nvst");
    coloured(spans, "secret", "meta.embedded.block.nvs");
    coloured(spans, "secret", "storage.modifier.nvs");
    coloured(spans, "$total", "variable.other.nvs");
  });

  it("selects $total whole", async () => {
    // A double-click asks the editor for the word, and the editor answers from the `wordPattern` in
    // `language-configuration.json`. Asserting it here rather than against the regex is the point: a
    // pattern the editor declines to apply selects `total` and leaves the `$` behind, which is the
    // failure a unit test over the regex alone cannot see.
    const document = await open("app.nvs");
    const inside = document.positionAt(document.getText().indexOf("$total") + 3);
    const word = document.getWordRangeAtPosition(inside);
    assert.ok(word, "the editor found no word inside $total");
    assert.equal(document.getText(word), "$total");
  });
});
