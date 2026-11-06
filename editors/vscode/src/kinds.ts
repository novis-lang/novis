// What each Novis file declares, as a badge beside its name in the Explorer and on its tab
// (`rule:ide/a-file-shows-what-it-declares`).
//
// The server answers `nvs/fileKinds` with every file that declares exactly one class, interface,
// enum or `type` alias and nothing else, which is the shape an autoloaded file must have, and with
// every file that has a route handler, with its routes. A file with routes shows `R` whatever else
// it declares, because serving requests is the more useful thing to see in a list of files. This
// file turns one answer into badges and says which files changed between two answers. `badges.ts`
// asks the server and hands the result to the editor. Nothing here imports `vscode`, which is what
// lets the headless tier hold it without an editor.
//
// A badge is one plain letter and carries no colour. A decoration's colour is drawn on the file's
// name as well as on its badge, so a colour would recolour every name in the list. With no colour
// the editor draws the letter in the name's own colour, dimmed.

/** The server's own request, spelled where `crates/nvs-lsp/src/file_kinds.rs` spells it. */
export const METHOD = "nvs/fileKinds";

/** What the server sends once it has read every waiting file, so the client asks again. */
export const CHANGED = "nvs/fileKindsChanged";

/** The four words the server answers with. */
export type Kind = "class" | "interface" | "enum" | "type";

/**
 * One entry of the server's answer. `kind` is absent for a file that declares anything but one
 * type, and `routes` is absent for a file with no route handler. One of the two is always there.
 */
export interface FileKind {
  readonly uri: string;
  readonly kind?: string;
  readonly routes?: readonly string[];
}

/** What the editor draws for one file: the letter, and the hover text. */
export interface Badge {
  readonly letter: string;
  readonly tooltip: string;
}

/** The badge for each kind. */
export const BADGES: Readonly<Record<Kind, Badge>> = {
  class: { letter: "C", tooltip: "This file declares a class." },
  interface: { letter: "I", tooltip: "This file declares an interface." },
  enum: { letter: "E", tooltip: "This file declares an enum." },
  type: { letter: "T", tooltip: "This file declares a type alias." },
};

/** The letter of a file with routes. */
export const ROUTES = "R";

/** How many routes a tooltip names before it gives the count of the others. */
export const NAMED = 5;

/** Whether `word` is one of the four kinds. A newer server may answer a word this client does not know. */
export function isKind(word: string | undefined): word is Kind {
  return word !== undefined && Object.prototype.hasOwnProperty.call(BADGES, word);
}

/** The badge for one entry of the answer, or `undefined` when this client knows nothing it says. */
export function badgeOf(file: FileKind): Badge | undefined {
  const routes = Array.isArray(file.routes) ? file.routes : [];
  if (routes.length > 0) {
    const what = file.kind === "class" ? "This file declares a class with" : "This file has";
    const count = routes.length === 1 ? "1 route" : `${routes.length} routes`;
    const named = routes.slice(0, NAMED).join(", ");
    const rest = routes.length > NAMED ? ` and ${routes.length - NAMED} more` : "";
    return { letter: ROUTES, tooltip: `${what} ${count}: ${named}${rest}.` };
  }
  return isKind(file.kind) ? BADGES[file.kind] : undefined;
}

/**
 * The key a file is looked up by: its file-system path, in lower case on Windows. The server and the
 * editor spell one file's URI differently (`file:///E:/a` and `file:///e%3A/a`), and on Windows they
 * also differ in the case of the drive letter, which the file system ignores.
 */
export function key(fsPath: string, platform: string): string {
  return platform === "win32" ? fsPath.toLowerCase() : fsPath;
}

/** The keys whose badge is not the same in `before` and `after`, a file added or removed included. */
export function changed(before: ReadonlyMap<string, Badge>, after: ReadonlyMap<string, Badge>): string[] {
  const keys = new Set([...before.keys(), ...after.keys()]);
  const same = (a: Badge | undefined, b: Badge | undefined): boolean =>
    a?.letter === b?.letter && a?.tooltip === b?.tooltip;
  return [...keys].filter((file) => !same(before.get(file), after.get(file)));
}
