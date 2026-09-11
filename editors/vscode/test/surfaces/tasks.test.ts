// The Tasks, and the Problems entries they produce.
//
// `rule:ide/tasks-carry-a-problem-matcher` is two claims, and this file holds both. The manifest
// contributes `nvs run` and `nvs test` as Tasks; each carries a `problemMatcher`, and that matcher
// is a regex over the renderer's own format (`rule:errors/renderings`) rather than a parser in the
// client. A matcher is exactly the kind of thing that is reviewed as obviously correct and then
// captures the wrong group, so it is run here against a real rendering: `recorded/check.txt` is
// what `nvs check` printed for `recorded/app.nvs`, byte for byte, and the assertions below are
// what the Problems panel would hold after it.
//
// Nothing here imports `vscode` — the headless tier has no such module (`scripts/headless.mjs`) —
// so what the client-side claims assert is the code that would run, as text, the way the
// contributions suite asserts a registered command.

import * as assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join, resolve } from "node:path";

const ROOT = resolve(__dirname, "..", "..", "..");

interface Pattern {
  regexp: string;
  file?: number;
  line?: number;
  column?: number;
  severity?: number;
  code?: number;
  message?: number;
}

interface Matcher {
  name: string;
  label?: string;
  owner: string;
  source?: string;
  severity?: string;
  fileLocation?: string[];
  pattern: Pattern[];
}

interface TaskDefinition {
  type: string;
  required?: string[];
  properties: Record<string, { type: string; enum?: string[] }>;
}

interface Manifest {
  contributes: {
    taskDefinitions?: TaskDefinition[];
    problemMatchers?: Matcher[];
  };
}

const manifest = JSON.parse(readFileSync(join(ROOT, "package.json"), "utf8")) as Manifest;
const definitions = manifest.contributes.taskDefinitions ?? [];
const matchers = manifest.contributes.problemMatchers ?? [];

const CLIENT = readFileSync(join(ROOT, "src", "tasks.ts"), "utf8");

// The recording, and the program it was taken from. Both are read as lines because a matcher runs
// over a terminal a line at a time, and because the line endings a checkout hands back are the
// platform's rather than the recording's.
const RECORDED = lines(readFileSync(join(ROOT, "test", "surfaces", "recorded", "check.txt"), "utf8"));
const PROGRAM = lines(readFileSync(join(ROOT, "test", "surfaces", "recorded", "app.nvs"), "utf8"));

function lines(text: string): string[] {
  return text.replace(/\r\n/g, "\n").replace(/\n$/, "").split("\n");
}

/** One entry as the Problems panel would hold it, with every field the matcher captured. */
interface Problem {
  severity: string;
  code: string;
  message: string;
  file: string;
  line: number;
  column: number;
}

/**
 * The recorded output, run through the contributed matcher the way the editor runs it.
 *
 * A multi-line pattern matches over consecutive lines: the first line has to match `pattern[0]`
 * and the one after it `pattern[1]`, and a first line whose successor does not match produces
 * nothing. That is the whole of the algorithm this matcher needs, and writing it out is what lets
 * the assertions below be about the patterns rather than about a mock.
 */
function collect(matcher: Matcher, output: string[]): Problem[] {
  const [head, location] = matcher.pattern.map((p) => new RegExp(p.regexp));
  const [first, second] = matcher.pattern;
  const found: Problem[] = [];
  for (let i = 0; i + 1 < output.length; i += 1) {
    const start = head.exec(output[i]);
    const where = location.exec(output[i + 1]);
    if (start === null || where === null) {
      continue;
    }
    found.push({
      severity: start[first.severity ?? 0],
      code: start[first.code ?? 0],
      message: start[first.message ?? 0],
      file: where[second.file ?? 0],
      line: Number(where[second.line ?? 0]),
      column: Number(where[second.column ?? 0]),
    });
    i += 1;
  }
  return found;
}

describe("the Tasks the manifest contributes", () => {
  it("declares one task type, taking a subcommand and the path it runs on", () => {
    // Both subcommands take a path — `nvs run <FILE>` and `nvs test <PATHS>...` — so a definition
    // that made `file` optional would describe a task that cannot run.
    assert.equal(definitions.length, 1);
    const definition = definitions[0];
    assert.equal(definition.type, "nvs");
    assert.deepEqual(definition.required, ["command", "file"]);
    assert.deepEqual(definition.properties["command"].enum, ["run", "test"]);
    assert.equal(definition.properties["file"].type, "string");
    assert.equal(definition.properties["args"].type, "array");
  });

  it("starts each Task with the matcher attached, and spawns nothing beside it", () => {
    // `nvs.run` and `nvs.test` are entry points onto the Task rather than a second way to start a
    // process, which is what keeps one terminal, one re-run and one set of Problems entries. A
    // `child_process` here would be that second way.
    assert.ok(/new Task\(/.test(CLIENT), "the client builds no Task");
    assert.ok(CLIENT.includes('"$nvs"') || /MATCHER = "\$nvs"/.test(CLIENT),
              "the Task names no problem matcher");
    assert.ok(/tasks\.executeTask\(/.test(CLIENT), "nothing executes the Task");
    assert.ok(/tasks\.registerTaskProvider\(/.test(CLIENT), "a tasks.json entry would not resolve");
    assert.equal(/child_process|require\("node:child_process"\)/.test(CLIENT), false,
                 "the client spawns nvs itself, beside the Task");
  });

  it("runs nvs with colour off, which is the rendering the matcher is written against", () => {
    // The renderer colours when the stream is a terminal and `NO_COLOR` is unset
    // (`crates/nvs-cli/src/main.rs`), and a Task's terminal is one. The recording below is the
    // uncoloured rendering, so the Task has to ask for that one.
    assert.ok(CLIENT.includes("NO_COLOR"), "the Task does not turn the renderer's colour off");
  });
});

describe("the problem matcher over a recorded rendering", () => {
  const matcher = matchers[0];

  it("contributes one matcher, owned by nvs, over two lines", () => {
    assert.equal(matchers.length, 1);
    // `$nvs` is how a `tasks.json` entry names it; the name is the identifier, and
    // `rule:ide/contributions-are-frozen-and-only-ever-added` covers it like every other one.
    assert.equal(matcher.name, "nvs");
    assert.equal(matcher.owner, "nvs");
    assert.equal(matcher.pattern.length, 2);
    // Relative in the rendering, absolute when the file was given absolutely: the editor tries the
    // path as it stands and falls back to the workspace folder.
    assert.deepEqual(matcher.fileLocation, ["autoDetect", "${workspaceFolder}"]);
  });

  it("finds every diagnostic in the recording, and nothing else", () => {
    const found = collect(matcher, RECORDED);
    assert.deepEqual(found, [
      {
        severity: "error",
        code: "E0215",
        message: "a function must be a method",
        file: "test/surfaces/recorded/app.nvs",
        line: 5,
        column: 1,
      },
      {
        severity: "error",
        code: "E0301",
        message: "`$total` is not declared",
        file: "test/surfaces/recorded/app.nvs",
        line: 9,
        column: 6,
      },
      {
        severity: "error",
        code: "E0301",
        message: "`$missing` is not declared",
        file: "test/surfaces/recorded/app.nvs",
        line: 11,
        column: 6,
      },
    ]);
  });

  it("matches the -> line however wide the gutter got", () => {
    // The renderer indents `-->` to the width of the widest line number in the diagnostic, so the
    // third entry above is indented one column further than the first two. A pattern anchored on a
    // fixed two spaces silently loses every diagnostic past line 9 of a file.
    assert.ok(RECORDED.includes("   --> test/surfaces/recorded/app.nvs:11:6"),
              "the recording no longer holds a widened gutter; re-record it against a longer file");
  });

  it("starts no entry on a line that carries no location", () => {
    // `error: aborting due to 3 errors` is the summary, and `= help:` is a note inside a
    // diagnostic already reported. Either one becoming an entry would put a Problems row on
    // whatever file happened to be named two lines earlier.
    const head = new RegExp(matcher.pattern[0].regexp);
    for (const line of RECORDED.filter((l) => !l.startsWith("error["))) {
      assert.equal(head.test(line), false, `${line} starts a Problems entry`);
    }
  });

  it("captures a location that is really in the file", () => {
    // The cheap way for a matcher to be wrong is to capture the right values into the wrong
    // groups, which every assertion above would still pass if `line` and `column` were swapped —
    // 5:1 and 9:6 read as plausible either way round. These are checked against the program the
    // recording was taken from: the line exists, and the column is inside it.
    for (const problem of collect(matcher, RECORDED)) {
      const text = PROGRAM[problem.line - 1];
      assert.ok(text !== undefined, `${problem.file}:${problem.line} is past the end of the file`);
      assert.ok(problem.column <= text.length,
                `column ${problem.column} is past the end of line ${problem.line}`);
    }
    // And the one that is not symmetric: `$total` starts at column 6 of line 9.
    assert.equal(PROGRAM[8].slice(5, 11), "$total");
  });
});
