// The claim every other host test rests on: the editor running this suite is the throwaway one
// `scripts/host.mjs` set up, and not the one the developer has open.
//
// It is asserted from inside the editor rather than from the launcher's argv, because what matters is
// the state that resulted — a flag that was passed and then ignored is exactly the failure this tier
// would otherwise report as a green. The isolation itself, and why no part of it is negotiable, is
// `rule:ide/headless-gates-the-loop-the-host-run-gates-the-milestone`.

import * as assert from "node:assert/strict";
import { isAbsolute, relative, resolve } from "node:path";
import * as vscode from "vscode";

/** The extension under test, per `rule:ide/the-extension-runs-where-the-binary-is`. */
const ID = "novis-lang.nvs";

/** The value `scripts/host.mjs` passed in, or a failure naming the launcher this suite needs. */
function expected(name: string): string {
  const value = process.env[name];
  if (value === undefined) {
    throw new Error(`${name} is not set: scripts/host.mjs is what launches this suite`);
  }
  return value;
}

function under(parent: string, child: string): boolean {
  const step = relative(resolve(parent), resolve(child));
  return step.length > 0 && !step.startsWith("..") && !isAbsolute(step);
}

function same(a: string, b: string): boolean {
  return relative(resolve(a), resolve(b)) === "";
}

describe("the editor this suite runs in", () => {
  it("runs in a throwaway profile and never the developer's", () => {
    const cache = expected("NVS_HOST_CACHE");
    const workspace = expected("NVS_HOST_WORKSPACE");

    const appRoot = vscode.env.appRoot;
    assert.ok(under(cache, appRoot), `appRoot ${appRoot} is outside ${cache}: this is an installed editor`);

    // What `--extensions-dir` and `--disable-extensions` amount to, observed: the only extensions
    // loaded are this build's own built-ins and the one under test. Anything the developer installed
    // would be a third kind.
    const outsiders = vscode.extensions.all
      .filter((extension) => extension.id !== ID)
      .filter((extension) => !under(appRoot, extension.extensionPath))
      .map((extension) => extension.id);
    assert.deepEqual(outsiders, [], "an extension outside this build is loaded: the extensions directory is not the throwaway one");
    assert.ok(vscode.extensions.getExtension(ID), `${ID} is not loaded`);

    // The user-data directory has no API of its own, and needs none: the settings this run wrote into
    // the throwaway profile are the settings this editor reads back, which no other profile's are.
    assert.equal(vscode.workspace.getConfiguration("nvs").get<string>("path"), expected("NVS_HOST_NVS"));
    assert.equal(vscode.workspace.getConfiguration("security.workspace.trust").get<boolean>("enabled"), false);

    const folders = vscode.workspace.workspaceFolders ?? [];
    assert.equal(folders.length, 1, "the fixture copy is the whole workspace");
    assert.ok(same(workspace, folders[0].uri.fsPath),
              `the open folder is ${folders[0].uri.fsPath}, not the fixture copy at ${workspace}`);
  });
});
