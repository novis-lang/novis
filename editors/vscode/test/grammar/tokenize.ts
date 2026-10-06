// Tokenizing a fixture the way an editor does, without an editor.
//
// `vscode-textmate` and `vscode-oniguruma` are the two libraries VS Code itself tokenizes with, so a
// scope this file reports is the scope a theme is handed. Both are devDependencies:
// `rule:ide/dependencies-are-allowlisted` is over what ships to a user, and a test library ships to
// nobody.
//
// `text.html.basic` is an editor's grammar rather than an npm package, so a registry here has none
// unless a caller asks for the stub below. Both shapes are wanted: the allowlist suite tokenizes with
// no HTML at all, so every scope it sees is one this grammar emitted, and the openers suite tokenizes
// with the stub, so it can put an opener inside an HTML rule that has already begun.

import { readFileSync, readdirSync } from "node:fs";
import { join, resolve } from "node:path";
import { OnigScanner, OnigString, loadWASM } from "vscode-oniguruma";
import { INITIAL, Registry, parseRawGrammar } from "vscode-textmate";
import type { IGrammar } from "vscode-textmate";

export const ROOT = resolve(__dirname, "..", "..", "..");
export const GRAMMAR = join(ROOT, "syntaxes", "nvs.tmLanguage.json");
export const CASE_GRAMMAR = join(ROOT, "syntaxes", "nvst.tmLanguage.json");
export const FIXTURES = join(ROOT, "test", "grammar", "fixtures");

/** One tokenized span: the bytes, and the scopes stacked on them, outermost first. */
export interface Span {
  text: string;
  scopes: string[];
}

// One rule, because one rule is what proves the point: a `"` that has begun an attribute value owns
// every position until its own `"`, so an opener inside it reaches a pattern of ours only by being
// injected. VS Code's real `text.html.basic` has some hundreds more, and none of them changes that.
const HTML_STUB = {
  scopeName: "text.html.basic",
  patterns: [{ name: "string.quoted.double.html", begin: "\"", end: "\"" }],
};

let wasm: Promise<void> | undefined;

function oniguruma(): Promise<void> {
  if (!wasm) {
    const bytes = readFileSync(
      join(ROOT, "node_modules", "vscode-oniguruma", "release", "onig.wasm"),
    );
    wasm = loadWASM(bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.byteLength));
  }
  return wasm;
}

// Both grammars are in every registry, whichever one is asked for: `source.nvst` includes
// `source.nvs#code`, so the case grammar cannot be loaded without it.
async function load(scopeName: string, html: boolean): Promise<IGrammar> {
  await oniguruma();
  const registry = new Registry({
    onigLib: Promise.resolve({
      createOnigScanner: (sources: string[]) => new OnigScanner(sources),
      createOnigString: (text: string) => new OnigString(text),
    }),
    loadGrammar: async (name: string) => {
      if (name === "source.nvs") {
        return parseRawGrammar(readFileSync(GRAMMAR, "utf8"), GRAMMAR);
      }
      if (name === "source.nvst") {
        return parseRawGrammar(readFileSync(CASE_GRAMMAR, "utf8"), CASE_GRAMMAR);
      }
      if (name === "text.html.basic" && html) {
        return parseRawGrammar(JSON.stringify(HTML_STUB), "text.html.basic.json");
      }
      return null;
    },
  });
  const grammar = await registry.loadGrammar(scopeName);
  if (!grammar) {
    throw new Error(`no grammar under syntaxes/ declares ${scopeName}`);
  }
  return grammar;
}

function spans(grammar: IGrammar, text: string): Span[] {
  const found: Span[] = [];
  let stack = INITIAL;
  for (const line of text.split(/\r?\n/)) {
    const result = grammar.tokenizeLine(line, stack);
    stack = result.ruleStack;
    for (const token of result.tokens) {
      found.push({ text: line.slice(token.startIndex, token.endIndex), scopes: token.scopes });
    }
  }
  return found;
}

/** Every span of `text`, in order, with the rule stack carried across line ends as an editor does. */
export async function tokenize(text: string, html = false): Promise<Span[]> {
  return spans(await load("source.nvs", html), text);
}

/** The same, for a `.nvst` or `.lspt` case: the section grammar, with Novis embedded in it. */
export async function tokenizeCase(text: string): Promise<Span[]> {
  return spans(await load("source.nvst", false), text);
}

/** The one span whose bytes are exactly `text`. Throws rather than returning undefined. */
export function span(spans: Span[], text: string): Span {
  const found = spans.filter((s) => s.text === text);
  if (found.length !== 1) {
    throw new Error(`${found.length} spans read exactly ${JSON.stringify(text)}, wanted 1`);
  }
  return found[0];
}

/** Every fixture under `test/grammar/fixtures`, as `[name, content]`. */
export function fixtures(): [string, string][] {
  return named(".nvs");
}

/** The case fixtures under the same directory: what `source.nvst` is tokenized against. */
export function caseFixtures(): [string, string][] {
  return named(".nvst");
}

function named(extension: string): [string, string][] {
  return readdirSync(FIXTURES)
    .filter((name) => name.endsWith(extension))
    .sort()
    .map((name) => [name, readFileSync(join(FIXTURES, name), "utf8")]);
}
