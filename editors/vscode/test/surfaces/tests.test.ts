// The Test Explorer, and the two documents it is fed from.
//
// `rule:ide/the-extension-builds-no-ui-the-editor-already-has` is the claim: the client fills a
// `TestController` and VS Code draws everything. What can be asserted here is therefore the pair of
// decisions the client actually makes — what a listing becomes, and what a verdict reads as — plus
// the shape of the code that makes them, the way the AST suite asserts the panel's shell-out.
//
// `recorded/list.json` and `recorded/report.json` are what `nvs test --list --format=json` and `nvs
// test --format=json` printed for `recorded/suite.nvs`, byte for byte. Re-record both after any
// edit to that program.
//
// Nothing here imports `vscode` — the headless tier has no such module (`scripts/headless.mjs`) —
// so the decisions live in `src/report.ts`, which imports none either and is run directly.

import * as assert from "node:assert/strict";
import { readdirSync, readFileSync } from "node:fs";
import { join, resolve } from "node:path";

import {
  Listed,
  Reported,
  filter,
  listed,
  messages,
  name,
  ran,
  state,
  suites,
} from "../../src/report";

const ROOT = resolve(__dirname, "..", "..", "..");

const recorded = (file: string): string =>
  readFileSync(join(ROOT, "test", "surfaces", "recorded", file), "utf8");

const LIST = recorded("list.json");
const REPORT = recorded("report.json");

// The client with its prose removed, for the same reason the AST suite strips it: a comment naming
// a flag would otherwise answer an assertion about whether the flag is passed.
//
// Comment *lines* go, rather than a `/* … */` region: this file's globs are `"**/*.nvs"`, and a
// regex hunting a block comment finds a `/*` two characters into that string and eats everything up
// to the next `*/`, which is inside the next glob. Every block comment in this package is a doc
// comment on its own lines, so dropping lines that open one, continue one or close one is both
// exact and blind to what a string literal happens to contain.
const CLIENT = readFileSync(join(ROOT, "src", "tests.ts"), "utf8")
  .split("\n")
  .filter((line) => !/^\s*(\/\/|\/\*|\*)/.test(line))
  .join("\n");

// Every module and not this one alone, because what must not appear twice is the controller, and a
// second one would most likely arrive in the file that wanted it.
const EVERY_MODULE = readdirSync(join(ROOT, "src"))
  .filter((file) => file.endsWith(".ts"))
  .map((file) => readFileSync(join(ROOT, "src", file), "utf8"))
  .join("\n");

/** A record the recordings do not hold, for the two verdicts a green corpus never produces. */
const synthetic = (verdict: Reported["verdict"], rest: Partial<Reported> = {}): Reported => ({
  class: "GreetingTest",
  method: "aGreetingIsItsOwnName",
  file: null,
  line: null,
  column: null,
  verdict,
  durationMs: 1,
  ...rest,
});

describe("the test explorer", () => {
  it("populates the tree from the listing before anything runs", () => {
    const tests = listed(LIST);
    assert.ok(tests !== undefined, "the recorded listing is not a document this client reads");
    // One entry per `#[Test]` call, in the order the compiler's own table holds them.
    assert.deepEqual(tests.map(name), [
      "CountingTest::aSkippedCaseCarriesItsReason",
      "CountingTest::aCountIsWhatWasCounted",
      "GreetingTest::aGreetingIsItsOwnName",
      "GreetingTest::aWrongGreetingSaysWhatItGot",
    ]);
    // The class is the grouping, and it is the only one the document offers.
    assert.deepEqual(suites(tests).map((suite) => suite.class), ["CountingTest", "GreetingTest"]);
    assert.deepEqual(suites(tests).map((suite) => suite.tests.length), [2, 2]);
    // Version 2's reason for existing: a leaf without this is a test the editor cannot place.
    assert.deepEqual(
      tests.map((test) => [test.file, test.line, test.column]),
      [
        ["editors/vscode/test/surfaces/recorded/suite.nvs", 23, 21],
        ["editors/vscode/test/surfaces/recorded/suite.nvs", 28, 21],
        ["editors/vscode/test/surfaces/recorded/suite.nvs", 11, 21],
        ["editors/vscode/test/surfaces/recorded/suite.nvs", 16, 21],
      ],
    );

    // Discovery is the listing mode and nothing else. `nvs test` without `--list` would run every
    // test in the workspace the moment somebody opened the Testing view.
    assert.ok(/"test", "--list", "--format=json"/.test(CLIENT),
              "discovery does not ask for the listing");
    // A run document read as a listing would be a tree built out of a run that already happened,
    // which is what the two documents' different keys exist to prevent.
    assert.equal(listed(REPORT), undefined);
    assert.equal(ran(LIST), undefined);
    assert.equal(listed("not json"), undefined);
    assert.equal(listed('{"schemaVersion": 1, "listed": []}'), undefined);
  });

  it("runs one case and reads its verdict and failure back", () => {
    const report = ran(REPORT);
    assert.ok(report !== undefined, "the recorded report is not a document this client reads");
    // The runner's own count, not one this client added up.
    assert.deepEqual(report.summary, {
      total: 4,
      passed: 2,
      failed: 1,
      skipped: 1,
      flaky: 0,
      durationMs: 0.83,
    });

    const failed = report.tests.find((test) => test.method === "aWrongGreetingSaysWhatItGot");
    assert.ok(failed, "the recording lost the failing test; re-record");
    assert.equal(state(failed), "failed");
    assert.deepEqual(messages(failed), [
      'Core\\Test::assertSame failed: `$actual` is "hello", `$expected` is "goodbye"',
    ]);

    const skipped = report.tests.find((test) => test.verdict === "skipped");
    assert.equal(state(skipped as Reported), "skipped");
    assert.equal(skipped?.reason, "nothing counts yet");
    assert.deepEqual(messages(skipped as Reported), []);
    const passed = report.tests.find((test) => test.method === "aCountIsWhatWasCounted");
    assert.equal(state(passed as Reported), "passed");

    // The two verdicts the editor has no state of its own for. A flaky test is never reported as
    // green (`rule:testing/test-attribute` § 20), and a test that called `exit` returned no verdict
    // at all.
    const flaky = synthetic("flaky", { attempts: 2, failures: ["it flaked"] });
    assert.equal(state(flaky), "failed");
    assert.deepEqual(messages(flaky), ["it passed on attempt 2, having failed with:", "it flaked"]);
    const exited = synthetic("exited", { exitCode: 3 });
    assert.equal(state(exited), "errored");
    assert.deepEqual(messages(exited), ["the test called exit(3)"]);

    // One flag reaches one run, so only three requests can be narrowed and the rest run the file
    // whole and report what was asked for.
    const tests = listed(LIST) as Listed[];
    assert.equal(filter(tests, tests), undefined);
    assert.equal(filter([tests[2]], tests), "GreetingTest::aGreetingIsItsOwnName");
    assert.equal(filter(tests.slice(0, 2), tests), "CountingTest::");
    assert.equal(filter([tests[0], tests[2]], tests), undefined);
  });

  it("holds the .nvst corpus as a second suite in the same controller", () => {
    // `rule:testing/nvst-is-separate`: two suites, one controller. A second controller would be a
    // second "Novis" root in the Testing view for one binary's two ways of being asked.
    assert.equal(EVERY_MODULE.match(/createTestController\(/g)?.length, 1);
    assert.ok(/"\*\*\/\*\.nvst"/.test(CLIENT), "the corpus is never discovered");
    assert.ok(/CORPUS = "nvst"/.test(CLIENT), "the corpus has no item of its own to hang under");

    // The corpus leg asks for no document, because there is none: `nvs test` refuses `--format`
    // over a `.nvst` tree (`crates/nvs-cli/src/main.rs`), so the exit status is the verdict and
    // the case's own output is the message.
    const leg = /async function corpus\([\s\S]*?\n}/.exec(CLIENT)?.[0] ?? "";
    assert.ok(leg.length > 0, "the corpus leg has moved; this assertion no longer reads it");
    assert.equal(/--format|--list/.test(leg), false,
                 "the corpus leg asks for a document nvs test refuses to write");
    assert.ok(/code === 0/.test(leg), "the corpus leg does not read the exit status");

    // No UI of this project's, the claim the whole file rests on. Coverage is `test/coverage/`'s.
    assert.equal(/[Ww]ebview|<html|innerHTML/.test(CLIENT), false,
                 "the explorer draws UI of its own");
  });
});
