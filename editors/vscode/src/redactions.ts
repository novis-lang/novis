// What this client draws over the ranges `nvs/redactions` answers: a blur over the bytes of a
// `secret`, and a glyph after the name of a `tainted` declaration where the user asked for one.
//
// Two decorations because the two kinds are two instructions, not two colours of one. The blur is on
// by default and unconditional (`rule:ide/redaction-ranges-come-from-the-server`); the glyph is
// drawn only where `nvs.taint.mark` is not `off`, because a marker is added content and shipping one
// by default would write into someone else's editor
// (`rule:ide/tainted-has-no-default-decoration`). Which range is which is the server's answer
// and `concealment.ts`'s partition of it — nothing here reads the source text.
//
// The request is Novis's own and the only one that is (`rule:ide/the-request-set-is-closed`); its
// spelling and its answer's shape live in `crates/nvs-lsp/src/redactions.rs`, and this file is the
// half that renders them. It reads no source text and matches no literal — which literal carries
// `secret` is a question with one answer and the server holds it
// (`rule:ide/redaction-ranges-come-from-the-server`).
//
// What is drawn is a decoration and not an edit. The buffer is the file on disk byte for byte, and
// the character cells stay where they were, so the cursor, the selection and every edit still
// address the real text (`rule:ide/redaction-covers-bytes-only`).
//
// A reveal is one range, and there are three ways to ask for one. Two are explicit and are held in
// `concealment.ts`, where no `vscode` import reaches them: the hover's command link and the palette
// command, both of which stay uncovered until the editor closes or `nvs.hideSecrets` runs. The
// third is the cursor, and it holds nothing — a range with the cursor in it is drawn uncovered and
// is covered again the moment the cursor leaves, so it is read off `editor.selections` in `options`
// rather than recorded anywhere.
//
// This file is the part that cannot be tested without a display, which is why it is as thin as it
// is: it converts positions, asks, and calls `setDecorations`.

import {
  DecorationOptions,
  DecorationRangeBehavior,
  ExtensionContext,
  MarkdownString,
  Position,
  Range,
  TextDocument,
  TextEditor,
  TextEditorDecorationType,
  ThemeColor,
  window,
  workspace,
} from "vscode";
import { LanguageClient } from "vscode-languageclient/node";

import { Concealment, Position as Where, Range as Wire, Redaction } from "./concealment";
import { Drawn } from "./surface";

// The server's own request, spelled where `crates/nvs-lsp/src/redactions.rs` spells it. Its params
// are an LSP `TextDocumentIdentifier` and its answer a list of `{range, kind}`.
const METHOD = "nvs/redactions";

// The setting that turns the concealment off, default `true`. It respawns nothing: the server
// answers the same ranges either way and only the drawing changes, which is why it is not one of
// `extension.ts`'s `RESPAWNING_SETTINGS`.
const SETTING = "nvs.secrets.redact";

// The setting that asks for the marker, default `off`. Its three values are ADR 0101 § 4's, and
// `sink` draws what `declaration` draws until the server answers a sink's argument positions —
// `crates/nvs-lsp/src/redactions.rs` § *What it does not reach* is where that gap is stated.
const MARK = "nvs.taint.mark";

// The command the decoration's own hover offers, which ADR 0101 § 3 makes the discoverable path to
// a reveal — the palette and a keybinding are the other two, and all three are the same command.
const REVEAL = "nvs.revealSecret";

// The radius the concealed bytes are blurred by.
//
// A blur rather than an opaque bar, because a bar over a value the user is editing reads as damage
// rather than as concealment: nothing about it says the bytes are still there and deliberately
// covered. A blur says exactly that, and it is the one form of concealment that needs no colour of
// Novis's own — `rule:ide/novis-ships-names-not-colours` satisfied by having nothing to ship.
//
// It is a **weaker** concealment than an opaque fill and that is the trade this constant is:
// the smear is a convolution, so a recording of the screen carries more of the value than a fill
// would, and a short low-entropy literal keeps its shape. `rule:ide/redaction-does-not-reach`
// is where the reach of the whole mechanism is written down, this included.
const BLUR = "10px";

const held = new Concealment();
// What the last draw handed each visible Novis editor, which is `surface.ts`'s reading of this file.
// A decoration cannot be read back off an editor, so this is the only record of what is on screen.
// It is rebuilt whole by every draw rather than amended, because `draw` recomputes every range it
// hands out anyway and a record kept any other way is a second copy of this state to fall behind it.
let last: Drawn[] = [];
let concealing: TextEditorDecorationType | undefined;
let marking: TextEditorDecorationType | undefined;
let serving: LanguageClient | undefined;

/**
 * Create the decoration and start following what this window shows.
 *
 * Called once from `activate`, before any server is running: the listeners cost nothing while
 * nothing has answered, and installing them after the handshake would miss the documents that were
 * already open when the window did.
 */
export function install(context: ExtensionContext): void {
  concealing = bar();
  marking = glyph();
  context.subscriptions.push(
    concealing,
    marking,
    workspace.onDidOpenTextDocument((document) => void ask(document)),
    workspace.onDidChangeTextDocument((event) => void ask(event.document)),
    workspace.onDidCloseTextDocument((document) => held.forget(document.uri.toString())),
    window.onDidChangeVisibleTextEditors((editors) => shown(editors)),
    // Becoming the active editor changes no document and makes no editor newly visible, so neither
    // listener above fires for it. It is still the moment a user is most likely to be looking at a
    // concealed range for the first time, and a window restored with a document already open
    // reaches its first draw through here.
    window.onDidChangeActiveTextEditor((editor) => shown(editor === undefined ? [] : [editor])),
    // A cursor moving in or out of a concealed range changes what is drawn, because a range with
    // the cursor inside it is uncovered for exactly as long as that is true
    // (`rule:ide/reveal-is-explicit-and-window-local`). This fires on every cursor move and
    // `draw` is O(ranges held for the visible documents), which is the count the server answered
    // for one file — small enough to redo rather than to track a delta against.
    window.onDidChangeTextEditorSelection(() => draw()),
    workspace.onDidChangeConfiguration((event) => {
      if (event.affectsConfiguration(SETTING) || event.affectsConfiguration(MARK)) {
        draw();
      }
    }),
  );
}

/**
 * The client every ask goes to, or `undefined` while none is running.
 *
 * A server that stops leaves the concealment on screen untouched. That is the whole of
 * `rule:ide/redaction-ranges-come-from-the-server`'s fail direction: a missing answer means
 * nothing at all, so a restart, a crash or a version refusal must not be the thing that uncovers
 * the bytes.
 */
export function serve(client: LanguageClient | undefined): void {
  serving = client;
  for (const editor of window.visibleTextEditors) {
    void ask(editor.document);
  }
}

/**
 * The decoration one concealed range is drawn with: the glyphs smeared past reading and drained of
 * colour, in place.
 *
 * The blur takes the characters where they are, so every cell stays exactly the width it was and a
 * click still lands on the column the buffer has there. A `before`/`after` content or a
 * `display: none` would move them, and then it would not. The grayscale beside it takes the
 * syntax colouring off the smear, so a concealed range reads as one grey shape rather than as a
 * blurred string that is still recognisably a string.
 *
 * **`textDecoration` is the field a filter reaches through, and that is not what its name says.**
 * This API has no property for a CSS filter, and the string given here is written into the rendered
 * rule verbatim — so the `text-decoration` declaration is closed with `none` and the filter follows
 * it. Anything put here is CSS this extension is choosing on the user's behalf, which is why it
 * only ever subtracts: no colour is named, the grayscale removes the ones the theme chose, and
 * `rule:ide/novis-ships-names-not-colours` is met by having nothing to ship rather than by naming a
 * theme key.
 *
 * **A blur bleeds past the box it is drawn in, and the clip is what cuts it back.** The smear
 * reaches about its own radius past the range, over the punctuation and the identifier either side,
 * which reads as damage to the line rather than as one concealed value. `clip-path` is the
 * containment this box can take: `overflow` does not apply to a non-replaced inline element, and
 * the `display: inline-block` that would make it apply moves the character cells, which is the one
 * thing this decoration may not do. Filters are applied before clipping, so the clip trims the
 * smear and changes no layout at all.
 *
 * Drawn without a background for a separate reason: a fill behind it would have a hard edge the
 * smear does not, and the two together read as a box with a blurred label in it rather than as text
 * that has been taken away.
 */
function bar(): TextEditorDecorationType {
  return window.createTextEditorDecorationType({
    textDecoration: `none; filter: blur(${BLUR}) grayscale(1); clip-path: inset(0)`,
    // A character typed against either edge is outside the concealed range until the server says
    // otherwise, which is the direction that cannot draw over bytes nobody answered for.
    rangeBehavior: DecorationRangeBehavior.ClosedClosed,
  });
}

/**
 * The marker drawn after a `tainted` declaration's name, where the setting asks for one.
 *
 * A text glyph and not a codicon: an inline attachment takes `contentText` or an image path and
 * never both, and only the text takes a `ThemeColor` — an image would ship a colour of Novis's own
 * into the user's theme, which is the thing `rule:ide/novis-ships-names-not-colours` refuses. The
 * character is a BMP geometric shape rather than an emoji, so it has no colour font to be at the
 * mercy of and renders from the editor's own monospace face. The README records that trade.
 */
function glyph(): TextEditorDecorationType {
  return window.createTextEditorDecorationType({
    after: {
      contentText: "◆",
      color: new ThemeColor("editorWarning.foreground"),
      margin: "0 0 0 0.25em",
    },
    // The glyph belongs to the name it follows, so typing at either edge must not stretch what it
    // is attached to; the next answer is what moves it.
    rangeBehavior: DecorationRangeBehavior.ClosedClosed,
  });
}

/** Draw on the editors that just became visible, and ask for any document nothing is held for. */
function shown(editors: readonly TextEditor[]): void {
  draw();
  for (const editor of editors) {
    if (novis(editor.document) && !held.knows(editor.document.uri.toString())) {
      void ask(editor.document);
    }
  }
}

/** Ask the server what `document` conceals, and draw the answer. */
async function ask(document: TextDocument): Promise<void> {
  const client = serving;
  if (client === undefined || !novis(document)) {
    return;
  }
  const asked = document.version;
  let answered: Redaction[];
  try {
    answered = await client.sendRequest<Redaction[]>(METHOD, { uri: document.uri.toString() });
  } catch {
    // Hold the last answer rather than clearing: see `serve`.
    return;
  }
  if (document.isClosed || document.version !== asked) {
    // A later edit has an ask of its own in flight, or the editor has gone. Either way this answer
    // describes text that is no longer on screen, and drawing it would conceal the wrong columns.
    return;
  }
  held.hold(document.uri.toString(), answered);
  draw();
}

/** Put both decorations on every visible editor, each as far as its own setting asks. */
function draw(): void {
  const bar = concealing;
  const mark = marking;
  if (bar === undefined || mark === undefined) {
    return;
  }
  const drawing: Drawn[] = [];
  for (const editor of window.visibleTextEditors) {
    if (!novis(editor.document)) {
      continue;
    }
    const settings = workspace.getConfiguration("nvs", editor.document.uri);
    const uri = editor.document.uri.toString();
    const concealed = settings.get<boolean>("secrets.redact", true) ? options(editor) : [];
    const marked = settings.get<string>("taint.mark", "off") === "off" ? [] : marks(uri);
    editor.setDecorations(bar, concealed);
    editor.setDecorations(mark, marked);
    drawing.push({
      document: uri,
      column: editor.viewColumn,
      concealed: concealed.map((option) => option.range),
      marked,
    });
  }
  last = drawing;
}

/** The ranges each visible editor was last handed, which is what `surface.ts` exposes of this file. */
export function drawn(): readonly Drawn[] {
  return last;
}

/**
 * The concealment drawn in `editor`: what is held for its document, less whatever a cursor is
 * sitting in.
 *
 * The cursor is the third way to uncover a range and the only one that undoes itself. A user who
 * clicks into a `secret` is reading or editing that value and wants to see it; moving out covers it
 * again with nothing held anywhere, so the range is never left uncovered behind them. That is why
 * this is computed here from `editor.selections` rather than recorded in `concealment.ts` beside
 * the explicit reveals: there is no state to get wrong, and no state to outlive the moment.
 *
 * `Range.contains` takes the end position too, so a cursor parked immediately after the closing
 * quote counts as inside. That is the direction that matches what a user means by clicking at the
 * end of a string, and the range is covered again as soon as they leave it either way.
 */
function options(editor: TextEditor): DecorationOptions[] {
  const uri = editor.document.uri.toString();
  return held
    .concealed(uri)
    .filter((range) => !under(editor, range))
    .map((range) => ({
      range: span(range),
      hoverMessage: hover(uri, range.start),
    }));
}

/** Whether a cursor in `editor` sits inside `range`. */
function under(editor: TextEditor, range: Wire): boolean {
  const at = span(range);
  return editor.selections.some((selection) => at.contains(selection.active));
}

/**
 * The marked ranges held for `uri`, as the editor takes them.
 *
 * No hover: the glyph says the declaration beside it carries `tainted`, and the declaration itself
 * already spells the qualifier. There is nothing concealed here to offer a link to uncover.
 */
function marks(uri: string): Range[] {
  return held.marked(uri).map(span);
}

/** One range off the wire as the editor's own, which is the whole conversion this file does. */
function span(range: Wire): Range {
  return new Range(
    new Position(range.start.line, range.start.character),
    new Position(range.end.line, range.end.character),
  );
}

/**
 * What is said over one concealed range: what it is, and the one link that uncovers it.
 *
 * The sentence describes the declaration and never the bytes — a hover that quoted the value would
 * put it on the screen the concealment took it off. The link carries the range's own position
 * because hovering does not move the cursor, so the command that a palette entry aims at the
 * cursor is aimed here at the range under the pointer.
 */
function hover(uri: string, at: Where): MarkdownString {
  const where: Where & { uri: string } = { uri, line: at.line, character: at.character };
  const message = new MarkdownString(
    `A \`secret\` value, concealed. The buffer holds the real text.\n\n`
      + `[Reveal](command:${REVEAL}?${encodeURIComponent(JSON.stringify([where]))})`,
  );
  // The narrowest trust this API offers: a command link runs nothing but the reveal, so a
  // `secret` on screen is never also a way to run something else.
  message.isTrusted = { enabledCommands: [REVEAL] };
  return message;
}

/**
 * Reveal one concealed range: the one `where` names, or the one under the cursor.
 *
 * One range and not a document's worth (`rule:ide/reveal-is-explicit-and-window-local`), and
 * nothing is written down — the state is `concealment.ts`'s, in this process, and closing the
 * editor is the end of it.
 */
export function reveal(where?: Where & { uri?: string }): void {
  const editor = window.activeTextEditor;
  const uri = where?.uri ?? (editor !== undefined && novis(editor.document)
    ? editor.document.uri.toString()
    : undefined);
  const at = where ?? (editor === undefined ? undefined : editor.selection.active);
  if (uri === undefined || at === undefined) {
    return;
  }
  if (!held.reveal(uri, { line: at.line, character: at.character })) {
    void window.showInformationMessage("Novis: nothing is concealed at the cursor.");
    return;
  }
  draw();
}

/** Re-conceal every revealed range in this window, which is `nvs.hideSecrets`. */
export function hide(): void {
  held.hide();
  draw();
}

// The documents this draws on, which are `extension.ts`'s `SELECTOR` in the form a listener asks
// it: a notebook cell or a diff's right-hand side is not a file the server has open.
function novis(document: TextDocument): boolean {
  return document.languageId === "nvs" && document.uri.scheme === "file";
}
