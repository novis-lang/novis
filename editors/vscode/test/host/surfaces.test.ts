// The four surfaces this extension puts in an editor, asserted in a real one: the status item, the
// Tasks and the Problems panel behind them, the AST panel, and the concealment of a `secret`.
//
// None of them can be observed anywhere else. A `LanguageStatusItem` and a decoration are sinks —
// `setDecorations` takes ranges and hands none back — which is why `activate` returns the read-only
// reading `src/surface.ts` defines and this suite reads it here rather than inferring it from a
// screenshot. Everything else is the editor's own API: `languages.getDiagnostics` for what reached the
// Problems panel, the tree provider for what the panel is showing.
//
// This file runs after `colour.test.ts` — Mocha loads the directory sorted — and that matters: the
// profile starts with `nvs.lsp.enable` false and the colour tier's third test is what turns it on, so a
// server is answering by the time the first test below reads the status item.

import * as assert from "node:assert/strict";
import * as vscode from "vscode";

import { Drawn, Surface } from "../../src/surface";

/** The extension under test, per `rule:ide/the-extension-runs-where-the-binary-is`. */
const ID = "novis-lang.nvs";

/** How long a server start, a task or a first draw is given before it counts as not having happened. */
const DEADLINE = 20_000;

/** One of `secrets.nvs`'s two literals, which is how a concealed range is told from its neighbour. */
const FIRST = "sk-live-7c9f4d2b8a1e";
const SECOND = "correct horse battery staple";

function fixture(name: string): vscode.Uri {
  const folders = vscode.workspace.workspaceFolders ?? [];
  assert.equal(folders.length, 1, "the fixture copy is the whole workspace");
  return vscode.Uri.joinPath(folders[0].uri, name);
}

async function open(name: string): Promise<vscode.TextDocument> {
  const document = await vscode.workspace.openTextDocument(fixture(name));
  await vscode.window.showTextDocument(document);
  return document;
}

// A start, a task and a draw are all the editor's to schedule, so each is waited for rather than
// assumed to have finished by the time the call that triggers it returns. A deadline is what turns "it
// never happened" into a failure naming what was waited for instead of a suite that hangs. `seen`
// is what the failure says was there instead: a wait that ends on the bare deadline tells the
// reader of a ledger nothing about which of the states it did not reach the editor was in, and the
// driver's sweep is the one place this suite runs where nobody can open the editor to look.
async function until<T>(
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

/** Where each range sits, as `line:from-to` -- the positions and never the bytes under them. */
function placed(ranges: readonly vscode.Range[]): string {
  return ranges.map((range) => `${range.start.line}:${range.start.character}-${range.end.character}`).join(" ");
}

/** The extension itself, for the version it was built with. */
function extension(): vscode.Extension<Surface> {
  const found = vscode.extensions.getExtension<Surface>(ID);
  assert.ok(found, `${ID} is not installed in this editor`);
  return found;
}

/**
 * What the client says about itself right now.
 *
 * `activate` answers the already-activated extension's exports, so this is the reading and not a
 * second activation; asking through it rather than through `exports` is what makes a test that runs
 * first still get one.
 */
async function surface(): Promise<Surface> {
  return extension().activate();
}

/** What was last drawn on the editor showing `document`, if anything has been drawn there yet. */
function drawn(reading: Surface, document: vscode.TextDocument): Drawn | undefined {
  return reading.drawn.find((editor) => editor.document === document.uri.toString());
}

/** The text under each range, which is what an assertion about a concealed range is really about. */
function covered(document: vscode.TextDocument, ranges: readonly vscode.Range[]): string[] {
  return ranges.map((range) => document.getText(range));
}

describe("the surfaces", () => {
  it("shows the server's health and version in the status item", async () => {
    await open("app.nvs");
    const reading = await surface();

    // The item carries one state word and one version, and both are the point: a client that spawned
    // nothing and a client answering are two different sentences in the same place
    // (`rule:ide/the-extension-builds-no-ui-the-editor-already-has`).
    const status = await until("the status item naming a running server", async () =>
      reading.status?.text.includes("lsp") === true ? reading.status : undefined);
    assert.equal(status.severity, vscode.LanguageStatusSeverity.Information,
                 `the status item says ${status.text}`);
    const version = /nvs lsp (\d+)\.(\d+)\.\d+/.exec(status.text);
    assert.ok(version, `the status item says ${status.text}, which names no server version`);

    // The series is the client's own, because a server outside it is refused at `initialize` and
    // never reaches this state (`rule:ide/the-extension-refuses-a-binary-it-does-not-understand`).
    // So this asserts what the number *is*, not merely that there is one.
    const ours = String(extension().packageJSON.version).split(".");
    assert.deepEqual([version[1], version[2]], [ours[0], ours[1]],
                     `${status.text} is outside this client's ${ours[0]}.${ours[1]} series`);
  });

  it("runs nvs run as a task whose failure reaches the Problems panel", async () => {
    const document = await open("broken.nvs");

    // The server is withheld for this one, and that is the whole shape of the test: it publishes a
    // diagnostic for this file too, with the same `source` and the same code, so a Problems entry
    // while it is running says nothing about the Task. With it off, the only thing that can put one
    // there is the `$nvs` problemMatcher reading the task's output
    // (`rule:ide/tasks-carry-a-problem-matcher`).
    //
    // Both waits are load-bearing. The server is given until it has published for this file before it
    // is stopped, and stopping is given until what it published is gone: a stop that is still in
    // flight when the task runs would put its diagnostic beside the matcher's and the count below
    // would be reading two producers again.
    await until("the server publishing for a file that does not compile", async () =>
      vscode.languages.getDiagnostics(document.uri).length > 0 ? true : undefined);
    await vscode.workspace.getConfiguration().update("nvs.lsp.enable", false, vscode.ConfigurationTarget.Global);
    try {
      await until("the server's own diagnostics clearing", async () =>
        vscode.languages.getDiagnostics(document.uri).length === 0 ? true : undefined);

      const ended = new Promise<number | undefined>((resolve) => {
        const listening = vscode.tasks.onDidEndTaskProcess((event) => {
          if (event.execution.task.definition.type === "nvs") {
            listening.dispose();
            resolve(event.exitCode);
          }
        });
      });
      await vscode.commands.executeCommand("nvs.run");
      assert.notEqual(await ended, 0, "nvs run over a file that does not compile exited 0");

      const problems = await until("the problemMatcher publishing a diagnostic", async () => {
        const found = vscode.languages.getDiagnostics(document.uri);
        return found.length > 0 ? found : undefined;
      });
      assert.equal(problems.length, 1, `${problems.length} entries for a run with one error in it`);
      assert.equal(problems[0].code, "E0301");
      assert.equal(problems[0].severity, vscode.DiagnosticSeverity.Error);
      assert.match(problems[0].message, /\$missing/);
      // The matcher's second pattern is what carries the position, so this is the line the `-->`
      // named and not the line the first pattern matched on.
      assert.equal(document.lineAt(problems[0].range.start.line).text.includes("$missing"), true,
                   `the entry points at line ${problems[0].range.start.line + 1}, which is not the one with $missing on it`);
    } finally {
      await vscode.workspace.getConfiguration().update("nvs.lsp.enable", true, vscode.ConfigurationTarget.Global);
    }
  });

  it("renders the AST panel for a file that does not compile", async () => {
    const document = await open("broken.nvs");
    await vscode.commands.executeCommand("nvs.showAst");
    const reading = await surface();

    // `nvs ast --json` is resilient by default and prints the tree it recovered whatever the parser
    // reported, which is the whole reason the panel is worth opening on this file at all
    // (`rule:ide/the-ast-panel-shells-out-to-the-cli`). A stump would satisfy "a tree appeared", so
    // what is asserted is the recovered statements themselves.
    const roots = await until("the AST panel drawing a tree", async () => {
      const shown = await reading.ast.getChildren(undefined);
      return shown !== null && shown !== undefined && shown.length > 0 ? shown : undefined;
    });
    assert.deepEqual(roots.map((node) => node.kind), ["File"]);

    const statements = await reading.ast.getChildren(roots[0]);
    assert.ok(statements, "the file node has no children");
    const kinds = statements.map((node) => node.kind);
    assert.ok(kinds.includes("LocalDecl"), `the declaration is missing from [${kinds.join(", ")}]`);

    // The node the error is about is there too, and it covers the text the diagnostic named. The
    // fixture is ASCII, so a byte span and a UTF-16 index are the same number here.
    const text = document.getText();
    const echo = statements.find((node) => node.kind === "Echo");
    assert.ok(echo, `no Echo statement in [${kinds.join(", ")}]`);
    assert.match(text.slice(echo.span[0], echo.span[1]), /\$missing/);
  });

  it("conceals both secrets on open and reveals exactly one", async () => {
    // The server is answering before the file is opened, on purpose: the test above restarts it,
    // and a file opened while the client is still starting is asked about once the client has
    // synced every open document, which is the easy order. A file opened against a running client
    // is the order a user gets, and the one where the ask can reach the server before the client's
    // own `didOpen` does -- the server answers `[]` for a document it has nothing open for, and
    // that answer is held. Waiting here makes that the order every run takes rather than the one a
    // loaded machine happens to take.
    const reading = await surface();
    await until("the status item naming a running server", async () =>
      reading.status?.text.includes("lsp") === true ? true : undefined);
    const document = await open("secrets.nvs");

    // Two literals, because one proves nothing: a reveal that uncovered the document would pass a
    // single-secret file (`rule:ide/reveal-is-explicit-and-window-local`). The ranges are the
    // server's answer, and this client knows nothing about which bytes are a credential
    // (`rule:ide/redaction-ranges-come-from-the-server`).
    const both = await until("both secret literals concealed", async () => {
      const editor = drawn(reading, document);
      return editor !== undefined && editor.concealed.length === 2 ? editor : undefined;
    }, () => {
      const editor = drawn(reading, document);
      const status = reading.status?.text ?? "no status item";
      return editor === undefined
        ? `nothing drawn on the editor showing secrets.nvs, and the status item says "${status}"`
        : `${editor.concealed.length} range(s) concealed at ${placed(editor.concealed) || "none"}, `
          + `${editor.marked.length} marked, and the status item says "${status}"`;
    });
    const texts = covered(document, both.concealed);
    assert.ok(texts.some((held) => held.includes(FIRST)), `the concealed ranges are ${texts.join(" | ")}`);
    assert.ok(texts.some((held) => held.includes(SECOND)), `the concealed ranges are ${texts.join(" | ")}`);

    const first = both.concealed.find((range) => document.getText(range).includes(FIRST));
    assert.ok(first, "the first literal is not concealed");
    await vscode.commands.executeCommand("nvs.revealSecret", {
      uri: document.uri.toString(),
      line: first.start.line,
      character: first.start.character,
    });

    const after = drawn(await surface(), document);
    assert.ok(after, "nothing is drawn on the editor that was drawn on a moment ago");
    const left = covered(document, after.concealed);
    assert.equal(left.length, 1, `${left.length} ranges left concealed: ${left.join(" | ")}`);
    assert.ok(left[0].includes(SECOND), `the range left concealed is ${left[0]}`);
  });

  it("decorates nothing for tainted at the default setting", async () => {
    const document = await open("secrets.nvs");
    assert.equal(vscode.workspace.getConfiguration("nvs").get<string>("taint.mark"), "off",
                 "the profile asks for a taint marker, so this is not the default state");

    // The fixture carries a `tainted` declaration, so the emptiness below is a decision rather than
    // an accident of there being nothing to mark: teaching does not get to write into the user's
    // editor by default (`rule:ide/tainted-has-no-default-decoration`).
    assert.match(document.getText(), /tainted string \$submitted/);
    const editor = drawn(await surface(), document);
    assert.ok(editor, "nothing is drawn on the editor showing the fixture");
    assert.deepEqual(covered(document, editor.marked), [],
                     "a taint decoration was handed a range at the default setting");
  });
});
