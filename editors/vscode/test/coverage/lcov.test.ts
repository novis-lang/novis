// The Coverage run profile, and the lcov file it reads into the editor's coverage view.
//
// `rule:ide/the-extension-builds-no-ui-the-editor-already-has` is the claim: the client turns the
// tracefile `nvs test --coverage-lcov` wrote into `FileCoverage` details and VS Code draws the
// gutter and the summary. What can be asserted headless is what a tracefile becomes, in
// `src/report.ts`, and the shape of the code in `src/tests.ts` that runs the profile.
//
// `recorded/branches.lcov` and `recorded/report.json` are what `nvs test --format=json
// --coverage-lcov <file>` wrote and printed for `recorded/branches.nvs`, run from the repository
// root. Re-record both after any edit to that program.

import * as assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join, resolve } from "node:path";

import { listed, ran, statements, traced } from "../../src/report";

const ROOT = resolve(__dirname, "..", "..", "..");

const recorded = (file: string): string =>
  readFileSync(join(ROOT, "test", "coverage", "recorded", file), "utf8");

const TRACEFILE = recorded("branches.lcov");
const REPORT = recorded("report.json");

// The client with its comment lines removed, for the reason `test/surfaces/tests.test.ts` gives.
const CLIENT = readFileSync(join(ROOT, "src", "tests.ts"), "utf8")
  .split("\n")
  .filter((line) => !/^\s*(\/\/|\/\*|\*)/.test(line))
  .join("\n");

describe("the coverage view", () => {
  it("reads lines, functions and branches out of the recorded tracefile", () => {
    const files = traced(TRACEFILE);
    assert.ok(files !== undefined, "the recorded tracefile is not one this client reads");
    assert.deepEqual(files.map((file) => file.file), [
      "editors/vscode/test/coverage/recorded/branches.nvs",
    ]);
    const [file] = files;
    assert.deepEqual(file.lines, [
      { line: 9, count: 1 },
      { line: 10, count: 0 },
      { line: 12, count: 1 },
      { line: 16, count: 0 },
      { line: 23, count: 1 },
    ]);
    // A function's count is its `FNDA`, met with its `FN` by name.
    assert.deepEqual(file.functions, [
      { name: "Sign::of", line: 9, count: 1 },
      { name: "Sign::unused", line: 16, count: 0 },
      { name: "SignTest::aPositiveNumberIsNotNegative", line: 23, count: 1 },
    ]);
    // Side 0 is the true side. The test passes 3, so only the false side ran.
    assert.deepEqual(file.branches, [
      { line: 9, block: 0, side: 0, count: 0 },
      { line: 9, block: 0, side: 1, count: 1 },
    ]);

    // What the editor is given: every line once, with the branch sides on the line they start on.
    assert.deepEqual(statements(file), [
      {
        line: 9,
        count: 1,
        branches: [
          { count: 0, label: "true" },
          { count: 1, label: "false" },
        ],
      },
      { line: 10, count: 0, branches: [] },
      { line: 12, count: 1, branches: [] },
      { line: 16, count: 0, branches: [] },
      { line: 23, count: 1, branches: [] },
    ]);
  });

  it("reads the branch forms the recording does not have, and refuses what is not lcov", () => {
    const [file] = traced(
      [
        "TN:",
        "SF:/elsewhere/a.nvs",
        "FN:2,A::with,comma",
        "FNDA:4,A::with,comma",
        "BRDA:3,0,0,-",
        "BRDA:3,0,1,-",
        "BRDA:5,0,0,2",
        "BRDA:5,0,1,1",
        "BRDA:5,1,0,0",
        "BRDA:5,1,1,2",
        "BRDA:7,0,0,1",
        "BRDA:7,0,1,3",
        "DA:3,0",
        "DA:5,3",
        "end_of_record",
      ].join("\r\n"),
    ) ?? [];
    assert.equal(file.file, "/elsewhere/a.nvs");
    assert.deepEqual(file.functions, [{ name: "A::with,comma", line: 2, count: 4 }]);
    assert.deepEqual(statements(file), [
      // `-` is a branch whose line never ran: both sides read 0.
      {
        line: 3,
        count: 0,
        branches: [
          { count: 0, label: "true" },
          { count: 0, label: "false" },
        ],
      },
      // Two conditions on one line are numbered, so the two `true`s can be told apart.
      {
        line: 5,
        count: 3,
        branches: [
          { count: 2, label: "condition 1: true" },
          { count: 1, label: "condition 1: false" },
          { count: 0, label: "condition 2: true" },
          { count: 2, label: "condition 2: false" },
        ],
      },
      // A condition on a line no statement starts on: the line ran as often as the condition did.
      {
        line: 7,
        count: 4,
        branches: [
          { count: 1, label: "true" },
          { count: 3, label: "false" },
        ],
      },
    ]);

    assert.deepEqual(traced(""), []);
    assert.equal(traced("DA:1,1\n"), undefined, "a count with no file is not lcov");
    assert.equal(traced("SF:a.nvs\nDA:one,1\n"), undefined);
    assert.equal(traced("SF:a.nvs\nFNDA:1,Missing::name\n"), undefined);
  });

  it("reads the run document a coverage flag makes, and nothing else changes", () => {
    const report = ran(REPORT);
    assert.ok(report !== undefined, "the recorded version 3 report is not one this client reads");
    assert.deepEqual(report.tests.map((test) => [test.verdict, test.coverage]), [
      ["passed", { "editors/vscode/test/coverage/recorded/branches.nvs": [9, 12, 23] }],
    ]);
    // A listing never carries coverage, so a version 3 one is not a listing.
    assert.equal(listed('{"schemaVersion": 3, "listed": []}'), undefined);
    assert.equal(ran('{"schemaVersion": 4, "summary": {}, "tests": []}'), undefined);
  });

  it("runs the Coverage profile through the CLI and hands the editor its own model", () => {
    assert.ok(/TestRunProfileKind\.Coverage/.test(CLIENT), "there is no Coverage profile");
    assert.ok(/"--coverage-lcov", tracefile/.test(CLIENT),
              "the Coverage profile does not ask nvs test for a tracefile");
    assert.ok(/traced\(/.test(CLIENT), "the tracefile is not read back");
    assert.ok(/FileCoverage\.fromDetails\(/.test(CLIENT),
              "the file totals are not the editor's own count of the details");
    assert.ok(/loadDetailedCoverage = /.test(CLIENT),
              "the editor cannot open a file's statements and branches");
    // Names are relative to where the run starts, so the run must start where they are resolved.
    assert.ok(/cli\(\[\.\.\.asked, "--coverage-lcov", tracefile\], folder\)/.test(CLIENT),
              "the coverage run does not start in the folder its names are resolved against");
    assert.ok(/resolve\(folder, trace\.file\)/.test(CLIENT));
    // The scratch tracefile is in the extension's own storage, and is deleted whatever the run did.
    assert.ok(/context\.storageUri \?\? context\.globalStorageUri/.test(CLIENT));
    assert.equal(/tmpdir\(/.test(CLIENT), false, "the tracefile is written outside the storage");
    assert.ok(/finally \{\s*await rm\(scratch/.test(CLIENT), "the scratch tracefile is left behind");
    // VS Code draws the gutter: no decoration of this project's.
    assert.equal(/createTextEditorDecorationType|[Ww]ebview/.test(CLIENT), false,
                 "the coverage view draws UI of its own");
  });
});
