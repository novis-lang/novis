// The debugger: VS Code's own debug UI over `nvs dap`, which this file starts and nothing more
// (`rule:ide/the-extension-builds-no-ui-the-editor-already-has`).
//
// `package.json`'s `debuggers` contribution is the `launch.json` schema, and every field it names
// except `nvsCommand` goes to the adapter as it is written: `nvs dap` reads the launch request
// itself (`rule:ide/the-debug-adapter-does-not-wait-for-an-editor`). This file reads `nvsCommand`
// alone, because it says which command starts the adapter, which is the one thing the adapter
// cannot. That command may be a wrapper, such as `docker exec -i`, so the adapter can run inside a
// container and still be reached over stdio.
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
    const [command, ...args] = await commandLine(session.configuration.nvsCommand);
    // In the session's folder, so a relative `cwd`, and a relative `program` with no `cwd`, is
    // read from the workspace folder `launch.json` belongs to.
    return new DebugAdapterExecutable(command, [...args, ...SUBCOMMAND], {
      cwd: session.workspaceFolder?.uri.fsPath,
    });
  },
};

/**
 * The command line that starts `nvs dap`, without the subcommand.
 *
 * A string `nvsCommand` names an `nvs` binary, which is copied like a candidate. An array is a
 * whole command line, such as `["docker", "exec", "-i", "app", "nvs"]`, and runs as it is written:
 * its first word is a wrapper and not an `nvs` build, so there is nothing of ours to copy. With
 * neither, the binary is the one every other spawn runs.
 */
async function commandLine(named: unknown): Promise<string[]> {
  if (typeof named === "string" && named.trim() !== "") {
    return [await runnableNamed(named.trim())];
  }
  if (Array.isArray(named) && named.length > 0 && named.every((word) => typeof word === "string")) {
    return named as string[];
  }
  return [(await runnable()).command];
}
