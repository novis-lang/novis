// The template regions and the format pass, as the decisions the client makes about them.
//
// `rule:ide/a-template-region-gets-the-editors-services-and-formatter` puts the boundaries on the
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

import {
  Chunk,
  Position,
  Region,
  chunks,
  edited,
  formatted,
  forwarded,
  hidden,
  merged,
  virtual,
} from "../../src/template";

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

// The one hole both fixtures carry, written once so a case can name its bytes without counting
// them: twelve characters of Novis inside a line of markup.
const HOLE = "<?= $name ?>";

// Markup inside a `for` body, which is where a chunk's base is not column zero. `nvs fmt` puts the
// `?>` at the depth of the block it sits in (`rule:tooling/fmt-novis-constructs`), so this is a
// file that pass has already been over, and the markup under it is what the client lays out from
// there (ADR 0173 § 3).
const LOOP = [
  "<?nvs",
  "for ($row = 0; $row < 3; $row = $row + 1) {",
  "    ?>",
  "<ul>",
  `<li>hi ${HOLE}</li>`,
  "</ul>",
  "    <?nvs",
  "}",
  "?>",
  "",
].join("\n");

// What `nvs/regions` answers for it: the markup either side of the hole, ending where the `<?nvs`
// that closes the chunk begins — so the whitespace in front of that tag is the markup's own. The
// boundaries are the lexer's and are held on the other side of the wire by
// `crates/nvs-lsp/tests/regions.rs`; what is written here is where they fall in these bytes, named
// by `indexOf` so the fixture above can be edited without re-counting a column.
const RUNS = html(
  LOOP,
  [LOOP.indexOf("<ul>"), LOOP.indexOf(HOLE)],
  [LOOP.indexOf(HOLE) + HOLE.length, LOOP.indexOf("    <?nvs") + 4],
);

/** The server's answer for `text`, from the offsets its markup runs lie between. */
function html(text: string, ...runs: readonly (readonly [number, number])[]): Region[] {
  return runs.map(([start, end]) => ({
    range: { start: spot(text, start), end: spot(text, end) },
    language: "html",
  }));
}

/** One offset as the line and character the wire counts it in. */
function spot(text: string, at: number): Position {
  const rows = text.slice(0, at).split("\n");
  return { line: rows.length - 1, character: rows[rows.length - 1].length };
}

/** The chunk as the formatter is shown it, with a fixture that may not be shown one failing here. */
function shownTo(text: string, chunk: Chunk): string {
  const body = hidden(text, chunk);
  if (body === undefined) {
    throw new Error("the fixture's markup holds the stand-in's own character");
  }
  return body;
}

/**
 * The editor's HTML formatter, as the headless tier has one.
 *
 * It has none: the built-in formatter needs a running editor
 * (`rule:ide/headless-gates-the-loop-the-host-run-gates-the-milestone`), and the milestone's host
 * run is where a real template goes through a real one. This answers in the shape a real one does —
 * every row in its own columns, starting at the first, with `nest` naming the rows it puts a level
 * into — because what the merge may not simply trust is the column the formatter chose.
 */
function answered(text: string, chunk: Chunk, nest: readonly number[]): string {
  return shownTo(text, chunk)
    .split("\n")
    .map((row, index) =>
      row.trim() === "" ? "" : (nest.includes(index) ? "    " : "") + row.trim())
    .join("\n");
}

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
    // the editor's (`rule:ide/a-template-region-gets-the-editors-services-and-formatter`), and what
    // is refused here is a second formatter of Novis.
    for (const api of [
      "registerDocumentRangeFormattingEditProvider",
      "registerOnTypeFormattingEditProvider",
      "executeFormatDocumentProvider",
    ]) {
      assert.ok(!EVERY_MODULE.includes(api), `${api}: a second formatter inside a .nvs file`);
    }
  });

  it("formats only the Novis half when nvs.template.format is false", () => {
    // `false` answers no chunk at all, so the merge has nothing to put anywhere and the buffer
    // becomes exactly what `nvs fmt` wrote. That is the whole of the pass for a user whose markup
    // whitespace is output, or whose HTML tooling is their own.
    const off = chunks(false, PROGRAM, REGIONS, "html");
    assert.deepEqual(off, []);
    assert.equal(merged(PROGRAM, off, []), PROGRAM);

    // The same answer from the other side: a spelling this client has no formatter for is nobody's
    // chunk, and those bytes stay as `nvs fmt` left them too.
    assert.deepEqual(chunks(true, PROGRAM, REGIONS, "css"), []);

    // The setting is read by its whole name, which is what makes the boolean above the user's
    // answer rather than this file's. The contributions suite is what freezes the name itself.
    assert.ok(
      EVERY_MODULE.includes('"nvs.template.format"'),
      "no module names the setting, so nothing turns the markup pass off",
    );
  });

  it("starts a chunk after a close tag at that line's indentation", () => {
    const list = chunks(true, PROGRAM, REGIONS, "html");

    // One chunk and not two: the hole in the middle of the paragraph is part of its line, and the
    // two regions the server answered either side of it are one run of markup to lay out.
    assert.equal(list.length, 1, "the hole in the paragraph was read as a boundary");
    const [chunk] = list;
    assert.equal(PROGRAM.slice(chunk.start, chunk.end), `<p>hi ${HOLE}!</p>\n`);
    assert.deepEqual(chunk.holes.map((hole) => PROGRAM.slice(hole.start, hole.end)), [HOLE]);

    // The `?>` that opened it begins its own line, so the chunk is laid out from column zero — and
    // the markup starts at the first column of the line after it, which is how a chunk is told from
    // a run continuing one.
    assert.equal(chunk.base, "");
    assert.equal(PROGRAM[chunk.start - 1], "\n");
  });

  it("indents a chunk inside a loop body one level deeper than the loop", () => {
    const [chunk] = chunks(true, LOOP, RUNS, "html");

    // The `?>` is one level into the `for` body, so that is where its markup starts.
    assert.equal(chunk.base, "    ");

    assert.equal(merged(LOOP, [chunk], [answered(LOOP, chunk, [1])]), [
      "<?nvs",
      "for ($row = 0; $row < 3; $row = $row + 1) {",
      "    ?>",
      "    <ul>",
      `        <li>hi ${HOLE}</li>`,
      "    </ul>",
      "    <?nvs",
      "}",
      "?>",
      "",
    ].join("\n"));
  });

  it("puts the open tag that ends a chunk at the chunk's base", () => {
    const [chunk] = chunks(true, LOOP, RUNS, "html");

    // The chunk's last line is the whitespace in front of that `<?nvs`, and a formatter is free to
    // answer with it, without it, or in a column of its own. The tag ends up at the base either
    // way, so it starts where the markup it closes does rather than where the formatter stopped.
    const full = answered(LOOP, chunk, [1]);
    for (const answer of [full, full.replace(/\n+$/, ""), `${full}\n\n`]) {
      const out = merged(LOOP, [chunk], [answer]);
      assert.ok(out.includes("\n    <?nvs\n"), "the closing tag did not come back to the base");
      assert.ok(!out.includes("\n<?nvs"), "the closing tag lost the base");
    }
  });

  it("never edits a byte of a hole", () => {
    const [chunk] = chunks(true, LOOP, RUNS, "html");
    const body = shownTo(LOOP, chunk);

    // The formatter is shown a stand-in of the hole's own length and never a byte of its Novis, so
    // every column of the markup around it is where its author put it.
    assert.equal(body.length, chunk.end - chunk.start);
    assert.ok(!body.includes("$row") && !body.includes("$name"), "a hole's Novis reached the formatter");
    assert.ok(!body.includes("<?"), "an open tag reached the formatter");

    // And what goes back into the buffer carries the hole's bytes exactly, on the line its author
    // wrote them on — the line the formatter moved, with the bytes it was never shown.
    const out = merged(LOOP, [chunk], [answered(LOOP, chunk, [1])]);
    assert.equal(out.split("\n")[4], `        <li>hi ${HOLE}</li>`);
    assert.equal(out.match(/<\?=/g)?.length, 1);
  });

  it("leaves a chunk as written when formatting it would reach a hole", () => {
    const [chunk] = chunks(true, LOOP, RUNS, "html");
    const body = shownTo(LOOP, chunk);
    const cut = chunk.holes[0].start - chunk.start;

    // A stand-in that came back a character short, and one a formatter wrapped a line inside:
    // either is a layout that would have moved a byte of Novis, so that chunk keeps the text it
    // had. An unformatted chunk is a layout complaint and an edited hole is a changed program.
    const short = body.slice(0, cut) + body.slice(cut + 1);
    const wrapped = `${body.slice(0, cut + 2)}\n${body.slice(cut + 2)}`;
    for (const answer of [short, wrapped, undefined]) {
      assert.equal(merged(LOOP, [chunk], [answer]), LOOP);
    }

    // Markup already holding the stand-in's own character is shown to no formatter at all, since a
    // run in that answer would no longer be the client's to read back. The replacement is one
    // character for one, so the regions above still describe these bytes.
    const own = LOOP.replace("<ul>", `<${String.fromCodePoint(0xe000)}l>`);
    const [same] = chunks(true, own, RUNS, "html");
    assert.equal(hidden(own, same), undefined);
    assert.equal(merged(own, [same], [undefined]), own);
  });

  it("changes no byte outside a region", () => {
    const [chunk] = chunks(true, LOOP, RUNS, "html");
    const out = merged(LOOP, [chunk], [answered(LOOP, chunk, [1])]);

    // The Novis above the chunk and the Novis below it come through a slice, so `nvs fmt` stays the
    // only thing that has laid out a byte of either (`rule:ide/one-server-two-thin-clients`) and
    // `nvs fmt --check` still passes over what this writes.
    assert.ok(out.startsWith(LOOP.slice(0, chunk.start)), "a byte before the chunk moved");
    assert.ok(out.endsWith(LOOP.slice(chunk.end)), "a byte after the chunk moved");
    assert.notEqual(out, LOOP);

    // A formatter that answers with what it was shown changes nothing anywhere: the stand-ins go
    // back as the holes they stood in for, and the file is the one `nvs fmt` wrote.
    const [only] = chunks(true, PROGRAM, REGIONS, "html");
    assert.equal(merged(PROGRAM, [only], [shownTo(PROGRAM, only)]), PROGRAM);
  });

  it("reads a formatter's edits as one text, whatever order they arrive in", () => {
    const body = "<ul>\n<li>hi</li>\n</ul>\n";
    const indent = (line: number, text: string) => ({
      range: { start: { line, character: 0 }, end: { line, character: 0 } },
      newText: text,
    });

    // A range formatter answers in edits and this pass decides in texts, so the edits are written
    // out in one pass and in position order — which is not the order they need have arrived in.
    assert.equal(edited(body, [indent(2, "  "), indent(1, "    ")]),
                 "<ul>\n    <li>hi</li>\n  </ul>\n");

    // Two that overlap are an answer the client cannot read, and it leaves the text alone rather
    // than applying the half of it that fits.
    const over = [
      { range: { start: { line: 0, character: 0 }, end: { line: 1, character: 2 } }, newText: "x" },
      { range: { start: { line: 1, character: 0 }, end: { line: 1, character: 4 } }, newText: "y" },
    ];
    assert.equal(edited(body, over), body);
  });
});
