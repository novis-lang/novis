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
// two texts — what the user has and what `nvs fmt` answered — and that is a decision, not an
// editor call; running the process and handing the editor an edit is `src/format.ts`.
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
 * The text a format request replaces the buffer with, or nothing where the buffer stands as
 * written.
 *
 * `novis` is what `nvs fmt --stdin` answered, and nothing is its refusal: a file it will not parse
 * keeps every byte its author typed rather than being replaced by a partial rendering of one they
 * have not finished (`crates/nvs-cli/src/fmt.rs`'s `stdin`). A buffer already in the canonical
 * layout is left alone as well — an edit replacing a text with itself still marks the document
 * dirty and lands on the undo stack, so a save would do something the user can see for nothing.
 */
export function formatted(text: string, novis: string | undefined): string | undefined {
  return novis === undefined || novis === text ? undefined : novis;
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
