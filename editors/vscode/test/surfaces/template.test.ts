// The template regions and the format pass, as the decisions the client makes about them.
//
// `rule:ide/a-template-region-gets-services-but-no-second-formatter` puts the boundaries on the
// server, so what is left here is the client's own half: whether a position is inside a region,
// whether the user asked for any of this, what the editor's service is shown, and what a format
// request leaves in the buffer. Those live in `src/template.ts`, which imports no `vscode` — the
// headless tier has no such module (`scripts/headless.mjs`) — and are run directly; `src/regions.ts`
// and `src/format.ts` are the halves that need a running editor, asserted as text the way the AST
// and Test Explorer suites assert theirs.
//
// `recorded/template.nvs` is the program
// `tests/lsp/regions/a-short-echo-splits-one-paragraph-into-two-regions.lspt` freezes an answer for,
// and `REGIONS` below is that answer. The server's half is held by that case and by
// `crates/nvs-lsp/tests/regions.rs`; nothing here re-derives a boundary, which is the whole point of
// asking for them.

import * as assert from "node:assert/strict";
import { readdirSync, readFileSync } from "node:fs";
import { join, resolve } from "node:path";

import { Region, formatted, forwarded, virtual } from "../../src/template";

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

// Every module under `src/`, with its prose removed and keyed by its file name, because the
// formatting registration is asserted both ways round: one in the whole client, and that one in
// `format.ts`. A comment naming an API is not a call to it, and the file that wanted a second
// formatter would be the file that explained itself in a comment first.
//
// Comment *lines* go rather than a `/* … */` region, for the reason the Test Explorer suite gives:
// a regex hunting a block comment finds the `/*` inside a string literal and eats to the next `*/`.
const MODULES = new Map<string, string>(
  readdirSync(join(ROOT, "src"))
    .filter((file) => file.endsWith(".ts"))
    .map((file) => [
      file,
      readFileSync(join(ROOT, "src", file), "utf8")
        .split("\n")
        .filter((line) => !/^\s*(\/\/|\/\*|\*)/.test(line))
        .join("\n"),
    ]),
);

const EVERY_MODULE = [...MODULES.values()].join("\n");

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

});

describe("the template format", () => {
  it("leaves the buffer unchanged when nvs fmt refuses", () => {
    // `nvs fmt --stdin` writes nothing at all to standard output for a file it will not parse and
    // exits non-zero (`crates/nvs-cli/src/fmt.rs`'s `stdin`), which reaches the decision as
    // nothing: the buffer keeps every byte its author typed rather than taking a partial
    // rendering of a file they have not finished.
    assert.equal(formatted(PROGRAM, undefined), undefined);

    // A buffer already in the canonical layout is left alone too. Replacing a text with itself is
    // an edit the editor still marks the document dirty for and still puts on the undo stack.
    assert.equal(formatted(PROGRAM, PROGRAM), undefined);

    // And a buffer the formatter did change becomes exactly what it answered, whole: every layout
    // decision in that text is `nvs fmt`'s (`rule:tooling/fmt-is-one-canonical-style`), so there
    // is nothing for the client to keep, merge or re-indent.
    const written = PROGRAM.replace("var $name =", "var  $name  =");
    assert.notEqual(written, PROGRAM);
    assert.equal(formatted(written, PROGRAM), PROGRAM);
  });

  it("registers one formatting provider, in the module that starts nvs fmt", () => {
    // A `.nvs` file has one formatter of Novis and it is `nvs fmt`
    // (`rule:ide/one-server-two-thin-clients`), so what the client may hold is one registration
    // that starts that process — `src/format.ts` — and no second opinion about a layout anywhere.
    // The server declares no formatting provider at all, which
    // `the_server_declares_no_formatting_provider` holds on the other side of the wire.
    const PROVIDER = "registerDocumentFormattingEditProvider";
    assert.equal(
      EVERY_MODULE.match(new RegExp(PROVIDER, "g"))?.length,
      1,
      "the client registers a formatter of Novis in more than one place",
    );
    assert.ok(MODULES.get("format.ts")?.includes(PROVIDER), `${PROVIDER}: not format.ts's`);

    // Formatting a selection or a keystroke is a layout decision over part of a file, and
    // `nvs fmt` has no mode that formats less than a whole one — its flags are I/O modes and
    // nothing else (`rule:tooling/fmt-check-writes-nothing`). Asking the editor to format the
    // document would re-enter the provider above over the Novis file, and over the virtual one it
    // would lay out the whitespace standing in for the Novis half as though it were markup.
    //
    // The editor's own HTML formatter over a chunk of markup is not on this list: those bytes are
    // the editor's (`rule:ide/a-template-region-gets-services-but-no-second-formatter`), and what
    // is refused here is a second formatter of Novis.
    for (const api of [
      "registerDocumentRangeFormattingEditProvider",
      "registerOnTypeFormattingEditProvider",
      "executeFormatDocumentProvider",
    ]) {
      assert.ok(!EVERY_MODULE.includes(api), `${api}: a second formatter inside a .nvs file`);
    }
  });
});
