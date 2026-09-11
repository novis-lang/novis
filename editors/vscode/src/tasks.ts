// The two Tasks — `nvs run` and `nvs test` — and the one execution path behind both entry points.
//
// A Task rather than a spawn of this extension's own, because the editor already owns a terminal, a
// re-run, a "Tasks: Run Task" list and the Problems panel. `rule:ide/tasks-carry-a-problem-matcher`
// is what makes the last of those work: a diagnostic becomes a clickable Problems entry through the
// `$nvs` matcher `package.json` contributes, which is a regex over the renderer's own format
// (`rule:errors/renderings`). **Nothing in this file reads a diagnostic.** The matcher is named here
// and the editor does the matching, which is the same division as
// `rule:ide/the-extension-builds-no-ui-the-editor-already-has` keeps everywhere else.
//
// `nvs.run` and `nvs.test` start these same Tasks rather than spawning a second way, so a user who
// runs one from the palette and one from the task list gets the same process, the same terminal and
// the same Problems entries.

import {
  ExtensionContext,
  ProcessExecution,
  Task,
  TaskDefinition,
  TaskProvider,
  TaskScope,
  Uri,
  WorkspaceFolder,
  tasks,
  window,
  workspace,
} from "vscode";

import { binary } from "./binary";

/** The `type` a `tasks.json` entry names, and the manifest's `taskDefinitions` entry. */
export const TYPE = "nvs";

/** The matcher contributed beside that type. The editor resolves `$nvs` to it. */
export const MATCHER = "$nvs";

/** A task as `tasks.json` holds it: the manifest's `taskDefinitions` entry, in TypeScript. */
export interface NvsTaskDefinition extends TaskDefinition {
  command: "run" | "test";
  file: string;
  args?: string[];
}

/** Register the provider, which is what makes a `"type": "nvs"` entry in `tasks.json` resolve. */
export function install(context: ExtensionContext): void {
  context.subscriptions.push(tasks.registerTaskProvider(TYPE, provider));
}

/**
 * Start `nvs <command>` over the file in front of the user, which is what `nvs.run` and `nvs.test`
 * do from the palette.
 *
 * A subcommand with no file is not a task that can run — both of them take a path — so the answer
 * to *no Novis file is open* is said once, here, rather than left to a process that would exit with
 * a usage error in a terminal the user has to go and read.
 */
export async function execute(command: NvsTaskDefinition["command"]): Promise<void> {
  const file = active();
  if (file === undefined) {
    void window.showInformationMessage(`nvs ${command} needs a Novis file: open one first.`);
    return;
  }
  await tasks.executeTask(taskFor({ type: TYPE, command, file }));
}

const provider: TaskProvider = {
  // What "Tasks: Run Task" offers before anybody has written a `tasks.json`: the two subcommands
  // over the file in front of the user.
  provideTasks(): Task[] {
    const file = active();
    if (file === undefined) {
      return [];
    }
    return [
      taskFor({ type: TYPE, command: "run", file }),
      taskFor({ type: TYPE, command: "test", file }),
    ];
  },

  // A `tasks.json` entry arrives here with its definition and no execution. The definition object
  // is handed back as it came, because the editor matches the returned task to the entry by it.
  resolveTask(task: Task): Task | undefined {
    const definition = task.definition as NvsTaskDefinition;
    if (typeof definition.command !== "string" || typeof definition.file !== "string") {
      return undefined;
    }
    return taskFor(definition, task.scope);
  },
};

/**
 * One task, whichever entry point asked for it.
 *
 * `NO_COLOR` is what the matcher rests on: the renderer colours its output when the stream is a
 * terminal and `NO_COLOR` is unset (`crates/nvs-cli/src/main.rs:2184`), and a Task's terminal is
 * one. The `$nvs` patterns are written against the uncoloured rendering, and this is what
 * guarantees they get one.
 *
 * A `ProcessExecution` and not a shell: the file is a path the user did not type, and a shell is
 * one more set of quoting rules for it to be wrong under.
 */
function taskFor(
  definition: NvsTaskDefinition,
  scope?: WorkspaceFolder | TaskScope,
): Task {
  const execution = new ProcessExecution(
    binary(),
    [definition.command, definition.file, ...(definition.args ?? [])],
    { env: { NO_COLOR: "1" } },
  );
  return new Task(
    definition,
    scope ?? folderOf(definition.file),
    `${definition.command} ${basename(definition.file)}`,
    TYPE,
    execution,
    [MATCHER],
  );
}

/** The path of the Novis file the user is looking at, or nothing when they are looking at something else. */
function active(): string | undefined {
  const document = window.activeTextEditor?.document;
  return document?.languageId === "nvs" ? document.uri.fsPath : undefined;
}

/** The folder a task run over `file` belongs to, falling back to the workspace as a whole. */
function folderOf(file: string): WorkspaceFolder | TaskScope {
  return workspace.getWorkspaceFolder(Uri.file(file)) ?? TaskScope.Workspace;
}

/** The last segment of a path, under either separator, for the name the task list shows. */
function basename(file: string): string {
  return file.split(/[\\/]/).pop() ?? file;
}
