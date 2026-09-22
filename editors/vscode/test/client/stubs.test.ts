// The stub directory this client names, held without an editor.
//
// `src/stubs.ts` imports no `vscode`, so it runs in the headless tier the unattended loop gates on
// (`rule:ide/headless-gates-the-loop-the-host-run-gates-the-milestone`). What is pinned is the one
// promise the setting makes: a directory the user wrote reaches the server exactly, and a user who
// wrote none gets one inside this extension's own storage, per server version, with nothing else in
// the section changed (`rule:ide/the-stub-tree-is-where-core-is-declared`).

import * as assert from "node:assert/strict";
import { join } from "node:path";

import { stubsDirectory, withStubs } from "../../src/stubs";

const STORAGE = join("home", "storage");

describe("the stub directory", () => {
  it("is under the extension's storage, one per server version", () => {
    assert.equal(stubsDirectory(STORAGE, "0.4.2"), join(STORAGE, "stubs", "0.4.2"));
  });

  it("names a directory even for an extension with no version", () => {
    assert.equal(stubsDirectory(STORAGE, ""), join(STORAGE, "stubs", "unversioned"));
  });
});

describe("the forwarded section", () => {
  it("fills stubs.dir where the user set none", () => {
    const section = { check: { scope: "open" }, stubs: { dir: "" } };
    assert.deepEqual(withStubs(section, "fallback"), {
      check: { scope: "open" },
      stubs: { dir: "fallback" },
    });
  });

  it("keeps a directory the user wrote", () => {
    assert.deepEqual(withStubs({ stubs: { dir: "/theirs" } }, "fallback"), {
      stubs: { dir: "/theirs" },
    });
  });

  it("builds the section from nothing", () => {
    assert.deepEqual(withStubs(undefined, "fallback"), { stubs: { dir: "fallback" } });
  });

  it("does not change what it was handed", () => {
    const section = { stubs: { dir: "" }, codeLens: { enable: false } };
    withStubs(section, "fallback");
    assert.equal(section.stubs.dir, "");
    assert.equal(section.codeLens.enable, false);
  });
});
