// What this client looks like from outside it: what the status item says, the tree the AST panel is
// drawn from, and the ranges each visible editor was last handed for each decoration.
//
// It exists because those three are write-only through VS Code's own API. `setDecorations` takes
// ranges and gives none back, and a `LanguageStatusItem` is a sink in the same way, so a suite
// running inside the extension host can observe neither without this client saying so. Everything
// else the host tier asserts it reads through the editor — the published diagnostics, the semantic
// tokens, the scopes, the loaded extensions — and nothing of that kind belongs here.
//
// **No member changes, reveals or runs anything**, and that is a security property rather than a
// matter of taste. `activate`'s return value is `extension.exports`, which every other extension in
// the window can reach: a reveal on this object would be a way for any of them to uncover a `secret`
// on the user's screen without the user asking, which is exactly what
// `rule:ide/reveal-is-explicit-and-window-local` reserves for the person at the keyboard. So the
// readings below are plain values copied out at the moment they are asked for, holding no editor, no
// decoration type, no status item and no client. The README's § *Decided here* is where that is
// recorded.

import { LanguageStatusSeverity, Range, TreeDataProvider, ViewColumn } from "vscode";

import { Node } from "./nodes";

/**
 * What the status item shows right now: the text as the user reads it, its detail and its severity.
 *
 * The detail is the sentence under the state word, and it is here for the same reason the text is:
 * it is the only place the client says *why* it is in the state it is in — which binary it found,
 * why a copy was not started, what a refused server reported — and the host suite has no other way
 * to read it back.
 */
export interface Health {
  readonly text: string;
  readonly detail: string;
  readonly severity: LanguageStatusSeverity;
}

/**
 * What one visible editor was last handed, per decoration kind.
 *
 * It is per editor and not per document because the two differ: a document open in a split has a
 * cursor in each half, and a cursor inside a concealed range uncovers it in that editor alone
 * (`rule:ide/reveal-is-explicit-and-window-local`).
 */
export interface Drawn {
  /** The editor's document, as `Uri.toString()` spells it. */
  readonly document: string;
  readonly column: ViewColumn | undefined;
  /** The ranges the concealing decoration covers, which is what is blurred on screen. */
  readonly concealed: readonly Range[];
  /** The ranges the `tainted` marker is drawn after, which is empty until a setting asks for one. */
  readonly marked: readonly Range[];
}

/** The object `activate` returns. */
export interface Surface {
  /** The status item's reading, or nothing if activation did not get as far as creating it. */
  readonly status: Health | undefined;
  /** The provider the AST view draws from, which answers what the panel is showing. */
  readonly ast: TreeDataProvider<Node>;
  /** One entry per visible Novis editor, in the order the editor lists them. */
  readonly drawn: readonly Drawn[];
}
