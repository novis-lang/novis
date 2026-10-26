// The report a coverage run keeps for the server's "used by" lens, and the click that runs the tests
// the lens names.
//
// `rule:ide/used-by-counts-the-routes-and-tests-that-reach-a-method` is the claim. The server reads
// each program's last coverage report out of the directory `initializationOptions` names as
// `coverage.dir`, and runs `nvs.runTests` when a lens is clicked. What can be asserted headless is
// what a report becomes before it is written (`src/report.ts`), the section the server is started
// with (`src/stubs.ts`), and, as source text, that the client registers the command and lists it.
//
// `recorded/report.json` is the recording `lcov.test.ts` describes.

import * as assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join, resolve } from "node:path";

import { kept, keptName, ran } from "../../src/report";
import { coverageDirectory, withCoverage, withStubs } from "../../src/stubs";

const ROOT = resolve(__dirname, "..", "..", "..");

const REPORT = readFileSync(join(ROOT, "test", "coverage", "recorded", "report.json"), "utf8");

// A client file with its comment lines removed, for the reason `test/surfaces/tests.test.ts` gives.
const source = (file: string): string =>
  readFileSync(join(ROOT, "src", file), "utf8")
    .split("\n")
    .filter((line) => !/^\s*(\/\/|\/\*|\*)/.test(line))
    .join("\n");

const FOLDER = resolve("workspace");

describe("the kept report", () => {
  it("names every reached file absolute, against the folder the run started in", () => {
    const written = kept(REPORT, FOLDER);
    assert.ok(written !== undefined, "the recorded coverage report is not kept");
    const document = JSON.parse(written);
    assert.equal(document.schemaVersion, 3);
    assert.deepEqual(document.tests[0].coverage, {
      [resolve(FOLDER, "editors/vscode/test/coverage/recorded/branches.nvs")]: [9, 12, 23],
    });
    // Everything else is the run's own report, which this client still reads as one.
    assert.equal(document.tests[0].method, "aPositiveNumberIsNotNegative");
    assert.ok(ran(written) !== undefined);
  });

  it("keeps a name that is already absolute", () => {
    const elsewhere = resolve("elsewhere", "a.nvs");
    const text = JSON.stringify({
      schemaVersion: 3,
      summary: {},
      tests: [{ class: "A", method: "b", coverage: { [elsewhere]: [1] } }],
    });
    assert.deepEqual(JSON.parse(kept(text, FOLDER) ?? "{}").tests[0].coverage, {
      [elsewhere]: [1],
    });
  });

  it("is not kept from a run without coverage, a listing or what is not JSON", () => {
    assert.equal(kept(JSON.stringify({ schemaVersion: 2, summary: {}, tests: [] }), FOLDER), undefined);
    assert.equal(kept(JSON.stringify({ schemaVersion: 3, summary: {} }), FOLDER), undefined);
    assert.equal(kept("not json", FOLDER), undefined);
    assert.equal(kept("[]", FOLDER), undefined);
  });

  it("is written to one file per program, the same on every run", () => {
    const program = resolve("workspace", "shop.nvs");
    assert.match(keptName(program), /^[0-9a-f]{16}\.json$/);
    assert.equal(keptName(program), keptName(program));
    assert.notEqual(keptName(program), keptName(resolve("workspace", "blog.nvs")));
  });
});

describe("the coverage directory", () => {
  it("is under the workspace's storage", () => {
    assert.equal(coverageDirectory(join("home", "storage")), join("home", "storage", "coverage"));
  });

  it("is always named to the server, beside the stub directory", () => {
    const section = withCoverage(withStubs({ check: { scope: "open" } }, "stubs"), "reports");
    assert.deepEqual(section, {
      check: { scope: "open" },
      stubs: { dir: "stubs" },
      coverage: { dir: "reports" },
    });
    assert.deepEqual(withCoverage({ coverage: { dir: "/theirs" } }, "reports"), {
      coverage: { dir: "reports" },
    });
    assert.deepEqual(withCoverage(undefined, "reports"), { coverage: { dir: "reports" } });
  });
});

describe("the lens click", () => {
  it("is a command the client registers and lists to the server", () => {
    assert.match(source("tests.ts"), /registerCommand\(\s*"nvs\.runTests"/);
    const listed = /const EDITOR_COMMANDS = \[([^\]]*)\]/.exec(source("extension.ts"));
    assert.ok(listed !== null, "extension.ts lists no editor commands");
    assert.match(listed[1], /"nvs\.runTests"/);
  });

  it("runs through the Run profile, and a coverage run keeps its report", () => {
    const client = source("tests.ts");
    assert.match(client, /new TestRunRequest\(matched, undefined, profile\)/);
    assert.match(client, /await keep\(uri, folder, scratch, document\?\.stdout\)/);
    assert.match(client, /await rename\(partial, join\(reports, name\)\)/);
    assert.match(source("extension.ts"), /initializationOptions: withCoverage\(/);
  });
});
