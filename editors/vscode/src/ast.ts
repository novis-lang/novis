// The AST panel: `nvs ast --json` for the file in front of you, drawn in the editor's own tree.
//
// It shells out to the CLI (`rule:ide/the-ast-panel-shells-out-to-the-cli`) rather than parsing
// anything here and rather than reaching for `Core\Ast`, which is a running program's reflective
// parse and not an editor's. One grammar answers both the compiler and this panel
// (`rule:ide/one-grammar-one-tree`), so the tree drawn here cannot disagree with the one the file
// compiles through.
//
// **A non-zero exit is the ordinary case, not a failure.** `nvs ast` is resilient by default and
// prints the tree it recovered whatever the parser reported, exiting non-zero because diagnostics
// were reported — and the file a developer opens this panel for is usually the one that does not
// compile. Standard output is therefore what decides, and `--strict`, which would print nothing for
// exactly that file, is never passed.
//
// The tree view is VS Code's own (`rule:ide/the-extension-builds-no-ui-the-editor-already-has`):
// no webview, no HTML, no renderer of this project's. What a node reads as is `src/nodes.ts`, which
// imports no `vscode` and is where the headless suite tests it.

import { execFile } from "node:child_process";

import {
  EventEmitter,
  ExtensionContext,
  Range,
  Selection,
  TextDocument,
  TextEditorRevealType,
  TreeDataProvider,
  TreeItem,
  TreeItemCollapsibleState,
  TreeView,
  Uri,
  commands,
  window,
  workspace,
} from "vscode";

import { binary } from "./binary";
import { Node, detail, index, label, read, tooltip } from "./nodes";

/** The view `package.json` contributes, and the id the editor's own `<view>.focus` hangs off. */
export const VIEW = "nvs.ast";

// An AST document is larger than the file it describes — every whitespace run is a node
// (`crates/nvs-cli/src/ast.rs`) — and Node's default cap on a child process's output is a
// megabyte, which a few thousand lines of Novis reaches. The panel holds one document per window
// for as long as it is shown, which is the memory this feature spends and all of it.
const OUTPUT_CEILING = 64 * 1024 * 1024;

/** The tree on display: the file it was taken from, and the root it hangs from. */
let shown: { file: string; root: Node } | undefined;

let view: TreeView<Node> | undefined;
const changed = new EventEmitter<void>();

/** Create the view, and keep what it shows in step with the file it was taken from. */
export function install(context: ExtensionContext): void {
  const created = window.createTreeView(VIEW, { treeDataProvider: provider });
  view = created;
  context.subscriptions.push(
    created,
    changed,
    // Selecting a node selects its source. The panel's whole use is answering *what is this
    // text*, and the answer is only half given if the reader has to find the span by eye.
    created.onDidChangeSelection((event) => void select(event.selection[0])),
    // A saved file whose tree is on display is re-read. Nothing else refreshes it: a tree that
    // silently described the file as it was ten edits ago is worse than one the user re-runs.
    workspace.onDidSaveTextDocument((document) => {
      if (document.uri.fsPath === shown?.file) {
        void render(document.uri.fsPath);
      }
    }),
  );
}

/**
 * What `nvs.showAst` does: the tree for the Novis file in front of the user.
 *
 * A panel with no file to describe is said once, here, the same way `tasks.ts` says it — the
 * alternative is a tree view that silently stays empty and looks broken.
 */
export async function show(): Promise<void> {
  const file = active();
  if (file === undefined) {
    void window.showInformationMessage("The AST panel needs a Novis file: open one first.");
    return;
  }
  await render(file);
  void commands.executeCommand(`${VIEW}.focus`);
}

const provider: TreeDataProvider<Node> = {
  onDidChangeTreeData: changed.event,

  getChildren(element?: Node): Node[] {
    if (element !== undefined) {
      return element.children;
    }
    return shown === undefined ? [] : [shown.root];
  },

  getTreeItem(node: Node): TreeItem {
    const item = new TreeItem(label(node), collapsibility(node));
    item.description = detail(node);
    item.tooltip = tooltip(node);
    return item;
  },
};

/**
 * The provider the view draws from, which is how the panel's contents are read from outside it.
 *
 * A `TreeDataProvider` and not the `TreeView`: the view has `reveal` on it, and `surface.ts` hands
 * back nothing that acts. Asking this for its children is asking the same question the editor asks
 * when it draws the panel.
 */
export function tree(): TreeDataProvider<Node> {
  return provider;
}

/** Run the CLI over `file` and hang the tree it printed in the view, or say why there is none. */
async function render(file: string): Promise<void> {
  const printed = await dump(file);
  if (printed === undefined) {
    return;
  }
  const root = read(printed);
  if (root === undefined) {
    void window.showErrorMessage(`${binary()} ast --json printed no tree this client understands.`);
    return;
  }
  shown = { file, root };
  if (view !== undefined) {
    view.description = basename(file);
  }
  changed.fire();
}

/**
 * `nvs ast --json <file>`, as the document it printed.
 *
 * The exit status is deliberately not consulted: a document on standard output is a tree whether
 * or not diagnostics were reported beside it on standard error, which is the resilience this panel
 * exists for. Nothing is printed only when the binary did not run or could not read the file, and
 * that is the one case the user is told about.
 */
function dump(file: string): Promise<string | undefined> {
  const command = binary();
  return new Promise((resolve) => {
    execFile(
      command,
      ["ast", "--json", file],
      { maxBuffer: OUTPUT_CEILING },
      (failure, stdout, stderr) => {
        if (stdout.trim().length > 0) {
          resolve(stdout);
          return;
        }
        void window.showErrorMessage(`${command} ast: ${reason(stderr, failure)}`);
        resolve(undefined);
      },
    );
  });
}

/** Select the source a node covers, in the file the tree was taken from. */
async function select(node: Node | undefined): Promise<void> {
  if (node === undefined || shown === undefined) {
    return;
  }
  const document = await workspace.openTextDocument(Uri.file(shown.file));
  const range = span(document, node);
  // `preserveFocus` keeps the keyboard in the tree, so arrowing down it walks the source.
  const editor = await window.showTextDocument(document, { preserveFocus: true, preview: false });
  editor.selection = new Selection(range.start, range.end);
  editor.revealRange(range, TextEditorRevealType.InCenterIfOutsideViewport);
}

/** A node's span as a range of the document, through `nodes.ts`'s byte-to-code-unit conversion. */
function span(document: TextDocument, node: Node): Range {
  const text = document.getText();
  return new Range(
    document.positionAt(index(text, node.span[0])),
    document.positionAt(index(text, node.span[1])),
  );
}

/** A leaf has no twisty; the root opens, because a tree that starts closed shows one word. */
function collapsibility(node: Node): TreeItemCollapsibleState {
  if (node.children.length === 0) {
    return TreeItemCollapsibleState.None;
  }
  return node === shown?.root
    ? TreeItemCollapsibleState.Expanded
    : TreeItemCollapsibleState.Collapsed;
}

/** The path of the Novis file the user is looking at, or nothing when it is something else. */
function active(): string | undefined {
  const document = window.activeTextEditor?.document;
  return document?.languageId === "nvs" ? document.uri.fsPath : undefined;
}

/** The last segment of a path, under either separator, for what the view says it is showing. */
function basename(file: string): string {
  return file.split(/[\\/]/).pop() ?? file;
}

/** What went wrong, preferring what the binary said over what the spawn made of it. */
function reason(stderr: string, failure: unknown): string {
  const said = stderr.trim().split("\n")[0];
  if (said.length > 0) {
    return said;
  }
  return failure instanceof Error ? failure.message : String(failure);
}
