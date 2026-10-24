// What a changed setting does to the running server, held without an editor.
//
// `src/settings.ts` imports no `vscode`, so it runs in the headless tier the unattended loop gates on
// (`rule:ide/headless-gates-the-loop-the-host-run-gates-the-milestone`). What is pinned is that only
// `nvs.path` and `nvs.lsp.enable` restart the client, that every other `nvs.*` change is sent to the
// running server, that a change outside the section does nothing, and that the section is sent in
// the shape `initialize` sends.

import * as assert from "node:assert/strict";

import { RESPAWNING_SETTINGS, changeParams, reaction } from "../../src/settings";

// The editor's `affectsConfiguration` for a change to `changed`: a setting is affected when it is
// the changed one or a section that holds it.
function affecting(...changed: string[]): (setting: string) => boolean {
  return (setting) => changed.some((name) => name === setting || name.startsWith(`${setting}.`));
}

describe("a changed setting", () => {
  it("restarts the client for the binary and for whether a server runs", () => {
    assert.deepEqual(RESPAWNING_SETTINGS, ["nvs.path", "nvs.lsp.enable"]);
    assert.equal(reaction(affecting("nvs.path")), "restart");
    assert.equal(reaction(affecting("nvs.lsp.enable")), "restart");
  });

  it("is sent to the running server for every other nvs setting", () => {
    for (const setting of ["nvs.check.scope", "nvs.codeLens.enable", "nvs.lsp.debounce"]) {
      assert.equal(reaction(affecting(setting)), "send", setting);
    }
  });

  it("restarts when one change holds both kinds", () => {
    assert.equal(reaction(affecting("nvs.check.scope", "nvs.path")), "restart");
  });

  it("does nothing outside the nvs section", () => {
    assert.equal(reaction(affecting("editor.fontSize")), "none");
  });
});

describe("the section a change sends", () => {
  it("is the whole section, nested as the editor holds it", () => {
    const section = { check: { scope: "open" }, codeLens: { enable: false }, lsp: { debounce: 0 } };
    assert.deepEqual(changeParams(section, "/storage/stubs/1.0.0"), {
      settings: { ...section, stubs: { dir: "/storage/stubs/1.0.0" } },
    });
  });

  it("keeps a stub directory the user wrote", () => {
    const sent = changeParams({ stubs: { dir: "/mine" } }, "/storage/stubs/1.0.0");
    assert.deepEqual(sent.settings.stubs, { dir: "/mine" });
  });

  it("is an object with the stub directory when the editor holds no section", () => {
    assert.deepEqual(changeParams(undefined, "/storage/stubs/1.0.0"), {
      settings: { stubs: { dir: "/storage/stubs/1.0.0" } },
    });
  });
});
