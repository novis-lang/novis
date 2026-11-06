// What a file's badge is, held without an editor.
//
// `src/kinds.ts` imports no `vscode`, so it runs in the headless tier the unattended loop gates on
// (`rule:ide/headless-gates-the-loop-the-host-run-gates-the-milestone`). What is pinned is that each
// of the four kinds has its own letter and no colour, that a word a newer server might send is not a
// kind, that a file with routes shows `R` and names its routes, that one file is one key however its
// URI was spelled, and that only the files whose badge changed between two answers are redrawn.

import * as assert from "node:assert/strict";

import { BADGES, Badge, METHOD, NAMED, ROUTES, badgeOf, changed, isKind, key } from "../../src/kinds";

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
    for (const word of ["trait", "toString", "constructor", "", undefined]) {
      assert.equal(isKind(word), false, String(word));
    }
  });
});

describe("a file with routes", () => {
  it("shows its own letter, and names the class and each route", () => {
    const badge = badgeOf({ uri: "file:///a", kind: "class", routes: ["GET /posts", "POST /posts/{id}"] });
    assert.deepEqual(badge, {
      letter: ROUTES,
      tooltip: "This file declares a class with 2 routes: GET /posts, POST /posts/{id}.",
    });
  });

  it("counts one route as one", () => {
    assert.equal(badgeOf({ uri: "file:///a", routes: ["GET /"] })?.tooltip, "This file has 1 route: GET /.");
  });

  it("names the first routes and counts the others", () => {
    const routes = Array.from({ length: NAMED + 2 }, (_, at) => `GET /${at}`);
    assert.equal(
      badgeOf({ uri: "file:///a", kind: "class", routes })?.tooltip,
      "This file declares a class with 7 routes: GET /0, GET /1, GET /2, GET /3, GET /4 and 2 more.",
    );
  });

  it("shows the kind's letter when it has none, and nothing for a word this client does not know", () => {
    assert.equal(badgeOf({ uri: "file:///a", kind: "enum", routes: [] }), BADGES.enum);
    assert.equal(badgeOf({ uri: "file:///a", kind: "trait" }), undefined);
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
  it("are the files added, removed or with another badge, and no other", () => {
    const routed = (path: string): Badge | undefined => badgeOf({ uri: "file:///a", kind: "class", routes: [path] });
    const before = new Map<string, Badge | undefined>([
      ["a", BADGES.class],
      ["b", BADGES.enum],
      ["c", BADGES.type],
      ["e", routed("/posts")],
      ["f", routed("/posts")],
    ]);
    const after = new Map<string, Badge | undefined>([
      ["a", { ...BADGES.class }],
      ["b", BADGES.interface],
      ["d", BADGES.enum],
      ["e", routed("/posts")],
      ["f", routed("/pages")],
    ]);
    const defined = (map: Map<string, Badge | undefined>): Map<string, Badge> =>
      new Map([...map].flatMap(([file, badge]) => (badge === undefined ? [] : [[file, badge]])));
    assert.deepEqual(changed(defined(before), defined(after)).sort(), ["b", "c", "d", "f"]);
  });

  it("are none when the answer is the same", () => {
    const answer = new Map<string, Badge>([["a", BADGES.class]]);
    assert.deepEqual(changed(answer, new Map(answer)), []);
  });
});
