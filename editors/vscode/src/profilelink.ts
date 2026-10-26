// A *View Profile* link: `vscode://novis-lang.nvs/profile?file=<percent-encoded absolute path>`,
// which the debug viewer's page writes so that a profile it lists opens in the editor
// (`rule:testing/a-site-in-the-viewer-opens-in-the-editor`).
//
// Any web page can open a `vscode://` link, so a link is untrusted input. It is accepted only when
// it names a speedscope file by an absolute local path, and the one thing it can do is open that
// file in the read-only *View Profile* webview. A network path (`\\host\share`, `//host/share`) is
// refused, because reading one on Windows sends the user's credentials to that host.
//
// Nothing here imports `vscode`, so the headless suite runs it in plain Node. `src/extension.ts`
// registers the handler and `src/profile.ts` opens the panel.

/** The name `nvs run --profile=speedscope` writes ends with this. */
export const SUFFIX = ".speedscope.json";

/** The one path a profile link has. */
export const LINK_PATH = "/profile";

/** What a link names: the absolute file to open, or why it is refused. */
export type Link = { file: string } | { refused: string };

/**
 * Reads a profile link from its URI's `path` and `query`.
 *
 * `query` is `Uri.query`, which VS Code has percent-decoded once. Each value is decoded once more,
 * so a link whose path was encoded once arrives the same as one that went through two encoders.
 * The parameters are split on `&` before that, so a path with an `&` in it is refused.
 */
export function profileLink(path: string, query: string): Link {
  if (path !== LINK_PATH) {
    return { refused: `the link's path is ${JSON.stringify(path)}, not ${LINK_PATH}` };
  }
  const files: string[] = [];
  for (const part of query.split("&")) {
    if (part === "") {
      continue;
    }
    const equals = part.indexOf("=");
    const name = equals < 0 ? part : part.slice(0, equals);
    if (decoded(name) !== "file") {
      return { refused: `the link has a parameter ${JSON.stringify(name)}, and only file is read` };
    }
    files.push(decoded(equals < 0 ? "" : part.slice(equals + 1)));
  }
  if (files.length !== 1) {
    return { refused: `the link has ${files.length} file parameters, not one` };
  }
  const file = files[0] ?? "";
  if (!absolute(file)) {
    return { refused: `${JSON.stringify(file)} is not an absolute local path` };
  }
  if (!file.endsWith(SUFFIX)) {
    return { refused: `${JSON.stringify(file)} is not a ${SUFFIX} file` };
  }
  return { file };
}

/**
 * A POSIX path from the root, or a Windows path from a drive letter. A network path, a device path
 * (`\\?\`, `\\.\`) and a control character are not.
 */
function absolute(file: string): boolean {
  for (const char of file) {
    const code = char.charCodeAt(0);
    if (code < 0x20 || code === 0x7f) {
      return false;
    }
  }
  if (/^[\\/]{2}/.test(file)) {
    return false;
  }
  return file.startsWith("/") || /^[A-Za-z]:[\\/]/.test(file);
}

function decoded(text: string): string {
  try {
    return decodeURIComponent(text);
  } catch {
    // A `%` that is not an escape is part of the name. VS Code has already decoded the escapes.
    return text;
  }
}
