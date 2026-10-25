// *View Profile*: a speedscope file, from `nvs run --profile=speedscope` or `Core\Debug::startProfile`,
// in a webview that runs the bundled speedscope viewer
// (`rule:ide/the-extension-builds-no-ui-the-editor-already-has`).
//
// The file is read through `workspace.fs`, so it works in a remote window, and the viewer runs inside
// the editor, so a profile never leaves the machine. What the page is, and the policy it runs under,
// is `src/viewer.ts`, which the headless suite tests. This file picks the file and opens the panel.

import { randomBytes } from "node:crypto";
import { basename } from "node:path";

import { ExtensionContext, Uri, ViewColumn, window, workspace } from "vscode";

import { VIEWER, page } from "./viewer";

/** The name `nvs run --profile=speedscope` writes ends with this. */
const SUFFIX = ".speedscope.json";

/** Opens `target`, or the active editor's speedscope file, or the file the user picks. */
export async function view(context: ExtensionContext, target?: Uri): Promise<void> {
  const file = target ?? active() ?? (await pick());
  if (file === undefined) {
    return;
  }
  const bytes = await workspace.fs.readFile(file);
  const folder = Uri.joinPath(context.extensionUri, ...VIEWER);
  const index = new TextDecoder().decode(await workspace.fs.readFile(Uri.joinPath(folder, "index.html")));
  const name = basename(file.path);
  const panel = window.createWebviewPanel("nvs.profile", name, ViewColumn.Active, {
    enableScripts: true,
    enableCommandUris: false,
    // The viewer's folder alone. The profile's bytes are in the page, so its folder is not served.
    localResourceRoots: [folder],
    retainContextWhenHidden: true,
  });
  panel.webview.html = page({
    index,
    base: panel.webview.asWebviewUri(folder).toString(),
    source: panel.webview.cspSource,
    nonce: randomBytes(16).toString("base64"),
    name,
    bytes,
  });
}

function active(): Uri | undefined {
  const uri = window.activeTextEditor?.document.uri;
  return uri !== undefined && uri.path.endsWith(SUFFIX) ? uri : undefined;
}

async function pick(): Promise<Uri | undefined> {
  const chosen = await window.showOpenDialog({
    canSelectMany: false,
    openLabel: "View Profile",
    filters: { "speedscope profile": ["json"] },
  });
  return chosen?.[0];
}
