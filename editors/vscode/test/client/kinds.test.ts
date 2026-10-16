// What a file's badge is, held without an editor.
//
// `src/kinds.ts` imports no `vscode`, so it runs in the headless tier the unattended loop gates on
// (`rule:ide/headless-gates-the-loop-the-host-run-gates-the-milestone`). What is pinned is that each
// of the four kinds has its own letter and no colour, that a word a newer server might send is not a
// kind, that one file is one key however its URI was spelled, and that only the files whose kind
// changed between two answers are redrawn.

import * as assert from "node:assert/strict";

import { BADGES, Kind, METHOD, changed, isKind, key } from "../../src/kinds";

describe("a file's badge", () => {
  it("is asked for under the server's own method", () => {
    assert.equal(METHOD, "nvs/fileKinds");
  });

  it("has its own letter for each of the four kinds", () => {
    const letters = Object.values(BADGES).map((badge) => badge.letter);
    assert.deepEqual(letters, ["C", "I", "E", "T"]);
  });

  it("carries no colour, so the file's name keeps its own", () => {
    for (const badge of Object.values(BADGES)) {
      assert.deepEqual(Object.keys(badge).sort(), ["letter", "tooltip"]);
    }
  });

  it("knows the four words and no other", () => {
    for (const word of ["class", "interface", "enum", "type"]) {
      assert.equal(isKind(word), true, word);
    }
    for (const word of ["trait", "toString", "constructor", ""]) {
      assert.equal(isKind(word), false, word);
    }
  });
});

describe("the key a file is looked up by", () => {
  it("ignores case on Windows, where the server and the editor spell the drive letter differently", () => {
    assert.equal(key("E:\\app\\src\\Post.nvs", "win32"), key("e:\\app\\src\\Post.nvs", "win32"));
  });

  it("keeps case elsewhere", () => {
    assert.notEqual(key("/app/src/Post.nvs", "linux"), key("/app/src/post.nvs", "linux"));
  });
});

describe("the files redrawn after an answer", () => {
  it("are the files added, removed or of another kind, and no other", () => {
    const before = new Map<string, Kind>([["a", "class"], ["b", "enum"], ["c", "type"]]);
    const after = new Map<string, Kind>([["a", "class"], ["b", "interface"], ["d", "enum"]]);
    assert.deepEqual(changed(before, after).sort(), ["b", "c", "d"]);
  });

  it("are none when the answer is the same", () => {
    const answer = new Map<string, Kind>([["a", "class"]]);
    assert.deepEqual(changed(answer, new Map(answer)), []);
  });
});
