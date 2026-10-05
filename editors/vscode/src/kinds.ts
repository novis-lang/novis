// What each Novis file declares, as a badge beside its name in the Explorer and on its tab
// (`rule:ide/a-file-shows-what-it-declares`).
//
// The server answers `nvs/fileKinds` with every file that declares exactly one class, interface,
// enum or `type` alias and nothing else, which is the shape an autoloaded file must have. This file
// turns one answer into badges and says which files changed between two answers. `badges.ts` asks
// the server and hands the result to the editor. Nothing here imports `vscode`, which is what lets
// the headless tier hold it without an editor.
//
// The colour is a theme colour the editor already has for the same kind of symbol in the Outline
// and in completion, and never a colour of this extension's own
// (`rule:ide/novis-ships-names-not-colours`).

/** The server's own request, spelled where `crates/nvs-lsp/src/file_kinds.rs` spells it. */
export const METHOD = "nvs/fileKinds";

/** The four words the server answers with. */
export type Kind = "class" | "interface" | "enum" | "type";

/** One entry of the server's answer. */
export interface FileKind {
  readonly uri: string;
  readonly kind: string;
}

/** What the editor draws for one kind: the letter, the theme colour's id, and the hover text. */
export interface Badge {
  readonly letter: string;
  readonly color: string;
  readonly tooltip: string;
}

/** The badge for each kind. */
export const BADGES: Readonly<Record<Kind, Badge>> = {
  class: { letter: "C", color: "symbolIcon.classForeground", tooltip: "This file declares a class." },
  interface: {
    letter: "I",
    color: "symbolIcon.interfaceForeground",
    tooltip: "This file declares an interface.",
  },
  enum: { letter: "E", color: "symbolIcon.enumeratorForeground", tooltip: "This file declares an enum." },
  type: {
    letter: "T",
    color: "symbolIcon.typeParameterForeground",
    tooltip: "This file declares a type alias.",
  },
};

/** Whether `word` is one of the four kinds. A newer server may answer a word this client does not know. */
export function isKind(word: string): word is Kind {
  return Object.prototype.hasOwnProperty.call(BADGES, word);
}

/**
 * The key a file is looked up by: its file-system path, in lower case on Windows. The server and the
 * editor spell one file's URI differently (`file:///E:/a` and `file:///e%3A/a`), and on Windows they
 * also differ in the case of the drive letter, which the file system ignores.
 */
export function key(fsPath: string, platform: string): string {
  return platform === "win32" ? fsPath.toLowerCase() : fsPath;
}

/** The keys whose kind is not the same in `before` and `after`, a file added or removed included. */
export function changed(before: ReadonlyMap<string, Kind>, after: ReadonlyMap<string, Kind>): string[] {
  const keys = new Set([...before.keys(), ...after.keys()]);
  return [...keys].filter((file) => before.get(file) !== after.get(file));
}
