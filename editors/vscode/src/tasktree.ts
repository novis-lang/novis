// The Novis Tasks view, held without an editor: what `nvs dap`'s `nvsTaskTree` answer reads as, and
// what each message the adapter sends does to the view.
//
// This file imports no `vscode`, so the headless tier runs it
// (`rule:ide/headless-gates-the-loop-the-host-run-gates-the-milestone`). `src/taskview.ts` is the
// tree view over it, and does nothing the model has not decided.
//
// The view shows tasks only while a session is stopped. The adapter answers `nvsTaskTree` at any
// time, and while nothing is stopped it answers one idle `main` entry; that answer describes no
// stop, so the view is never asked for it and drops it if it arrives after the program went on. A
// stop asks again, because which tasks exist and where each stands changes between two stops. The
// view holds one answer per session, which is the memory this feature spends.

/** The custom request `nvs dap` answers with the task tree. It takes no arguments. */
export const REQUEST = "nvsTaskTree";

/** The capability the adapter's `initialize` response sets when it answers `REQUEST`. */
export const CAPABILITY = "supportsNvsTaskTree";

/** One entry of the answer: a task, or a parent that waits on its children. */
export interface Task {
  /** The task's DAP thread id when it is stopped; an id after every thread's when it waits. */
  readonly threadId: number;
  /** The `threadId` of the task that started this one. Absent for a root. */
  readonly parentThreadId?: number;
  readonly name: string;
  /** False for a parent that waits on its children, which has no frame and so no source. */
  readonly stopped: boolean;
  /** The editor path of the top frame's file, already mapped by the adapter. */
  readonly source?: string;
  /** The top frame's line, 1-based. */
  readonly line?: number;
}

/** A task, and the tasks it started, in the order the adapter listed them. */
export interface TaskNode {
  readonly task: Task;
  readonly children: TaskNode[];
}

/**
 * The entries of an `nvsTaskTree` response body. An entry without a numeric `threadId` and a string
 * `name` is skipped, and so is an optional field of the wrong type, so a newer adapter's extra or
 * changed fields never break the view.
 */
export function tasks(body: unknown): Task[] {
  const list = (body as { tasks?: unknown } | undefined)?.tasks;
  if (!Array.isArray(list)) {
    return [];
  }
  const read: Task[] = [];
  for (const entry of list as Record<string, unknown>[]) {
    if (typeof entry !== "object" || entry === null) {
      continue;
    }
    const { threadId, parentThreadId, name, stopped, source, line } = entry;
    if (typeof threadId !== "number" || typeof name !== "string") {
      continue;
    }
    read.push({
      threadId,
      name,
      stopped: stopped === true,
      ...(typeof parentThreadId === "number" ? { parentThreadId } : {}),
      ...(typeof source === "string" && source !== "" ? { source } : {}),
      ...(typeof line === "number" && line >= 1 ? { line } : {}),
    });
  }
  return read;
}

/**
 * The tasks of `body`, each under the task that started it.
 *
 * An entry whose parent is not in the list is a root, and so is one whose parent chain comes back
 * to itself, so a malformed answer still shows every task once. Siblings, and the roots, keep the
 * order the adapter listed them in.
 */
export function nest(body: unknown): TaskNode[] {
  const read = tasks(body);
  const nodes = new Map<number, TaskNode>();
  for (const task of read) {
    if (!nodes.has(task.threadId)) {
      nodes.set(task.threadId, { task, children: [] });
    }
  }
  const roots: TaskNode[] = [];
  for (const node of nodes.values()) {
    const parent = parentOf(node.task, nodes);
    (parent === undefined ? roots : parent.children).push(node);
  }
  return roots;
}

/** The node `task` hangs under, or nothing when it is a root or its parent chain is a cycle. */
function parentOf(task: Task, nodes: Map<number, TaskNode>): TaskNode | undefined {
  const parent = task.parentThreadId === undefined ? undefined : nodes.get(task.parentThreadId);
  if (parent === undefined) {
    return undefined;
  }
  const seen = new Set<number>([task.threadId]);
  for (let at: Task | undefined = parent.task; at !== undefined;) {
    if (seen.has(at.threadId)) {
      return undefined;
    }
    seen.add(at.threadId);
    at = at.parentThreadId === undefined ? undefined : nodes.get(at.parentThreadId)?.task;
  }
  return parent;
}

/**
 * What a message from the adapter does to the view: send `REQUEST` and draw its answer, draw the
 * view again because it now shows no task, or nothing.
 */
export type Action = "ask" | "redraw" | "none";

/** A debug adapter protocol message, as far as the view reads one. */
interface Message {
  type?: unknown;
  event?: unknown;
  command?: unknown;
  success?: unknown;
  body?: unknown;
}

/**
 * The view's state for one debug session: whether its adapter answers `REQUEST`, whether it is
 * stopped, and the task tree of the current stop.
 */
export class TaskTree {
  private answers = false;
  private stopped = false;
  private roots: TaskNode[] = [];
  // Counts stops, continues and the end. An answer is drawn only if no such message arrived
  // between its request and its response, so an answer about a stop that is over is dropped.
  private generation = 0;

  /** Whether the adapter said, in its `initialize` response, that it answers `REQUEST`. */
  get capable(): boolean {
    return this.answers;
  }

  /** The tasks the view shows: the current stop's tree, or none while the session runs. */
  get shown(): readonly TaskNode[] {
    return this.roots;
  }

  /** The ticket a request sent now carries, which `answer` checks the response against. */
  ticket(): number {
    return this.generation;
  }

  /** Read one message the adapter sent, and say what the view does about it. */
  receive(message: unknown): Action {
    const { type, event, command, success, body } = (message ?? {}) as Message;
    if (type === "response" && command === "initialize") {
      this.answers = success === true
        && (body as Record<string, unknown> | undefined)?.[CAPABILITY] === true;
      return "none";
    }
    if (type !== "event") {
      return "none";
    }
    switch (event) {
      case "stopped":
        this.generation += 1;
        this.stopped = true;
        return this.answers ? "ask" : "none";
      case "continued":
      case "terminated":
      case "exited":
        return this.clear();
      default:
        return "none";
    }
  }

  /**
   * Take the response to a request sent with `ticket`. Returns whether the view changed: an answer
   * is drawn only while the stop it was asked about is still the current one.
   */
  answer(ticket: number, body: unknown): boolean {
    if (!this.stopped || ticket !== this.generation) {
      return false;
    }
    this.roots = nest(body);
    return true;
  }

  /** The session ended. The view shows no task. */
  end(): void {
    this.clear();
  }

  private clear(): Action {
    this.generation += 1;
    this.stopped = false;
    if (this.roots.length === 0) {
      return "none";
    }
    this.roots = [];
    return "redraw";
  }
}
