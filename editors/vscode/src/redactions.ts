// The concealment this client draws: `nvs/redactions` asked of the server, and one decoration over
// every range it answers.
//
// The request is Novis's own and the only one that is (`rule:ide/the-request-set-is-closed`); its
// spelling and its answer's shape live in `crates/nvs-lsp/src/redactions.rs`, and this file is the
// half that renders them. It reads no source text and matches no literal — which literal carries
// `secret` is a question with one answer and the server holds it
// (`rule:security/redaction-ranges-come-from-the-server`).
//
// What is drawn is a decoration and not an edit. The buffer is the file on disk byte for byte, and
// the character cells stay where they were, so the cursor, the selection and every edit still
// address the real text (`rule:security/redaction-covers-bytes-only`).
//
// A reveal is one range, and what is revealed lives in `concealment.ts` where no `vscode` import
// reaches it. This file is the part that cannot be tested without a display, which is why it is as
// thin as it is: it converts positions, asks, and calls `setDecorations`.

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

import { Concealment, Position as Where, Redaction } from "./concealment";

// The server's own request, spelled where `crates/nvs-lsp/src/redactions.rs` spells it. Its params
// are an LSP `TextDocumentIdentifier` and its answer a list of `{range, kind}`.
const METHOD = "nvs/redactions";

// The setting that turns the concealment off, default `true`. It respawns nothing: the server
// answers the same ranges either way and only the drawing changes, which is why it is not one of
// `extension.ts`'s `RESPAWNING_SETTINGS`.
const SETTING = "nvs.secrets.redact";

// The command the decoration's own hover offers, which ADR 0101 § 3 makes the discoverable path to
// a reveal — the palette and a keybinding are the other two, and all three are the same command.
const REVEAL = "nvs.revealSecret";

const held = new Concealment();
let concealing: TextEditorDecorationType | undefined;
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
  context.subscriptions.push(
    concealing,
    workspace.onDidOpenTextDocument((document) => void ask(document)),
    workspace.onDidChangeTextDocument((event) => void ask(event.document)),
    workspace.onDidCloseTextDocument((document) => held.forget(document.uri.toString())),
    window.onDidChangeVisibleTextEditors((editors) => shown(editors)),
    workspace.onDidChangeConfiguration((event) => {
      if (event.affectsConfiguration(SETTING)) {
        draw();
      }
    }),
  );
}

/**
 * The client every ask goes to, or `undefined` while none is running.
 *
 * A server that stops leaves the concealment on screen untouched. That is the whole of
 * `rule:security/redaction-ranges-come-from-the-server`'s fail direction: a missing answer means
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
 * The decoration one concealed range is drawn with.
 *
 * The glyphs and the bar over them are one colour, so the bytes are unreadable while every
 * character cell stays exactly the width it was. A `before`/`after` content or a `display: none`
 * would move them, and then a click would land on a column the buffer does not have there.
 *
 * Both colours are `ThemeColor` references and neither is a literal: what a concealed range looks
 * like is the user's theme's to decide, the same way `rule:ide/novis-ships-names-not-colours` has
 * the two colouring layers ship names alone.
 */
function bar(): TextEditorDecorationType {
  return window.createTextEditorDecorationType({
    color: new ThemeColor("editor.foreground"),
    backgroundColor: new ThemeColor("editor.foreground"),
    // A character typed against either edge is outside the concealed range until the server says
    // otherwise, which is the direction that cannot draw over bytes nobody answered for.
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

/** Put the current concealment on every visible editor. */
function draw(): void {
  const decoration = concealing;
  if (decoration === undefined) {
    return;
  }
  for (const editor of window.visibleTextEditors) {
    if (!novis(editor.document)) {
      continue;
    }
    const redacting = workspace
      .getConfiguration("nvs", editor.document.uri)
      .get<boolean>("secrets.redact", true);
    editor.setDecorations(decoration, redacting ? options(editor.document.uri.toString()) : []);
  }
}

/** The concealment held for `uri`, as the editor takes it. */
function options(uri: string): DecorationOptions[] {
  return held.concealed(uri).map((range) => ({
    range: new Range(
      new Position(range.start.line, range.start.character),
      new Position(range.end.line, range.end.character),
    ),
    hoverMessage: hover(uri, range.start),
  }));
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
 * One range and not a document's worth (`rule:security/reveal-is-explicit-and-window-local`), and
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
