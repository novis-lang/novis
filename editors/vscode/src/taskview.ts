// The Novis Tasks view: the tasks of a stopped `nvs dap` session, each under the task that started
// it, drawn in the editor's own tree in the Run and Debug sidebar.
//
// The Call Stack view lists the same tasks flat, as threads, and cannot say which task started
// which; that relation is what this view adds, and nothing else
// (`rule:ide/the-extension-builds-no-ui-the-editor-already-has`). No webview: a `TreeDataProvider`
// feeds a view VS Code draws.
//
// A debug adapter tracker reads every message the adapter sends, and `src/tasktree.ts`, which
// imports no `vscode`, decides what each does: a stop asks `nvsTaskTree` again, and a continue,
// the program's end and the session's end empty the view. The request is sent only to an adapter
// whose `initialize` response said it answers it. Selecting a task opens its top frame's line.

import {
  DebugAdapterTracker,
  DebugSession,
  EventEmitter,
  ExtensionContext,
  Range,
  TreeDataProvider,
  TreeItem,
  TreeItemCollapsibleState,
  Uri,
  debug,
  window,
} from "vscode";

import { REQUEST, TaskNode, TaskTree } from "./tasktree";

/** The view `package.json` contributes in the `debug` container. Frozen: never renamed. */
export const VIEW = "nvs.tasks";

/** One model per running session, keyed by the session's id, dropped when the session ends. */
const sessions = new Map<string, { session: DebugSession; tree: TaskTree }>();

const changed = new EventEmitter<void>();

/** Create the view and the tracker that keeps it in step with every `type` session's adapter. */
export function install(context: ExtensionContext, type: string): void {
  context.subscriptions.push(
    changed,
    window.createTreeView(VIEW, { treeDataProvider: provider }),
    debug.registerDebugAdapterTrackerFactory(type, { createDebugAdapterTracker: tracker }),
    // The view shows the session the Call Stack view has selected, so choosing another one there
    // redraws it.
    debug.onDidChangeActiveDebugSession(() => changed.fire()),
    debug.onDidTerminateDebugSession((session) => end(session)),
  );
}

function tracker(session: DebugSession): DebugAdapterTracker {
  const tree = new TaskTree();
  sessions.set(session.id, { session, tree });
  return {
    onDidSendMessage: (message: unknown) => {
      const action = tree.receive(message);
      if (action === "ask") {
        void ask(session, tree);
      } else if (action === "redraw") {
        changed.fire();
      }
    },
    onExit: () => end(session),
  };
}

async function ask(session: DebugSession, tree: TaskTree): Promise<void> {
  if (!tree.capable) {
    return;
  }
  const ticket = tree.ticket();
  let body: unknown;
  try {
    body = await session.customRequest(REQUEST);
  } catch {
    // The session ended or the program went on before the adapter answered. The next stop asks
    // again, and until then the view shows no task.
    return;
  }
  if (tree.answer(ticket, body)) {
    changed.fire();
  }
}

function end(session: DebugSession): void {
  const entry = sessions.get(session.id);
  if (entry !== undefined) {
    entry.tree.end();
    sessions.delete(session.id);
    changed.fire();
  }
}

/** The model of the session the view shows: the active one, or the only one there is. */
function current(): TaskTree | undefined {
  const active = debug.activeDebugSession;
  if (active !== undefined && sessions.has(active.id)) {
    return sessions.get(active.id)?.tree;
  }
  return sessions.size === 1 ? [...sessions.values()][0].tree : undefined;
}

const provider: TreeDataProvider<TaskNode> = {
  onDidChangeTreeData: changed.event,

  getChildren(node?: TaskNode): TaskNode[] {
    return node === undefined ? [...(current()?.shown ?? [])] : node.children;
  },

  getTreeItem(node: TaskNode): TreeItem {
    const { task } = node;
    const item = new TreeItem(
      task.name,
      node.children.length > 0 ? TreeItemCollapsibleState.Expanded : TreeItemCollapsibleState.None,
    );
    item.id = String(task.threadId);
    if (!task.stopped) {
      item.description = "waiting";
      item.tooltip = `${task.name} waits for the tasks it started.`;
    }
    if (task.source !== undefined) {
      const line = (task.line ?? 1) - 1;
      const uri = Uri.file(task.source);
      item.tooltip = task.line === undefined ? task.source : `${task.source}:${task.line}`;
      item.command = {
        command: "vscode.open",
        title: "Open the task's line",
        arguments: [uri, { selection: new Range(line, 0, line, 0) }],
      };
    }
    return item;
  },
};
