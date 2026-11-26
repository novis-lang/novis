// *Open Debug Viewer* and *Open Debug Viewer in Browser*: the debug viewer that `nvs debug` serves,
// in a panel of the editor or in the developer's browser, from one button
// (`rule:ide/the-debug-viewer-opens-from-one-button`).
//
// The link comes from the language server's `nvs/debugViewer`, which reads the data folder and
// `[debug] listen` as `nvs debug link` does. `nvs.debugViewer.openIn` decides what the button does.
// The panel is `src/debugpage.ts`'s page, a frame and a script, and draws nothing of its own
// (`rule:ide/the-extension-builds-no-ui-the-editor-already-has`).
//
// **A second click reveals the open panel**, and a command with a path, such as a recorded lens,
// sends that panel `go`, so one panel is open per window. **A file:line clicked in the panel** opens
// in the editor only when the file exists inside a workspace folder, after its links are resolved.
// Nothing here runs a command the page names.
//
// When no service runs, the command offers *Start nvs debug*, which runs `nvs debug` as a task of
// the `nvs` task type and then waits for the service to answer. When `[debug] store` is off, it says
// which line of `nvs.toml` turns recording on and offers to open the file.
//
// Spends: one webview while the panel is open, which holds a frame of the viewer.

import { randomBytes } from "node:crypto";
import { realpath } from "node:fs/promises";

import {
  ColorThemeKind,
  ExtensionContext,
  Range,
  Uri,
  ViewColumn,
  WebviewPanel,
  commands,
  env,
  window,
  workspace,
} from "vscode";
import type { LanguageClient } from "vscode-languageclient/node";

import { goable, opened, page, themeOf, within } from "./debugpage";
import { startDebug } from "./tasks";

/** The request the language server answers with the viewer's links. */
export const METHOD = "nvs/debugViewer";

/** The panel's view type. */
export const PANEL = "nvs.debugViewer";

/** The command the button runs, which a lens also runs with a path. */
export const OPEN = "nvs.openDebugViewer";

/** What `nvs/debugViewer` answers, as `nvs debug link --json` prints it. */
export interface Answer {
  url: string;
  embed: string;
  running: boolean;
  recording: boolean;
}

/** How long *Start nvs debug* waits for the service to answer, and how often it asks. */
const START_WAIT_MS = 15_000;
const START_ASK_MS = 250;

let panel: WebviewPanel | undefined;

/** The panel, while it is open, for the suite that runs inside the extension host. */
export function openPanel(): WebviewPanel | undefined {
  return panel;
}

/** Registers both commands. `client` gives the language client that is answering now. */
export function install(context: ExtensionContext, client: () => LanguageClient | undefined): void {
  context.subscriptions.push(
    commands.registerCommand("nvs.openDebugViewer", (path?: unknown) =>
      open(client, setting(), goable(path) ? path : undefined)),
    commands.registerCommand("nvs.openDebugViewerInBrowser", (path?: unknown) =>
      open(client, "browser", goable(path) ? path : undefined)),
    window.onDidChangeActiveColorTheme((theme) => {
      void panel?.webview.postMessage({ type: "theme", value: themeOf(theme.kind) });
    }),
  );
}

/** `nvs.debugViewer.openIn`: `editor` or `browser`. */
function setting(): "editor" | "browser" {
  return workspace.getConfiguration("nvs").get<string>("debugViewer.openIn") === "browser" ? "browser" : "editor";
}

async function open(
  client: () => LanguageClient | undefined,
  where: "editor" | "browser",
  path: string | undefined,
): Promise<void> {
  if (where === "editor" && panel !== undefined) {
    panel.reveal();
    if (path !== undefined) {
      void panel.webview.postMessage({ type: "go", path });
    }
    return;
  }
  let answer = await ask(client);
  if (answer === undefined) {
    return;
  }
  if (!answer.recording) {
    void notRecording();
  }
  if (!answer.running) {
    answer = await offerStart(client);
    if (answer === undefined) {
      return;
    }
  }
  if (where === "browser") {
    const url = new URL(answer.url);
    if (path !== undefined) {
      const [at = "/", query] = path.split("?", 2);
      url.pathname = at.split("#")[0] ?? "/";
      if (query !== undefined) {
        new URLSearchParams(query).forEach((value, key) => url.searchParams.set(key, value));
      }
    }
    const external = await env.asExternalUri(Uri.parse(url.toString()));
    await env.openExternal(external);
    return;
  }
  const embed = (await env.asExternalUri(Uri.parse(answer.embed))).toString(true);
  show(embed, path ?? "/");
}

/** The server's answer, or `undefined` after a message says why there is none. */
async function ask(client: () => LanguageClient | undefined): Promise<Answer | undefined> {
  const answering = client();
  if (answering === undefined) {
    void window.showErrorMessage("Novis: the debug viewer needs the Novis language server, and it is not running.");
    return undefined;
  }
  try {
    const uri = window.activeTextEditor?.document.uri;
    return await answering.sendRequest<Answer>(METHOD, uri?.scheme === "file" ? { uri: uri.toString() } : {});
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    void window.showErrorMessage(`Novis: ${message}`);
    return undefined;
  }
}

/** Offers to start `nvs debug`, and gives the answer once the service runs. */
async function offerStart(client: () => LanguageClient | undefined): Promise<Answer | undefined> {
  const start = "Start nvs debug";
  const chosen = await window.showInformationMessage(
    "Novis: the debug viewer needs `nvs debug`, and it is not running.",
    start,
  );
  if (chosen !== start) {
    return undefined;
  }
  const folder = folderOfActive();
  if (folder === undefined) {
    void window.showErrorMessage("Novis: `nvs debug` needs a workspace folder. Open one first.");
    return undefined;
  }
  await startDebug(folder);
  const until = Date.now() + START_WAIT_MS;
  while (Date.now() < until) {
    await new Promise((done) => setTimeout(done, START_ASK_MS));
    const answer = await ask(client);
    if (answer === undefined) {
      return undefined;
    }
    if (answer.running) {
      return answer;
    }
  }
  void window.showErrorMessage("Novis: `nvs debug` did not start. Its terminal says why.");
  return undefined;
}

/** Says that no request is recorded, and offers to open `nvs.toml`. */
async function notRecording(): Promise<void> {
  const openFile = "Open nvs.toml";
  const chosen = await window.showWarningMessage(
    "Novis: requests are not recorded. To record them, set `store = true` under `[debug]` in `nvs.toml`.",
    openFile,
  );
  if (chosen !== openFile) {
    return;
  }
  const [file] = await workspace.findFiles("**/nvs.toml", "**/{vendor,node_modules}/**", 1);
  if (file !== undefined) {
    await window.showTextDocument(file);
  }
}

/** The folder of the active document, or the first workspace folder. */
function folderOfActive(): string | undefined {
  const uri = window.activeTextEditor?.document.uri;
  const folder = (uri && workspace.getWorkspaceFolder(uri)) ?? workspace.workspaceFolders?.[0];
  return folder?.uri.scheme === "file" ? folder.uri.fsPath : undefined;
}

/** Opens the panel on `embed` at `path`. */
function show(embed: string, path: string): void {
  const created = window.createWebviewPanel(PANEL, "Novis Debug", ViewColumn.Active, {
    enableScripts: true,
    enableCommandUris: false,
    // The page is inline and its frame is a remote origin, so no local file is served.
    localResourceRoots: [],
    retainContextWhenHidden: true,
  });
  created.webview.html = page({
    embed,
    path,
    theme: themeOf(window.activeColorTheme.kind as ColorThemeKind),
    nonce: randomBytes(16).toString("base64"),
  });
  created.webview.onDidReceiveMessage((message: unknown) => openSite(message, created));
  created.onDidDispose(() => {
    if (panel === created) {
      panel = undefined;
    }
  });
  panel = created;
}

/** Opens the file and line of an `open` message, when the file is inside a workspace folder. */
async function openSite(message: unknown, from: WebviewPanel): Promise<void> {
  const site = opened(message);
  if (site === undefined) {
    return;
  }
  const folders = (workspace.workspaceFolders ?? [])
    .filter((folder) => folder.uri.scheme === "file")
    .map((folder) => folder.uri.fsPath);
  let file: string;
  let roots: string[];
  try {
    file = await realpath(site.file);
    roots = await Promise.all(folders.map((folder) => realpath(folder).catch(() => folder)));
  } catch {
    void window.showWarningMessage(`Novis: \`${site.file}\` does not exist.`);
    return;
  }
  if (!within(file, roots)) {
    void window.showWarningMessage(`Novis: \`${site.file}\` is not inside a workspace folder, so it was not opened.`);
    return;
  }
  const at = new Range(site.line - 1, (site.column ?? 1) - 1, site.line - 1, (site.column ?? 1) - 1);
  await window.showTextDocument(Uri.file(file), {
    selection: at,
    preview: false,
    viewColumn: from.viewColumn === ViewColumn.One ? ViewColumn.Two : ViewColumn.One,
  });
}
