// The debug viewer button, in a real editor: `nvs.openDebugViewer` asks the language server for the
// link, finds a running `nvs debug`, and opens the panel that frames the viewer
// (`rule:ide/the-debug-viewer-opens-from-one-button`).
//
// What the headless suite cannot see is the whole path at once: the server reading the workspace's
// `[debug] listen`, the data folder the copy is told with `--data`, the lock a running service holds,
// and the editor opening a webview of the panel's type. A copy that reads its own data folder finds
// no service, and the command then offers to start one instead of opening the panel, which is the
// gap this test closes.
//
// The service is started by this file in the workspace copy, with the data folder of the binary
// `scripts/host.mjs` named, and stopped in `after`. The suite waits on the line the service prints
// when it is ready, never on a time.
//
// This file runs after `colour.test.ts`, which turns the server on.

import * as assert from "node:assert/strict";
import { ChildProcess, spawn } from "node:child_process";
import { rmSync, writeFileSync } from "node:fs";
import { createServer } from "node:net";
import { join } from "node:path";
import * as vscode from "vscode";

import { dataBeside } from "../../src/shadow";
import { until } from "./editor";

/** The webview type of the panel, as `src/debugviewer.ts` names it. */
const PANEL = "nvs.debugViewer";

/** The line `nvs debug` prints on standard error once it listens (`crates/nvs-cli/src/debug.rs`). */
const READY = "the debug viewer is at ";

/** A loopback port nothing listens on at this moment. */
function freePort(): Promise<number> {
  return new Promise((resolve, reject) => {
    const probe = createServer();
    probe.once("error", reject);
    probe.listen(0, "127.0.0.1", () => {
      const address = probe.address();
      const port = typeof address === "object" && address !== null ? address.port : 0;
      probe.close(() => resolve(port));
    });
  });
}

/** Start `nvs --data <data> debug` in `cwd`, and wait for the line that says it listens. */
function start(nvs: string, data: string, cwd: string): Promise<ChildProcess> {
  const child = spawn(nvs, ["--data", data, "debug"], { cwd, env: { ...process.env, NO_COLOR: "1" } });
  return new Promise((resolve, reject) => {
    let printed = "";
    child.stderr?.setEncoding("utf8");
    child.stderr?.on("data", (chunk: string) => {
      printed += chunk;
      if (printed.includes(READY)) {
        resolve(child);
      }
    });
    child.once("error", reject);
    child.once("exit", (code) => reject(new Error(`nvs debug exited with ${code} before it listened: ${printed}`)));
  });
}

/** The tab of an open debug viewer panel, or nothing when none is open. */
function panelTab(): vscode.Tab | undefined {
  for (const group of vscode.window.tabGroups.all) {
    for (const tab of group.tabs) {
      // The editor puts its own prefix in front of an extension's view type.
      if (tab.input instanceof vscode.TabInputWebview && tab.input.viewType.endsWith(PANEL)) {
        return tab;
      }
    }
  }
  return undefined;
}

/** Every tab's label, for a failure that has to say what was open instead. */
function tabs(): string {
  return vscode.window.tabGroups.all.flatMap((group) => group.tabs.map((tab) => tab.label)).join(", ") || "no tab";
}

describe("the debug viewer button", () => {
  let service: ChildProcess | undefined;
  let config: string | undefined;

  before(async function () {
    const nvs = process.env.NVS_HOST_NVS ?? "";
    if (nvs === "") {
      // `scripts/host.mjs` found no built binary, and a service cannot be started without one.
      this.skip();
    }
    const folder = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
    assert.ok(folder !== undefined, "the fixture copy is the whole workspace");
    config = join(folder, "nvs.toml");
    writeFileSync(config, `[debug]\nlisten = "127.0.0.1:${await freePort()}"\n`);
    // The data folder the extension's copy is told, so the service and the server share a lock.
    service = await start(nvs, await dataBeside(nvs), folder);
  });

  after(async () => {
    service?.kill();
    if (config !== undefined) {
      rmSync(config, { force: true });
    }
    await vscode.commands.executeCommand("workbench.action.closeAllEditors");
  });

  it("opens one panel that frames the viewer, and a second click shows the same one", async () => {
    await vscode.workspace.getConfiguration().update("nvs.lsp.enable", true, vscode.ConfigurationTarget.Global);
    // The command needs the server, which may still be starting, so it is run again until a panel opens.
    const tab = await until("a debug viewer panel", async () => {
      await vscode.commands.executeCommand("nvs.openDebugViewer");
      return panelTab();
    }, tabs);
    assert.ok(tab.input instanceof vscode.TabInputWebview);

    await vscode.commands.executeCommand("nvs.openDebugViewer");
    const open = vscode.window.tabGroups.all.flatMap((group) => group.tabs).filter((t) =>
      t.input instanceof vscode.TabInputWebview && t.input.viewType.endsWith(PANEL)
    );
    assert.equal(open.length, 1, `a second click opened another panel: ${tabs()}`);
  });
});
