// The template regions, and the three decisions the client makes about them.
//
// `rule:ide/a-template-region-gets-services-but-no-second-formatter` puts the boundaries on the
// server, so what is left here is the client's own half: whether a position is inside a region,
// whether the user asked for any of this, and what the editor's service is shown. Those live in
// `src/template.ts`, which imports no `vscode` — the headless tier has no such module
// (`scripts/headless.mjs`) — and are run directly; `src/regions.ts` is the part that needs a running
// editor, asserted as text the way the AST and Test Explorer suites assert theirs.
//
// `recorded/template.nvs` is the program
// `tests/lsp/regions/a-short-echo-splits-one-paragraph-into-two-regions.lspt` freezes an answer for,
// and `REGIONS` below is that answer. The server's half is held by that case and by
// `crates/nvs-lsp/tests/regions.rs`; nothing here re-derives a boundary, which is the whole point of
// asking for them.

import * as assert from "node:assert/strict";
import { readdirSync, readFileSync } from "node:fs";
import { join, resolve } from "node:path";

import { Region, forwarded, virtual } from "../../src/template";

const ROOT = resolve(__dirname, "..", "..", "..");

const recorded = (file: string): string =>
  readFileSync(join(ROOT, "test", "surfaces", "recorded", file), "utf8").replace(/\r\n/g, "\n");

const PROGRAM = recorded("template.nvs");

// What `nvs/regions` answers for that program, on the wire's own 0-based lines and characters —
// the `.lspt` rendering counts both from one. Line 3 is `<p>hi <?= $name ?>!</p>`, and it is two
// regions rather than one: the hole in the middle is Novis, and an HTML service asked about it
// would be answering about a variable.
const REGIONS: Region[] = [
  { range: { start: { line: 3, character: 0 }, end: { line: 3, character: 6 } }, language: "html" },
  { range: { start: { line: 3, character: 18 }, end: { line: 4, character: 0 } }, language: "html" },
];

// Every module under `src/`, with its prose removed, because what must not appear anywhere is a
// formatting registration and the file that wanted one would be the file that made it.
//
// Comment *lines* go rather than a `/* … */` region, for the reason the Test Explorer suite gives:
// a regex hunting a block comment finds the `/*` inside a string literal and eats to the next `*/`.
const EVERY_MODULE = readdirSync(join(ROOT, "src"))
  .filter((file) => file.endsWith(".ts"))
  .map((file) => readFileSync(join(ROOT, "src", file), "utf8"))
  .join("\n")
  .split("\n")
  .filter((line) => !/^\s*(\/\/|\/\*|\*)/.test(line))
  .join("\n");

describe("the template regions", () => {
  it("forwards inside a region and nothing outside one", () => {
    // Inside the markup either side of the hole, which is what the services exist for.
    assert.equal(forwarded(true, REGIONS, { line: 3, character: 2 }), "html");
    assert.equal(forwarded(true, REGIONS, { line: 3, character: 19 }), "html");

    // Inside the hole, and on the `<` that opens it: the range is half-open at `end`, so the first
    // byte of `<?=` belongs to the Novis that follows it rather than to the markup before it.
    assert.equal(forwarded(true, REGIONS, { line: 3, character: 10 }), undefined);
    assert.equal(forwarded(true, REGIONS, { line: 3, character: 6 }), undefined);

    // In the code above the markup, where the file is Novis from its first byte.
    assert.equal(forwarded(true, REGIONS, { line: 1, character: 4 }), undefined);

    // A spelling this client has not been taught forwards nothing, which is the safe direction:
    // the bytes stay Novis's, and Novis already answers for them.
    const unknown: Region[] = [{ range: REGIONS[0].range, language: "jinja" }];
    assert.equal(forwarded(true, unknown, { line: 3, character: 2 }), undefined);

    // And a document the server has answered nothing for has no region to be inside.
    assert.equal(forwarded(true, [], { line: 3, character: 2 }), undefined);
  });

  it("forwards nothing at all when nvs.template.services is false", () => {
    // The same three positions the first case forwards from, including both markup runs.
    for (const character of [2, 19, 10]) {
      assert.equal(forwarded(false, REGIONS, { line: 3, character }), undefined);
    }

    // The setting is read by its whole name, which is what makes the boolean above the user's
    // answer rather than this file's. The contributions suite is what freezes the name itself.
    assert.ok(
      EVERY_MODULE.includes('"nvs.template.services"'),
      "no module names the setting, so nothing turns the forwarding off",
    );
  });

  it("shows the service the markup and whitespace, position for position", () => {
    const html = virtual(PROGRAM, REGIONS, "html");

    // Same length and same line breaks, so a position in the virtual document is the same position
    // in the file and no range coming back needs mapping.
    assert.equal(html.length, PROGRAM.length);
    assert.equal(html.split("\n").length, PROGRAM.split("\n").length);

    // The markup where it is, the hole blanked, and the two runs joined into one document.
    assert.equal(html.split("\n")[3], `<p>hi ${" ".repeat(12)}!</p>`);
    assert.equal(html.split("\n").slice(0, 3).join("\n").trim(), "");

    // Nothing of the Novis half reaches the service.
    assert.ok(!html.includes("$name"), "the hole reached the HTML service");
    assert.ok(!html.includes("<?"), "an open tag reached the HTML service");

    // A language with no region of its own is shown a document with no markup in it.
    assert.equal(virtual(PROGRAM, REGIONS, "css").trim(), "");
  });

  it("registers no formatting provider anywhere in the client", () => {
    // The load-bearing absence: a `.nvs` file has one formatter and it is `nvs fmt`
    // (`rule:tooling/fmt-is-never-a-diagnostic`). The server declares none either, which
    // `the_server_declares_no_formatting_provider` holds on the other side of the wire.
    for (const api of [
      "registerDocumentFormattingEditProvider",
      "registerDocumentRangeFormattingEditProvider",
      "registerOnTypeFormattingEditProvider",
      "executeFormatDocumentProvider",
      "executeFormatRangeProvider",
    ]) {
      assert.ok(!EVERY_MODULE.includes(api), `${api}: a second formatter inside a .nvs file`);
    }
  });
});
