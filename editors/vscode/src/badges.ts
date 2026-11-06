// The badge beside each Novis file that declares one type or has routes: the editor's own file
// decorations, fed from the server's `nvs/fileKinds` (`rule:ide/a-file-shows-what-it-declares`).
//
// The server knows a file once it is in its index: while it reads the workspace at start, after
// `nvs.checkWorkspace`, and when the file is opened or edited. So this asks again at the same
// moments. It asks when a server starts answering, when the server says it has read every waiting
// file, when a Novis document is opened or saved, and after a workspace pass. Several asks close
// together are one ask, because opening a folder of files opens many documents at once. An answer
// that arrives after a newer ask was sent is dropped.
//
// With no server answering there is nothing to ask, and every badge is removed.

import {
  Disposable,
  EventEmitter,
  ExtensionContext,
  FileDecoration,
  FileDecorationProvider,
  TextDocument,
  Uri,
  window,
  workspace,
} from "vscode";
import { LanguageClient } from "vscode-languageclient/node";

import { Badge, CHANGED, FileKind, METHOD, badgeOf, changed, key } from "./kinds";

/** How long a burst of opened or saved documents is waited out before the server is asked. */
const SETTLE_MS = 300;

/** One file the last answer named. */
interface Known {
  readonly uri: Uri;
  readonly badge: Badge;
}

let serving: LanguageClient | undefined;
let listening: Disposable | undefined;
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
    { dispose: () => listening?.dispose() },
  );
}

/** The client every ask goes to, or `undefined` while none is running. */
export function serve(client: LanguageClient | undefined): void {
  listening?.dispose();
  listening = client?.onNotification(CHANGED, () => soon());
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
    const badge = badgeOf(file);
    if (badge !== undefined) {
      const uri = Uri.parse(file.uri);
      next.set(key(uri.fsPath, process.platform), { uri, badge });
    }
  }
  const before = known;
  known = next;
  const redraw = changed(badges(before), badges(next)).flatMap((file) => {
    const uri = (next.get(file) ?? before.get(file))?.uri;
    return uri === undefined ? [] : [uri];
  });
  if (redraw.length > 0) {
    changes.fire(redraw);
  }
}

/** The badge of each file in `files`, by key. */
function badges(files: ReadonlyMap<string, Known>): Map<string, Badge> {
  return new Map([...files].map(([file, { badge }]) => [file, badge]));
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
    return new FileDecoration(found.badge.letter, found.badge.tooltip);
  }
}
