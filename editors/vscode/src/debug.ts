// The debugger: VS Code's own debug UI over `nvs dap`, which this file starts and nothing more
// (`rule:ide/the-extension-builds-no-ui-the-editor-already-has`).
//
// `package.json`'s `debuggers` contribution is the `launch.json` schema, and every field it names
// except `nvsCommand` goes to the adapter as it is written: `nvs dap` reads the launch request
// itself (`rule:ide/the-debug-adapter-does-not-wait-for-an-editor`). This file reads `nvsCommand`
// alone, because it says which binary is started, which is the one thing the adapter cannot.
//
// The adapter is started through `binary.ts` like every other spawn, so a debug session runs the
// same build as the server, and never the named file itself
// (`rule:ide/the-extension-runs-a-copy-of-the-binary`).

import {
  DebugAdapterDescriptor,
  DebugAdapterDescriptorFactory,
  DebugAdapterExecutable,
  DebugSession,
  ExtensionContext,
  debug,
} from "vscode";

import { runnable, runnableNamed } from "./binary";

/** The `type` a `launch.json` configuration names, and the manifest's `debuggers` entry. */
export const TYPE = "nvs";

/** The subcommand that is the adapter. It takes no arguments: the launch request carries them. */
const SUBCOMMAND = ["dap"];

/** Register the factory that starts `nvs dap` for every `"type": "nvs"` session. */
export function install(context: ExtensionContext): void {
  context.subscriptions.push(debug.registerDebugAdapterDescriptorFactory(TYPE, factory));
}

const factory: DebugAdapterDescriptorFactory = {
  async createDebugAdapterDescriptor(session: DebugSession): Promise<DebugAdapterDescriptor> {
    const named: unknown = session.configuration.nvsCommand;
    const command = typeof named === "string" && named.trim() !== ""
      ? await runnableNamed(named.trim())
      : (await runnable()).command;
    // In the session's folder, so a relative `cwd`, and a relative `program` with no `cwd`, is
    // read from the workspace folder `launch.json` belongs to.
    return new DebugAdapterExecutable(command, SUBCOMMAND, { cwd: session.workspaceFolder?.uri.fsPath });
  },
};
