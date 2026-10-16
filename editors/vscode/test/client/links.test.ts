// What a link to a directory turns into, held without an editor.
//
// `src/links.ts` imports no `vscode`, so it runs in the headless tier the unattended loop gates on
// (`rule:ide/headless-gates-the-loop-the-host-run-gates-the-milestone`). What is pinned is the one
// promise the client makes about a link: a directory, which the server sends as a `file:` URI ending
// in `/`, opens in the Explorer, and a file opens as the server sent it.

import * as assert from "node:assert/strict";

import { REVEAL, Target, revealing } from "../../src/links";

/** A stand-in for `vscode.Uri` with the two fields the module reads. */
function target(scheme: string, path: string): Target {
  return {
    scheme,
    path,
    with(change: { path: string }): Target {
      return target(scheme, change.path);
    },
  };
}

describe("a link to a directory", () => {
  it("reveals the directory, without its trailing slash, in the Explorer", () => {
    const command = revealing(target("file", "/srv/app/src/"));
    assert.ok(command !== undefined);
    const [name, query] = command.split("?");
    assert.equal(name, `command:${REVEAL}`);
    const args: unknown = JSON.parse(decodeURIComponent(query));
    assert.deepEqual(args, [{ scheme: "file", path: "/srv/app/src" }]);
  });

  it("leaves a link to a file as the server sent it", () => {
    assert.equal(revealing(target("file", "/srv/app/lib/User.nvs")), undefined);
  });

  it("leaves a target under another scheme alone", () => {
    assert.equal(revealing(target("https", "/docs/")), undefined);
  });
});
