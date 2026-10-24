// *Paste as doc comment*, decided without an editor (`rule:ide/doc-comment-authoring-is-the-editors-own`).
//
// A `///` run is line-oriented, so text of several lines pasted into one would leave every line after
// the first outside the comment. When the cursor sits after a line's `///` marker, the paste is offered
// a second time with each later line given the run's indentation and marker, and an empty line written
// as a bare `///`. The first line needs nothing: it lands after the marker already on the cursor's line.
// Nothing here imports `vscode`, which is what lets the headless tier hold it; `doccomments.ts` is the
// provider that calls it.

/**
 * The kind the paste is offered under. It reaches a user's `editor.pasteAs.preferences`, so it is
 * frozen on `rule:ide/contributions-are-frozen-and-only-ever-added`'s terms. The last segment is this
 * language's, the way the import paste's is.
 */
export const DOC_COMMENT_KIND = "text.docComment.nvs";

/** The title the editor's paste widget shows for the edit. */
export const DOC_COMMENT_TITLE = "Paste as doc comment";

// The text before the cursor on a line in a `///` run: indentation, the marker, and anything after it.
const IN_RUN = /^([ \t]*)\/\/\/(?!\/)/;

/**
 * `text` with every line after the first prefixed as a line of the run, or `undefined` where the
 * paste is not offered: the cursor is not after a `///` marker, or `text` is a single line.
 *
 * `before` is the cursor's line up to the cursor. `eol` is the document's line ending, which every
 * pasted line ending is written as. One line ending at the very end of `text` is dropped, so a copied
 * run of whole lines leaves the cursor on the last of them rather than on a new, empty one.
 */
export function asDocComment(before: string, text: string, eol: string): string | undefined {
  const run = IN_RUN.exec(before);
  if (run === null) {
    return undefined;
  }
  const lines = text.replace(/\r?\n$/, "").split(/\r?\n/);
  if (lines.length < 2) {
    return undefined;
  }
  const indent = run[1];
  return lines
    .map((line, index) => {
      if (index === 0) {
        return line;
      }
      return line.trim() === "" ? `${indent}///` : `${indent}/// ${line}`;
    })
    .join(eol);
}
