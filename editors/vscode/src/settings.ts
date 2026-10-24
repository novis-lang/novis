// What a changed `nvs.*` setting does to the running server, decided without an editor.
//
// Two settings decide which server runs: `nvs.path` names the binary, and `nvs.lsp.enable` says
// whether one runs at all. A change to either restarts the client. Every other `nvs.*` setting goes
// to the running server in a `workspace/didChangeConfiguration`, and the server applies it without a
// restart (`crates/nvs-lsp/src/settings.rs`), so its index stays warm. Nothing here imports
// `vscode`, which is what lets the headless tier hold it.

import { withStubs } from "./stubs";

/** The section every setting of this extension is under. */
export const SECTION = "nvs";

/** The settings a running client is built from. Neither reaches a server that is already spawned. */
export const RESPAWNING_SETTINGS = ["nvs.path", "nvs.lsp.enable"];

/** What one configuration change does: restart the client, send the section, or nothing. */
export type Reaction = "restart" | "send" | "none";

/**
 * The reaction to a change, from the editor's own test of which settings it affects. A change that
 * affects a respawning setting restarts, even when it affects other settings too: the new server
 * reads all of them in `initialize`.
 */
export function reaction(affects: (setting: string) => boolean): Reaction {
  if (RESPAWNING_SETTINGS.some((setting) => affects(setting))) {
    return "restart";
  }
  return affects(SECTION) ? "send" : "none";
}

/**
 * The `didChangeConfiguration` params for `section` as the editor holds it. The section goes over
 * whole and nested, with `stubs.dir` filled in, which is the shape `initialize` sends: the server
 * reads a setting the change leaves out as the user having removed it.
 */
export function changeParams(section: unknown, stubs: string): { settings: Record<string, unknown> } {
  return { settings: withStubs(section, stubs) };
}
