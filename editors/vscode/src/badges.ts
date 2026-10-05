// The badge beside each Novis file that declares one type: the editor's own file decorations, fed
// from the server's `nvs/fileKinds` (`rule:ide/a-file-shows-what-it-declares`).
//
// The server knows a file once it is in its index: at start, after `nvs.checkWorkspace`, and when
// the file is opened or edited. So this asks again at the same moments. It asks when a server starts
// answering, when a Novis document is opened or saved, and after a workspace pass. Several asks close
// together are one ask, because opening a folder of files opens many documents at once. An answer
// that arrives after a newer ask was sent is dropped.
//
// With no server answering there is nothing to ask, and every badge is removed.

import {
  EventEmitter,
  ExtensionContext,
  FileDecoration,
  FileDecorationProvider,
  TextDocument,
  ThemeColor,
  Uri,
  window,
  workspace,
} from "vscode";
import { LanguageClient } from "vscode-languageclient/node";

import { BADGES, FileKind, Kind, METHOD, changed, isKind, key } from "./kinds";

/** How long a burst of opened or saved documents is waited out before the server is asked. */
const SETTLE_MS = 300;

/** One file the last answer named. */
interface Known {
  readonly uri: Uri;
  readonly kind: Kind;
}

let serving: LanguageClient | undefined;
let known = new Map<string, Known>();
let asked = 0;
let timer: ReturnType<typeof setTimeout> | undefined;
const changes = new EventEmitter<Uri[]>();

/** Register the provider and the moments it asks again. Called once from `activate`. */
export function install(context: ExtensionContext): void {
  const asking = (document: TextDocument): void => {
    if (document.languageId === "nvs") {
      soon();
    }
  };
  context.subscriptions.push(
    changes,
    window.registerFileDecorationProvider(new Provider()),
    workspace.onDidOpenTextDocument(asking),
    workspace.onDidSaveTextDocument(asking),
    { dispose: () => clearTimeout(timer) },
  );
}

/** The client every ask goes to, or `undefined` while none is running. */
export function serve(client: LanguageClient | undefined): void {
  serving = client;
  void refresh();
}

/** Ask the server once a burst of opened or saved documents is over. */
export function soon(): void {
  clearTimeout(timer);
  timer = setTimeout(() => void refresh(), SETTLE_MS);
}

/** Ask the server now, and redraw every file whose badge changed. */
export async function refresh(): Promise<void> {
  const ticket = ++asked;
  const client = serving;
  let answer: FileKind[] = [];
  if (client !== undefined) {
    try {
      answer = await client.sendRequest<FileKind[]>(METHOD);
    } catch {
      // A server that cannot answer leaves no badge, which is what the Explorer showed before.
      answer = [];
    }
  }
  if (ticket !== asked) {
    return;
  }
  const next = new Map<string, Known>();
  for (const file of answer) {
    if (isKind(file.kind)) {
      const uri = Uri.parse(file.uri);
      next.set(key(uri.fsPath, process.platform), { uri, kind: file.kind });
    }
  }
  const before = known;
  known = next;
  const redraw = changed(kinds(before), kinds(next)).flatMap((file) => {
    const uri = (next.get(file) ?? before.get(file))?.uri;
    return uri === undefined ? [] : [uri];
  });
  if (redraw.length > 0) {
    changes.fire(redraw);
  }
}

/** The kind of each file in `files`, by key. */
function kinds(files: ReadonlyMap<string, Known>): Map<string, Kind> {
  return new Map([...files].map(([file, { kind }]) => [file, kind]));
}

class Provider implements FileDecorationProvider {
  readonly onDidChangeFileDecorations = changes.event;

  provideFileDecoration(uri: Uri): FileDecoration | undefined {
    if (uri.scheme !== "file") {
      return undefined;
    }
    const found = known.get(key(uri.fsPath, process.platform));
    if (found === undefined) {
      return undefined;
    }
    const badge = BADGES[found.kind];
    return new FileDecoration(badge.letter, badge.tooltip, new ThemeColor(badge.color));
  }
}
