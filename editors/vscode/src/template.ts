// Which of the editor's services owns a position in a `.nvs` file, the document one of them is
// asked about, and what a format request leaves in the buffer.
//
// The boundaries are the server's answer to `nvs/regions` and nothing here derives one
// (`rule:ide/a-template-region-gets-services-but-no-second-formatter`): a client that read a
// TextMate grammar to find where markup starts would be a second implementation of the lexer, which
// is `crates/nvs-lsp/src/regions.rs` § *the lexer is asked, never a grammar in the client*. What is
// decided here is the half the server deliberately left open — which spelling maps to which service,
// whether the user asked for any of this at all, and what the service is shown.
//
// Formatting is here on the same split. What a format request does to the buffer is decided from
// what the user has, what `nvs fmt` answered and the regions of that answer: where the markup
// chunks are, what each one is laid out from, what a formatter is shown of a hole and what goes
// back into the buffer are all decisions rather than editor calls. Running the process, asking the
// editor's own HTML formatter about a chunk and handing back one edit is `src/format.ts`.
//
// Nothing here imports `vscode`, for the reason `src/concealment.ts` gives: the headless tier runs
// it in plain Node, and a decision whose test needs a display is a decision nobody tests. The
// `vscode` halves are `src/regions.ts`, which asks, registers and forwards, and `src/format.ts`.

/** A position on the wire, in the encoding the client negotiated at `initialize`. */
export interface Position {
  line: number;
  character: number;
}

/** A range on the wire, half-open at `end` as LSP has it. */
export interface Range {
  start: Position;
  end: Position;
}

/**
 * One region the server answered: the bytes, and whose they are.
 *
 * `language` is an open string, the way a redaction's `kind` is — the server names a spelling and
 * this module maps it to one of the editor's own language ids. A spelling that maps to nothing
 * forwards nothing, which is the safe direction: the bytes stay Novis's, and Novis already answers
 * for them.
 */
export interface Region {
  range: Range;
  language: string;
}

/**
 * The spellings this client has been taught, each to the editor's own language id for it.
 *
 * One entry, because the lexer answers one kind of region today. A second markup language is a row
 * here and nothing else — neither end of the wire changes shape for it
 * (`crates/nvs-lsp/src/regions.rs`, `HTML`).
 */
const SERVICES: Record<string, string> = { html: "html" };

/** The editor's language id for a region's spelling, or nothing where this client knows none. */
export function service(language: string): string | undefined {
  return SERVICES[language];
}

/**
 * The region `position` is inside, or nothing.
 *
 * Half-open at `end`, as the range on the wire is: the cursor on the first byte of `<?=` is in the
 * hole and not in the markup before it, so a completion there is Novis's to answer. The list is
 * walked rather than searched — it is one entry per markup run in one open file, which is the count
 * a server answered for a single document.
 */
export function at(regions: readonly Region[], position: Position): Region | undefined {
  return regions.find((region) =>
    !before(position, region.range.start) && before(position, region.range.end));
}

/**
 * The editor service that answers at `position`, or nothing where none does.
 *
 * The one decision both halves of the rule turn on, so it is one function: `enabled` is
 * `nvs.template.services`, and a `false` there forwards nothing anywhere rather than forwarding to a
 * service the user has told us to stay out of the way of. Nothing at all is the answer outside a
 * region, in a region whose spelling this client does not know, and while the server has answered
 * nothing for the document.
 */
export function forwarded(
  enabled: boolean,
  regions: readonly Region[],
  position: Position,
): string | undefined {
  if (!enabled) {
    return undefined;
  }
  const region = at(regions, position);
  return region === undefined ? undefined : service(region.language);
}

/**
 * `text` as the service for `language` is shown it: its own regions, and whitespace everywhere else.
 *
 * The server answers one region per markup run and leaves the joining here
 * (`crates/nvs-lsp/src/regions.rs` § *one region per run, and the client joins them*), because what
 * goes in the gap is a question only the asking service can answer. What goes in the gap is
 * whitespace: every byte outside a region becomes a space and every line break survives, so a
 * position in this document is the same position in the file — no map is kept, and an answer's own
 * ranges come back already addressing the buffer the user is looking at.
 *
 * Whitespace rather than a comment of the host language, because `<?= $name ?>` sits *inside* a tag
 * as often as between two, and `<p><!-- … -->class="x">` is not the document the user wrote. A run
 * of spaces is text in every position markup allows one, and an attribute the hole was going to
 * name is simply absent rather than malformed.
 */
export function virtual(
  text: string,
  regions: readonly Region[],
  language: string,
): string {
  const starts = lines(text);
  const kept = regions.filter((region) => service(region.language) === language);
  const shown = new Array<boolean>(text.length).fill(false);
  for (const region of kept) {
    const from = offset(starts, text.length, region.range.start);
    const to = offset(starts, text.length, region.range.end);
    for (let index = from; index < to; index += 1) {
      shown[index] = true;
    }
  }
  return Array.from(text, (character, index) =>
    shown[index] || character === "\n" || character === "\r" ? character : " ").join("");
}

/**
 * One chunk of markup: where it is, what its lines are laid out from, and the Novis inside it.
 *
 * `start` and `end` are offsets into the text the chunk was built from, and so are a hole's, so the
 * whole of the format pass reads one coordinate system — the text `nvs fmt` answered, which is the
 * text the server was asked about.
 */
export interface Chunk {
  start: number;
  end: number;
  /** The indentation every line of the chunk starts at. */
  base: string;
  /** The holes inside it, in the order they are written. */
  holes: Hole[];
}

/** One hole: Novis inside a chunk, whose bytes no formatter is shown and none ever edits. */
export interface Hole {
  start: number;
  end: number;
}

// What one character of a hole is shown to the formatter as. The first private-use code point,
// because a markup formatter treats it as the ordinary text it is while no document written to be
// read holds one — and a chunk that does hold one is formatted not at all, rather than read back by
// counting runs that are not all ours.
const STANDIN = String.fromCodePoint(0xe000);

// The stand-in runs of a formatted chunk, as the formatter handed them back.
const STANDINS = new RegExp(`${STANDIN}+`, "g");

/**
 * The markup of `text` in the chunks the editor's formatter for `language` is asked about, and none
 * at all where the user has turned the pass off.
 *
 * A chunk is the markup between a `?>` that ends its line and the `<?nvs` that reopens code, with
 * every `<?= … ?>` hole inside it (ADR 0173 § 2): a hole is part of its line rather than a boundary,
 * so a paragraph a `<?= $name ?>` sits in the middle of is laid out as the one paragraph it is.
 * What says where a chunk starts is the server's region list and the text's line breaks and nothing
 * else — a markup run beginning in the first column of its line is one the lexer reached by eating
 * the newline after a close tag, which is exactly a `?>` that ends its line. Nothing here goes
 * looking for a `<?`: that is the second lexer
 * `rule:ide/a-template-region-gets-services-but-no-second-formatter` keeps out of this client.
 *
 * The base is the indentation of that `?>` line, which `rule:tooling/fmt-novis-constructs` has put
 * at the depth of the block it sits in, so the markup nests from the code around it and the file
 * does not jump between the two (ADR 0173 § 3). The markup before a file's first open tag has no
 * such line and is based at column zero.
 *
 * `enabled` is `nvs.template.format`, and `false` answers no chunk at all: a format request is then
 * `nvs fmt` and nothing else, which is what a user whose markup whitespace is output, or whose HTML
 * tooling is their own, has to be able to ask for.
 */
export function chunks(
  enabled: boolean,
  text: string,
  regions: readonly Region[],
  language: string,
): Chunk[] {
  if (!enabled) {
    return [];
  }
  const starts = lines(text);
  const built: Chunk[] = [];
  for (const region of regions) {
    if (service(region.language) !== language) {
      continue;
    }
    const start = offset(starts, text.length, region.range.start);
    const end = offset(starts, text.length, region.range.end);
    if (start >= end) {
      continue;
    }
    const open = built.length === 0 ? undefined : built[built.length - 1];
    if (open === undefined || start < open.end || opens(text, start)) {
      built.push({ start, end, base: based(text, start), holes: [] });
    } else {
      open.holes.push({ start: open.end, end: start });
      open.end = end;
    }
  }
  return built;
}

/**
 * The chunk's text as the formatter is shown it, or nothing where it may not be shown one at all.
 *
 * A hole's bytes are Novis's, and a formatter handed them would be laying out a variable, so each
 * one is a stand-in run of its own length: the markup around it keeps every column its author gave
 * it, and the run coming back whole is what says the formatter left the hole alone. A hole written
 * across lines becomes one long line of stand-in, which a formatter is free to wrap — that is a
 * chunk left as written, and the safe direction. Nothing at all where the markup already holds the
 * stand-in's own character, since a run in that answer is no longer this module's to read back.
 */
export function hidden(text: string, chunk: Chunk): string | undefined {
  if (text.slice(chunk.start, chunk.end).includes(STANDIN)) {
    return undefined;
  }
  let seen = chunk.start;
  let out = "";
  for (const hole of chunk.holes) {
    out += text.slice(seen, hole.start) + STANDIN.repeat(hole.end - hole.start);
    seen = hole.end;
  }
  return out + text.slice(seen, chunk.end);
}

/**
 * `text` with each chunk replaced by what the formatter answered for it, and left as written
 * wherever it answered nothing or answered something that reached a hole.
 *
 * `answers` runs with `chunks`, each entry the formatter's reply for that chunk's [`hidden`] text.
 * Every byte this changes is inside a region: a chunk's own range is markup the server reported,
 * the holes inside it go back exactly as they came, and what is outside a chunk is copied by a
 * slice. A chunk that came back with its stand-ins split, shortened or reordered is one whose
 * layout would have moved a byte of Novis, and it keeps the text it had.
 */
export function merged(
  text: string,
  chunks: readonly Chunk[],
  answers: readonly (string | undefined)[],
): string {
  let seen = 0;
  let out = "";
  chunks.forEach((chunk, index) => {
    const answer = answers[index];
    const laid = answer === undefined ? undefined : rebuilt(text, chunk, answer);
    out += text.slice(seen, chunk.start) + (laid ?? text.slice(chunk.start, chunk.end));
    seen = chunk.end;
  });
  return out + text.slice(seen);
}

/** One edit of a text: the range it replaces, in the positions the editor counts them in. */
export interface Edit {
  range: Range;
  newText: string;
}

/**
 * `text` with `edits` applied, or `text` itself where they do not fit it.
 *
 * A formatter answers in edits and this module decides in texts, so this is the one conversion
 * between them and it holds no opinion about a layout: the edits are sorted and written out in a
 * single pass. Two that overlap are an answer this cannot read, and an answer it cannot read leaves
 * the text alone — the direction everything in this pass fails in.
 */
export function edited(text: string, edits: readonly Edit[]): string {
  const starts = lines(text);
  const spans = edits
    .map((edit) => ({
      start: offset(starts, text.length, edit.range.start),
      end: offset(starts, text.length, edit.range.end),
      written: edit.newText,
    }))
    .sort((left, right) => left.start - right.start);
  let seen = 0;
  let out = "";
  for (const span of spans) {
    if (span.start < seen || span.end < span.start) {
      return text;
    }
    out += text.slice(seen, span.start) + span.written;
    seen = span.end;
  }
  return out + text.slice(seen);
}

/**
 * The text a format request replaces the buffer with, or nothing where the buffer stands as
 * written.
 *
 * `novis` is what the pass produced — `nvs fmt --stdin`'s answer with every markup chunk laid out
 * into it — and nothing is the formatter's refusal: a file it will not parse keeps every byte its
 * author typed rather than being replaced by a partial rendering of one they have not finished
 * (`crates/nvs-cli/src/fmt.rs`'s `stdin`). A buffer already in the canonical layout is left alone as
 * well — an edit replacing a text with itself still marks the document dirty and lands on the undo
 * stack, so a save would do something the user can see for nothing.
 */
export function formatted(text: string, novis: string | undefined): string | undefined {
  return novis === undefined || novis === text ? undefined : novis;
}

/** Whether the markup at `start` opens a chunk, which is to say a close tag ended the line above. */
function opens(text: string, start: number): boolean {
  return start === 0 || text[start - 1] === "\n";
}

/**
 * The indentation a chunk starting at `start` lays its lines out from.
 *
 * The close tag that opened it ends the line above when the markup begins a line, and sits on that
 * markup's own line when it does not, so one of those two lines is read up to the tag and its
 * leading whitespace is the base. A chunk at the first byte of the file has neither and is based at
 * column zero, which is where markup before a file's first open tag belongs.
 */
function based(text: string, start: number): string {
  const closed = start > 0 && opens(text, start) ? start - 1 : start;
  return indentation(text.slice(text.lastIndexOf("\n", closed - 1) + 1, closed));
}

/** The leading spaces and tabs of one line. */
function indentation(row: string): string {
  const blank = /^[ \t]*/.exec(row);
  return blank === null ? "" : blank[0];
}

/**
 * One chunk's answer as it goes back into the text: laid out from the base, with its holes in it.
 *
 * The base is put on before the holes are, so no re-indentation can reach a hole's bytes — what is
 * moved is the stand-in standing in for it, and the bytes themselves are written in afterwards
 * exactly as they were read. Nothing where the stand-ins did not survive, which leaves that chunk
 * as its author wrote it.
 */
function rebuilt(text: string, chunk: Chunk, answer: string): string | undefined {
  const runs = answer.match(STANDINS) ?? [];
  const fits = (run: string, index: number): boolean =>
    run.length === chunk.holes[index].end - chunk.holes[index].start;
  if (runs.length !== chunk.holes.length || !runs.every(fits)) {
    return undefined;
  }
  let index = 0;
  return rebased(text, chunk, answer).replace(STANDINS, () => {
    const hole = chunk.holes[index];
    index += 1;
    return text.slice(hole.start, hole.end);
  });
}

/**
 * `answer` with every line laid out from the chunk's base, whatever column the formatter wrote it
 * at.
 *
 * The formatter is asked about a chunk on its own and answers in its own columns, so what is kept
 * from it is the nesting — each line's depth below the shallowest one — and the base is what that
 * nesting is added to. A line holding nothing but whitespace is emptied rather than based, since
 * trailing spaces on an empty line are not a layout. The line carrying the `<?nvs` that closes the
 * chunk is written as the base alone (ADR 0173 § 3): that tag follows the chunk's last byte, so
 * this is the whitespace in front of it, and it starts where the markup it closes does.
 */
function rebased(text: string, chunk: Chunk, answer: string): string {
  const rows = answer.split("\n");
  const written = rows.filter((row) => row.trim() !== "");
  const depth = Math.min(...written.map((row) => indentation(row).length));
  const out = rows.map((row) => (row.trim() === "" ? "" : chunk.base + row.slice(depth)));
  if (closes(text, chunk)) {
    if (out[out.length - 1] === "") {
      out.pop();
    }
    out.push(chunk.base);
  }
  return out.join("\n");
}

/** Whether an open tag begins the line after the chunk's last byte, on whitespace of the chunk's. */
function closes(text: string, chunk: Chunk): boolean {
  if (chunk.end >= text.length) {
    return false;
  }
  const start = Math.max(text.lastIndexOf("\n", chunk.end - 1) + 1, chunk.start);
  return text.slice(start, chunk.end).trim() === "";
}

/** Whether `left` is strictly before `right`. */
function before(left: Position, right: Position): boolean {
  return left.line === right.line ? left.character < right.character : left.line < right.line;
}

/**
 * Where each line of `text` starts, in UTF-16 units.
 *
 * Which is what a position's `character` counts, because that is the encoding this client
 * negotiates and a JavaScript string is indexed in the same units — so an emoji in the markup needs
 * no conversion here, and getting it wrong would show the service a document shifted by one from the
 * user's third paragraph on.
 */
function lines(text: string): number[] {
  const starts = [0];
  for (let index = text.indexOf("\n"); index >= 0; index = text.indexOf("\n", index + 1)) {
    starts.push(index + 1);
  }
  return starts;
}

/**
 * One wire position as an offset into the text those line starts came from.
 *
 * Clamped at both ends rather than trusted. The answer describes the document as it was when the
 * request went out, and an edit since then can leave a range past the last byte; a slice computed
 * from an out-of-range offset would blank the wrong half of the file, where a clamp shows the
 * service a little less markup until the next answer arrives.
 */
function offset(starts: readonly number[], length: number, position: Position): number {
  if (position.line >= starts.length) {
    return length;
  }
  const start = starts[Math.max(position.line, 0)];
  return Math.min(start + Math.max(position.character, 0), length);
}
