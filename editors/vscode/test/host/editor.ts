// What every test in this directory needs before it can assert anything: the extension's identity, the
// fixture copy the launcher made, and the wait that turns the editor's own scheduling into a failure
// with a reason on it.
//
// These four stood in a copy per suite, and a copy is where they drift: a `fixture` that stopped
// asserting the workspace is one file, an `until` that grew a `seen` reading in two files and not the
// third. `scripts/host.mjs` launches one editor for the whole directory, so one definition is also the
// honest description of what the suites share.
//
// Mocha loads `*.test.js` and nothing else (`index.ts`), so this file is imported and never run as a
// suite of its own.

import * as assert from "node:assert/strict";
import * as vscode from "vscode";

/** The extension under test, per `rule:ide/the-extension-runs-where-the-binary-is`. */
export const ID = "novis-lang.nvs";

/** How long anything the editor schedules is given before it counts as not having happened. */
export const DEADLINE = 20_000;

/** One file of the throwaway workspace `scripts/host.mjs` copied, named the way a test names it. */
export function fixture(name: string): vscode.Uri {
  const folders = vscode.workspace.workspaceFolders ?? [];
  assert.equal(folders.length, 1, "the fixture copy is the whole workspace");
  return vscode.Uri.joinPath(folders[0].uri, name);
}

/** Open a fixture and show it, answering the document — which is what most assertions here read. */
export async function open(name: string): Promise<vscode.TextDocument> {
  return (await shown(name)).document;
}

/** The same, answering the editor instead: what a command such as a paste acts on. */
export async function shown(name: string): Promise<vscode.TextEditor> {
  const document = await vscode.workspace.openTextDocument(fixture(name));
  return vscode.window.showTextDocument(document);
}

/**
 * Wait for something the editor schedules, and fail with a reason rather than a hang.
 *
 * An activation, a server start, a task and a paste are all the editor's to schedule, so each is
 * waited for rather than assumed to have finished by the time the call that triggers it returns. The
 * deadline is what turns "it never happened" into a failure naming what was waited for. `seen` is what
 * that failure says was there instead: a wait ending on the bare deadline tells the reader of a ledger
 * nothing about which of the states it did not reach the editor was in, and the driver's sweep is the
 * one place these suites run where nobody can open the editor to look.
 */
export async function until<T>(
  what: string,
  attempt: () => Thenable<T | undefined>,
  seen?: () => string,
): Promise<T> {
  const giveUp = Date.now() + DEADLINE;
  for (;;) {
    const answer = await attempt();
    if (answer !== undefined) {
      return answer;
    }
    if (Date.now() > giveUp) {
      const instead = seen === undefined ? "" : ` -- instead: ${seen()}`;
      throw new Error(`${what} did not happen within ${DEADLINE}ms${instead}`);
    }
    await new Promise((wake) => setTimeout(wake, 100));
  }
}
