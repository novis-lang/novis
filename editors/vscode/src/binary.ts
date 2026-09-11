// Which `nvs` answers, for every process this extension starts.
//
// It has one home because there are two spawns — the server in `extension.ts` and the Tasks in
// `tasks.ts` — and a user who points `nvs.path` at a build directory means both of them. A second
// read with its own fallback is how one of them ends up running a different binary than the other.

import { workspace } from "vscode";

/**
 * The command that is `nvs`.
 *
 * An empty `nvs.path` is a lookup on `PATH`: the command is the bare name, and the platform's own
 * resolution finds it or does not.
 */
export function binary(): string {
  return workspace.getConfiguration("nvs").get<string>("path", "").trim() || "nvs";
}
