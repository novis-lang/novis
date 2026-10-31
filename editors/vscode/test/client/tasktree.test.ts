// The Novis Tasks view, held without an editor.
//
// `src/tasktree.ts` imports no `vscode`, so it runs in the headless tier
// (`rule:ide/headless-gates-the-loop-the-host-run-gates-the-milestone`). What is pinned is how an
// `nvsTaskTree` answer nests, that the view asks again at each stop and only an adapter that said it
// answers, and that it shows no task while the session runs, ended, or was never stopped.

import * as assert from "node:assert/strict";

import { CAPABILITY, REQUEST, TaskNode, TaskTree, nest } from "../../src/tasktree";

/** Each node as `name`, or `name [children]`, so a whole tree compares as one value. */
function shape(nodes: readonly TaskNode[]): unknown[] {
  return nodes.map((node) =>
    node.children.length === 0 ? node.task.name : [node.task.name, shape(node.children)]);
}

const INITIALIZED = {
  type: "response",
  command: "initialize",
  success: true,
  body: { supportsConfigurationDoneRequest: true, [CAPABILITY]: true },
};
const STOPPED = { type: "event", event: "stopped", body: { reason: "breakpoint", threadId: 2 } };
const CONTINUED = { type: "event", event: "continued", body: { threadId: 2, allThreadsContinued: true } };
const IDLE = { tasks: [{ threadId: 1, name: "main", stopped: true }] };
const STOP = {
  tasks: [
    { threadId: 2, parentThreadId: 9, name: "fetch a", stopped: true, source: "/app/a.nvs", line: 4 },
    { threadId: 3, parentThreadId: 9, name: "fetch b", stopped: true, source: "/app/b.nvs", line: 7 },
    { threadId: 9, name: "main", stopped: false },
  ],
};

describe("the Novis Tasks view", () => {
  it("asks the adapter under its own request name and capability", () => {
    assert.equal(REQUEST, "nvsTaskTree");
    assert.equal(CAPABILITY, "supportsNvsTaskTree");
  });

  it("nests each task under the task that started it", () => {
    const roots = nest({
      tasks: [
        { threadId: 4, parentThreadId: 2, name: "grandchild", stopped: true },
        { threadId: 2, parentThreadId: 10, name: "first", stopped: true },
        { threadId: 3, parentThreadId: 10, name: "second", stopped: true },
        { threadId: 7, parentThreadId: 99, name: "orphan", stopped: true },
        { threadId: 10, name: "main", stopped: false },
      ],
    });
    // An entry whose parent is missing is a root, and every list keeps the adapter's order.
    assert.deepEqual(shape(roots), ["orphan", ["main", [["first", ["grandchild"]], "second"]]]);
    const main = roots[1].task;
    assert.equal(main.stopped, false);
    assert.equal(main.source, undefined);
  });

  it("shows every task once when the parent chain is a cycle", () => {
    const roots = nest({
      tasks: [
        { threadId: 1, parentThreadId: 2, name: "a", stopped: true },
        { threadId: 2, parentThreadId: 1, name: "b", stopped: true },
      ],
    });
    assert.deepEqual(shape(roots), ["a", "b"]);
  });

  it("skips an entry it cannot read, and reads a body with no tasks as none", () => {
    assert.deepEqual(nest(undefined), []);
    assert.deepEqual(nest({ tasks: "no" }), []);
    const roots = nest({ tasks: [null, { name: "no id" }, { threadId: 5, name: "kept", line: 0 }] });
    assert.deepEqual(shape(roots), ["kept"]);
    assert.equal(roots[0].task.line, undefined);
    assert.equal(roots[0].task.stopped, false);
  });

  it("asks again at each stop, and draws the answer to the current one", () => {
    const tree = new TaskTree();
    assert.equal(tree.receive(INITIALIZED), "none");
    assert.equal(tree.capable, true);
    assert.equal(tree.receive(STOPPED), "ask");
    assert.equal(tree.answer(tree.ticket(), STOP), true);
    assert.deepEqual(shape(tree.shown), [["main", ["fetch a", "fetch b"]]]);
    assert.equal(tree.shown[0].children[0].task.source, "/app/a.nvs");
    assert.equal(tree.shown[0].children[0].task.line, 4);
    assert.equal(tree.receive(STOPPED), "ask");
  });

  it("shows no task while no nvs session is stopped", () => {
    const tree = new TaskTree();
    // Before any stop, even the adapter's idle `main` answer is not drawn.
    tree.receive(INITIALIZED);
    assert.deepEqual(tree.shown, []);
    assert.equal(tree.answer(tree.ticket(), IDLE), false);
    assert.deepEqual(tree.shown, []);

    // A continue empties the view, and an answer that arrives after it is dropped.
    tree.receive(STOPPED);
    const late = tree.ticket();
    assert.equal(tree.receive(CONTINUED), "none");
    assert.equal(tree.answer(late, STOP), false);
    assert.deepEqual(tree.shown, []);

    for (const message of [CONTINUED, { type: "event", event: "terminated" }, { type: "event", event: "exited", body: { exitCode: 0 } }]) {
      tree.receive(STOPPED);
      tree.answer(tree.ticket(), STOP);
      assert.equal(tree.shown.length, 1);
      assert.equal(tree.receive(message), "redraw", String(message.event));
      assert.deepEqual(tree.shown, []);
    }

    // The session's end empties it too.
    tree.receive(STOPPED);
    tree.answer(tree.ticket(), STOP);
    tree.end();
    assert.deepEqual(tree.shown, []);
  });

  it("never asks an adapter that did not say it answers the request", () => {
    const tree = new TaskTree();
    tree.receive({ type: "response", command: "initialize", success: true, body: {} });
    assert.equal(tree.capable, false);
    assert.equal(tree.receive(STOPPED), "none");
    assert.deepEqual(tree.shown, []);
  });

  it("ignores every other message", () => {
    const tree = new TaskTree();
    tree.receive(INITIALIZED);
    for (const message of [
      undefined,
      { type: "event", event: "output", body: { output: "x" } },
      { type: "response", command: "threads", success: true, body: { threads: [] } },
    ]) {
      assert.equal(tree.receive(message), "none");
    }
  });
});
