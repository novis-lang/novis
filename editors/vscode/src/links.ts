// What a ctrl-click on a link to a directory does.
//
// The server links an `autoload` root and a `discover` glob to the directory each one names, and it
// sends a directory as a `file:` URI ending in `/`, which is how a URI names a directory
// (`crates/nvs-lsp/src/links.rs`). An editor tab cannot show a directory, so this client sends such a
// link to the Explorer: the target becomes the editor's own `revealInExplorer` command, as a
// `command:` URI with the directory as its one argument. A link to a file is left as the server sent
// it. Nothing here imports `vscode`, which is what lets the headless tier hold it without an editor.

/** The part of a `vscode.Uri` this module reads, so a test can pass a plain object. */
export interface Target {
  readonly scheme: string;
  readonly path: string;
  with(change: { path: string }): Target;
}

/** The command that shows a file or a directory in the Explorer. */
export const REVEAL = "revealInExplorer";

/**
 * The `command:` URI that reveals `target` in the Explorer, or `undefined` for a target that is not a
 * directory. The argument is the directory without its trailing `/`, serialized the way a command URI
 * carries its arguments: a JSON array, percent-encoded.
 */
export function revealing(target: Target): string | undefined {
  if (target.scheme !== "file" || !target.path.endsWith("/")) {
    return undefined;
  }
  const directory = target.with({ path: target.path.replace(/\/+$/, "") });
  return `command:${REVEAL}?${encodeURIComponent(JSON.stringify([directory]))}`;
}
