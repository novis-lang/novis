// The refactorings that need text from the person who runs them: change parameters, move a member
// and extract an interface.
//
// The server declares each command id, and `vscode-languageclient` registers it. A code action
// carries the id with two strings: what to change, and the text the input box starts with. This
// middleware shows VS Code's own input box (`rule:ide/the-extension-builds-no-ui-the-editor-already-has`)
// and sends the command with what was typed. The server computes the edit and sends it back with
// `workspace/applyEdit`, so nothing here edits a document. A refusal is the server's error, and the
// client shows it.

import { window } from "vscode";

// What each input box asks, by command id. An id not listed here goes to the server unchanged.
const PROMPTS: Record<string, string> = {
  "nvs.changeParameters": "The new parameter list",
  "nvs.moveMember": "The class to move the member to",
  "nvs.extractInterface": "The name of the new interface",
};

type Next = (command: string, args: unknown[]) => unknown;

// The client's `executeCommand` middleware. A command with a prompt shows the input box, prefilled
// with its second argument, and sends nothing when the box is closed.
export async function askFirst(command: string, args: unknown[], next: Next): Promise<unknown> {
  const prompt = PROMPTS[command];
  const [target, current] = args;
  if (prompt === undefined || typeof target !== "string" || typeof current !== "string") {
    return next(command, args);
  }
  const typed = await window.showInputBox({ title: target, prompt, value: current });
  if (typed === undefined) {
    return undefined;
  }
  return next(command, [target, typed]);
}
